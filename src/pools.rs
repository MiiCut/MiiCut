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
    Action, ClipboardItem, PasteAction, HS,
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
    pub fn add_shape(&mut self, shape: BasicShape) {
        self.sh.add_shape(shape);
    }
    pub fn add_helper(&mut self, helper: Helper) {
        self.hp.add_helper(helper);
    }
    pub fn save_vars(&mut self) {
        self.hp.values_mut().for_each(|helper| {
            helper.get_kind_mut().save_vars();
        });
        self.sh.values_mut().for_each(|helper| {
            helper.get_kind_mut().save_vars();
        });
    }
    pub fn set_hs_objects(&mut self, value: bool, hs: HS) {
        self.sh.set_hs(value, hs);
        self.hp.set_hs(value, hs);
    }
    pub fn set_hs_objects_in_order(
        &mut self,
        cursor_pos: Vec2,
        snap: f64,
        grab: f64,
        hors: HS,
    ) -> Option<Vec2> {
        if let Some(pos) = self
            .sh
            .set_hs_modifiers_from_pos(cursor_pos, snap, grab, hors)
        {
            self.sh.set_hs(false, hors);
            self.hp.set_hs(false, hors);
            self.hp.set_hs_modifiers(false, hors);
            return Some(pos);
        }
        if self.hp.set_hs_from_pos(cursor_pos, snap, hors) {
            self.sh.set_hs(false, hors);
            self.hp.set_hs_modifiers(false, hors);
            return None;
        }

        if let Some(pos) = self
            .hp
            .set_hs_modifiers_from_pos(cursor_pos, snap, grab, hors)
        {
            self.sh.set_hs(false, hors);
            return Some(pos);
        }

        self.sh.set_hs_from_pos(cursor_pos, snap, hors);
        None
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
    pub fn paste_to_pool(&mut self, clip_item: ClipboardItem) -> Box<PasteAction> {
        match &clip_item {
            ClipboardItem::Shapes((shapes, ..)) => {
                let new_shapes = self.sh.duplicate_shapes(shapes.clone());
                // Push the PasteAction to the undo/redo system
                Box::new(PasteAction {
                    clip_item: ClipboardItem::Shapes((new_shapes, Vec2::ZERO)),
                })
            }
            ClipboardItem::Helpers((helpers, ..)) => {
                let new_helpers = self.hp.duplicate_helpers(helpers.clone());
                // Push the PasteAction to the undo/redo system
                Box::new(PasteAction {
                    clip_item: ClipboardItem::Helpers((new_helpers, Vec2::ZERO)),
                })
            }
        }
    }
    pub fn move_objects(
        &mut self,
        pos_dwn: Vec2,
        cursor_pos: Vec2,
        snap: f64,
        shift_pressed: bool,
    ) -> Option<Vec2> {
        if let Some((shid, _)) = self.sh.get_first_selected_modifier_vars() {
            return self
                .sh
                .move_modifier(shid, pos_dwn, cursor_pos, snap, shift_pressed);
        }

        let shapes_selected = self.sh.get_hs(HS::Select);
        if shapes_selected.len() == 1 {
            self.sh
                .move_position(shapes_selected[0], pos_dwn, cursor_pos, snap, shift_pressed);
            return None;
        } else {
            if shapes_selected.len() > 0 {
                for shid in shapes_selected {
                    self.sh
                        .move_position(shid, pos_dwn, cursor_pos, snap, shift_pressed);
                }
                return None;
            }
        }

        let helpers_selected = self.hp.get_hs(HS::Select);
        if helpers_selected.len() > 0 {
            if helpers_selected.len() > 0 {
                for dhid in helpers_selected {
                    self.hp
                        .move_position(dhid, pos_dwn, cursor_pos, snap, shift_pressed);
                }
                return None;
            }
        } else {
            if let Some((hpid, _)) = self.hp.get_first_selected_modifier_vars() {
                self.hp
                    .move_modifier(hpid, pos_dwn, cursor_pos, snap, shift_pressed);
            }
        }
        None
    }
    pub fn get_move_action(&mut self) -> Option<Box<MoveAction>> {
        let mut move_action: Option<Box<MoveAction>> = None;
        let shapes_moved = self.sh.get_hs_vars(HS::Select);
        if shapes_moved.len() > 0 {
            move_action = Some(Box::new(MoveAction::Shapes(MoveShapesAction {
                shids_vars: shapes_moved,
            })));
        } else {
            if let Some(shid_vars) = self.sh.get_first_selected_modifier_vars() {
                move_action = Some(Box::new(MoveAction::Shapes(MoveShapesAction {
                    shids_vars: vec![shid_vars],
                })));
            }
        }
        if move_action.is_none() {
            let helpers_moved = self.hp.get_hs_vars(HS::Select);
            if helpers_moved.len() > 0 {
                move_action = Some(Box::new(MoveAction::Helpers(MoveHelpersAction {
                    dhids_vars: helpers_moved,
                })));
            } else {
                if let Some(dhid_vars) = self.hp.get_first_selected_modifier_vars() {
                    move_action = Some(Box::new(MoveAction::Helpers(MoveHelpersAction {
                        dhids_vars: vec![dhid_vars],
                    })));
                }
            }
        }
        move_action
    }
    pub fn recalc_full_segs(&mut self) {
        self.sh.recalc_full_segs();
    }
    pub fn magnet_to_helpers(&mut self, pos: Vec2) -> Vec2 {
        self.hp.magnet_to_helpers(pos)
    }

    pub fn delete_shapes_selection(&mut self) -> Option<Vec<BasicShape>> {
        self.sh.delete_selection()
    }
    pub fn delete_helpers_selection(&mut self) -> Option<Vec<Helper>> {
        self.hp.delete_selection()
    }
}
