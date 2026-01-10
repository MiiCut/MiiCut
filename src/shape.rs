// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use crate::{
    dom::ShapeType,
    inputs::UserUI,
    math::{
        bez_path_to_geo_polygon, bezpath_from_apices, geo_multipolygon_to_bez_paths, rotate_vector,
        snap_angle, snap_val, snap_vertex, EPSILON,
    },
    types::{EUId, SegBundle, VUId, Value, VecRing},
};
use geo::algorithm::bounding_rect::BoundingRect;
use geo::algorithm::contains::Contains;
use geo::algorithm::orient::Orient;
use geo::algorithm::translate::Translate;
use geo::{orient::Direction, Coord, LineString, MultiPolygon, Point, Polygon};
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

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SvgFillRule {
    EvenOdd,
    NonZero,
}

#[derive(Clone, Debug)]
pub struct SvgData {
    pub rings: Vec<Vec<Vec2>>,
    pub fill_rule: SvgFillRule,
    original_min: Vec2,
    original_max: Vec2,
    cached_bbox_min: Vec2,
    cached_bbox_max: Vec2,
    cached_polygon: Option<MultiPolygon<f64>>,
    cached_paths_raw: Option<Vec<BezPath>>,
    cached_paths: Option<Vec<BezPath>>,
}
impl SvgData {
    pub fn new(rings: Vec<Vec<Vec2>>, fill_rule: SvgFillRule) -> Self {
        let mut min = Vec2::new(f64::INFINITY, f64::INFINITY);
        let mut max = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for ring in rings.iter() {
            for pt in ring.iter() {
                min.x = min.x.min(pt.x);
                min.y = min.y.min(pt.y);
                max.x = max.x.max(pt.x);
                max.y = max.y.max(pt.y);
            }
        }
        if !min.x.is_finite() || !min.y.is_finite() || !max.x.is_finite() || !max.y.is_finite() {
            min = Vec2::ZERO;
            max = Vec2::ZERO;
        }
        Self {
            rings,
            fill_rule,
            original_min: min,
            original_max: max,
            cached_bbox_min: Vec2::ZERO,
            cached_bbox_max: Vec2::ZERO,
            cached_polygon: None,
            cached_paths_raw: None,
            cached_paths: None,
        }
    }
    fn invalidate_cache(&mut self) {
        self.cached_polygon = None;
        self.cached_paths_raw = None;
        self.cached_paths = None;
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ClosedShape {
    shape_type: ShapeType,
    operation: Operation,
    vertices: VecRing<VUId>,

    bezpath: BezPath,
    polygon: MultiPolygon<f64>,
    text: Option<TextData>,
    svg: Option<SvgData>,
    rotation: f64,
    rotation_saved: f64,
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

    pub fn new(shape_type: ShapeType, vertices: &[Vec2]) -> Option<Self> {
        let mut vertices: Vec<Vec2> = vertices.iter().cloned().collect();
        if vertices.is_empty() {
            return None;
        }
        // Sanity check
        match shape_type {
            ShapeType::Arrow => {
                return None;
            }
            ShapeType::Disc => {
                if vertices.len() != 2 || vertices[0] == vertices[1] {
                    return None;
                }
            }
            ShapeType::Square => {
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
            ShapeType::Oblong => {
                if vertices.len() != 2 || vertices[0] == vertices[1] {
                    return None;
                }
                let m = (vertices[0] + vertices[1]) * 0.5;
                let dir = (vertices[1] - vertices[0]).normalize();
                let side = m - Vec2::new(dir.y, -dir.x) * 20.;
                vertices = vec![vertices[0], vertices[1], side];
            }
            ShapeType::Poly => {
                if vertices.len() < 3 {
                    return None;
                }
            }
            ShapeType::Text => {
                return None;
            }
            ShapeType::Svg => {
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
            svg: None,
            rotation: 0.0,
            rotation_saved: 0.0,
        };
        shape.set_bezpath();
        Some(shape)
    }

    pub fn from_raw(
        shape_type: ShapeType,
        operation: Operation,
        vertices: Vec<Vec2>,
        rounded: &[Option<u32>],
        text: Option<TextData>,
        svg: Option<SvgData>,
    ) -> Option<Self> {
        match shape_type {
            ShapeType::Arrow => return None,
            ShapeType::Disc if vertices.len() != 2 => return None,
            ShapeType::Square if vertices.len() != 4 => return None,
            ShapeType::Oblong if vertices.len() != 3 => return None,
            ShapeType::Poly if vertices.len() < 3 => return None,
            ShapeType::Text | ShapeType::Svg if vertices.len() != 4 => return None,
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
            svg,
            rotation: 0.0,
            rotation_saved: 0.0,
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
            shape_type: ShapeType::Text,
            operation: Operation::Union,
            vertices: VecRing::from_slice(&vertices[..]).unwrap(),
            bezpath: BezPath::new(),
            polygon: MultiPolygon::new(vec![]),
            text: Some(TextData::new(text, font, None, false)),
            svg: None,
            rotation: 0.0,
            rotation_saved: 0.0,
        };
        shape.set_bezpath();
        Some(shape)
    }

    pub fn new_svg(rings: Vec<Vec<Vec2>>, fill_rule: SvgFillRule) -> Option<Self> {
        let svg = SvgData::new(rings, fill_rule);
        let min = svg.original_min;
        let max = svg.original_max;
        if min == max {
            return None;
        }
        let bl = Vec2::new(min.x, min.y);
        let tr = Vec2::new(max.x, max.y);
        let tl = Vec2::new(bl.x, tr.y);
        let br = Vec2::new(tr.x, bl.y);
        let vertices = vec![bl, tl, tr, br]
            .iter()
            .map(|v| (VUId::new(), Value::new(*v)))
            .collect::<Vec<_>>();

        let mut shape = ClosedShape {
            shape_type: ShapeType::Svg,
            operation: Operation::Union,
            vertices: VecRing::from_slice(&vertices[..]).unwrap(),
            bezpath: BezPath::new(),
            polygon: MultiPolygon::new(vec![]),
            text: None,
            svg: Some(svg),
            rotation: 0.0,
            rotation_saved: 0.0,
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
            let pos = self.vertex_display_pos(value.curr);
            if (pos - draw_pos).hypot() < Self::GRAB_RADIUS {
                return Some(*uid);
            }
        }
        return None;
    }
    pub fn highlight_vertex(&mut self, draw_pos: Vec2) -> Option<VUId> {
        for (uid, value) in self.vertices.iter() {
            let pos = self.vertex_display_pos(value.curr);
            if (pos - draw_pos).hypot() < Self::GRAB_RADIUS {
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
            ShapeType::Disc => {
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
            ShapeType::Oblong => {
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
            ShapeType::Square | ShapeType::Text | ShapeType::Svg => {
                let len = self.vertices.len();
                if len != 4 {
                    return false;
                }
                let center = self.bbox_center_saved();
                let saved = rotate_vector(user_ui.pointer.saved - center, -self.rotation) + center;
                let curr = rotate_vector(user_ui.pointer.curr - center, -self.rotation) + center;
                let mut local_delta = curr - saved;
                local_delta = (local_delta / snap.linear()).round() * snap.linear();
                log!("center {:?} local delta {:?}", center, local_delta);
                if user_ui.keys_states.shift_pressed {
                    let start = saved - center;
                    let curr = curr - center;
                    if start.hypot() < EPSILON || curr.hypot() < EPSILON {
                        return false;
                    }
                    let delta_a = curr.atan2() - start.atan2();
                    self.rotation = snap_angle(self.rotation_saved + delta_a, snap);
                    self.set_bezpath();
                    return true;
                }
                if self.shape_type == ShapeType::Text {
                    if let Some(text) = self.text.as_mut() {
                        text.scale = None;
                        text.invalidate_cache();
                    }
                } else if self.shape_type == ShapeType::Svg {
                    if let Some(svg) = self.svg.as_mut() {
                        svg.invalidate_cache();
                    }
                }

                let i = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                    Some(i) => i as i64,
                    None => return false,
                };
                self.vertices.val_mut(i).saved = snap_vertex(self.vertices.val(i).saved, snap);
                self.vertices.val_mut(i).add(local_delta);
                if i % 2 == 1 {
                    self.vertices.val_mut(i - 1).saved.x = self.vertices.val_mut(i).saved.x;
                    self.vertices
                        .val_mut(i - 1)
                        .add(Vec2::new(local_delta.x, 0.));
                    self.vertices.val_mut(i + 1).saved.y = self.vertices.val_mut(i).saved.y;
                    self.vertices
                        .val_mut(i + 1)
                        .add(Vec2::new(0., local_delta.y));
                } else {
                    self.vertices.val_mut(i - 1).saved.y = self.vertices.val_mut(i).saved.y;
                    self.vertices
                        .val_mut(i - 1)
                        .add(Vec2::new(0., local_delta.y));
                    self.vertices.val_mut(i + 1).saved.x = self.vertices.val_mut(i).saved.x;
                    self.vertices
                        .val_mut(i + 1)
                        .add(Vec2::new(local_delta.x, 0.));
                }
                self.set_bezpath();
                true
            }
            ShapeType::Poly => {
                let idx = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                    Some(i) => i as i64,
                    None => return false,
                };
                self.vertices.val_mut(idx).saved = snap_vertex(self.vertices.val(idx).saved, snap);
                self.vertices.val_mut(idx).add(delta);
                self.set_bezpath();
                true
            }
            ShapeType::Arrow => {
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
        self.rotation_saved = self.rotation;
    }
    pub fn get_binded_elements(&self) -> HashSet<EUId> {
        let mut binds = HashSet::new();
        for (_, v) in self.vertices.iter() {
            binds.extend(v.bind.iter().map(|(eid, _)| *eid));
        }
        binds
    }

    pub fn contains(&self, pos: Vec2) -> bool {
        if matches!(self.shape_type, ShapeType::Text | ShapeType::Svg) {
            if self.rotation.abs() > EPSILON {
                return self.get_bezpath().contains(pos.to_point());
            }
            let bbox = self.bezpath.bounding_box();
            return bbox.contains(pos.to_point());
        }
        self.get_bezpath().contains(pos.to_point())
    }
    pub fn get_shape_type(&self) -> ShapeType {
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
    pub fn get_svg(&self) -> Option<&SvgData> {
        self.svg.as_ref()
    }
    pub fn get_svg_paths(&self) -> Option<&Vec<BezPath>> {
        self.svg.as_ref().and_then(|svg| svg.cached_paths.as_ref())
    }
    pub fn vertex_display_pos(&self, pos: Vec2) -> Vec2 {
        if matches!(
            self.shape_type,
            ShapeType::Square | ShapeType::Text | ShapeType::Svg
        ) && self.rotation.abs() > EPSILON
        {
            let center = self.bbox_center();
            return rotate_vector(pos - center, self.rotation) + center;
        }
        pos
    }
    pub fn get_rotation(&self) -> f64 {
        self.rotation
    }
    pub fn set_rotation(&mut self, rotation: f64) {
        self.rotation = rotation;
        self.rotation_saved = rotation;
        self.set_bezpath();
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
        if self.shape_type != ShapeType::Text {
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
        if self.shape_type != ShapeType::Text {
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
        let mut bezpath_only = true;
        match self.shape_type {
            ShapeType::Disc => {
                let center = self.vertices.val(0).curr;
                let radius = (self.vertices.val(1).curr - center).hypot();
                self.bezpath =
                    kurbo::Circle::new(center.to_point(), radius).to_path(Self::TOLERANCE);
            }
            ShapeType::Oblong => {
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
            ShapeType::Square | ShapeType::Poly => {
                let apices = self.vertices.get_apices(); // -> Vec<ApexType>
                self.bezpath = bezpath_from_apices(&apices);
            }
            ShapeType::Text => {
                let apices = self.vertices.get_apices();
                self.bezpath = bezpath_from_apices(&apices);
                self.update_text_polygon();
                bezpath_only = false;
            }
            ShapeType::Svg => {
                let apices = self.vertices.get_apices();
                self.bezpath = bezpath_from_apices(&apices);
                self.update_svg_polygon();
                bezpath_only = false;
            }
            ShapeType::Arrow => return,
        }
        if bezpath_only {
            self.update_polygon();
        }
        self.apply_rotation();
    }

    fn apply_rotation(&mut self) {
        if self.rotation.abs() <= EPSILON {
            if let Some(svg) = self.svg.as_mut() {
                if let Some(raw) = svg.cached_paths_raw.as_ref() {
                    svg.cached_paths = Some(raw.clone());
                }
            }
            return;
        }
        let center = self.bbox_center();
        self.bezpath = rotate_bezpath(&self.bezpath, center, self.rotation);
        self.polygon = rotate_multipolygon(&self.polygon, center, self.rotation);
        if let Some(svg) = self.svg.as_mut() {
            if let Some(raw) = svg.cached_paths_raw.as_ref() {
                svg.cached_paths = Some(rotate_bezpaths(raw, center, self.rotation));
            }
        }
    }

    fn bbox_center(&self) -> Vec2 {
        let mut min = Vec2::new(f64::INFINITY, f64::INFINITY);
        let mut max = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (_, value) in self.vertices.iter() {
            min.x = min.x.min(value.curr.x);
            min.y = min.y.min(value.curr.y);
            max.x = max.x.max(value.curr.x);
            max.y = max.y.max(value.curr.y);
        }
        if !min.x.is_finite() || !min.y.is_finite() || !max.x.is_finite() || !max.y.is_finite() {
            return Vec2::ZERO;
        }
        (min + max) * 0.5
    }

    fn bbox_center_saved(&self) -> Vec2 {
        let mut min = Vec2::new(f64::INFINITY, f64::INFINITY);
        let mut max = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (_, value) in self.vertices.iter() {
            min.x = min.x.min(value.saved.x);
            min.y = min.y.min(value.saved.y);
            max.x = max.x.max(value.saved.x);
            max.y = max.y.max(value.saved.y);
        }
        if !min.x.is_finite() || !min.y.is_finite() || !max.x.is_finite() || !max.y.is_finite() {
            return Vec2::ZERO;
        }
        (min + max) * 0.5
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
        let cache_key_match = text.cached_polygon.is_some()
            && text.cached_text == text.text
            && text.cached_font == text.font
            && text.cached_scale_override == scale_override
            && text.cached_auto_fit == text.auto_fit;
        if cache_key_match {
            if text.cached_bbox_min == bbox_min && text.cached_bbox_max == bbox_max {
                self.polygon = text
                    .cached_polygon
                    .clone()
                    .unwrap_or_else(|| MultiPolygon::new(vec![]));
                return;
            }
            let cached_size = text.cached_bbox_max - text.cached_bbox_min;
            let bbox_size = bbox_max - bbox_min;
            if cached_size == bbox_size {
                let dx = bbox_min.x - text.cached_bbox_min.x;
                let dy = bbox_min.y - text.cached_bbox_min.y;
                let translated = text
                    .cached_polygon
                    .as_ref()
                    .map(|poly| poly.translate(dx, dy))
                    .unwrap_or_else(|| MultiPolygon::new(vec![]));
                self.polygon = translated.clone();
                text.cached_polygon = Some(translated);
                text.cached_bbox_min = bbox_min;
                text.cached_bbox_max = bbox_max;
                return;
            }
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

    fn update_svg_polygon(&mut self) {
        let Some(svg) = self.svg.as_mut() else {
            self.polygon = MultiPolygon::new(vec![]);
            return;
        };
        if self.vertices.len() < 4 || self.bezpath.is_empty() || svg.rings.is_empty() {
            self.polygon = MultiPolygon::new(vec![]);
            svg.invalidate_cache();
            return;
        }
        let bbox = self.bezpath.bounding_box();
        if bbox.is_zero_area() {
            self.polygon = MultiPolygon::new(vec![]);
            svg.invalidate_cache();
            return;
        }
        let bbox_min = Vec2::new(bbox.x0, bbox.y0);
        let bbox_max = Vec2::new(bbox.x1, bbox.y1);

        if let Some(cached) = svg.cached_polygon.as_ref() {
            if svg.cached_bbox_min == bbox_min && svg.cached_bbox_max == bbox_max {
                self.polygon = cached.clone();
                return;
            }
            let cached_size = svg.cached_bbox_max - svg.cached_bbox_min;
            let bbox_size = bbox_max - bbox_min;
            if cached_size == bbox_size {
                let dx = bbox_min.x - svg.cached_bbox_min.x;
                let dy = bbox_min.y - svg.cached_bbox_min.y;
                let translated = cached.translate(dx, dy);
                self.polygon = translated.clone();
                svg.cached_polygon = Some(translated);
                if let Some(paths) = svg.cached_paths_raw.as_ref() {
                    let translated_paths = translate_bezpaths(paths, dx, dy);
                    svg.cached_paths_raw = Some(translated_paths.clone());
                    svg.cached_paths = Some(translated_paths);
                }
                svg.cached_bbox_min = bbox_min;
                svg.cached_bbox_max = bbox_max;
                return;
            }
        }

        let orig_min = svg.original_min;
        let orig_max = svg.original_max;
        let orig_w = (orig_max.x - orig_min.x).max(1e-6);
        let orig_h = (orig_max.y - orig_min.y).max(1e-6);
        let bbox_w = (bbox_max.x - bbox_min.x).max(1e-6);
        let bbox_h = (bbox_max.y - bbox_min.y).max(1e-6);
        let scale = (bbox_w / orig_w).min(bbox_h / orig_h);
        let dx = bbox_min.x + (bbox_w - orig_w * scale) * 0.5 - orig_min.x * scale;
        let dy = bbox_min.y + (bbox_h - orig_h * scale) * 0.5 - orig_min.y * scale;

        let mut rings: Vec<Vec<Vec2>> = Vec::new();
        for ring in svg.rings.iter() {
            let mut pts = Vec::with_capacity(ring.len());
            for pt in ring.iter() {
                pts.push(Vec2::new(pt.x * scale + dx, pt.y * scale + dy));
            }
            let pts = normalize_svg_ring(pts);
            if pts.len() >= 3 {
                rings.push(pts);
            }
        }
        if rings.is_empty() {
            self.polygon = MultiPolygon::new(vec![]);
            return;
        }

        let hole_flags = svg_compute_hole_flags(&rings, svg.fill_rule);
        let mut outers: Vec<Vec<Vec2>> = Vec::new();
        let mut holes: Vec<Vec<Vec2>> = Vec::new();
        for (ring, is_hole) in rings.into_iter().zip(hole_flags.into_iter()) {
            if is_hole {
                holes.push(ring);
            } else {
                outers.push(ring);
            }
        }
        if outers.is_empty() {
            outers = holes;
            holes = Vec::new();
        }

        let mut polys = Vec::new();
        for outer in outers.iter() {
            let outer_poly = Polygon::new(vec2_to_linestring(outer), vec![]);
            let mut inner_lines = Vec::new();
            for hole in holes.iter() {
                if let Some(pt) = hole.first() {
                    if outer_poly.contains(&Point::new(pt.x, pt.y)) {
                        inner_lines.push(vec2_to_linestring(hole));
                    }
                }
            }
            polys.push(Polygon::new(vec2_to_linestring(outer), inner_lines));
        }

        self.polygon = MultiPolygon::new(polys);
        svg.cached_polygon = Some(self.polygon.clone());
        let raw_paths = geo_multipolygon_to_bez_paths(&self.polygon);
        svg.cached_paths_raw = Some(raw_paths.clone());
        svg.cached_paths = Some(raw_paths);
        svg.cached_bbox_min = bbox_min;
        svg.cached_bbox_max = bbox_max;
    }
}

fn normalize_svg_ring(mut ring: Vec<Vec2>) -> Vec<Vec2> {
    if ring.len() > 1 {
        let first = ring[0];
        let last = ring[ring.len() - 1];
        if (first - last).hypot() < 1e-6 {
            ring.pop();
        }
    }
    ring
}

fn translate_bezpaths(paths: &[BezPath], dx: f64, dy: f64) -> Vec<BezPath> {
    paths
        .iter()
        .map(|path| translate_bezpath(path, dx, dy))
        .collect()
}

fn translate_bezpath(path: &BezPath, dx: f64, dy: f64) -> BezPath {
    let mut out = BezPath::new();
    for elem in path.iter() {
        match elem {
            PathEl::MoveTo(pt) => {
                out.push(PathEl::MoveTo(kurbo::Point::new(pt.x + dx, pt.y + dy)));
            }
            PathEl::LineTo(pt) => {
                out.push(PathEl::LineTo(kurbo::Point::new(pt.x + dx, pt.y + dy)));
            }
            PathEl::QuadTo(pt1, pt2) => {
                out.push(PathEl::QuadTo(
                    kurbo::Point::new(pt1.x + dx, pt1.y + dy),
                    kurbo::Point::new(pt2.x + dx, pt2.y + dy),
                ));
            }
            PathEl::CurveTo(pt1, pt2, pt3) => {
                out.push(PathEl::CurveTo(
                    kurbo::Point::new(pt1.x + dx, pt1.y + dy),
                    kurbo::Point::new(pt2.x + dx, pt2.y + dy),
                    kurbo::Point::new(pt3.x + dx, pt3.y + dy),
                ));
            }
            PathEl::ClosePath => out.push(PathEl::ClosePath),
        }
    }
    out
}

fn rotate_bezpaths(paths: &[BezPath], center: Vec2, angle: f64) -> Vec<BezPath> {
    paths
        .iter()
        .map(|path| rotate_bezpath(path, center, angle))
        .collect()
}

fn rotate_bezpath(path: &BezPath, center: Vec2, angle: f64) -> BezPath {
    let mut out = BezPath::new();
    for elem in path.iter() {
        match elem {
            PathEl::MoveTo(pt) => {
                out.push(PathEl::MoveTo(rotate_point(pt, center, angle)));
            }
            PathEl::LineTo(pt) => {
                out.push(PathEl::LineTo(rotate_point(pt, center, angle)));
            }
            PathEl::QuadTo(pt1, pt2) => {
                out.push(PathEl::QuadTo(
                    rotate_point(pt1, center, angle),
                    rotate_point(pt2, center, angle),
                ));
            }
            PathEl::CurveTo(pt1, pt2, pt3) => {
                out.push(PathEl::CurveTo(
                    rotate_point(pt1, center, angle),
                    rotate_point(pt2, center, angle),
                    rotate_point(pt3, center, angle),
                ));
            }
            PathEl::ClosePath => out.push(PathEl::ClosePath),
        }
    }
    out
}

fn rotate_point(point: kurbo::Point, center: Vec2, angle: f64) -> kurbo::Point {
    let v = Vec2::new(point.x, point.y);
    let rotated = rotate_vector(v - center, angle) + center;
    kurbo::Point::new(rotated.x, rotated.y)
}

fn rotate_multipolygon(polygon: &MultiPolygon<f64>, center: Vec2, angle: f64) -> MultiPolygon<f64> {
    let mut polys = Vec::with_capacity(polygon.0.len());
    for poly in polygon.0.iter() {
        let exterior = rotate_linestring(poly.exterior(), center, angle);
        let interiors = poly
            .interiors()
            .iter()
            .map(|ring| rotate_linestring(ring, center, angle))
            .collect();
        polys.push(Polygon::new(exterior, interiors));
    }
    MultiPolygon::new(polys)
}

fn rotate_linestring(line: &LineString<f64>, center: Vec2, angle: f64) -> LineString<f64> {
    let coords: Vec<Coord<f64>> = line
        .points()
        .map(|pt| {
            let v = Vec2::new(pt.x(), pt.y());
            let rotated = rotate_vector(v - center, angle) + center;
            Coord {
                x: rotated.x,
                y: rotated.y,
            }
        })
        .collect();
    LineString::from(coords)
}

fn vec2_to_linestring(points: &[Vec2]) -> LineString<f64> {
    let mut coords: Vec<geo::Coord<f64>> = points
        .iter()
        .map(|pt| geo::Coord { x: pt.x, y: pt.y })
        .collect();
    if coords.len() >= 2 && coords.first() != coords.last() {
        coords.push(coords[0]);
    }
    LineString::from(coords)
}

fn ring_area(points: &[Vec2]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..points.len() {
        let p1 = points[i];
        let p2 = points[(i + 1) % points.len()];
        area += (p1.x * p2.y) - (p2.x * p1.y);
    }
    area * 0.5
}

fn svg_compute_hole_flags(rings: &[Vec<Vec2>], fill_rule: SvgFillRule) -> Vec<bool> {
    let mut flags = vec![false; rings.len()];
    match fill_rule {
        SvgFillRule::NonZero => {
            for (idx, ring) in rings.iter().enumerate() {
                flags[idx] = ring_area(ring) < 0.0;
            }
        }
        SvgFillRule::EvenOdd => {
            let polys: Vec<Polygon<f64>> = rings
                .iter()
                .map(|ring| Polygon::new(vec2_to_linestring(ring), vec![]))
                .collect();
            for (idx, ring) in rings.iter().enumerate() {
                let Some(probe) = ring.first() else {
                    continue;
                };
                let mut depth = 0;
                for (j, poly) in polys.iter().enumerate() {
                    if idx == j {
                        continue;
                    }
                    if poly.contains(&Point::new(probe.x, probe.y)) {
                        depth += 1;
                    }
                }
                flags[idx] = depth % 2 == 1;
            }
        }
    }
    flags
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
            svg: self.svg.clone(),
            rotation: self.rotation,
            rotation_saved: self.rotation,
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
