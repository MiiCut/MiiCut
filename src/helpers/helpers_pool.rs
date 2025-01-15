use super::{
    helper_circle::HelperCircle,
    helper_line::HelperLine,
    helpers::{Helper, HelperKind, HelperKindvars},
};
use crate::{pools::Pools, traits::*, Action, IconsConstruction, HS};
use kurbo::Vec2;
use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct AddHelperAction {
    pub helper: Helper,
}
impl Action for AddHelperAction {
    fn undo(&self, pools: &mut Pools) {
        log!("Undoing shape creation: {:?}", self.helper.get_id());
        pools.helpers.delete_helper(self.helper.get_id());
    }

    fn redo(&self, pools: &mut Pools) {
        log!("Redoing shape creation: {:?}", self.helper.get_id());
        pools.helpers.add_helper(self.helper.clone());
    }
}

pub struct DeleteHelperAction {
    pub helpers: Vec<Helper>,
}
impl Action for DeleteHelperAction {
    fn undo(&self, pools: &mut Pools) {
        log!("Undoing shapes creation");
        self.helpers.iter().for_each(|helper| {
            pools.helpers.add_helper(helper.clone());
        });
    }

    fn redo(&self, pools: &mut Pools) {
        log!("Redoing shapes creation");
        self.helpers.iter().for_each(|helper| {
            pools.helpers.delete_helper(helper.get_id());
        });
    }
}

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
    pub fn new_helper(icon_helper: IconsConstruction, pos1: Vec2, pos2: Vec2) -> Helper {
        let dhid = DHid::new();
        let helper_kind = match icon_helper {
            IconsConstruction::Line => HelperLine::new(pos1, pos2),
            IconsConstruction::Circle => HelperCircle::new(pos1, pos2),
        };
        Helper::new(dhid, helper_kind)
    }
    // Methods
    pub fn duplicate_helpers(&mut self, shapes: Vec<Helper>) -> Vec<Helper> {
        use SetEntityState::*;
        let mut new_helpers = vec![];
        for mut helper in shapes.into_iter() {
            helper.set_new_id(DHid::new());
            helper.get_kind_mut().set_state(SetSelect(true));
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

    pub fn save_vars(&mut self) {
        self.helpers.values_mut().for_each(|helper| {
            helper.get_kind_mut().save_vars();
        });
    }

    pub fn set_hs_from_pos(&mut self, pos: Vec2, snap: f64, hors: HS) -> Option<Vec2> {
        use GetEntityState::*;
        use SetEntityState::*;
        use HS::*;
        if let Highlight = hors {
            self.helpers.values_mut().for_each(|helper| {
                helper
                    .get_kind_mut()
                    .set_state(HighlightFromPos(pos, snap, 5.0))
            });
            for helper in self.helpers.values_mut() {
                if helper.get_kind().get_state(IsHighlighted).is_some() {
                    return Some(helper.get_kind().get_position());
                }
            }
        } else {
            self.helpers.values_mut().for_each(|helper| {
                helper
                    .get_kind_mut()
                    .set_state(SelectFromPos(pos, snap, 5.0));
            });
            for helper in self.helpers.values_mut() {
                if helper.get_kind().get_state(IsSelected).is_some() {
                    return Some(helper.get_kind().get_position());
                }
            }
        }
        None
    }
    pub fn set_hs(&mut self, value: bool, hors: HS) {
        use SetEntityState::*;
        match hors {
            HS::Highlight => {
                self.helpers.values_mut().for_each(|helper| {
                    helper.get_kind_mut().set_state(SetHighlight(value));
                });
            }
            HS::Select => {
                self.helpers.values_mut().for_each(|helper| {
                    helper.get_kind_mut().set_state(SetSelect(value));
                });
            }
        }
    }
    pub fn get_hs(&self, hors: HS) -> Vec<DHid> {
        use GetEntityState::*;
        let mut result = vec![];
        match hors {
            HS::Highlight => {
                for helper in self.helpers.values() {
                    if helper.get_kind().get_state(IsHighlighted).is_some() {
                        result.push(helper.get_id());
                    }
                }
            }
            HS::Select => {
                for helper in self.helpers.values() {
                    if helper.get_kind().get_state(IsSelected).is_some() {
                        result.push(helper.get_id());
                    }
                }
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
        use GetEntityState::*;
        let mut result = vec![];
        match hors {
            HS::Highlight => {
                for helper in self.helpers.values() {
                    if helper.get_kind().get_state(IsHighlighted).is_some() {
                        result.push((helper.get_id(), helper.get_kind().get_vars()));
                    }
                }
            }
            HS::Select => {
                for helper in self.helpers.values() {
                    if helper.get_kind().get_state(IsSelected).is_some() {
                        result.push((helper.get_id(), helper.get_kind().get_vars()));
                    }
                }
            }
        }
        result
    }
    pub fn set_hs_from_dhid(&mut self, dhid: DHid, value: bool, hors: HS) {
        use SetEntityState::*;
        if let Some(helper) = self.helpers.get_mut(&dhid) {
            match hors {
                HS::Highlight => {
                    helper.get_kind_mut().set_state(SetHighlight(value));
                }
                HS::Select => {
                    helper.get_kind_mut().set_state(SetSelect(value));
                }
            }
        }
    }
    pub fn set_mod_hs_from_pos(&mut self, pos: Vec2, snap: f64, _precision: f64, hors: HS) -> bool {
        use GetEntityState::*;
        use SetEntityState::*;
        match hors {
            HS::Highlight => {
                self.helpers.values_mut().for_each(|helper| {
                    helper
                        .get_kind_mut()
                        .set_state(HighlightModifierFromPos(pos, snap, 5.0));
                });
                for helper in self.helpers.values_mut() {
                    if helper
                        .get_kind()
                        .get_state(IsAnyModifierHighlighted)
                        .is_some()
                    {
                        return true;
                    }
                }
            }
            HS::Select => {
                self.helpers.values_mut().for_each(|helper| {
                    helper
                        .get_kind_mut()
                        .set_state(SelectModifierFromPos(pos, snap, 5.0));
                });
                for shape in self.helpers.values_mut() {
                    if shape.get_kind().get_state(IsAnyModifierSelected).is_some() {
                        return true;
                    }
                }
            }
        }
        false
    }
    pub fn set_mod_hs(&mut self, value: bool, hors: HS) {
        use SetEntityState::*;
        match hors {
            HS::Highlight => {
                self.helpers.values_mut().for_each(|helper| {
                    helper
                        .get_kind_mut()
                        .set_state(HighlightAllModifiers(value));
                });
            }
            HS::Select => {
                self.helpers.values_mut().for_each(|helper| {
                    helper.get_kind_mut().set_state(SelectAllModifiers(value));
                });
            }
        }
    }
    pub fn get_first_selected_modifier_vars(&self) -> Option<(DHid, HelperKindvars)> {
        use GetEntityState::*;
        for helper in self.helpers.values() {
            if helper.get_kind().get_state(IsAnyModifierSelected).is_some() {
                return Some((helper.get_id(), helper.get_kind().get_vars()));
            }
        }
        None
    }

    pub fn move_position(
        &mut self,
        dhid: DHid,
        pos_init: Vec2,
        pos: Vec2,
        snap: f64,
        _shift_pressed: bool,
    ) -> Option<Vec2> {
        self.helpers.get_mut(&dhid).and_then(|helper| {
            return helper.get_kind_mut().move_position(pos - pos_init, snap);
        })
    }
    pub fn move_modifier(
        &mut self,
        dhid: DHid,
        pos_init: Vec2,
        pos: Vec2,
        snap: f64,
        shift_pressed: bool,
    ) {
        if let Some(helper) = self.helpers.get_mut(&dhid) {
            helper
                .get_kind_mut()
                .move_modifier(pos_init, pos, snap, shift_pressed);
        }
    }

    pub fn delete_selection(&mut self) -> Option<Vec<Helper>> {
        use GetEntityState::*;
        let mut helpers_deleted = vec![];

        for shape in self.helpers.values_mut() {
            if shape.get_kind_mut().get_state(IsSelected).is_some() {
                helpers_deleted.push(shape.clone());
            }
        }

        self.helpers
            .retain(|_, v| !v.get_kind_mut().get_state(IsSelected).is_some());

        if !helpers_deleted.is_empty() {
            Some(helpers_deleted)
        } else {
            None
        }
    }
    pub fn magnet_to_helpers(&mut self, pos: Vec2) -> Vec2 {
        let mut result: Vec<(&HelperKind, Vec2)> = vec![];
        for helper in self.helpers.values() {
            if let Some(pos) = helper.get_kind().magnet_to(pos) {
                result.push((helper.get_kind(), pos));
            }
        }
        // If result contains at least one Point, return the nearest
        let result_lines: Vec<Vec2> = result
            .iter()
            .filter_map(|(kind, pos)| {
                if let HelperKind::Line(_) = kind {
                    Some(pos.clone())
                } else {
                    None
                }
            })
            .collect();
        if result_lines.len() > 0 {
            let mut min_dist = f64::MAX;
            let mut min_pos = Vec2::ZERO;
            for pos_result in result_lines {
                let dist = (pos_result - pos).hypot();
                if dist < min_dist {
                    min_dist = dist;
                    min_pos = pos_result;
                }
            }
            log!("Magnet to line: {:?}", min_pos);
            return min_pos;
        }
        pos
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
impl NewId for DHid {
    fn new() -> Self {
        DHid {
            id: COUNTER_DRAW_HELPERS.fetch_add(1, Ordering::Relaxed),
        }
    }
}
