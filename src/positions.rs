// use crate::math::*;
use kurbo::Vec2;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SnapValue {
    Snap1,
    Snap5,
    Snap10,
}
impl SnapValue {
    pub fn val(&self) -> f64 {
        match self {
            SnapValue::Snap1 => 1.,
            SnapValue::Snap5 => 5.,
            SnapValue::Snap10 => 10.,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Pointer {
    // Pointer position
    pos_saved: Vec2,
    pos: Vec2,

    snap: SnapValue,
    draw_scale: f64,
    active: bool,
    magnetized: bool,
}
impl Pointer {
    pub fn new() -> Self {
        Self {
            pos_saved: Vec2::new(0., 0.),
            pos: Vec2::new(0., 0.),

            snap: SnapValue::Snap10,
            draw_scale: 1.,
            active: false,
            magnetized: false,
        }
    }
    pub fn dpos(&self) -> Vec2 {
        self.pos - self.pos_saved
    }
    pub fn pos(&self) -> Vec2 {
        self.pos
    }
    pub fn pos_saved(&self) -> Vec2 {
        self.pos_saved
    }
    pub fn set_pos(&mut self, pos: Vec2) {
        self.pos = pos;
    }
    pub fn set_pos_rel(&mut self, dpos: Vec2) {
        self.pos = self.pos_saved + dpos;
    }
    pub fn save_pos(&mut self) {
        self.pos_saved = self.pos;
    }
    pub fn set_draw_scale(&mut self, scale: f64) {
        self.draw_scale = scale;
    }
    pub fn set_snap(&mut self, snap: SnapValue) {
        self.snap = snap;
    }
    pub fn get_snap(&self) -> SnapValue {
        self.snap
    }
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }
    pub fn is_active(&self) -> bool {
        self.active
    }
    pub fn set_magnetized(&mut self, magnetized: bool) {
        self.magnetized = magnetized;
    }
    pub fn is_magnetized(&self) -> bool {
        self.magnetized
    }
}
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
