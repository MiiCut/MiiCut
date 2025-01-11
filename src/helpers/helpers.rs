use super::helper_circle::HelperCircle;
use super::helper_line::HelperLine;
use super::helper_point::HelperPoint;
use super::helpers_pool::DHid;
use crate::canvas::CanvasText;
use crate::canvas::Pattern;
use crate::pools::Pools;
use crate::traits::*;
use crate::Action;
use crate::Position;
use crate::Value;
use crate::HS;
use kurbo::BezPath;
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
            if let Some(shape) = pools.hp.get_helper_mut(*dhid) {
                shape.get_kind_mut().set_vars(vars);
                shape.get_kind_mut().restore_saved();
            }
        }
    }

    fn redo(&self, pools: &mut Pools) {
        log!("Redoing last shapes move");
        for (dhid, vars) in &self.dhids_vars {
            if let Some(shape) = pools.hp.get_helper_mut(*dhid) {
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
    Point(HelperPoint),
    Line(HelperLine),
    Circle(HelperCircle),
}
impl HelperKind {
    pub fn magnet_to(&self, pos: Vec2) -> Option<Vec2> {
        use HelperKind::*;
        match self {
            Point(sh) => sh.magnet_to(pos),
            Line(sh) => sh.magnet_to(pos),
            Circle(sh) => sh.magnet_to(pos),
        }
    }
}
impl Display for HelperKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HelperKind::Point(p) => write!(f, "{}", p),
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
            Point(sh) => sh.save_vars(),
            Line(sh) => sh.save_vars(),
            Circle(sh) => sh.save_vars(),
        }
    }
    fn restore_saved(&mut self) {
        use HelperKind::*;
        match self {
            Point(sh) => sh.restore_saved(),
            Line(sh) => sh.restore_saved(),
            Circle(sh) => sh.restore_saved(),
        }
    }
    fn get_vars(&self) -> HelperKindvars {
        use HelperKind::*;
        match &self {
            Point(sh) => sh.get_vars(),
            Line(sh) => sh.get_vars(),
            Circle(sh) => sh.get_vars(),
        }
    }
    fn set_vars(&mut self, vars: &HelperKindvars) {
        use HelperKind::*;
        match self {
            Point(sh) => sh.set_vars(vars),
            Line(sh) => sh.set_vars(vars),
            Circle(sh) => sh.set_vars(vars),
        }
    }
    fn good_size(&self) -> bool {
        use HelperKind::*;
        match &self {
            Point(sh) => sh.good_size(),
            Line(sh) => sh.good_size(),
            Circle(sh) => sh.good_size(),
        }
    }

    fn set_hs_from_pos(&mut self, pos: Vec2, snap: f64, hors: HS) -> bool {
        use HelperKind::*;
        match self {
            Point(sh) => sh.set_hs_from_pos(pos, snap, hors),
            Line(sh) => sh.set_hs_from_pos(pos, snap, hors),
            Circle(sh) => sh.set_hs_from_pos(pos, snap, hors),
        }
    }
    fn set_hs(&mut self, value: bool, hors: HS) {
        use HelperKind::*;
        match self {
            Point(sh) => sh.set_hs(value, hors),
            Line(sh) => sh.set_hs(value, hors),
            Circle(sh) => sh.set_hs(value, hors),
        }
    }
    fn get_hs(&self, hors: HS) -> bool {
        use HelperKind::*;
        match &self {
            Point(sh) => sh.get_hs(hors),
            Line(sh) => sh.get_hs(hors),
            Circle(sh) => sh.get_hs(hors),
        }
    }
    fn get_hhss(&self) -> (bool, bool) {
        use HelperKind::*;
        match &self {
            Point(sh) => sh.get_hhss(),
            Line(sh) => sh.get_hhss(),
            Circle(sh) => sh.get_hhss(),
        }
    }

    fn set_hs_modifiers_from_pos(&mut self, pos: Vec2, snap: f64, hors: HS) -> Option<Vec2> {
        use HelperKind::*;
        match self {
            Point(sh) => sh.set_hs_modifiers_from_pos(pos, snap, hors),
            Line(sh) => sh.set_hs_modifiers_from_pos(pos, snap, hors),
            Circle(sh) => sh.set_hs_modifiers_from_pos(pos, snap, hors),
        }
    }
    fn set_hs_modifiers(&mut self, value: bool, hors: HS) {
        use HelperKind::*;
        match self {
            Point(sh) => sh.set_hs_modifiers(value, hors),
            Line(sh) => sh.set_hs_modifiers(value, hors),
            Circle(sh) => sh.set_hs_modifiers(value, hors),
        }
    }
    fn get_hs_modifiers(&self, hors: HS) -> bool {
        use HelperKind::*;
        match &self {
            Point(sh) => sh.get_hs_modifiers(hors),
            Line(sh) => sh.get_hs_modifiers(hors),
            Circle(sh) => sh.get_hs_modifiers(hors),
        }
    }

    fn toggle_prop(&mut self) {
        use HelperKind::*;
        match self {
            Point(sh) => sh.toggle_prop(),
            Line(sh) => sh.toggle_prop(),
            Circle(sh) => sh.toggle_prop(),
        }
    }

    fn move_position(&mut self, dpos: Vec2, snap: f64) {
        use HelperKind::*;
        match self {
            Point(sh) => sh.move_position(dpos, snap),
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
            Point(sh) => sh.move_modifier(pos_init, pos, snap, _shift_pressed),
            Line(sh) => sh.move_modifier(pos_init, pos, snap, _shift_pressed),
            Circle(sh) => sh.move_modifier(pos_init, pos, snap, _shift_pressed),
        }
    }
    fn get_position(&self) -> Vec2 {
        use HelperKind::*;
        match self {
            Point(sh) => sh.get_position(),
            Line(sh) => sh.get_position(),
            Circle(sh) => sh.get_position(),
        }
    }

    fn get_paths(&self, drawing_area_size: &Size) -> Vec<BezPath> {
        use HelperKind::*;
        match self {
            Point(sh) => sh.get_paths(drawing_area_size),
            Line(sh) => sh.get_paths(drawing_area_size),
            Circle(sh) => sh.get_paths(drawing_area_size),
        }
    }
    fn get_modifiers_paths(&self, drawing_area_size: &Size) -> Vec<(BezPath, Pattern)> {
        use HelperKind::*;
        match self {
            Point(sh) => sh.get_modifiers_paths(drawing_area_size),
            Line(sh) => sh.get_modifiers_paths(drawing_area_size),
            Circle(sh) => sh.get_modifiers_paths(drawing_area_size),
        }
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        use HelperKind::*;
        match self {
            Point(sh) => sh.get_dimensions_paths(),
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
    fn get_paths_and_patterns(&self, canvas_size: &Size) -> Vec<(BezPath, Pattern)> {
        let hs = self.helper_kind.get_hhss();
        let pattern = match (hs.0, hs.1) {
            (false, false) => Pattern::HelperNormal,
            (false, true) => Pattern::HelperHighlighted,
            (true, false) => Pattern::HelperSelected,
            (true, true) => Pattern::HelperSelected,
        };

        let mut paths = self.helper_kind.get_paths(canvas_size);
        let result = paths
            .iter_mut()
            .map(|path| (path.clone(), pattern))
            .collect();
        result
    }
}
