use super::helper_circle::HelperCircle;
use super::helper_line::HelperLine;
use super::helpers_pool::DHid;
use crate::canvas::CanvasText;
use crate::canvas::Pattern;
use crate::pools::Pools;
use crate::traits::*;
use crate::Action;
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
            if let Some(shape) = pools.helpers.get_helper_mut(*dhid) {
                shape.get_kind_mut().set_vars(vars);
                shape.get_kind_mut().restore_saved();
            }
        }
    }

    fn redo(&self, pools: &mut Pools) {
        log!("Redoing last shapes move");
        for (dhid, vars) in &self.dhids_vars {
            if let Some(shape) = pools.helpers.get_helper_mut(*dhid) {
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
impl HelperKind {
    pub fn magnet_to(&self, pos: Vec2) -> Option<Vec2> {
        use HelperKind::*;
        match self {
            Line(sh) => sh.magnet_to(pos),
            Circle(sh) => sh.magnet_to(pos),
        }
    }
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
    fn restore_saved(&mut self) {
        use HelperKind::*;
        match self {
            Line(sh) => sh.restore_saved(),
            Circle(sh) => sh.restore_saved(),
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

    fn toggle_prop(&mut self) {
        use HelperKind::*;
        match self {
            Line(sh) => sh.toggle_prop(),
            Circle(sh) => sh.toggle_prop(),
        }
    }

    fn move_position(&mut self, dpos: Vec2, snap: f64) -> Option<Vec2> {
        use HelperKind::*;
        match self {
            Line(sh) => sh.move_position(dpos, snap),
            Circle(sh) => sh.move_position(dpos, snap),
        }
    }
    fn move_modifier(
        &mut self,
        pos_init: Vec2,
        pos: Vec2,
        snap: f64,
        _shift_pressed: bool,
    ) -> Option<Vec2> {
        use HelperKind::*;
        match self {
            Line(sh) => sh.move_modifier(pos_init, pos, snap, _shift_pressed),
            Circle(sh) => sh.move_modifier(pos_init, pos, snap, _shift_pressed),
        }
    }
    fn get_position(&self) -> Vec2 {
        use HelperKind::*;
        match self {
            Line(sh) => sh.get_position(),
            Circle(sh) => sh.get_position(),
        }
    }

    fn get_paths(&self, das: &Size) -> Vec<BezPath> {
        use HelperKind::*;
        match self {
            Line(sh) => sh.get_paths(das),
            Circle(sh) => sh.get_paths(das),
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
    fn get_mod_paths_and_patterns(
        &self,
        das: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        use HelperKind::*;
        match self {
            Line(sh) => sh.get_mod_paths_and_patterns(das, cinfo),
            Circle(sh) => sh.get_mod_paths_and_patterns(das, cinfo),
        }
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        use HelperKind::*;
        match self {
            Line(sh) => sh.get_dimensions_paths(),
            Circle(sh) => sh.get_dimensions_paths(),
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
