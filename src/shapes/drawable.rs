// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use crate::{math::EPSILON, nodes::Set, types::Value};
use geo::{LineString, Polygon};
use kurbo::{Arc, BezPath, Circle, PathEl, Shape, Vec2};
use std::{
    f64::consts::PI,
    fmt::{Debug, Display},
    sync::atomic::{AtomicUsize, Ordering},
    vec,
};

pub trait Drawable: Clone + 'static {
    const TOLERANCE: f64;

    fn op_next(&mut self);
    fn op_union(&mut self);
    fn op_force_union(&mut self);
    fn op_difference(&mut self);
    fn move_vertex(&mut self, value_uid: ValueUId, delta: Vec2) -> Vec<ValueUId>;
    fn contains(&self, pos: Vec2) -> bool {
        self.get_bezpath().contains(pos.to_point())
    }
    fn get_bezpath(&self) -> &BezPath;
}

impl<E: Drawable> Set<E> {
    pub fn select_nodes(&mut self, position: Vec2) {
        // Select the nodes whose element contains the position
        for (id, node) in &self.nodes {
            self.nodes_selected.clear();
            if node.element.contains(position) {
                self.nodes_selected.insert(*id);
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

#[derive(Debug, Clone)]
pub enum ClosedShapeType {
    Disc,
    Oblong,
    Rectangle,
    PolyRectangle,
    Polygon,
}
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ClosedShapes {
    pub shape_type: ClosedShapeType,
    pub operation: Operation,
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
                    let pt1 = e1 + perp * radius;
                    let _pt2 = e1 - perp * radius;
                    // Two points at e2 ± perp * radius
                    let pt3 = e2 + perp * radius;
                    let _pt4 = e2 - perp * radius;

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
                    path.push(PathEl::MoveTo(pt1.to_point()));
                    path.extend(
                        Arc::new(
                            e2.to_point(),
                            Vec2::new(radius, radius),
                            PI / 2.,
                            -PI,
                            angle,
                        )
                        .path_elements(Self::TOLERANCE),
                    );
                    path.push(PathEl::MoveTo(pt3.to_point()));
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
    fn move_vertex(&mut self, value_uid: ValueUId, delta: Vec2) -> Vec<ValueUId> {
        match self.shape_type {
            ClosedShapeType::Disc => {
                if self.vertices.len() != 2 {
                    return vec![];
                }
                // The first vertex is the center
                // The second vertex is the radius
                if self.vertices[0].0 == value_uid {
                    self.vertices[0].1.add(delta);
                    self.vertices[1].1.add(delta);
                    self.set_bezpath();
                    return vec![self.vertices[0].0, self.vertices[1].0];
                } else if self.vertices[1].0 == value_uid {
                    self.vertices[1].1.add(delta);
                    self.set_bezpath();
                    return vec![self.vertices[1].0];
                } else {
                    return vec![];
                }
            }
            ClosedShapeType::Oblong => {
                if self.vertices.len() != 3 {
                    return vec![];
                }
                let mut e1 = self.vertices[0].1.curr;
                let side = self.vertices[1].1.curr;
                let mut e2 = self.vertices[2].1.curr;

                let idx = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                    Some(i) => i,
                    None => return vec![],
                };
                // The side is moved, this doesn't change the pos of e1, e2
                if idx == 1 {
                    if (e1 - e2).length() < EPSILON {
                        self.vertices[1].1.add(delta);
                    } else {
                        let dir = (e2 - e1).normalize();
                        let proj_along = dir * delta.dot(dir);
                        self.vertices[1].1.add(delta - proj_along);
                    }
                    self.set_bezpath();
                    return vec![self.vertices[1].0];
                } else {
                    // e1 or e2 is moved, this affect the position of the side
                    let mut m = (e1 + e2) * 0.5;
                    let mut dir = (e2 - e1).normalize();
                    let mut perp = Vec2::new(-dir.x, dir.y);
                    let signed_d = (side - m).dot(perp);
                    // Apply the move to whichever endpoint
                    if idx == 0 {
                        e1 += delta;
                        self.vertices[0].1.set(e1);
                    } else {
                        e2 += delta;
                        self.vertices[2].1.set(e2);
                    }
                    // recalculate perp
                    m = (e1 + e2) * 0.5;
                    dir = (e2 - e1).normalize();
                    perp = Vec2::new(-dir.x, dir.y);
                    // and update side
                    self.vertices[1].1.set(m + perp * signed_d);
                    self.set_bezpath();
                    if idx == 0 {
                        return vec![self.vertices[0].0, self.vertices[1].0];
                    } else {
                        return vec![self.vertices[2].0, self.vertices[1].0];
                    }
                }
            }
            ClosedShapeType::Rectangle | ClosedShapeType::PolyRectangle => {
                // 0) Check length
                let len = self.vertices.len();
                if len < 4 {
                    return vec![];
                }
                // 1) find dragged corner index
                let idx = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                    Some(i) => i,
                    None => return vec![],
                };
                // 2) compute neighbor indices
                let next = (idx + 1) % len;
                let prev = (idx + len - 1) % len;

                // 3) grab positions
                let pos_m = self.vertices[idx].1.curr;
                let pos_next = self.vertices[next].1.curr;
                let pos_prev = self.vertices[prev].1.curr;
                // 4) local axes and original lengths
                let dir_next = (pos_next - pos_m).normalize();
                let dir_prev = (pos_prev - pos_m).normalize();
                let len_next = (pos_next - pos_m).hypot();
                let len_prev = (pos_prev - pos_m).hypot();
                // 5) decompose delta
                let d_next = delta.dot(dir_next);
                let d_prev = delta.dot(dir_prev);
                // 6) move main corner
                self.vertices[idx].1.add(delta);
                let mut moved = vec![self.vertices[idx].0];
                // 7) for each axis, only slide if above threshold
                if d_next.abs() > EPSILON {
                    let new_1 = (pos_m + delta) + dir_next * (len_next + d_next);
                    let d1 = new_1 - pos_next;
                    self.vertices[next].1.add(d1);
                    moved.push(self.vertices[next].0);
                }
                if d_prev.abs() > EPSILON {
                    let new_2 = (pos_m + delta) + dir_prev * (len_prev + d_prev);
                    let d2 = new_2 - pos_prev;
                    self.vertices[prev].1.add(d2);
                    moved.push(self.vertices[prev].0);
                }
                self.set_bezpath();
                moved
            }
            ClosedShapeType::Polygon => {
                let idx = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                    Some(i) => i,
                    None => return vec![],
                };
                self.vertices[idx].1.add(delta);
                self.set_bezpath();
                return vec![self.vertices[idx].0];
            }
        }
    }
    fn get_bezpath(&self) -> &BezPath {
        &self.bezpath
    }
}

static COUNTER_VALUE: AtomicUsize = AtomicUsize::new(0);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd)]
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
