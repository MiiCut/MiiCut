// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use crate::{
    math::EPSILON,
    nodes::ElemUId,
    types::{SegBundle, Value},
};
use geo::{LineString, Polygon};
use kurbo::{Arc, BezPath, Circle, PathEl, Shape, Vec2};
use std::{collections::HashSet, hash::Hash};
use std::{
    f64::consts::PI,
    fmt::{Debug, Display},
    sync::atomic::{AtomicUsize, Ordering},
    vec,
};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ClosedShapes {
    shape_type: ClosedShapeType,
    operation: Operation,
    vertices: Vec<(ValueUId, Value<Vec2>)>,

    bezpath: BezPath,
    polygon: Polygon<f64>,
}
impl ClosedShapes {
    pub fn new(shape_type: ClosedShapeType, mut vertices: Vec<Vec2>) -> Option<Self> {
        // Sanity check
        match shape_type {
            ClosedShapeType::Disc => {
                if vertices.len() != 2 {
                    return None;
                }
            }
            ClosedShapeType::Oblong => {
                if vertices.len() != 3 {
                    return None;
                }
            }
            ClosedShapeType::Rectangle => {
                if vertices.len() != 2 {
                    return None;
                } else {
                    let bl = vertices[0];
                    let tr = vertices[1];
                    let tl = Vec2::new(bl.x, tr.y);
                    let br = Vec2::new(tr.x, bl.y);
                    vertices = vec![bl, tl, tr, br];
                }
            }
            ClosedShapeType::PolyRectangle => {
                if vertices.len() < 4 {
                    return None;
                }
            }
            ClosedShapeType::Polygon => {
                if vertices.len() < 3 {
                    return None;
                }
            }
        }
        let mut shape = ClosedShapes {
            shape_type,
            operation: Operation::Union,
            vertices: vertices
                .iter()
                .map(|v| (ValueUId::new(), Value::new(*v)))
                .collect(),
            bezpath: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        };
        shape.set_bezpath();
        Some(shape)
    }
    fn set_bezpath(&mut self) {
        match self.shape_type {
            ClosedShapeType::Disc => {
                let center = self.vertices[0].1.curr;
                let radius = (self.vertices[1].1.curr - center).hypot();
                self.bezpath =
                    kurbo::Circle::new(center.to_point(), radius).to_path(Self::TOLERANCE);
            }
            ClosedShapeType::Oblong => {
                let e1 = self.vertices[0].1.curr;
                let side = self.vertices[1].1.curr;
                let e2 = self.vertices[2].1.curr;
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
            ClosedShapeType::Rectangle
            | ClosedShapeType::PolyRectangle
            | ClosedShapeType::Polygon => {
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
        }
    }
}
impl Drawable for ClosedShapes {
    const TOLERANCE: f64 = 0.01;

    fn op_next(&mut self) {
        self.operation.next();
    }
    fn op_union(&mut self) {
        self.operation.union();
    }
    fn op_force_union(&mut self) {
        self.operation.force_union();
    }
    fn op_difference(&mut self) {
        self.operation.difference();
    }

    fn get_vertex(&self, value_uid: &ValueUId) -> Option<&Value<Vec2>> {
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
    fn get_vertex_mut(&mut self, value_uid: &ValueUId) -> Option<&mut Value<Vec2>> {
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
    fn get_vertices(&self) -> &Vec<(ValueUId, Value<Vec2>)> {
        &self.vertices
    }
    fn get_vertices_mut(&mut self) -> &mut Vec<(ValueUId, Value<Vec2>)> {
        &mut self.vertices
    }
    fn select_vertex(&mut self, position: Vec2) -> Option<ValueUId> {
        for (uid, value) in &self.vertices {
            if (value.curr - position).hypot() < Self::GRAB_RADIUS {
                return Some(*uid);
            }
        }
        return None;
    }
    fn highlight_vertex(&mut self, position: Vec2) -> Option<ValueUId> {
        for (uid, value) in &self.vertices {
            if (value.curr - position).hypot() < Self::GRAB_RADIUS {
                return Some(*uid);
            }
        }
        return None;
    }
    fn move_vertex(&mut self, value_uid: ValueUId, delta: Vec2) -> bool {
        match self.shape_type {
            ClosedShapeType::Disc => {
                if self.vertices.len() != 2 {
                    return false;
                }
                // The first vertex is the center
                // The second vertex is the radius
                if self.vertices[0].0 == value_uid {
                    self.vertices[0].1.add(delta);
                    self.vertices[1].1.add(delta);
                    self.set_bezpath();
                    true
                } else if self.vertices[1].0 == value_uid {
                    self.vertices[1].1.add(delta);
                    self.set_bezpath();
                    true
                } else {
                    false
                }
            }
            ClosedShapeType::Oblong => {
                if self.vertices.len() != 3 {
                    return false;
                }
                if let Some(seg) = SegBundle::new(self.vertices[0].1.curr, self.vertices[2].1.curr)
                {
                    let side = self.vertices[1].1.curr;
                    let signed_d = (side - seg.m).dot(seg.n);

                    let idx = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                        Some(i) => i,
                        None => return false,
                    };
                    // The side is moved, this doesn't change the pos of e1, e2
                    if idx == 1 {
                        let proj_along = seg.u * delta.dot(seg.u);
                        self.vertices[1].1.add(delta - proj_along);
                        self.set_bezpath();
                        true
                    } else {
                        // e1 or e2 is moved, this affect the position of the side
                        // Apply the move to whichever endpoint
                        if idx == 0 {
                            self.vertices[0].1.add(delta);
                        } else {
                            self.vertices[2].1.add(delta);
                        }
                        if let Some(seg) =
                            SegBundle::new(self.vertices[0].1.curr, self.vertices[2].1.curr)
                        {
                            self.vertices[1].1.curr = seg.m + seg.n * signed_d;
                            self.set_bezpath();
                            true
                        } else {
                            if idx == 0 {
                                self.vertices[0].1.add(-delta);
                            } else {
                                self.vertices[2].1.add(-delta);
                            }
                            false
                        }
                    }
                } else {
                    false
                }
            }
            ClosedShapeType::Rectangle | ClosedShapeType::PolyRectangle => {
                // 0) Check length
                let len = self.vertices.len();
                if len < 4 {
                    return false;
                }
                // 1) find indices
                let (idx_prev, idx, idx_next) =
                    match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                        Some(i) => ((i + len - 1) % len, i, (i + 1) % len),
                        None => return false,
                    };
                // 2) grab positions and derive segments
                let pos_m = self.vertices[idx].1.saved;
                let o_seg_a = SegBundle::new(self.vertices[idx_prev].1.saved, pos_m);
                let o_seg_b = SegBundle::new(pos_m, self.vertices[idx_next].1.saved);
                if let Some(seg_a) = o_seg_a {
                    if let Some(seg_b) = o_seg_b {
                        let delta_a = delta.dot(seg_a.u);
                        let delta_b = delta.dot(seg_b.u);

                        let new_prev = self.vertices[idx_prev].1.saved + delta_b * seg_b.u;
                        let new_next = self.vertices[idx_next].1.saved + delta_a * seg_a.u;
                        let new_m = pos_m + delta;

                        if (new_m - new_prev).hypot() > EPSILON
                            && (new_m - new_next).hypot() > EPSILON
                        {
                            self.vertices[idx_prev].1.set(new_prev);
                            self.vertices[idx].1.set(new_m);
                            self.vertices[idx_next].1.set(new_next);
                            self.set_bezpath();
                            true
                        } else {
                            return false;
                        }
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            ClosedShapeType::Polygon => {
                let idx = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                    Some(i) => i,
                    None => return false,
                };
                self.vertices[idx].1.add(delta);
                self.set_bezpath();
                true
            }
        }
    }
    fn move_all_vertices(&mut self, delta: Vec2) -> Vec<ValueUId> {
        let mut v_moved = vec![];
        for (v_id, value) in self.get_vertices_mut() {
            value.add(delta);
            v_moved.push(*v_id);
        }
        self.set_bezpath();
        v_moved
    }
    fn save_vertices_positions(&mut self) {
        for (_, value) in self.get_vertices_mut() {
            value.save();
        }
    }
    fn get_binded_elements(&self) -> HashSet<ElemUId> {
        let mut binds = HashSet::new();
        for (_, v) in self.get_vertices() {
            binds.extend(v.bind.iter().map(|(eid, _)| *eid));
        }
        binds
    }

    fn get_shape_type(&self) -> ClosedShapeType {
        self.shape_type
    }
    fn get_bezpath(&self) -> &BezPath {
        &self.bezpath
    }
}

static COUNTER_VALUE: AtomicUsize = AtomicUsize::new(0);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ValueUId {
    id: usize,
}
impl Display for ValueUId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
impl ValueUId {
    pub fn new() -> Self {
        let id = COUNTER_VALUE.fetch_add(1, Ordering::SeqCst);
        ValueUId { id }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Operation {
    Union,
    UnionForced,
    Difference,
}
impl Operation {
    pub fn next(&mut self) {
        match self {
            Operation::Union => *self = Operation::UnionForced,
            Operation::UnionForced => *self = Operation::Difference,
            Operation::Difference => *self = Operation::Union,
        }
    }
    pub fn union(&mut self) {
        *self = Operation::Union;
    }
    pub fn force_union(&mut self) {
        *self = Operation::UnionForced;
    }
    pub fn difference(&mut self) {
        *self = Operation::Difference;
    }
}
impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Union => write!(f, "Add"),
            Operation::UnionForced => write!(f, "Add to top"),
            Operation::Difference => write!(f, "Substract"),
        }
    }
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ClosedShapeType {
    Disc,
    Oblong,
    Rectangle,
    PolyRectangle,
    Polygon,
}

pub trait Drawable: Clone + 'static {
    const TOLERANCE: f64;
    const GRAB_RADIUS: f64 = 5.0;

    fn op_next(&mut self);
    fn op_union(&mut self);
    fn op_force_union(&mut self);
    fn op_difference(&mut self);

    fn get_vertex(&self, value_uid: &ValueUId) -> Option<&Value<Vec2>>;
    fn get_vertex_mut(&mut self, value_uid: &ValueUId) -> Option<&mut Value<Vec2>>;
    fn get_vertices(&self) -> &Vec<(ValueUId, Value<Vec2>)>;
    fn get_vertices_mut(&mut self) -> &mut Vec<(ValueUId, Value<Vec2>)>;
    fn select_vertex(&mut self, position: Vec2) -> Option<ValueUId>;
    fn highlight_vertex(&mut self, position: Vec2) -> Option<ValueUId>;
    fn move_vertex(&mut self, value_uid: ValueUId, delta: Vec2) -> bool;
    fn move_all_vertices(&mut self, delta: Vec2) -> Vec<ValueUId>;
    fn save_vertices_positions(&mut self);
    fn get_binded_elements(&self) -> HashSet<ElemUId>;

    fn get_shape_type(&self) -> ClosedShapeType;
    fn contains(&self, pos: Vec2) -> bool {
        self.get_bezpath().contains(pos.to_point())
    }
    fn get_bezpath(&self) -> &BezPath;
}
