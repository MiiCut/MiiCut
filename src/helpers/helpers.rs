use super::helper_circle::HelperCircle;
use super::helper_line::HelperLine;
use super::helpers_pool::DHid;
use crate::canvas::CanvasText;
use crate::canvas::Pattern;
use crate::pools::Pools;
use crate::pools::PoolsFunctions;
use crate::traits::*;
use crate::Action;
use crate::KeysStates;
use crate::Pointer;
use crate::Position;
use crate::Value;
use kurbo::BezPath;
use kurbo::Rect;
use kurbo::Size;
use kurbo::Vec2;
use std::fmt::Debug;
use std::fmt::Display;

pub struct MoveHelpersAction {
    pub dhids_vars: Vec<(DHid, HelperKindvars)>,
}
impl Action for MoveHelpersAction {
    fn undo(&self, pools: &mut Pools) {
        log!("Undoing last shapes move");
        for (dhid, vars) in &self.dhids_vars {
            if let Some(shape) = pools.helpers.get_mut(*dhid) {
                shape.get_kind_mut().set_vars(vars);
                shape.get_kind_mut().restore_vars();
            }
        }
    }

    fn redo(&self, pools: &mut Pools) {
        log!("Redoing last shapes move");
        for (dhid, vars) in &self.dhids_vars {
            if let Some(shape) = pools.helpers.get_mut(*dhid) {
                shape.get_kind_mut().set_vars(vars);
            }
        }
    }
}
pub enum HelperKindvars {
    Point(Position),
    Line(Position, Value),
    Circle(Position, Value),
}

#[derive(Clone, Debug)]
pub enum HelperKind {
    Line(HelperLine),
    Circle(HelperCircle),
}
impl Display for HelperKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HelperKind::Line(l) => write!(f, "{}", l),
            HelperKind::Circle(c) => write!(f, "{}", c),
        }
    }
}
impl ObjectsFuncs for HelperKind {
    const TOLERANCE: f64 = 0.01;
    const GRAB_RADIUS: f64 = 2.;
    type Kindvars = HelperKindvars;

    fn save_vars(&mut self) {
        use HelperKind::*;
        match self {
            Line(sh) => sh.save_vars(),
            Circle(sh) => sh.save_vars(),
        }
    }
    fn restore_vars(&mut self) {
        use HelperKind::*;
        match self {
            Line(sh) => sh.restore_vars(),
            Circle(sh) => sh.restore_vars(),
        }
    }
    fn get_vars(&self) -> HelperKindvars {
        use HelperKind::*;
        match &self {
            Line(sh) => sh.get_vars(),
            Circle(sh) => sh.get_vars(),
        }
    }
    fn set_vars(&mut self, vars: &HelperKindvars) {
        use HelperKind::*;
        match self {
            Line(sh) => sh.set_vars(vars),
            Circle(sh) => sh.set_vars(vars),
        }
    }
    fn good_size(&self) -> bool {
        use HelperKind::*;
        match &self {
            Line(sh) => sh.good_size(),
            Circle(sh) => sh.good_size(),
        }
    }
    fn finish_draw(&mut self) -> bool {
        use HelperKind::*;
        match self {
            Line(sh) => sh.finish_draw(),
            Circle(sh) => sh.finish_draw(),
        }
    }

    fn get_state(&self, get: GetEntityState) -> Option<Vec2> {
        use HelperKind::*;
        match self {
            Line(sh) => sh.get_state(get),
            Circle(sh) => sh.get_state(get),
        }
    }
    fn set_state(&mut self, set: SetEntityState) {
        use HelperKind::*;
        match self {
            Line(sh) => sh.set_state(set),
            Circle(sh) => sh.set_state(set),
        }
    }
    fn set_state_from_pos(
        &mut self,
        pointer: &mut Pointer,
        keys_states: KeysStates,
        set: SetEntityStateFromPos,
    ) {
        use HelperKind::*;
        match self {
            Line(sh) => sh.set_state_from_pos(pointer, keys_states, set),
            Circle(sh) => sh.set_state_from_pos(pointer, keys_states, set),
        }
    }

    fn toggle_selected_prop(&mut self) {
        use HelperKind::*;
        match self {
            Line(sh) => sh.toggle_selected_prop(),
            Circle(sh) => sh.toggle_selected_prop(),
        }
    }

    fn move_position(&mut self, pointer: &mut Pointer, keys_states: KeysStates) -> bool {
        use HelperKind::*;
        match self {
            Line(sh) => sh.move_position(pointer, keys_states),
            Circle(sh) => sh.move_position(pointer, keys_states),
        }
    }
    fn move_controls(&mut self, pointer: &Pointer, keys_states: KeysStates) -> bool {
        use HelperKind::*;
        match self {
            Line(sh) => sh.move_controls(pointer, keys_states),
            Circle(sh) => sh.move_controls(pointer, keys_states),
        }
    }
    fn get_position(&self) -> Vec2 {
        use HelperKind::*;
        match self {
            Line(sh) => sh.get_position(),
            Circle(sh) => sh.get_position(),
        }
    }

    fn get_paths_and_patterns(
        &self,
        das: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        use HelperKind::*;
        match self {
            Line(sh) => sh.get_paths_and_patterns(das, cinfo),
            Circle(sh) => sh.get_paths_and_patterns(das, cinfo),
        }
    }
    fn get_controls_paths_and_patterns(
        &self,
        das: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        use HelperKind::*;
        match self {
            Line(sh) => sh.get_controls_paths_and_patterns(das, cinfo),
            Circle(sh) => sh.get_controls_paths_and_patterns(das, cinfo),
        }
    }
    fn get_dimensions_paths_and_patterns(
        &self,
        das: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        use HelperKind::*;
        match self {
            Line(sh) => sh.get_dimensions_paths_and_patterns(das, cinfo),
            Circle(sh) => sh.get_dimensions_paths_and_patterns(das, cinfo),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Helper {
    dhid: DHid,
    helper_kind: HelperKind,
}
impl Helper {
    pub fn new(dhid: DHid, helper_kind: HelperKind) -> Helper {
        Helper { dhid, helper_kind }
    }
}
impl ObjectOps for Helper {
    type Id = DHid;
    type Kind = HelperKind;

    fn get_id(&self) -> DHid {
        self.dhid
    }
    fn get_kind(&self) -> &HelperKind {
        &self.helper_kind
    }
    fn get_kind_mut(&mut self) -> &mut HelperKind {
        &mut self.helper_kind
    }
    fn set_new_id(&mut self, id: Self::Id) {
        self.dhid = id;
    }
}
