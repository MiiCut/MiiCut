// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use std::fmt::{Debug, Display};

use super::{disc::ShapeDisc, poly::ShapePoly};

pub trait Drawable: Clone {
    fn op_next(&mut self);
    fn op_union(&mut self);
    fn op_force_union(&mut self);
    fn op_difference(&mut self);
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
    Disc(ShapeDisc),
    Polygon(ShapePoly),
}
#[derive(Debug, Clone)]
pub struct Shapes {
    pub shape_type: ShapeType,
    pub operation: Operation,
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
}
