use super::{
    helper_circle::HelperCircle,
    helper_line::HelperLine,
    helpers::{Helper, HelperKind, HelperKindvars},
};
use crate::{
    math::*,
    pools::{Pools, PoolsFunctions},
    traits::*,
    Action, IconsConstruction, KeysStates, Pointer, HS,
};
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
        pools.helpers.delete(self.helper.get_id());
        pools.helpers.create_helpers_magnet_points();
    }

    fn redo(&self, pools: &mut Pools) {
        log!("Redoing shape creation: {:?}", self.helper.get_id());
        pools.helpers.add(self.helper.clone());
        pools.helpers.create_helpers_magnet_points();
    }
}

pub struct DeleteHelperAction {
    pub helpers: Vec<Helper>,
}
impl Action for DeleteHelperAction {
    fn undo(&self, pools: &mut Pools) {
        log!("Undoing shapes creation");
        self.helpers.iter().for_each(|helper| {
            pools.helpers.add(helper.clone());
            pools.helpers.create_helpers_magnet_points();
        });
    }

    fn redo(&self, pools: &mut Pools) {
        log!("Redoing shapes creation");
        self.helpers.iter().for_each(|helper| {
            pools.helpers.delete(helper.get_id());
            pools.helpers.create_helpers_magnet_points();
        });
    }
}

#[derive(Clone, Debug)]
pub struct HelpersPool {
    helpers: HashMap<DHid, Helper>,
    magnet_points: Vec<Vec2>,
}
impl HelpersPool {
    const MAGNET_RADIUS: f64 = 20.;
    pub fn new_helper(icon_helper: IconsConstruction, pos1: Vec2, pos2: Vec2) -> Helper {
        let dhid = DHid::new();
        let helper_kind = match icon_helper {
            IconsConstruction::Line => HelperLine::new(pos1, pos2),
            IconsConstruction::Circle => HelperCircle::new(pos1, pos2),
        };
        Helper::new(dhid, helper_kind)
    }
    pub fn create_helpers_magnet_points(&mut self) {
        self.magnet_points = vec![];
        // Find the intersection of all lines
        let mut lines = vec![];
        for helper in self.helpers.values() {
            if let HelperKind::Line(line) = helper.get_kind() {
                lines.push(line);
            }
        }
        if lines.len() > 1 {
            for i in 0..lines.len() {
                for j in i + 1..lines.len() {
                    if let Some(point) = line_line_intersection(
                        lines[i].get_position(),
                        lines[i].get_angle(),
                        lines[j].get_position(),
                        lines[j].get_angle(),
                    ) {
                        self.magnet_points.push(point);
                    }
                }
            }
        }
        // Find the intersection of all circles
        let mut circles = vec![];
        for helper in self.helpers.values() {
            if let HelperKind::Circle(circle) = helper.get_kind() {
                circles.push(circle);
            }
        }
        if circles.len() > 1 {
            for i in 0..circles.len() {
                for j in i + 1..circles.len() {
                    if let Some((point1, o_point2)) = circle_circle_intersection(
                        circles[i].get_position(),
                        circles[i].get_radius(),
                        circles[j].get_position(),
                        circles[j].get_radius(),
                    ) {
                        self.magnet_points.push(point1);
                        if let Some(point2) = o_point2 {
                            self.magnet_points.push(point2);
                        }
                    }
                }
            }
        }
        // Find the intersection of all lines and circles
        for line in lines {
            for circle in circles.iter() {
                if let Some((point1, o_point2)) = line_circle_intersection_with_angle(
                    line.get_position(),
                    line.get_angle(),
                    circle.get_position(),
                    circle.get_radius(),
                ) {
                    self.magnet_points.push(point1);
                    if let Some(point2) = o_point2 {
                        self.magnet_points.push(point2);
                    }
                }
            }
        }
        // Always add origin
        self.magnet_points.push(Vec2::ZERO);
    }
    pub fn magnet_points(&self) -> impl Iterator<Item = &Vec2> {
        self.magnet_points.iter()
    }
    pub fn magnet_to_point(&self, pointer: &mut Pointer) {
        pointer.set_magnetized(false);
        for point in self.magnet_points.iter() {
            if (pointer.pos() - *point).hypot() < Self::MAGNET_RADIUS {
                pointer.set_pos(*point);
                pointer.set_magnetized(true);
                break;
            }
        }
    }
}

impl PoolsFunctions for HelpersPool {
    type Id = DHid;
    type Pool = HelpersPool;
    type Object = Helper;
    type ObjectKindvars = HelperKindvars;

    fn new() -> HelpersPool {
        HelpersPool {
            helpers: HashMap::new(),
            magnet_points: vec![Vec2::ZERO],
        }
    }
    // Methods
    fn duplicate(&mut self, shapes: Vec<Helper>) -> Vec<Helper> {
        use SetEntityState::*;
        let mut new_helpers = vec![];
        for mut helper in shapes.into_iter() {
            helper.set_new_id(DHid::new());
            helper.get_kind_mut().set_state(SetSelect(true));
            self.add(helper.clone());
            new_helpers.push(helper);
        }
        new_helpers
    }
    fn add(&mut self, helper: Helper) {
        self.helpers.insert(helper.get_id(), helper);
    }
    fn delete(&mut self, dhid: DHid) -> Option<Helper> {
        self.helpers.remove(&dhid)
    }
    fn get(&self, target_dhid: DHid) -> Option<&Helper> {
        self.helpers.get(&target_dhid)
    }
    fn get_mut(&mut self, target_dhid: DHid) -> Option<&mut Helper> {
        self.helpers.get_mut(&target_dhid)
    }
    fn iter(&self) -> impl Iterator<Item = (&DHid, &Helper)> {
        self.helpers.iter()
    }
    fn iter_mut(&mut self) -> impl Iterator<Item = (&DHid, &mut Helper)> {
        self.helpers.iter_mut()
    }
    fn values(&self) -> impl Iterator<Item = &Helper> {
        self.helpers.values()
    }
    fn values_mut(&mut self) -> impl Iterator<Item = &mut Helper> {
        self.helpers.values_mut()
    }

    fn save_vars(&mut self) {
        self.helpers.values_mut().for_each(|helper| {
            helper.get_kind_mut().save_vars();
        });
    }

    fn set_states_from_pos(&mut self, pointer: &mut Pointer, hors: HS) -> bool {
        use GetEntityState::*;
        use SetEntityStateFromPos::*;
        use HS::*;
        if let Highlight = hors {
            self.helpers.values_mut().for_each(|helper| {
                helper
                    .get_kind_mut()
                    .set_state_from_pos(pointer, HighliFromPos)
            });
            for helper in self.helpers.values_mut() {
                if helper.get_kind().get_state(IsHighligh).is_some() {
                    // return Some(helper.get_kind().get_position());
                    return true;
                }
            }
        } else {
            self.helpers.values_mut().for_each(|helper| {
                helper
                    .get_kind_mut()
                    .set_state_from_pos(pointer, SelectFromPos);
            });
            for helper in self.helpers.values_mut() {
                if helper.get_kind().get_state(IsSelected).is_some() {
                    // return Some(helper.get_kind().get_position());
                    return true;
                }
            }
        }
        false
    }
    fn set_state(&mut self, value: bool, hors: HS) {
        use SetEntityState::*;
        match hors {
            HS::Highlight => {
                self.helpers.values_mut().for_each(|helper| {
                    helper.get_kind_mut().set_state(SetHighli(value));
                });
            }
            HS::Select => {
                self.helpers.values_mut().for_each(|helper| {
                    helper.get_kind_mut().set_state(SetSelect(value));
                });
            }
        }
    }
    fn get_state(&self, hors: HS) -> Vec<DHid> {
        use GetEntityState::*;
        let mut result = vec![];
        match hors {
            HS::Highlight => {
                for helper in self.helpers.values() {
                    if helper.get_kind().get_state(IsHighligh).is_some() {
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
    fn get_state_if_one(&self, hors: HS) -> Option<DHid> {
        let result = self.get_state(hors);
        if result.len() == 1 {
            Some(result[0])
        } else {
            None
        }
    }
    fn get_state_and_vars(&self, hors: HS) -> Vec<(DHid, HelperKindvars)> {
        use GetEntityState::*;
        let mut result = vec![];
        match hors {
            HS::Highlight => {
                for helper in self.helpers.values() {
                    if helper.get_kind().get_state(IsHighligh).is_some() {
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
    fn set_id_state(&mut self, id: DHid, value: bool, hors: HS) {
        use SetEntityState::*;
        if let Some(helper) = self.helpers.get_mut(&id) {
            match hors {
                HS::Highlight => {
                    helper.get_kind_mut().set_state(SetHighli(value));
                }
                HS::Select => {
                    helper.get_kind_mut().set_state(SetSelect(value));
                }
            }
        }
    }
    fn set_modifiers_states_from_pos(&mut self, pointer: &mut Pointer, hors: HS) -> bool {
        use GetEntityState::*;
        use SetEntityStateFromPos::*;
        match hors {
            HS::Highlight => {
                self.helpers.values_mut().for_each(|helper| {
                    helper
                        .get_kind_mut()
                        .set_state_from_pos(pointer, HighliModifierFromPos);
                });
                for helper in self.helpers.values_mut() {
                    if helper.get_kind().get_state(IsAnyModifierHighligh).is_some() {
                        return true;
                    }
                }
            }
            HS::Select => {
                self.helpers.values_mut().for_each(|helper| {
                    helper
                        .get_kind_mut()
                        .set_state_from_pos(pointer, SelectModifierFromPos);
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
    fn set_modifiers_state(&mut self, value: bool, hors: HS) {
        use SetEntityState::*;
        match hors {
            HS::Highlight => {
                self.helpers.values_mut().for_each(|helper| {
                    helper.get_kind_mut().set_state(HighliAllModifiers(value));
                });
            }
            HS::Select => {
                self.helpers.values_mut().for_each(|helper| {
                    helper.get_kind_mut().set_state(SelectAllModifiers(value));
                });
            }
        }
    }
    fn get_first_selected_modifier_vars(&self) -> Option<(DHid, HelperKindvars)> {
        use GetEntityState::*;
        for helper in self.helpers.values() {
            if helper.get_kind().get_state(IsAnyModifierSelected).is_some() {
                return Some((helper.get_id(), helper.get_kind().get_vars()));
            }
        }
        None
    }

    fn move_position(
        &mut self,
        dhid: DHid,
        pointer: &mut Pointer,
        keys_states: KeysStates,
    ) -> bool {
        if let Some(helper) = self.helpers.get_mut(&dhid) {
            return helper.get_kind_mut().move_position(pointer, keys_states);
        }
        false
    }
    fn move_modifier(
        &mut self,
        dhid: DHid,
        pointer: &mut Pointer,
        keys_states: KeysStates,
    ) -> bool {
        if let Some(helper) = self.helpers.get_mut(&dhid) {
            helper.get_kind_mut().move_modifier(pointer, keys_states);
        }
        false
    }
    fn delete_selection(&mut self) -> Option<Vec<Helper>> {
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
