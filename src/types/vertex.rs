use kurbo::Vec2;
use std::fmt::Debug;

use crate::types::scalar::Scalar;

pub type ScalarU32 = Scalar<u32>;

#[derive(Debug, Clone)]
pub struct Vertex {
    curr: Vec2,
    saved: Vec2,
    // For rounded apexes
    has_radius: bool,
    radius: Option<ScalarU32>,
    last_radius: Option<ScalarU32>,
}
impl Vertex {
    pub fn new(value: Vec2) -> Self {
        Self {
            saved: value,
            curr: value,
            has_radius: false,
            radius: None,
            last_radius: None,
        }
    }
    pub fn new_from_coords(x: f64, y: f64) -> Self {
        Self {
            saved: Vec2::new(x, y),
            curr: Vec2::new(x, y),
            has_radius: false,
            radius: None,
            last_radius: None,
        }
    }
    pub fn save(&mut self) {
        self.saved = self.curr;
    }
    pub fn add(&mut self, value: Vec2) {
        self.curr = self.saved + value;
    }

    pub fn change_apex_type(&mut self) {
        if self.has_radius {
            if self.radius.is_none() {
                if self.last_radius.is_none() {
                    self.radius = Some(Scalar::new(10, 1, u32::MAX, 10));
                } else {
                    self.radius = self.last_radius.clone();
                }
            } else {
                self.last_radius = self.radius.clone();
                self.radius = None;
            }
        }
    }
    pub fn enable_radius(&mut self) {
        self.has_radius = true;
    }
    pub fn set_radius_value(&mut self, value: Option<u32>) {
        if let Some(radius) = value {
            self.has_radius = true;
            self.radius = Some(Scalar::new(radius, 10, u32::MAX, 10));
        } else if self.has_radius {
            self.radius = None;
        }
    }
    pub fn curr(&self) -> Vec2 {
        self.curr
    }
    pub fn set_curr(&mut self, value: Vec2) {
        self.curr = value;
    }

    pub fn saved(&self) -> Vec2 {
        self.saved
    }
    pub fn set_saved(&mut self, value: Vec2) {
        self.saved = value;
    }
    pub fn set_saved_x(&mut self, x: f64) {
        self.saved.x = x;
    }
    pub fn set_saved_y(&mut self, y: f64) {
        self.saved.y = y;
    }
    pub fn get_radius(&self) -> Option<u32> {
        if self.has_radius {
            if let Some(radius) = &self.radius {
                return Some(radius.curr());
            }
        }
        None
    }
    pub fn inc_radius(&mut self) {
        self.inc_radius_by(0);
    }
    pub fn dec_radius(&mut self) {
        self.dec_radius_by(0);
    }
    /// Increment radius by `step` (0 = use default step from Scalar)
    pub fn inc_radius_by(&mut self, step: u32) {
        if self.has_radius {
            if let Some(rad) = &mut self.radius {
                let s = if step > 0 { step } else { rad.step() };
                let new_val = rad.curr() + s;
                if new_val <= rad.max() {
                    rad.set_curr(new_val);
                }
            }
        }
    }
    /// Decrement radius by `step` (0 = use default step from Scalar). Clamps to min.
    pub fn dec_radius_by(&mut self, step: u32) {
        if self.has_radius {
            if let Some(rad) = &mut self.radius {
                let s = if step > 0 { step } else { rad.step() };
                let new_val = rad.curr().saturating_sub(s);
                let clamped = new_val.max(rad.min());
                rad.set_curr(clamped);
            }
        }
    }
}
