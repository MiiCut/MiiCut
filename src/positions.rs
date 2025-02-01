// use crate::math::*;
use kurbo::Vec2;

use crate::pools::HS;

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
#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct Status {
    highlighted: bool,
    selected: bool,
}
impl Status {
    pub fn is_hs(&self, hs: HS) -> bool {
        match hs {
            HS::Highlight => self.highlighted,
            HS::Select => self.selected,
        }
    }
    pub fn set_hs(&mut self, hs: HS, value: bool) {
        match hs {
            HS::Highlight => self.highlighted = value,
            HS::Select => self.selected = value,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Value {
    pub saved_val: f64,
    pub last_val: f64,
    pub value: f64,
    highlighted: bool,
    selected: bool,
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
    pub fn is_hs(&self, hs: HS) -> bool {
        match hs {
            HS::Highlight => self.highlighted,
            HS::Select => self.selected,
        }
    }
    pub fn set_hs(&mut self, hs: HS, value: bool) {
        match hs {
            HS::Highlight => self.highlighted = value,
            HS::Select => self.selected = value,
        }
    }
}
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ValueBool {
    pub saved_val: bool,
    pub last_val: bool,
    pub value: bool,
    highlighted: bool,
    selected: bool,
}
impl ValueBool {
    pub fn new(value: bool) -> Self {
        Self {
            saved_val: value,
            last_val: value,
            value,
            highlighted: false,
            selected: false,
        }
    }
    pub fn is_hs(&self, hs: HS) -> bool {
        match hs {
            HS::Highlight => self.highlighted,
            HS::Select => self.selected,
        }
    }
    pub fn set_hs(&mut self, hs: HS, value: bool) {
        match hs {
            HS::Highlight => self.highlighted = value,
            HS::Select => self.selected = value,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Position {
    pub saved_pos: Vec2,
    pub last_pos: Vec2,
    pub pos: Vec2,
    pub magnet: bool,
    highlighted: bool,
    selected: bool,
}

impl Position {
    const GRAB_RADIUS: f64 = 5.;

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
    pub fn is_hs(&self, hs: HS) -> bool {
        match hs {
            HS::Highlight => self.highlighted,
            HS::Select => self.selected,
        }
    }
    pub fn set_hs(&mut self, hs: HS, value: bool) {
        match hs {
            HS::Highlight => self.highlighted = value,
            HS::Select => self.selected = value,
        }
    }
    pub fn set_hs_from_pos(&mut self, hs: HS, pointer: &mut Pointer) {
        let state = (pointer.pos() - self.pos).hypot() < Self::GRAB_RADIUS;
        match hs {
            HS::Highlight => {
                self.highlighted = state;
                if self.highlighted {
                    pointer.set_pos(self.pos);
                }
            }
            HS::Select => {
                self.selected = state;
                if self.selected {
                    pointer.set_pos(self.pos);
                    pointer.save_pos();
                }
            }
        }
    }
    pub fn get_dist_from_pos(&self, pos: Vec2) -> f64 {
        (pos - self.pos).hypot()
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct RadiusValue {
    pub radius: Value,
    pub up: bool,
    pub saved_up: bool,
}
impl RadiusValue {
    const _GRAB_RADIUS: f64 = 5.;

    pub fn new(radius: f64, up: bool) -> Self {
        Self {
            radius: Value::new(radius),
            up,
            saved_up: up,
        }
    }
    pub fn is_hs(&self, hs: HS) -> bool {
        match hs {
            HS::Highlight => self.radius.highlighted,
            HS::Select => self.radius.selected,
        }
    }
    pub fn set_hs(&mut self, hs: HS, value: bool) {
        match hs {
            HS::Highlight => self.radius.highlighted = value,
            HS::Select => self.radius.selected = value,
        }
    }
}
