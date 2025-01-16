// use crate::math::*;
use kurbo::Vec2;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Modifier {
    pub highlighted: bool,
    pub selected: bool,
}
impl Modifier {
    pub fn new() -> Self {
        Self {
            highlighted: false,
            selected: false,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Value {
    pub saved_val: f64,
    pub last_val: f64,
    pub value: f64,
    pub highlighted: bool,
    pub selected: bool,
}
impl Value {
    pub fn new(value: f64) -> Self {
        Self {
            saved_val: value,
            last_val: value,
            value,
            highlighted: false,
            selected: false,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Position {
    pub saved_pos: Vec2,
    pub last_pos: Vec2,
    pub pos: Vec2,
    pub magnet: bool,
    pub highlighted: bool,
    pub selected: bool,
}

impl Position {
    pub fn new(pos: Vec2, magnet: bool) -> Self {
        Self {
            saved_pos: pos,
            last_pos: pos,
            pos,
            magnet,
            highlighted: false,
            selected: false,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Pointer {
    pub pos: Position,
    pub active: bool,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RadiusValue {
    pub radius: Value,
    pub up: bool,
    pub saved_up: bool,
}

impl RadiusValue {
    pub fn new(radius: f64, up: bool) -> Self {
        Self {
            radius: Value::new(radius),
            up,
            saved_up: up,
        }
    }
}
