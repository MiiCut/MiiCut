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
    Action, ClipboardItem, PasteAction,
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
        self.shapes.add_shape(shape);
    }
    pub fn add_helper(&mut self, helper: Helper) {
        self.helpers.add_helper(helper);
    }
    pub fn save_vars(&mut self) {
        self.helpers.values_mut().for_each(|helper| {
            helper.get_kind_mut().save_vars();
        });
        self.shapes.values_mut().for_each(|helper| {
            helper.get_kind_mut().save_vars();
        });
    }
    pub fn set_hs_objects(&mut self, value: bool, hs: HS) {
        self.shapes.set_hs(value, hs);
        self.helpers.set_hs(value, hs);
    }
    pub fn set_hs_objects_in_order(
        &mut self,
        cursor_pos: Vec2,
        snap: f64,
        grab: f64,
        hors: HS,
    ) -> Option<Vec2> {
        if self
            .shapes
            .set_mod_hs_from_pos(cursor_pos, snap, grab, hors)
        {
            // log!("S set_mod_hs_from_pos");
            self.shapes.set_hs(false, hors);
            self.helpers.set_hs(false, hors);
            self.helpers.set_mod_hs(false, hors);
            return Some(cursor_pos);
        }
        if self
            .helpers
            .set_hs_from_pos(cursor_pos, snap, hors)
            .is_some()
        {
            // log!("H set_hs_from_pos");
            self.shapes.set_hs(false, hors);
            self.helpers.set_mod_hs(false, hors);
            return Some(cursor_pos);
        }
        if self
            .helpers
            .set_mod_hs_from_pos(cursor_pos, snap, grab, hors)
        {
            // log!("H set_mod_hs_from_pos");
            self.shapes.set_hs(false, hors);
            return Some(cursor_pos);
        }
        if let Some(_) = self.shapes.set_hs_from_pos(cursor_pos, snap, hors) {
            // log!("S set_hs_from_pos");
            return Some(cursor_pos);
        } else {
            None
        }
    }
    pub fn clear_all_hs(&mut self) {
        self.shapes.set_hs(false, HS::Select);
        self.shapes.set_mod_hs(false, HS::Select);
        self.helpers.set_hs(false, HS::Select);
        self.helpers.set_mod_hs(false, HS::Select);
    }
    pub fn select_all_shapes_connected(&mut self) -> bool {
        self.shapes.select_all_connected()
    }
    pub fn paste_to_pool(&mut self, clip_item: ClipboardItem) -> Box<PasteAction> {
        match &clip_item {
            ClipboardItem::Shapes((shapes, ..)) => {
                let new_shapes = self.shapes.duplicate_shapes(shapes.clone());
                // Push the PasteAction to the undo/redo system
                Box::new(PasteAction {
                    clip_item: ClipboardItem::Shapes((new_shapes, Vec2::ZERO)),
                })
            }
            ClipboardItem::Helpers((helpers, ..)) => {
                let new_helpers = self.helpers.duplicate_helpers(helpers.clone());
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
        if let Some((shid, _)) = self.shapes.get_first_selected_modifier_vars() {
            return self
                .shapes
                .move_modifier(shid, pos_dwn, cursor_pos, snap, shift_pressed);
        }

        let shapes_selected = self.shapes.get_hs(HS::Select);
        if shapes_selected.len() == 1 {
            return self.shapes.move_position(
                shapes_selected[0],
                pos_dwn,
                cursor_pos,
                snap,
                shift_pressed,
            );
        } else {
            if shapes_selected.len() > 0 {
                for shid in shapes_selected {
                    self.shapes
                        .move_position(shid, pos_dwn, cursor_pos, snap, shift_pressed);
                }
                return None;
            }
        }

        let helpers_selected = self.helpers.get_hs(HS::Select);
        if helpers_selected.len() == 1 {
            return self.helpers.move_position(
                helpers_selected[0],
                pos_dwn,
                cursor_pos,
                snap,
                shift_pressed,
            );
        } else {
            if helpers_selected.len() > 0 {
                for dhid in helpers_selected {
                    self.helpers
                        .move_position(dhid, pos_dwn, cursor_pos, snap, shift_pressed);
                }
                return None;
            } else {
                if let Some((hpid, _)) = self.helpers.get_first_selected_modifier_vars() {
                    self.helpers
                        .move_modifier(hpid, pos_dwn, cursor_pos, snap, shift_pressed);
                }
            }
        }
        None
    }
    pub fn get_move_action(&mut self) -> Option<Box<MoveAction>> {
        let mut move_action: Option<Box<MoveAction>> = None;
        let shapes_moved = self.shapes.get_hs_vars(HS::Select);
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
            let helpers_moved = self.helpers.get_hs_vars(HS::Select);
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
    pub fn magnet_to_helpers(&mut self, pos: Vec2) -> Vec2 {
        self.helpers.magnet_to_helpers(pos)
    }

    pub fn delete_shapes_selection(&mut self) -> Option<Vec<BasicShape>> {
        self.shapes.delete_selection()
    }
    pub fn delete_helpers_selection(&mut self) -> Option<Vec<Helper>> {
        self.helpers.delete_selection()
    }
}
