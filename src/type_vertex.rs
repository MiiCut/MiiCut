use kurbo::Vec2;
use std::{collections::HashSet, fmt::Debug};

use crate::{
    type_scalar::Scalar,
    types::{EUId, VUId},
};

pub type ScalarU32 = Scalar<u32>;

#[derive(Debug, Clone)]
pub struct Vertex {
    curr: Vec2,
    saved: Vec2,
    // For rounded apexes
    has_radius: bool,
    radius: Option<ScalarU32>,
    last_radius: Option<ScalarU32>,
    // List of binded vertices
    binds: HashSet<(EUId, VUId)>,
}
impl Vertex {
    pub fn new(value: Vec2) -> Self {
        Self {
            saved: value,
            curr: value,
            has_radius: false,
            radius: None,
            last_radius: None,
            binds: HashSet::new(),
        }
    }
    pub fn new_from_coords(x: f64, y: f64) -> Self {
        Self {
            saved: Vec2::new(x, y),
            curr: Vec2::new(x, y),
            has_radius: false,
            radius: None,
            last_radius: None,
            binds: HashSet::new(),
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
                    self.radius = Some(Scalar::new(10, 10, u32::MAX, 10));
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
        if self.has_radius {
            if let Some(rad) = &mut self.radius {
                rad.incr();
            }
        }
    }
    pub fn dec_radius(&mut self) {
        if self.has_radius {
            if let Some(rad) = &mut self.radius {
                rad.decr();
            }
        }
    }
    pub fn get_binds(&self) -> &HashSet<(EUId, VUId)> {
        &self.binds
    }
    pub fn has_bind(&self, eu_id: EUId, vu_id: VUId) -> bool {
        self.binds.contains(&(eu_id, vu_id))
    }
    pub fn add_bind(&mut self, eu_id: EUId, vu_id: VUId) {
        self.binds.insert((eu_id, vu_id));
    }
    pub fn remove_bind(&mut self, eu_id: EUId, vu_id: VUId) {
        self.binds.remove(&(eu_id, vu_id));
    }
    pub fn clear_binds(&mut self) {
        self.binds.clear();
    }
}
