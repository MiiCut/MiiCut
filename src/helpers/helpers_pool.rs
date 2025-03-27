use super::helpers::{Helper, HelperKind, HelperKindvars};
use crate::{
    math::*,
    pools::{Pools, PoolsFunctions},
    traits::*,
    Action, KeysStates, Pointer, HS,
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
        pools.helpers.create_magnet_points();
    }
    fn redo(&self, pools: &mut Pools) {
        log!("Redoing shape creation: {:?}", self.helper.get_id());
        pools.helpers.add(self.helper.clone());
        pools.helpers.create_magnet_points();
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
            pools.helpers.create_magnet_points();
        });
    }
    fn redo(&self, pools: &mut Pools) {
        log!("Redoing shapes creation");
        self.helpers.iter().for_each(|helper| {
            pools.helpers.delete(helper.get_id());
            pools.helpers.create_magnet_points();
        });
    }
}

#[derive(Clone, Debug)]
pub struct HelpersPool {
    helpers: HashMap<DHid, Helper>,
    magnet_points: Vec<Vec2>,
}
impl HelpersPool {
    const MAGNET_RADIUS: f64 = 10.;

    pub fn create_magnet_points(&mut self) {
        self.magnet_points = vec![];
        // Find the intersection of all segments
        let mut segs = vec![];
        for helper in self.helpers.values() {
            if let HelperKind::Segment(seg) = helper.get_kind() {
                self.magnet_points.push(seg.get_seg_bdl().s());
                self.magnet_points.push(seg.get_seg_bdl().e());
                segs.push(seg);
            }
        }
        if segs.len() > 1 {
            for i in 0..segs.len() {
                for j in i + 1..segs.len() {
                    if let Some(point) = segment_intersection(
                        segs[i].get_seg_bdl().s(),
                        segs[i].get_seg_bdl().e(),
                        segs[j].get_seg_bdl().s(),
                        segs[j].get_seg_bdl().e(),
                    ) {
                        self.magnet_points.push(point);
                    }
                }
            }
        };

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
        // Find the intersection of all segments and circles
        for seg in segs {
            for circle in circles.iter() {
                segment_circle_intersections(
                    seg.get_seg_bdl().s(),
                    seg.get_seg_bdl().e(),
                    circle.get_position(),
                    circle.get_radius(),
                )
                .iter()
                .for_each(|point| self.magnet_points.push(*point));
            }
        }
        // Always add origin
        self.magnet_points.push(Vec2::ZERO);
    }
    pub fn magnet_points(&self) -> Vec<Vec2> {
        self.magnet_points.clone()
    }
    pub fn magnet_to_points(&self, pointer: &mut Pointer, keys_states: KeysStates) {
        for point in self.magnet_points.iter() {
            if (pointer.pos() - *point).hypot() < Self::MAGNET_RADIUS {
                if !keys_states.alt_pressed {
                    pointer.set_pos(*point);
                    pointer.set_magnetized(true);
                    return;
                }
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
        use HS::*;
        let mut new_helpers = vec![];
        for mut helper in shapes.into_iter() {
            helper.set_new_id(DHid::new());
            helper.get_kind_mut().set_state(SetHS(Select, true));
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

    fn set_state(&mut self, ses_hs: SetEntityState) {
        self.helpers.values_mut().for_each(|helper| {
            helper.get_kind_mut().set_state(ses_hs);
        });
    }
    fn get_state(&mut self, hs: HS) -> Vec<DHid> {
        use GetEntityState::*;
        let mut result = vec![];
        for helper in self.helpers.values_mut() {
            if helper.get_kind_mut().get_state(IsHS(hs)) {
                result.push(helper.get_id());
            }
        }
        result
    }
    fn set_states_from_pos(
        &mut self,
        pointer: &mut Pointer,
        keys_states: KeysStates,
        ses_hs: SetEntityStateFromPos,
    ) -> bool {
        for helper in self.helpers.values_mut() {
            if helper
                .get_kind_mut()
                .set_state_from_pos(pointer, keys_states, ses_hs)
            {
                return true;
            }
        }
        false
    }

    fn get_state_if_one(&mut self, hors: HS) -> Option<DHid> {
        let result = self.get_state(hors);
        if result.len() == 1 {
            Some(result[0])
        } else {
            None
        }
    }
    fn get_state_and_vars(&mut self, hs: HS) -> Vec<(DHid, HelperKindvars)> {
        use GetEntityState::*;
        let mut result = vec![];
        for helper in self.helpers.values_mut() {
            if helper.get_kind_mut().get_state(IsHS(hs)) {
                result.push((helper.get_id(), helper.get_kind().get_vars()));
            }
        }
        result
    }

    fn get_first_selected_modifier_vars(&mut self) -> Option<(DHid, HelperKindvars)> {
        use GetEntityState::*;
        use HS::*;
        for helper in self.helpers.values_mut() {
            if helper.get_kind_mut().get_state(IsAControlHS(Select)) {
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
            helper.get_kind_mut().move_controls(pointer, keys_states);
        }
        false
    }
    fn delete_selection(&mut self) -> Option<Vec<Helper>> {
        use GetEntityState::*;
        use HS::*;
        let mut helpers_deleted = vec![];

        for shape in self.helpers.values_mut() {
            if shape.get_kind_mut().get_state(IsHS(Select)) {
                helpers_deleted.push(shape.clone());
            }
        }

        self.helpers
            .retain(|_, v| !v.get_kind_mut().get_state(IsHS(Select)));

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
