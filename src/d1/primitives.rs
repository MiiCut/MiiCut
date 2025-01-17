use crate::{canvas::Pattern, GetEntityState, SetEntityState};

use super::d1::D1KindIter;
use kurbo::{BezPath, Size, Vec2};

#[derive(Debug, Clone)]
pub enum D1Kind {
    D1KLine,
    D1KArc,
    // D1KQBez,
    // D1KQBezSmooth,
    // D1KCBez,
    // D1KCBezSmooth,
}
impl D1Kind {
    pub fn next_kind(&self) -> D1Kind {
        use D1Kind::*;
        match self {
            D1KLine => D1KArc,
            D1KArc => D1KLine,
            // D1KQBez => D1KQBezSmooth,
            // D1KQBezSmooth => D1KCBez,
            // D1KCBez => D1KCBezSmooth,
            // D1KCBezSmooth => D1KLine,
        }
    }
    pub fn prev_kind(&self) -> D1Kind {
        use D1Kind::*;
        match self {
            D1KLine => D1KArc,
            D1KArc => D1KLine,
            // D1KQBez => D1KArc,
            // D1KQBezSmooth => D1KQBez,
            // D1KCBez => D1KQBezSmooth,
            // D1KCBezSmooth => D1KCBez,
        }
    }
}

pub trait PrimitiveControls {
    const TOLERANCE: f64;
    const GRAB: f64;

    fn toggle(&mut self);
    fn save_vars(&mut self);
    fn restore_saved(&mut self);
    fn update_vars(&mut self, start: Vec2, end: Vec2) -> Vec2;
    fn get_state(&self, start: Vec2, end: Vec2, state: GetEntityState) -> Option<Vec2>;
    fn set_state(&mut self, start: Vec2, end: Vec2, state: SetEntityState);

    fn move_control_selected(
        &mut self,
        start: Vec2,
        end: Vec2,
        pos_init: Vec2,
        pos: Vec2,
        snap: f64,
        shift_pressed: bool,
    ) -> Option<Vec2>;
    fn path_elements(&self, start: Vec2, end: Vec2) -> D1KindIter;
    fn get_pattern(&self, selected: bool, highlighted: bool) -> Pattern {
        match (selected, highlighted) {
            (false, false) => Pattern::BasicNormal,
            (false, true) => Pattern::BasicHighlighted,
            (true, false) => Pattern::BasicSelected,
            (true, true) => Pattern::BasicSelected,
        }
    }
    fn get_mod_pattern(&self, selected: bool, highlighted: bool) -> Pattern {
        match (selected, highlighted) {
            (false, false) => Pattern::Modifiers,
            (false, true) => Pattern::ModifiersHighlighted,
            (true, false) => Pattern::ModifiersSelected,
            (true, true) => Pattern::ModifiersSelected,
        }
    }
    fn get_paths_and_patterns(&self, start: Vec2, end: Vec2, _das: &Size) -> (BezPath, Pattern);
    fn get_mod_paths_and_patterns(
        &self,
        start: Vec2,
        end: Vec2,
        _das: &Size,
    ) -> Vec<(BezPath, Pattern)>;
}
