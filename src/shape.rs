// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use crate::{
    dom::Icons,
    inputs::UserUI,
    math::{bez_path_to_geo_polygon, snap_angle, snap_val, snap_vertex, EPSILON},
    types::{EUId, SegBundle, VUId, Value, VecRing},
};
use geo::{LineString, Polygon};
use kurbo::{Arc, BezPath, Circle, PathEl, Shape, Vec2};
use std::{collections::HashSet, hash::Hash};
use std::{
    f64::consts::PI,
    fmt::{Debug, Display},
    vec,
};

#[allow(dead_code)]
#[derive(Debug)]
pub struct ClosedShape {
    shape_type: Icons,
    operation: Operation,
    vertices: VecRing<VUId, Value<Vec2>>,

    bezpath: BezPath,
    polygon: Polygon<f64>,
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
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        };
        shape.set_bezpath();
        Some(shape)
    }

    pub fn get_vertex(&self, value_uid: &VUId) -> Option<&Value<Vec2>> {
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
    pub fn get_vertex_mut(&mut self, value_uid: &VUId) -> Option<&mut Value<Vec2>> {
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
    pub fn get_vertices(&self) -> &VecRing<VUId, Value<Vec2>> {
        &self.vertices
    }
    pub fn get_vertices_mut(&mut self) -> &mut VecRing<VUId, Value<Vec2>> {
        &mut self.vertices
    }
    pub fn select_vertex(&mut self, user_ui: &UserUI) -> Option<VUId> {
        for (_idx, (uid, value)) in self.vertices.iter().enumerate() {
            if (value.curr - user_ui.draw_pos).hypot() < Self::GRAB_RADIUS {
                return Some(*uid);
            }
        }
        return None;
    }
    pub fn highlight_vertex(&mut self, user_ui: &UserUI) -> Option<VUId> {
        for (uid, value) in self.vertices.iter() {
            if (value.curr - user_ui.draw_pos).hypot() < Self::GRAB_RADIUS {
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
            Icons::Square => {
                let len = self.vertices.len();
                if len != 4 {
                    return false;
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
        self.get_bezpath().contains(pos.to_point())
    }
    pub fn get_shape_type(&self) -> Icons {
        self.shape_type
    }
    pub fn get_operation(&self) -> Operation {
        self.operation
    }
    pub fn get_polygon(&self) -> &Polygon<f64> {
        &self.polygon
    }
    pub fn get_bezpath(&self) -> &BezPath {
        &self.bezpath
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
                let mut path = BezPath::new();
                for (i, (_, value)) in self.vertices.iter().enumerate() {
                    if i == 0 {
                        path.move_to(value.curr.to_point());
                    } else {
                        path.line_to(value.curr.to_point());
                    }
                }
                path.close_path();
                self.bezpath = path;
            }
            Icons::Arrow => return,
        }
        self.update_polygon();
    }
    fn update_polygon(&mut self) {
        self.polygon = bez_path_to_geo_polygon(&self.bezpath);
    }
}

impl Clone for ClosedShape {
    fn clone(&self) -> Self {
        let vertices: Vec<(VUId, Value<Vec2>)> = self
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
