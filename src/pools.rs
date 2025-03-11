use crate::{
    helpers::{
        helpers::MoveHelpersAction,
        helpers_pool::{AddHelperAction, HelpersPool},
    },
    shapes::{
        shapes::MoveShapesAction,
        shapes_pool::{AddShapeAction, ShapesPool},
    },
    traits::*,
    Action, ClipboardItem, KeysStates, PasteAction, Pointer,
};
use kurbo::Vec2;

pub enum MoveAction {
    Shapes(MoveShapesAction),
    Helpers(MoveHelpersAction),
}
impl Action for MoveAction {
    fn undo(&self, pools: &mut Pools) {
        match self {
            MoveAction::Shapes(action) => action.undo(pools),
            MoveAction::Helpers(action) => action.undo(pools),
        }
    }
    fn redo(&self, pools: &mut Pools) {
        match self {
            MoveAction::Shapes(action) => action.redo(pools),
            MoveAction::Helpers(action) => action.redo(pools),
        }
    }
}

pub enum AddObjectAction {
    Shapes(AddShapeAction),
    Helpers(AddHelperAction),
}
impl Action for AddObjectAction {
    fn undo(&self, pools: &mut Pools) {
        match self {
            AddObjectAction::Shapes(action) => action.undo(pools),
            AddObjectAction::Helpers(action) => action.undo(pools),
        }
    }
    fn redo(&self, pools: &mut Pools) {
        match self {
            AddObjectAction::Shapes(action) => action.redo(pools),
            AddObjectAction::Helpers(action) => action.redo(pools),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct MoveBundle {
    pub pointer: Pointer,
    pub cursor_pos: Vec2,
    pub cursor_pos_dwn: Vec2,
    pub snap_value: f64,
    pub magnet_pos: Vec2,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum HS {
    Highlight,
    Select,
}

#[derive(Clone, Debug)]
pub struct Pools {
    pub shapes: ShapesPool,
    pub helpers: HelpersPool,
}
impl Pools {
    const _GRAB_DIST: f64 = 10.;

    pub fn new() -> Self {
        Self {
            shapes: ShapesPool::new(),
            helpers: HelpersPool::new(),
        }
    }
    pub fn set_objects_states_in_order(
        &mut self,
        pointer: &mut Pointer,
        keys_states: KeysStates,
        hs: HS,
    ) -> bool {
        use SetEntityState::*;
        use SetEntityStateFromPos::*;
        if self
            .shapes
            .set_states_from_pos(pointer, keys_states, SetControlHSFromPos(hs))
        {
            // log!("A");
            self.shapes.set_state(SetHS(hs, false));
            self.helpers.set_state(SetHS(hs, false));
            self.helpers.set_state(SetAllControlsHS(hs, false));
            return true;
        }
        if self
            .helpers
            .set_states_from_pos(pointer, keys_states, SetHSFromPos(hs))
        {
            // log!("B");
            self.shapes.set_state(SetHS(hs, false));
            self.helpers.set_state(SetAllControlsHS(hs, false));
            return true;
        }
        if self
            .helpers
            .set_states_from_pos(pointer, keys_states, SetControlHSFromPos(hs))
        {
            // log!("C");
            self.shapes.set_state(SetHS(hs, false));
            return true;
        }
        if self
            .shapes
            .set_states_from_pos(pointer, keys_states, SetHSFromPos(hs))
        {
            // log!("D");
            return true;
        }

        self.shapes.set_state(SetHS(hs, false));
        self.shapes.set_state(SetAllControlsHS(hs, false));
        self.helpers.set_state(SetHS(hs, false));
        self.helpers.set_state(SetAllControlsHS(hs, false));
        false
    }
    pub fn paste_to_pool(&mut self, clip_item: ClipboardItem) -> Box<PasteAction> {
        match &clip_item {
            ClipboardItem::Shapes((shapes, ..)) => {
                let new_shapes = self.shapes.duplicate(shapes.clone());
                // Push the PasteAction to the undo/redo system
                Box::new(PasteAction {
                    clip_item: ClipboardItem::Shapes((new_shapes, Vec2::ZERO)),
                })
            }
            ClipboardItem::Helpers((helpers, ..)) => {
                let new_helpers = self.helpers.duplicate(helpers.clone());
                // Push the PasteAction to the undo/redo system
                Box::new(PasteAction {
                    clip_item: ClipboardItem::Helpers((new_helpers, Vec2::ZERO)),
                })
            }
        }
    }
    pub fn move_objects(&mut self, pointer: &mut Pointer, keys_states: KeysStates) -> bool {
        if let Some((shid, _)) = self.shapes.get_first_selected_modifier_vars() {
            return self.shapes.move_modifier(shid, pointer, keys_states);
        }

        let shapes_selected = self.shapes.get_state(HS::Select);
        if shapes_selected.len() == 1 {
            return self
                .shapes
                .move_position(shapes_selected[0], pointer, keys_states);
        } else {
            if shapes_selected.len() > 0 {
                let mut moved = false;
                for shid in shapes_selected {
                    moved |= self.shapes.move_position(shid, pointer, keys_states);
                }
                return moved;
            }
        }

        let helpers_selected = self.helpers.get_state(HS::Select);
        if helpers_selected.len() == 1 {
            return self
                .helpers
                .move_position(helpers_selected[0], pointer, keys_states);
        } else {
            if helpers_selected.len() > 0 {
                let mut moved = false;
                for dhid in helpers_selected {
                    moved |= self.helpers.move_position(dhid, pointer, keys_states);
                }
                return moved;
            } else {
                if let Some((hpid, _)) = self.helpers.get_first_selected_modifier_vars() {
                    return self.helpers.move_modifier(hpid, pointer, keys_states);
                }
            }
        }
        false
    }
    pub fn get_move_action(&mut self) -> Option<Box<MoveAction>> {
        let mut move_action: Option<Box<MoveAction>> = None;
        let shapes_moved = self.shapes.get_state_and_vars(HS::Select);
        if shapes_moved.len() > 0 {
            move_action = Some(Box::new(MoveAction::Shapes(MoveShapesAction {
                shids_vars: shapes_moved,
            })));
        } else {
            if let Some(shid_vars) = self.shapes.get_first_selected_modifier_vars() {
                move_action = Some(Box::new(MoveAction::Shapes(MoveShapesAction {
                    shids_vars: vec![shid_vars],
                })));
            }
        }
        if move_action.is_none() {
            let helpers_moved = self.helpers.get_state_and_vars(HS::Select);
            if helpers_moved.len() > 0 {
                move_action = Some(Box::new(MoveAction::Helpers(MoveHelpersAction {
                    dhids_vars: helpers_moved,
                })));
            } else {
                if let Some(dhid_vars) = self.helpers.get_first_selected_modifier_vars() {
                    move_action = Some(Box::new(MoveAction::Helpers(MoveHelpersAction {
                        dhids_vars: vec![dhid_vars],
                    })));
                }
            }
        }
        move_action
    }
}

pub trait PoolsFunctions {
    type Id: NewId;
    type Pool;
    type Object;
    type ObjectKindvars;

    fn new() -> Self::Pool;
    // Methods
    fn duplicate(&mut self, shapes: Vec<Self::Object>) -> Vec<Self::Object>;
    fn add(&mut self, helper: Self::Object);
    fn delete(&mut self, dhid: Self::Id) -> Option<Self::Object>;
    fn get(&self, target_dhid: Self::Id) -> Option<&Self::Object>;
    fn get_mut(&mut self, target_dhid: Self::Id) -> Option<&mut Self::Object>;
    fn iter(&self) -> impl Iterator<Item = (&Self::Id, &Self::Object)>;
    fn iter_mut(&mut self) -> impl Iterator<Item = (&Self::Id, &mut Self::Object)>;
    fn values(&self) -> impl Iterator<Item = &Self::Object>;
    fn values_mut(&mut self) -> impl Iterator<Item = &mut Self::Object>;

    fn save_vars(&mut self);

    fn get_state(&mut self, hors: HS) -> Vec<Self::Id>;
    fn set_state(&mut self, ses_hs: SetEntityState);
    fn set_states_from_pos(
        &mut self,
        pointer: &mut Pointer,
        keys_states: KeysStates,
        ses_hs: SetEntityStateFromPos,
    ) -> bool;

    fn get_state_if_one(&mut self, hors: HS) -> Option<Self::Id>;
    fn get_state_and_vars(&mut self, hors: HS) -> Vec<(Self::Id, Self::ObjectKindvars)>;

    fn get_first_selected_modifier_vars(&mut self) -> Option<(Self::Id, Self::ObjectKindvars)>;

    fn move_position(
        &mut self,
        dhid: Self::Id,
        pointer: &mut Pointer,
        keys_states: KeysStates,
    ) -> bool;
    fn move_modifier(
        &mut self,
        dhid: Self::Id,
        pointer: &mut Pointer,
        skeys_states: KeysStates,
    ) -> bool;
    fn delete_selection(&mut self) -> Option<Vec<Self::Object>>;
}
