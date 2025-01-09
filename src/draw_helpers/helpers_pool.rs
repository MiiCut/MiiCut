use kurbo::Vec2;

use crate::{draw_helpers::helpers::HelperKindFuncs, HS};

use super::helpers::{Helper, HelperKindvars};
use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Clone, Debug)]
pub struct HelpersPool {
    helpers: HashMap<DHid, Helper>,
}
impl HelpersPool {
    // Static methods
    pub fn new() -> HelpersPool {
        HelpersPool {
            helpers: HashMap::new(),
        }
    }

    pub fn duplicate_helpers(&mut self, shapes: Vec<Helper>) -> Vec<Helper> {
        let mut new_helpers = vec![];
        for mut helper in shapes.into_iter() {
            helper.set_new_id(DHid::new());
            helper.get_kind_mut().set_hs(true, HS::Select);
            self.add_helper(helper.clone());
            new_helpers.push(helper);
        }
        new_helpers
    }

    pub fn add_helper(&mut self, helper: Helper) {
        self.helpers.insert(helper.get_id(), helper);
    }

    pub fn delete_helper(&mut self, dhid: DHid) -> Option<Helper> {
        self.helpers.remove(&dhid)
    }

    pub fn get_helper(&self, target_dhid: DHid) -> Option<&Helper> {
        self.helpers.get(&target_dhid)
    }
    pub fn get_helper_mut(&mut self, target_dhid: DHid) -> Option<&mut Helper> {
        self.helpers.get_mut(&target_dhid)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&DHid, &Helper)> {
        self.helpers.iter()
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&DHid, &mut Helper)> {
        self.helpers.iter_mut()
    }
    pub fn values(&self) -> impl Iterator<Item = &Helper> {
        self.helpers.values()
    }
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Helper> {
        self.helpers.values_mut()
    }

    pub fn set_hs_from_pos(&mut self, pos: Vec2, hors: HS) -> bool {
        use HS::*;
        if let Highlight = hors {
            let mut res = false;
            self.helpers.values_mut().for_each(|helper| {
                res |= helper.get_kind_mut().set_hs_from_pos(pos, Highlight);
            });
            return res;
        } else {
            let mut res = false;
            self.helpers.values_mut().for_each(|helper| {
                res |= helper.get_kind_mut().set_hs_from_pos(pos, Select);
            });
            return res;
        }
    }
    pub fn set_hs(&mut self, value: bool, hors: HS) {
        self.helpers.values_mut().for_each(|helper| {
            helper.get_kind_mut().set_hs(value, hors);
        });
    }

    pub fn set_hs_modifiers_from_pos(&mut self, pos: Vec2, _precision: f64, hors: HS) -> bool {
        let mut setted = false;
        self.helpers.values_mut().for_each(|helper| {
            setted |= helper.get_kind_mut().set_hs_modifiers_from_pos(pos, hors);
        });
        setted
    }
    pub fn set_hs_modifiers(&mut self, value: bool, hors: HS) {
        self.helpers.values_mut().for_each(|helper| {
            helper.get_kind_mut().set_hs_modifiers(value, hors);
        });
    }

    pub fn move_positions(
        &mut self,
        dhid_sel: Vec<DHid>,
        pos_init: Vec2,
        pos: Vec2,
        _shift_pressed: bool,
    ) {
        dhid_sel.into_iter().for_each(|dhid| {
            if let Some(helper) = self.helpers.get_mut(&dhid) {
                helper.get_kind_mut().move_position(pos - pos_init);
            }
        });
    }
    pub fn move_modifier(&mut self, dhid: DHid, pos_init: Vec2, pos: Vec2, shift_pressed: bool) {
        if let Some(helper) = self.helpers.get_mut(&dhid) {
            helper
                .get_kind_mut()
                .move_modifier(pos_init, pos, shift_pressed);
        }
    }

    pub fn get_hs(&self, hors: HS) -> Vec<DHid> {
        let mut result = vec![];
        for helper in self.helpers.values() {
            if helper.get_kind().get_hs(hors) {
                result.push(helper.get_id());
            }
        }
        result
    }
    pub fn get_hs_if_one(&self, hors: HS) -> Option<DHid> {
        let result = self.get_hs(hors);
        if result.len() == 1 {
            Some(result[0])
        } else {
            None
        }
    }
    pub fn get_hs_vars(&self, hors: HS) -> Vec<(DHid, HelperKindvars)> {
        let mut result = vec![];
        for helper in self.helpers.values() {
            if helper.get_kind().get_hs(hors) {
                result.push((helper.get_id(), helper.get_kind().get_vars()));
            }
        }
        result
    }
    pub fn set_hs_from_dhid(&mut self, dhid: DHid, value: bool, hors: HS) {
        if let Some(helper) = self.helpers.get_mut(&dhid) {
            helper.get_kind_mut().set_hs(value, hors);
        }
    }
    pub fn get_first_selected_modifier_vars(&self) -> Option<(DHid, HelperKindvars)> {
        for helper in self.helpers.values() {
            if helper.get_kind().get_hs_modifiers(HS::Select) {
                return Some((helper.get_id(), helper.get_kind().get_vars()));
            }
        }
        None
    }
    pub fn delete_helpers_selected(&mut self) -> Vec<Helper> {
        let shapes_deleted: Vec<Helper> = self
            .helpers
            .iter()
            .filter(|(_, helper)| helper.get_kind().get_hs(HS::Select))
            .map(|(_, helper)| helper.clone())
            .collect();
        self.helpers.retain(|_, v| !v.get_kind().get_hs(HS::Select));
        shapes_deleted
    }
}
static COUNTER_DRAW_HELPERS: AtomicUsize = AtomicUsize::new(0);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd)]
pub struct DHid {
    id: usize,
}
impl Deref for DHid {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.id
    }
}
impl DerefMut for DHid {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.id
    }
}
impl Display for DHid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
impl DHid {
    pub fn new() -> DHid {
        DHid {
            id: COUNTER_DRAW_HELPERS.fetch_add(1, Ordering::Relaxed),
        }
    }
}
