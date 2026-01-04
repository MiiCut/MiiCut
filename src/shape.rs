// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use crate::{
    dom::Icons,
    inputs::UserUI,
    math::{
        bez_path_to_geo_polygon, bezpath_from_apices, snap_angle, snap_val, snap_vertex, EPSILON,
    },
    types::{EUId, SegBundle, VUId, Value, VecRing},
};
use geo::algorithm::bounding_rect::BoundingRect;
use geo::algorithm::contains::Contains;
use geo::algorithm::orient::Orient;
use geo::{orient::Direction, LineString, MultiPolygon, Point, Polygon};
use kurbo::{Arc, BezPath, Circle, PathEl, Shape, Vec2};
use std::{collections::HashSet, hash::Hash};
use std::{
    f64::consts::PI,
    fmt::{Debug, Display},
    vec,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextFont {
    Stencilia,
    Urbanist,
}
impl TextFont {
    fn data(&self) -> &'static [u8] {
        match self {
            TextFont::Stencilia => include_bytes!("../assets/stencilia/Stencilia-A.ttf"),
            TextFont::Urbanist => include_bytes!("../assets/urbanist/Urbanist-Variable.ttf"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextData {
    pub text: String,
    pub font: TextFont,
    pub scale: Option<f64>,
    pub auto_fit: bool,
    cached_text: String,
    cached_font: TextFont,
    cached_scale_override: Option<f64>,
    cached_auto_fit: bool,
    cached_bbox_min: Vec2,
    cached_bbox_max: Vec2,
    cached_polygon: Option<MultiPolygon<f64>>,
}
impl TextData {
    pub fn new(text: String, font: TextFont, scale: Option<f64>, auto_fit: bool) -> Self {
        Self {
            text,
            font,
            scale,
            auto_fit,
            cached_text: String::new(),
            cached_font: TextFont::Stencilia,
            cached_scale_override: None,
            cached_auto_fit: false,
            cached_bbox_min: Vec2::ZERO,
            cached_bbox_max: Vec2::ZERO,
            cached_polygon: None,
        }
    }
    fn invalidate_cache(&mut self) {
        self.cached_polygon = None;
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ClosedShape {
    shape_type: Icons,
    operation: Operation,
    vertices: VecRing<VUId>,

    bezpath: BezPath,
    polygon: MultiPolygon<f64>,
    text: Option<TextData>,
}
impl ClosedShape {
    const TOLERANCE: f64 = 0.01;
    const GRAB_RADIUS: f64 = 5.0;

    pub fn op_next(&mut self) {
        self.operation.next();
    }
    pub fn op_union(&mut self) {
        self.operation.union();
    }
    pub fn op_difference(&mut self) {
        self.operation.difference();
    }

    pub fn new(shape_type: Icons, vertices: &[Vec2]) -> Option<Self> {
        let mut vertices: Vec<Vec2> = vertices.iter().cloned().collect();
        if vertices.is_empty() {
            return None;
        }
        // Sanity check
        match shape_type {
            Icons::Arrow => {
                return None;
            }
            Icons::Disc => {
                if vertices.len() != 2 || vertices[0] == vertices[1] {
                    return None;
                }
            }
            Icons::Square => {
                if vertices.len() != 2 || vertices[0] == vertices[1] {
                    return None;
                } else {
                    let bl = vertices[0].clone();
                    let tr = vertices[1].clone();
                    let tl = Vec2::new(bl.x, tr.y);
                    let br = Vec2::new(tr.x, bl.y);
                    vertices = vec![bl, tl, tr, br];
                }
            }
            Icons::Oblong => {
                if vertices.len() != 2 || vertices[0] == vertices[1] {
                    return None;
                }
                let m = (vertices[0] + vertices[1]) * 0.5;
                let dir = (vertices[1] - vertices[0]).normalize();
                let side = m - Vec2::new(dir.y, -dir.x) * 20.;
                vertices = vec![vertices[0], vertices[1], side];
            }
            Icons::Poly => {
                if vertices.len() < 3 {
                    return None;
                }
            }
            Icons::Text => {
                return None;
            }
        }
        let vertices = vertices
            .iter()
            .map(|v| (VUId::new(), Value::new(v.clone())))
            .collect::<Vec<_>>();
        let vertices = &vertices[..];

        let mut shape = ClosedShape {
            shape_type,
            operation: Operation::Union,
            vertices: VecRing::from_slice(vertices).unwrap(),
            bezpath: BezPath::new(),
            polygon: MultiPolygon::new(vec![]),
            text: None,
        };
        shape.set_bezpath();
        Some(shape)
    }

    pub fn from_raw(
        shape_type: Icons,
        operation: Operation,
        vertices: Vec<Vec2>,
        rounded: &[Option<u32>],
        text: Option<TextData>,
    ) -> Option<Self> {
        match shape_type {
            Icons::Arrow => return None,
            Icons::Disc if vertices.len() != 2 => return None,
            Icons::Square if vertices.len() != 4 => return None,
            Icons::Oblong if vertices.len() != 3 => return None,
            Icons::Poly if vertices.len() < 3 => return None,
            Icons::Text if vertices.len() != 4 => return None,
            _ => {}
        }

        let vertices = vertices
            .iter()
            .map(|v| (VUId::new(), Value::new(*v)))
            .collect::<Vec<_>>();
        let mut shape = ClosedShape {
            shape_type,
            operation,
            vertices: VecRing::from_slice(&vertices[..]).unwrap(),
            bezpath: BezPath::new(),
            polygon: MultiPolygon::new(vec![]),
            text,
        };

        let count = rounded.len().min(shape.vertices.len());
        for idx in 0..count {
            if let Some(value) = rounded[idx] {
                shape.vertices.val_mut(idx as i64).rounded = Some(value);
            }
        }

        shape.set_bezpath();
        Some(shape)
    }

    pub fn new_text(text: String, font: TextFont, p1: Vec2, p2: Vec2) -> Option<Self> {
        if p1 == p2 {
            return None;
        }
        let bl = Vec2::new(p1.x.min(p2.x), p1.y.min(p2.y));
        let tr = Vec2::new(p1.x.max(p2.x), p1.y.max(p2.y));
        let tl = Vec2::new(bl.x, tr.y);
        let br = Vec2::new(tr.x, bl.y);
        let vertices = vec![bl, tl, tr, br]
            .iter()
            .map(|v| (VUId::new(), Value::new(*v)))
            .collect::<Vec<_>>();

        let mut shape = ClosedShape {
            shape_type: Icons::Text,
            operation: Operation::Union,
            vertices: VecRing::from_slice(&vertices[..]).unwrap(),
            bezpath: BezPath::new(),
            polygon: MultiPolygon::new(vec![]),
            text: Some(TextData::new(text, font, None, false)),
        };
        shape.set_bezpath();
        Some(shape)
    }

    pub fn get_vertex(&self, value_uid: &VUId) -> Option<&Value> {
        self.vertices.iter().find_map(
            |(uid, value)| {
                if uid == value_uid {
                    Some(value)
                } else {
                    None
                }
            },
        )
    }
    pub fn get_vertex_mut(&mut self, value_uid: &VUId) -> Option<&mut Value> {
        self.vertices.iter_mut().find_map(
            |(uid, value)| {
                if uid == value_uid {
                    Some(value)
                } else {
                    None
                }
            },
        )
    }
    pub fn get_vertices(&self) -> &VecRing<VUId> {
        &self.vertices
    }
    pub fn get_vertices_mut(&mut self) -> &mut VecRing<VUId> {
        &mut self.vertices
    }
    pub fn select_vertex(&mut self, draw_pos: Vec2) -> Option<VUId> {
        for (_idx, (uid, value)) in self.vertices.iter().enumerate() {
            if (value.curr - draw_pos).hypot() < Self::GRAB_RADIUS {
                return Some(*uid);
            }
        }
        return None;
    }
    pub fn highlight_vertex(&mut self, draw_pos: Vec2) -> Option<VUId> {
        for (uid, value) in self.vertices.iter() {
            if (value.curr - draw_pos).hypot() < Self::GRAB_RADIUS {
                return Some(*uid);
            }
        }
        return None;
    }
    pub fn move_vertex(&mut self, value_uid: VUId, user_ui: &UserUI) -> bool {
        let snap = user_ui.snap;
        let mut delta = user_ui.pointer.curr - user_ui.pointer.saved;
        delta = (delta / snap.linear()).round() * snap.linear();
        match self.shape_type {
            Icons::Disc => {
                if self.vertices.len() != 2 {
                    return false;
                }
                // The first vertex is the center
                // The second vertex is the radius
                if self.vertices.key(0) == &value_uid {
                    if let Some(seg) =
                        SegBundle::new(self.vertices.val(0).saved, self.vertices.val(1).saved)
                    {
                        // Snap the saved center
                        let snap_c = snap_vertex(seg.s, snap);
                        // Snap the radius relative to the saved center (keep same radius)
                        let snap_r = seg.e + snap_c - seg.s;
                        // Snap the angle
                        let r = (snap_r - snap_c).hypot();
                        let a = snap_angle((snap_r - snap_c).atan2(), snap);

                        self.vertices.val_mut(0).saved = snap_c;
                        self.vertices.val_mut(1).saved =
                            snap_c + Vec2::new(r * a.cos(), r * a.sin());

                        // Then move all
                        self.vertices.val_mut(0).add(delta);
                        self.vertices.val_mut(1).add(delta);
                        self.set_bezpath();
                        true
                    } else {
                        self.vertices.val_mut(0).add(delta);
                        self.set_bezpath();
                        true
                    }
                } else if self.vertices.key(1) == &value_uid {
                    self.vertices.val_mut(1).add(delta);
                    if let Some(seg) =
                        SegBundle::new(self.vertices.val(0).saved, self.vertices.val(1).curr)
                    {
                        let r = snap_val(seg.len, snap);
                        let a = snap_angle(seg.a, snap);
                        self.vertices.val_mut(1).curr = seg.s + Vec2::new(r * a.cos(), r * a.sin());
                    }
                    self.set_bezpath();
                    true
                } else {
                    false
                }
            }
            Icons::Oblong => {
                if self.vertices.len() != 3 {
                    return false;
                }
                let idx = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                    Some(i) => i,
                    None => return false,
                };
                // Save the side radius
                let r_saved = ((self.vertices.val(0).saved + self.vertices.val(1).saved) / 2.
                    - self.vertices.val(2).saved)
                    .hypot();

                // The side is moved, this doesn't change the pos of e1, e2
                if idx == 2 {
                    if let Some(seg) =
                        SegBundle::new(self.vertices.val(0).saved, self.vertices.val(1).saved)
                    {
                        let s_d = (self.vertices.val(2).saved - seg.m).dot(seg.n);
                        self.vertices.val_mut(2).curr =
                            seg.m + seg.n * snap_val(s_d + delta.dot(seg.n), snap);
                    } else {
                        self.vertices.val_mut(0).add(delta);
                    }
                } else {
                    // e1 is moved
                    if idx == 0 {
                        self.vertices.val_mut(0).saved =
                            snap_vertex(self.vertices.val(0).saved, snap);
                        self.vertices.val_mut(0).add(delta);
                        if let Some(seg) =
                            SegBundle::new(self.vertices.val(0).curr, self.vertices.val(1).saved)
                        {
                            let r = snap_val(seg.len, snap);
                            let a = snap_angle(seg.a, snap);
                            self.vertices.val_mut(0).curr =
                                snap_vertex(seg.e - Vec2::new(r * a.cos(), r * a.sin()), snap);
                        } else {
                            self.vertices.val_mut(0).add(delta);
                        }
                    } else {
                        // e2 is moved
                        self.vertices.val_mut(1).saved =
                            snap_vertex(self.vertices.val(1).saved, snap);
                        self.vertices.val_mut(1).add(delta);
                        if let Some(seg) =
                            SegBundle::new(self.vertices.val(0).saved, self.vertices.val(1).curr)
                        {
                            let r = snap_val(seg.len, snap);
                            let a = snap_angle(seg.a, snap);
                            self.vertices.val_mut(2).curr =
                                snap_vertex(seg.s + Vec2::new(r * a.cos(), r * a.sin()), snap);
                        } else {
                            self.vertices.val_mut(2).add(delta);
                        }
                    }
                    // Move the side
                    if let Some(seg) =
                        SegBundle::new(self.vertices.val(0).curr, self.vertices.val(1).curr)
                    {
                        self.vertices.val_mut(2).curr = seg.m + seg.n * r_saved;
                    }
                }
                self.set_bezpath();
                true
            }
            Icons::Square | Icons::Text => {
                let len = self.vertices.len();
                if len != 4 {
                    return false;
                }
                if self.shape_type == Icons::Text {
                    if let Some(text) = self.text.as_mut() {
                        text.scale = None;
                        text.invalidate_cache();
                    }
                }
                let i = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                    Some(i) => i as i64,
                    None => return false,
                };

                // Snap the saved vertex position, hence snap prev/next along h or v axis
                // let snap_v_idx = snap_vertex(self.vertices.val(idx as i64).saved, snap);
                self.vertices.val_mut(i).saved = snap_vertex(self.vertices.val(i).saved, snap);
                self.vertices.val_mut(i).add(delta);
                if i % 2 == 1 {
                    self.vertices.val_mut(i - 1).saved.x = self.vertices.val_mut(i).saved.x;
                    self.vertices.val_mut(i - 1).add(Vec2::new(delta.x, 0.));
                    self.vertices.val_mut(i + 1).saved.y = self.vertices.val_mut(i).saved.y;
                    self.vertices.val_mut(i + 1).add(Vec2::new(0., delta.y));
                } else {
                    self.vertices.val_mut(i - 1).saved.y = self.vertices.val_mut(i).saved.y;
                    self.vertices.val_mut(i - 1).add(Vec2::new(0., delta.y));
                    self.vertices.val_mut(i + 1).saved.x = self.vertices.val_mut(i).saved.x;
                    self.vertices.val_mut(i + 1).add(Vec2::new(delta.x, 0.));
                }
                self.set_bezpath();

                // if idx % 2 == 0 {
                //     self.vertices.val_mut(idx - 1).saved.x =
                //         snap_val(self.vertices.val(idx - 1).saved.x, snap);
                //     self.vertices.val_mut(idx + 1).saved.y =
                //         snap_val(self.vertices.val(idx + 1).saved.y, snap);
                // } else {
                //     self.vertices.val_mut(idx - 1).saved.y =
                //         snap_val(self.vertices.val(idx - 1).saved.y, snap);
                //     self.vertices.val_mut(idx + 1).saved.x =
                //         snap_val(self.vertices.val(idx + 1).saved.x, snap);
                // }

                false
                // self.vertices.val_mut(idx).saved = snap_v_idx;

                // // 2) grab positions and derive segments
                // let pos_m = self.vertices.val(idx).saved;
                // let o_seg_a = SegBundle::new(self.vertices.val(idx_prev).saved, pos_m);
                // let o_seg_b = SegBundle::new(pos_m, self.vertices.val(idx_next).saved);
                // if let Some(seg_a) = o_seg_a {
                //     if let Some(seg_b) = o_seg_b {
                //         let delta_a = delta.dot(seg_a.u);
                //         let delta_b = delta.dot(seg_b.u);

                //         let new_prev = self.vertices.val(idx_prev).saved + delta_b * seg_b.u;
                //         let new_next = self.vertices.val(idx_next).saved + delta_a * seg_a.u;
                //         let new_m = pos_m + delta;

                //         if (new_m - new_prev).hypot() > EPSILON
                //             && (new_m - new_next).hypot() > EPSILON
                //         {
                //             self.vertices.val_mut(idx_prev).set(new_prev);
                //             self.vertices.val_mut(idx).set(new_m);
                //             self.vertices.val_mut(idx_next).set(new_next);
                //             self.set_bezpath();
                //             true
                //         } else {
                //             return false;
                //         }
                //     } else {
                //         return false;
                //     }
                // } else {
                //     return false;
                // }
            }
            Icons::Poly => {
                let idx = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                    Some(i) => i as i64,
                    None => return false,
                };
                self.vertices.val_mut(idx).saved = snap_vertex(self.vertices.val(idx).saved, snap);
                self.vertices.val_mut(idx).add(delta);
                self.set_bezpath();
                true
            }
            Icons::Arrow => {
                // Arrow is not a closed shape, so we don't move it
                return false;
            }
        }
    }
    pub fn move_shape(&mut self, delta: Vec2) {
        for (_, value) in self.vertices.iter_mut() {
            value.add(delta);
        }
        self.set_bezpath();
    }
    pub fn save_vertices_positions(&mut self) {
        for (_, value) in self.vertices.iter_mut() {
            value.save();
        }
    }
    pub fn get_binded_elements(&self) -> HashSet<EUId> {
        let mut binds = HashSet::new();
        for (_, v) in self.vertices.iter() {
            binds.extend(v.bind.iter().map(|(eid, _)| *eid));
        }
        binds
    }

    pub fn contains(&self, pos: Vec2) -> bool {
        if self.shape_type == Icons::Text {
            let bbox = self.bezpath.bounding_box();
            return bbox.contains(pos.to_point());
        }
        self.get_bezpath().contains(pos.to_point())
    }
    pub fn get_shape_type(&self) -> Icons {
        self.shape_type
    }
    pub fn get_operation(&self) -> Operation {
        self.operation
    }
    pub fn get_polygon(&self) -> &MultiPolygon<f64> {
        &self.polygon
    }
    pub fn get_bezpath(&self) -> &BezPath {
        &self.bezpath
    }
    pub fn get_text(&self) -> Option<&TextData> {
        self.text.as_ref()
    }
    pub fn set_text_value(&mut self, text: String) {
        if let Some(data) = self.text.as_mut() {
            if !data.auto_fit && data.scale.is_none() && !self.bezpath.is_empty() {
                let bbox = self.bezpath.bounding_box();
                if !bbox.is_zero_area() {
                    let bbox_min = Vec2::new(bbox.x0, bbox.y0);
                    let bbox_max = Vec2::new(bbox.x1, bbox.y1);
                    if let Some(scale) =
                        text_scale_for_bbox(&data.text, data.font.clone(), bbox_min, bbox_max)
                    {
                        data.scale = Some(scale);
                    }
                }
            }
            data.text = text;
            data.invalidate_cache();
            self.set_bezpath();
        }
    }
    pub fn set_text_autofit(&mut self, enabled: bool) {
        if let Some(data) = self.text.as_mut() {
            data.auto_fit = enabled;
            if enabled {
                data.scale = None;
            }
            data.invalidate_cache();
            self.set_bezpath();
        }
    }
    pub fn ensure_text_scale(&mut self) {
        if let Some(data) = self.text.as_ref() {
            if data.auto_fit || data.scale.is_some() {
                return;
            }
        }
        self.set_bezpath();
    }
    pub fn fit_text_bbox_width_to_content(&mut self) {
        if self.shape_type != Icons::Text {
            return;
        }
        let Some(text) = self.text.as_ref() else {
            return;
        };
        if text.auto_fit {
            return;
        }
        let Some(scale) = text.scale else {
            return;
        };
        let Some(desired_w) = text_width_for_scale(&text.text, text.font.clone(), scale) else {
            return;
        };
        if self.vertices.len() < 4 || self.bezpath.is_empty() {
            return;
        }
        let bbox = self.bezpath.bounding_box();
        if bbox.is_zero_area() {
            return;
        }
        let center_x = (bbox.x0 + bbox.x1) * 0.5;
        let half_w = desired_w * 0.5;
        let min_x = center_x - half_w;
        let max_x = center_x + half_w;
        let min_y = bbox.y0;
        let max_y = bbox.y1;

        let corners = [
            Vec2::new(min_x, min_y),
            Vec2::new(min_x, max_y),
            Vec2::new(max_x, max_y),
            Vec2::new(max_x, min_y),
        ];
        for (idx, corner) in corners.iter().enumerate() {
            let value = self.vertices.val_mut(idx as i64);
            value.curr = *corner;
            value.saved = *corner;
            value.last = *corner;
        }
        self.set_bezpath();
    }
    pub fn fit_text_bbox_to_polygon(&mut self) {
        if self.shape_type != Icons::Text {
            return;
        }
        let Some(rect) = self.polygon.bounding_rect() else {
            return;
        };
        if self.vertices.len() < 4 {
            return;
        }
        let bl = Vec2::new(rect.min().x, rect.min().y);
        let tr = Vec2::new(rect.max().x, rect.max().y);
        let tl = Vec2::new(bl.x, tr.y);
        let br = Vec2::new(tr.x, bl.y);
        let corners = [bl, tl, tr, br];

        for (idx, corner) in corners.iter().enumerate() {
            let value = self.vertices.val_mut(idx as i64);
            value.curr = *corner;
            value.saved = *corner;
            value.last = *corner;
        }
        self.set_bezpath();
    }
    pub fn set_bezpath(&mut self) {
        match self.shape_type {
            Icons::Disc => {
                let center = self.vertices.val(0).curr;
                let radius = (self.vertices.val(1).curr - center).hypot();
                self.bezpath =
                    kurbo::Circle::new(center.to_point(), radius).to_path(Self::TOLERANCE);
            }
            Icons::Oblong => {
                let e1 = self.vertices.val(0).curr;
                let e2 = self.vertices.val(1).curr;
                let side = self.vertices.val(2).curr;
                let m = (e1 + e2) * 0.5;
                let radius = (side - m).hypot();
                let angle = (e2 - e1).atan2();
                let mut dir = e2 - e1;

                let mut path = BezPath::new();
                if dir.hypot() >= EPSILON {
                    dir = dir.normalize();
                    // Perpendicular unit vector
                    let perp = Vec2::new(-dir.y, dir.x);
                    // Two points at e1 ± perp * radius
                    let pt2 = e1 - perp * radius;
                    // Two points at e2 ± perp * radius
                    let pt3 = e2 + perp * radius;

                    path.extend(
                        Arc::new(
                            e1.to_point(),
                            Vec2::new(radius, radius),
                            3. * PI / 2.,
                            -PI,
                            angle,
                        )
                        .path_elements(Self::TOLERANCE),
                    );
                    path.push(PathEl::LineTo(pt3.to_point()));
                    let mut arc2 = Arc::new(
                        e2.to_point(),
                        Vec2::new(radius, radius),
                        PI / 2.,
                        -PI,
                        angle,
                    )
                    .path_elements(Self::TOLERANCE);
                    arc2.next(); // Remove the MoveTo
                    path.extend(arc2);
                    path.push(PathEl::LineTo(pt2.to_point()));
                    path.push(PathEl::ClosePath);
                } else {
                    path.extend(Circle::new(e2.to_point(), radius).path_elements(Self::TOLERANCE));
                }
                self.bezpath = path;
            }
            Icons::Square | Icons::Poly => {
                let apices = self.vertices.get_apices(); // -> Vec<ApexType>
                self.bezpath = bezpath_from_apices(&apices);
            }
            Icons::Text => {
                let apices = self.vertices.get_apices();
                self.bezpath = bezpath_from_apices(&apices);
                self.update_text_polygon();
                return;
            }
            Icons::Arrow => return,
        }
        self.update_polygon();
    }
    fn update_polygon(&mut self) {
        let poly = bez_path_to_geo_polygon(&self.bezpath);
        self.polygon = MultiPolygon::new(vec![poly]);
    }

    fn update_text_polygon(&mut self) {
        let Some(text) = self.text.as_mut() else {
            self.polygon = MultiPolygon::new(vec![]);
            return;
        };
        if self.vertices.len() < 4 || self.bezpath.is_empty() {
            self.polygon = MultiPolygon::new(vec![]);
            return;
        }
        let bbox = self.bezpath.bounding_box();
        if bbox.is_zero_area() {
            self.polygon = MultiPolygon::new(vec![]);
            return;
        }
        let bbox_min = Vec2::new(bbox.x0, bbox.y0);
        let bbox_max = Vec2::new(bbox.x1, bbox.y1);

        let scale_override = if text.auto_fit { None } else { text.scale };
        let cache_hit = text.cached_polygon.is_some()
            && text.cached_text == text.text
            && text.cached_font == text.font
            && text.cached_scale_override == scale_override
            && text.cached_auto_fit == text.auto_fit
            && text.cached_bbox_min == bbox_min
            && text.cached_bbox_max == bbox_max;

        if cache_hit {
            self.polygon = text
                .cached_polygon
                .clone()
                .unwrap_or_else(|| MultiPolygon::new(vec![]));
            return;
        }

        let (poly, scale) = text_to_multipolygon(
            &text.text,
            text.font.clone(),
            bbox_min,
            bbox_max,
            scale_override,
        );
        self.polygon = poly.clone();
        text.cached_polygon = Some(poly);
        text.cached_text = text.text.clone();
        text.cached_font = text.font.clone();
        text.cached_scale_override = scale_override;
        text.cached_auto_fit = text.auto_fit;
        text.cached_bbox_min = bbox_min;
        text.cached_bbox_max = bbox_max;

        if text.scale.is_none() && !text.auto_fit {
            text.scale = Some(scale);
        }
    }
}

struct GlyphBuilder {
    contours: Vec<BezPath>,
    current: Option<BezPath>,
    scale: f64,
    offset: Vec2,
}
impl GlyphBuilder {
    fn new(scale: f64, offset: Vec2) -> Self {
        Self {
            contours: Vec::new(),
            current: None,
            scale,
            offset,
        }
    }
    fn finish(mut self) -> Vec<BezPath> {
        if let Some(path) = self.current.take() {
            if !path.is_empty() {
                self.contours.push(path);
            }
        }
        self.contours
    }
    fn pt(&self, x: f32, y: f32) -> kurbo::Point {
        kurbo::Point::new(
            self.offset.x + (x as f64) * self.scale,
            self.offset.y - (y as f64) * self.scale,
        )
    }
}
impl ttf_parser::OutlineBuilder for GlyphBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        if let Some(path) = self.current.take() {
            if !path.is_empty() {
                self.contours.push(path);
            }
        }
        let mut path = BezPath::new();
        path.push(PathEl::MoveTo(self.pt(x, y)));
        self.current = Some(path);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let pt = self.pt(x, y);
        if let Some(path) = self.current.as_mut() {
            path.push(PathEl::LineTo(pt));
        }
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let pt1 = self.pt(x1, y1);
        let pt2 = self.pt(x, y);
        if let Some(path) = self.current.as_mut() {
            path.push(PathEl::QuadTo(pt1, pt2));
        }
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let pt1 = self.pt(x1, y1);
        let pt2 = self.pt(x2, y2);
        let pt3 = self.pt(x, y);
        if let Some(path) = self.current.as_mut() {
            path.push(PathEl::CurveTo(pt1, pt2, pt3));
        }
    }
    fn close(&mut self) {
        if let Some(path) = self.current.as_mut() {
            path.push(PathEl::ClosePath);
        }
    }
}

fn text_to_multipolygon(
    text: &str,
    font: TextFont,
    bbox_min: Vec2,
    bbox_max: Vec2,
    scale_override: Option<f64>,
) -> (MultiPolygon<f64>, f64) {
    let Ok(face) = ttf_parser::Face::parse(font.data(), 0) else {
        return (MultiPolygon::new(vec![]), 0.0);
    };
    let bbox_w = (bbox_max.x - bbox_min.x).max(EPSILON);
    let bbox_h = (bbox_max.y - bbox_min.y).max(EPSILON);

    let asc = face.ascender() as f64;
    let desc = face.descender() as f64;
    let text_height = (asc - desc).max(1.0);

    let mut advances: Vec<(ttf_parser::GlyphId, f64)> = Vec::new();
    let mut advance_total = 0.0;

    for ch in text.chars() {
        if ch == '\n' {
            continue;
        }
        if let Some(gid) = face.glyph_index(ch) {
            advances.push((gid, advance_total));
            let adv = face.glyph_hor_advance(gid).unwrap_or(0) as f64;
            advance_total += adv;
        } else {
            let adv = face.units_per_em() as f64 * 0.5;
            advance_total += adv;
        }
    }
    if advance_total <= EPSILON {
        return (MultiPolygon::new(vec![]), 0.0);
    }

    let scale = scale_override.unwrap_or((bbox_w / advance_total).min(bbox_h / text_height));
    let scaled_w = advance_total * scale;
    let scaled_h = text_height * scale;
    let top_y = bbox_min.y + (bbox_h - scaled_h) * 0.5;
    let offset_y = top_y + asc * scale;
    let offset_x = bbox_min.x + (bbox_w - scaled_w) * 0.5;

    let mut rings: Vec<LineString<f64>> = Vec::new();
    for (gid, advance_x) in advances {
        let offset = Vec2::new(offset_x + advance_x * scale, offset_y);
        let mut builder = GlyphBuilder::new(scale, offset);
        face.outline_glyph(gid, &mut builder);
        let contours = builder.finish();
        for contour in contours {
            let poly = bez_path_to_geo_polygon(&contour);
            let ring = poly.exterior().clone();
            if ring.0.len() >= 4 {
                rings.push(ring);
            }
        }
    }

    (rings_to_multipolygon(rings), scale)
}

fn text_scale_for_bbox(text: &str, font: TextFont, bbox_min: Vec2, bbox_max: Vec2) -> Option<f64> {
    let Ok(face) = ttf_parser::Face::parse(font.data(), 0) else {
        return None;
    };
    let bbox_w = (bbox_max.x - bbox_min.x).max(EPSILON);
    let bbox_h = (bbox_max.y - bbox_min.y).max(EPSILON);
    let asc = face.ascender() as f64;
    let desc = face.descender() as f64;
    let text_height = (asc - desc).max(1.0);

    let mut advance_total = 0.0;
    for ch in text.chars() {
        if ch == '\n' {
            continue;
        }
        if let Some(gid) = face.glyph_index(ch) {
            let adv = face.glyph_hor_advance(gid).unwrap_or(0) as f64;
            advance_total += adv;
        } else {
            let adv = face.units_per_em() as f64 * 0.5;
            advance_total += adv;
        }
    }
    if advance_total <= EPSILON {
        return None;
    }
    Some((bbox_w / advance_total).min(bbox_h / text_height))
}

fn text_width_for_scale(text: &str, font: TextFont, scale: f64) -> Option<f64> {
    let Ok(face) = ttf_parser::Face::parse(font.data(), 0) else {
        return None;
    };
    let mut advance_total = 0.0;
    for ch in text.chars() {
        if ch == '\n' {
            continue;
        }
        if let Some(gid) = face.glyph_index(ch) {
            let adv = face.glyph_hor_advance(gid).unwrap_or(0) as f64;
            advance_total += adv;
        } else {
            let adv = face.units_per_em() as f64 * 0.5;
            advance_total += adv;
        }
    }
    if advance_total <= EPSILON {
        return None;
    }
    Some(advance_total * scale)
}

fn rings_to_multipolygon(rings: Vec<LineString<f64>>) -> MultiPolygon<f64> {
    if rings.is_empty() {
        return MultiPolygon::new(vec![]);
    }
    let mut outers: Vec<usize> = Vec::new();
    let mut holes: Vec<(usize, LineString<f64>)> = Vec::new();
    let mut areas: Vec<f64> = Vec::new();

    for ring in &rings {
        areas.push(ring_signed_area(ring).abs());
    }

    for (i, ring) in rings.iter().enumerate() {
        let pt = ring.0.first().copied().unwrap_or_default();
        let mut container: Option<usize> = None;
        for (j, outer) in rings.iter().enumerate() {
            if i == j {
                continue;
            }
            let poly = Polygon::new(outer.clone(), vec![]);
            if poly.contains(&Point::new(pt.x, pt.y)) {
                if container.is_none() || areas[j] < areas[container.unwrap()] {
                    container = Some(j);
                }
            }
        }
        if let Some(idx) = container {
            holes.push((idx, ring.clone()));
        } else {
            outers.push(i);
        }
    }

    let mut polys = Vec::new();
    for outer_idx in outers {
        let mut inner = Vec::new();
        for (idx, ring) in holes.iter() {
            if *idx == outer_idx {
                inner.push(ring.clone());
            }
        }
        let poly = Polygon::new(rings[outer_idx].clone(), inner).orient(Direction::Default);
        polys.push(poly);
    }

    MultiPolygon::new(polys)
}

fn ring_signed_area(ring: &LineString<f64>) -> f64 {
    let pts = &ring.0;
    if pts.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..pts.len() - 1 {
        area += pts[i].x * pts[i + 1].y - pts[i + 1].x * pts[i].y;
    }
    area * 0.5
}

impl Clone for ClosedShape {
    fn clone(&self) -> Self {
        let vertices: Vec<(VUId, Value)> = self
            .vertices
            .iter()
            .map(|(_, value)| (VUId::new(), value.clone()))
            .collect::<Vec<_>>();
        ClosedShape {
            shape_type: self.shape_type,
            operation: self.operation,
            vertices: VecRing::from_slice(&vertices[..]).unwrap(),
            bezpath: self.bezpath.clone(),
            polygon: self.polygon.clone(),
            text: self.text.clone(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Operation {
    Union,
    Difference,
}
impl Operation {
    pub fn next(&mut self) {
        match self {
            Operation::Union => *self = Operation::Difference,
            Operation::Difference => *self = Operation::Union,
        }
    }
    pub fn union(&mut self) {
        *self = Operation::Union;
    }
    pub fn difference(&mut self) {
        *self = Operation::Difference;
    }
}
impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Union => write!(f, "Add"),
            Operation::Difference => write!(f, "Substract"),
        }
    }
}
