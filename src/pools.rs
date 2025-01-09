use kurbo::Vec2;

use crate::{
    helpers::{helpers::Helper, helpers_pool::HelpersPool},
    shapes::{shapes::BasicShape, shapes_pool::ShapesPool},
    traits::*,
    HS,
};

#[derive(Clone, Debug)]
pub enum DrawObjects {
    Shape(BasicShape),
    Helper(Helper),
    Nope,
}
#[allow(dead_code)]
impl DrawObjects {
    pub fn get_shape_into(&self) -> Option<BasicShape> {
        match self {
            DrawObjects::Shape(s) => Some(s.clone()),
            _ => None,
        }
    }
    pub fn get_shape(&self) -> Option<&BasicShape> {
        match self {
            DrawObjects::Shape(s) => Some(s),
            _ => None,
        }
    }
    pub fn get_shape_mut(&mut self) -> Option<&mut BasicShape> {
        match self {
            DrawObjects::Shape(s) => Some(s),
            _ => None,
        }
    }
    pub fn get_helper(&self) -> Option<&Helper> {
        match self {
            DrawObjects::Helper(h) => Some(h),
            _ => None,
        }
    }
    pub fn get_helper_mut(&mut self) -> Option<&mut Helper> {
        match self {
            DrawObjects::Helper(h) => Some(h),
            _ => None,
        }
    }
    pub fn get_helper_into(&self) -> Option<Helper> {
        match self {
            DrawObjects::Helper(h) => Some(h.clone()),
            _ => None,
        }
    }
    pub fn set_shape(&mut self, shape: BasicShape) {
        *self = DrawObjects::Shape(shape);
    }
    pub fn set_helper(&mut self, helper: Helper) {
        *self = DrawObjects::Helper(helper);
    }
}

#[derive(Clone, Debug)]
pub struct Pools {
    pub sh: ShapesPool,
    pub hp: HelpersPool,
}
impl Pools {
    pub fn new() -> Self {
        Self {
            sh: ShapesPool::new(),
            hp: HelpersPool::new(),
        }
    }
    pub fn save_vars(&mut self) {
        self.hp.values_mut().for_each(|helper| {
            helper.get_kind_mut().save_vars();
        });
        self.sh.values_mut().for_each(|helper| {
            helper.get_kind_mut().save_vars();
        });
    }
    pub fn hs_objects_in_order(&mut self, cursor_pos: Vec2, grab: f64, hors: HS) {
        if self.sh.set_hs_modifiers_from_pos(cursor_pos, grab, hors)
            || self.sh.set_hs_from_pos(cursor_pos, hors)
            || self.hp.set_hs_modifiers_from_pos(cursor_pos, grab, hors)
        {
            return;
        }
        self.hp.set_hs_from_pos(cursor_pos, hors);
    }
    pub fn clear_all_hs(&mut self) {
        self.sh.set_hs(false, HS::Select);
        self.sh.set_hs_modifiers(false, HS::Select);
        self.hp.set_hs(false, HS::Select);
        self.hp.set_hs_modifiers(false, HS::Select);
    }
    pub fn select_all_shapes_connected(&mut self) -> bool {
        self.sh.select_all_connected()
    }
}
