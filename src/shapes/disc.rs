use crate::types::{Status, Value};
use kurbo::Vec2;

#[derive(Debug, Clone)]
pub struct ShapeDisc {
    center: Value<Vec2>,
    radius: Value<Vec2>,
    radius_state: Status,
    state: Status,
}
