use crate::{
    helpers::{
        helpers::{Helper, MoveHelpersAction},
        helpers_pool::{AddHelperAction, HelpersPool},
    },
    shapes::{
        shapes::{BasicShape, MoveShapesAction},
        shapes_pool::{AddShapeAction, ShapesPool},
    },
    traits::*,
    Action, ClipboardItem, PasteAction, Pointer,
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

#[derive(Copy, Clone, Debug)]
pub enum HS {
    Highlight,
    Select,
}

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
    pub shapes: ShapesPool,
    pub helpers: HelpersPool,
}
impl Pools {
    pub fn new() -> Self {
        Self {
            shapes: ShapesPool::new(),
            helpers: HelpersPool::new(),
        }
    }
    pub fn add_shape(&mut self, shape: BasicShape) {
        self.shapes.add(shape);
    }
    pub fn add_helper(&mut self, helper: Helper) {
        self.helpers.add(helper);
    }
    pub fn save_vars(&mut self) {
        self.helpers.values_mut().for_each(|helper| {
            helper.get_kind_mut().save_vars();
        });
        self.shapes.values_mut().for_each(|helper| {
            helper.get_kind_mut().save_vars();
        });
    }
    pub fn set_objects_state(&mut self, value: bool, hs: HS) {
        self.shapes.set_state(value, hs);
        self.helpers.set_state(value, hs);
    }
    pub fn set_objects_states_in_order(&mut self, pointer: &mut Pointer, hors: HS) -> bool {
        if self.shapes.set_modifiers_states_from_pos(pointer, hors) {
            // log!("S set_mod_hs_from_pos");
            self.shapes.set_state(false, hors);
            self.helpers.set_state(false, hors);
            self.helpers.set_modifiers_state(false, hors);
            return true;
        }
        if self.helpers.set_states_from_pos(pointer, hors) {
            // log!("H set_hs_from_pos");
            self.shapes.set_state(false, hors);
            self.helpers.set_modifiers_state(false, hors);
            return true;
        }
        if self.helpers.set_modifiers_states_from_pos(pointer, hors) {
            // log!("H set_mod_hs_from_pos");
            self.shapes.set_state(false, hors);
            return true;
        }
        if self.shapes.set_states_from_pos(pointer, hors) {
            // log!("S set_hs_from_pos");
            return true;
        }
        false
    }
    pub fn clear_all_hs(&mut self) {
        self.shapes.set_state(false, HS::Select);
        self.shapes.set_modifiers_state(false, HS::Select);
        self.helpers.set_state(false, HS::Select);
        self.helpers.set_modifiers_state(false, HS::Select);
    }
    pub fn select_all_shapes_connected(&mut self) -> bool {
        self.shapes.select_all_connected()
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
    pub fn move_objects(&mut self, pointer: &mut Pointer, shift_pressed: bool) -> bool {
        if let Some((shid, _)) = self.shapes.get_first_selected_modifier_vars() {
            return self.shapes.move_modifier(shid, pointer, shift_pressed);
        }

        let shapes_selected = self.shapes.get_state(HS::Select);
        if shapes_selected.len() == 1 {
            return self
                .shapes
                .move_position(shapes_selected[0], pointer, shift_pressed);
        } else {
            if shapes_selected.len() > 0 {
                let mut moved = false;
                for shid in shapes_selected {
                    moved |= self.shapes.move_position(shid, pointer, shift_pressed);
                }
                return moved;
            }
        }

        let helpers_selected = self.helpers.get_state(HS::Select);
        if helpers_selected.len() == 1 {
            return self
                .helpers
                .move_position(helpers_selected[0], pointer, shift_pressed);
        } else {
            if helpers_selected.len() > 0 {
                let mut moved = false;
                for dhid in helpers_selected {
                    moved |= self.helpers.move_position(dhid, pointer, shift_pressed);
                }
                return moved;
            } else {
                if let Some((hpid, _)) = self.helpers.get_first_selected_modifier_vars() {
                    return self.helpers.move_modifier(hpid, pointer, shift_pressed);
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
    pub fn recalc_full_segs(&mut self) {
        self.shapes.recalc_full_segs();
    }
    pub fn magnet_to_helpers(&mut self, pointer: &mut Pointer, shift_pressed: bool) {
        if !shift_pressed {
            self.helpers.magnet_to_point(pointer);
        }
    }
    pub fn create_magnet_points(&mut self) {
        self.helpers.create_helpers_magnet_points();
    }
    pub fn delete_shapes_selection(&mut self) -> Option<Vec<BasicShape>> {
        self.shapes.delete_selection()
    }
    pub fn delete_helpers_selection(&mut self) -> Option<Vec<Helper>> {
        self.helpers.delete_selection()
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

    fn set_states_from_pos(&mut self, pointer: &mut Pointer, hors: HS) -> bool;
    fn set_state(&mut self, value: bool, hors: HS);
    fn get_state(&self, hors: HS) -> Vec<Self::Id>;
    fn get_state_if_one(&self, hors: HS) -> Option<Self::Id>;
    fn get_state_and_vars(&self, hors: HS) -> Vec<(Self::Id, Self::ObjectKindvars)>;
    fn set_id_state(&mut self, id: Self::Id, value: bool, hors: HS);
    fn set_modifiers_states_from_pos(&mut self, pointer: &mut Pointer, hors: HS) -> bool;
    fn set_modifiers_state(&mut self, value: bool, hors: HS);
    fn get_first_selected_modifier_vars(&self) -> Option<(Self::Id, Self::ObjectKindvars)>;

    fn move_position(&mut self, dhid: Self::Id, pointer: &mut Pointer, shift_pressed: bool)
        -> bool;
    fn move_modifier(&mut self, dhid: Self::Id, pointer: &mut Pointer, shift_pressed: bool)
        -> bool;
    fn delete_selection(&mut self) -> Option<Vec<Self::Object>>;
}
