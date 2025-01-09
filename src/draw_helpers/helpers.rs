use super::helper_circle::HelperCircle;
use super::helper_line::HelperLine;
use super::helper_point::HelperPoint;
use super::helpers_pool::DHid;
use crate::canvas::CanvasText;
use crate::canvas::Pattern;
use crate::Position;
use crate::Value;
use crate::HS;
use kurbo::BezPath;
use kurbo::Vec2;
use std::fmt::Debug;
use std::fmt::Display;

pub enum HelperKindvars {
    Point(Position),
    Line(Position, Value),
    Circle(Position, Value),
}

pub trait HelperKindFuncs: Debug + Clone {
    const TOLERANCE: f64;
    const GRAB: f64;

    fn save_vars(&mut self);
    fn restore_saved(&mut self);
    fn get_vars(&self) -> HelperKindvars;
    fn set_vars(&mut self, vars: &HelperKindvars);
    fn good_size(&self) -> bool;

    fn set_hs_from_pos(&mut self, pos: Vec2, hors: HS) -> bool;
    fn set_hs(&mut self, value: bool, hors: HS);
    fn get_hs(&self, hors: HS) -> bool;
    fn get_hhss(&self) -> (bool, bool);

    fn set_hs_modifiers_from_pos(&mut self, pos: Vec2, hors: HS) -> bool;
    fn set_hs_modifiers(&mut self, value: bool, hors: HS);
    fn get_hs_modifiers(&self, hors: HS) -> bool;

    fn toggle_prop(&mut self);

    fn move_position(&mut self, dpos: Vec2);
    fn move_modifier(&mut self, pos_init: Vec2, pos: Vec2, _shift_pressed: bool) -> bool;
    fn get_position(&self) -> Vec2;

    fn get_modifiers_paths(&self) -> Vec<(BezPath, Pattern)>;
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>);
    fn get_paths(&self) -> Vec<BezPath>;

    fn get_pattern_modifiers(&self, selected: bool, highlighted: bool) -> Pattern {
        match (selected, highlighted) {
            (false, false) => Pattern::Modifiers,
            (false, true) => Pattern::ModifiersHighlighted,
            (true, false) => Pattern::ModifiersSelected,
            (true, true) => Pattern::ModifiersSelected,
        }
    }
}

#[derive(Clone, Debug)]
pub enum HelperKind {
    Point(HelperPoint),
    Line(HelperLine),
    Circle(HelperCircle),
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

impl HelperKindFuncs for HelperKind {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 2.;

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

    fn set_hs_from_pos(&mut self, pos: Vec2, hors: HS) -> bool {
        use HelperKind::*;
        match self {
            Point(sh) => sh.set_hs_from_pos(pos, hors),
            Line(sh) => sh.set_hs_from_pos(pos, hors),
            Circle(sh) => sh.set_hs_from_pos(pos, hors),
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

    fn set_hs_modifiers_from_pos(&mut self, pos: Vec2, hors: HS) -> bool {
        use HelperKind::*;
        match self {
            Point(sh) => sh.set_hs_modifiers_from_pos(pos, hors),
            Line(sh) => sh.set_hs_modifiers_from_pos(pos, hors),
            Circle(sh) => sh.set_hs_modifiers_from_pos(pos, hors),
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

    fn move_position(&mut self, dpos: Vec2) {
        use HelperKind::*;
        match self {
            Point(sh) => sh.move_position(dpos),
            Line(sh) => sh.move_position(dpos),
            Circle(sh) => sh.move_position(dpos),
        }
    }
    fn move_modifier(&mut self, pos_init: Vec2, pos: Vec2, _shift_pressed: bool) -> bool {
        use HelperKind::*;
        match self {
            Point(sh) => sh.move_modifier(pos_init, pos, _shift_pressed),
            Line(sh) => sh.move_modifier(pos_init, pos, _shift_pressed),
            Circle(sh) => sh.move_modifier(pos_init, pos, _shift_pressed),
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

    fn get_paths(&self) -> Vec<BezPath> {
        use HelperKind::*;
        match self {
            Point(sh) => sh.get_paths(),
            Line(sh) => sh.get_paths(),
            Circle(sh) => sh.get_paths(),
        }
    }
    fn get_modifiers_paths(&self) -> Vec<(BezPath, Pattern)> {
        use HelperKind::*;
        match self {
            Point(sh) => sh.get_modifiers_paths(),
            Line(sh) => sh.get_modifiers_paths(),
            Circle(sh) => sh.get_modifiers_paths(),
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
    enabled: bool,
    dhid: DHid,
    helper_kind: HelperKind,
}
impl Helper {
    pub fn new(dhid: DHid, helper_kind: HelperKind) -> Helper {
        Helper {
            enabled: true,
            dhid,
            helper_kind,
        }
    }
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    pub fn clone_kind(&self) -> HelperKind {
        self.helper_kind.clone()
    }
    pub fn get_id(&self) -> DHid {
        self.dhid
    }
    pub fn set_new_id(&mut self, new_id: DHid) {
        self.dhid = new_id;
    }
    pub fn get_kind(&self) -> &HelperKind {
        &self.helper_kind
    }
    pub fn get_kind_mut(&mut self) -> &mut HelperKind {
        &mut self.helper_kind
    }
    pub fn get_paths_and_patterns(&self) -> Vec<(BezPath, Pattern)> {
        let hs = self.helper_kind.get_hhss();
        let pattern = match (hs.0, hs.1) {
            (false, false) => Pattern::BasicNormal,
            (false, true) => Pattern::BasicHighlighted,
            (true, false) => Pattern::BasicSelected,
            (true, true) => Pattern::BasicSelected,
        };

        let mut paths = self.helper_kind.get_paths();
        let result = paths
            .iter_mut()
            .map(|path| (path.clone(), pattern))
            .collect();
        result
    }
}
