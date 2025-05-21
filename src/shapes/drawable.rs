// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use crate::{math::EPSILON, types::Value};
use geo::Polygon;
use kurbo::{BezPath, Vec2};
use std::{
    fmt::{Debug, Display},
    sync::atomic::{AtomicUsize, Ordering},
    vec,
};

pub trait Drawable: Clone + 'static {
    fn op_next(&mut self);
    fn op_union(&mut self);
    fn op_force_union(&mut self);
    fn op_difference(&mut self);
    fn move_vertex(&mut self, value_uid: ValueUId, delta: Vec2) -> Vec<ValueUId>;
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
pub enum ShapeType {
    Disc,
    Rectangle,
    Oblong,
    Polygon,
}
#[derive(Debug, Clone)]
pub struct Shapes {
    pub shape_type: ShapeType,
    pub operation: Operation,
    vertices: Vec<(ValueUId, Value<Vec2>)>,

    segs: BezPath,
    polygon: Polygon<f64>,
}
impl Drawable for Shapes {
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
            ShapeType::Disc => {
                if self.vertices.len() != 2 {
                    return vec![];
                }
                // The first vertex is the center
                // The second vertex is the radius
                if self.vertices[0].0 == value_uid {
                    self.vertices[0].1.add(delta);
                    self.vertices[1].1.add(delta);
                    return vec![self.vertices[0].0, self.vertices[1].0];
                } else if self.vertices[1].0 == value_uid {
                    self.vertices[1].1.add(delta);
                    return vec![self.vertices[1].0];
                } else {
                    return vec![];
                }
            }
            ShapeType::Rectangle => {
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
                moved
            }
            ShapeType::Oblong => {
                if self.vertices.len() != 3 {
                    return vec![];
                }
                let e1 = self.vertices[0].1.curr;
                let side = self.vertices[1].1.curr;
                let e2 = self.vertices[2].1.curr;

                let idx = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                    Some(i) => i,
                    None => return vec![],
                };

                if idx == 1 {
                    if (e1 - e2).length() < EPSILON {
                        self.vertices[1].1.add(delta);
                    } else {
                        let dir = (e2 - e1).normalize();
                        let proj_along = dir * delta.dot(dir);
                        self.vertices[1].1.add(delta - proj_along);
                    }
                    return vec![self.vertices[1].0];
                } else {
                    let m_old = (e1 + e2) * 0.5;
                    let dir_old = (e2 - e1).normalize();
                    let perp_old = Vec2::new(-dir_old.x, dir_old.y);
                    let signed_d = (side - m_old).dot(perp_old);

                    // Apply the move to whichever endpoint, recompute bisector
                    if idx == 0 {
                        self.vertices[0].1.add(delta);
                    } else {
                        self.vertices[2].1.add(delta);
                    }
                    let m_new = (self.vertices[0].1.curr + self.vertices[2].1.curr) * 0.5;
                    let dir_new = (self.vertices[2].1.curr - self.vertices[0].1.curr).normalize();
                    let perp_new = Vec2::new(-dir_new.x, dir_new.y);

                    // 3) place e3 at the same signed distance along perp_new from m_new
                    self.vertices[1].1.set(m_new + perp_new * signed_d);
                    if idx == 0 {
                        return vec![self.vertices[0].0, self.vertices[1].0];
                    } else {
                        return vec![self.vertices[2].0, self.vertices[1].0];
                    }
                }
            }
            ShapeType::Polygon => {
                let idx = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                    Some(i) => i,
                    None => return vec![],
                };
                self.vertices[idx].1.add(delta);
                return vec![self.vertices[idx].0];
            }
        }
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
