use crate::{canvas::Pattern, GetEntityState, SetEntityState};

use super::d1::D1KindIter;
use kurbo::Vec2;

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

    fn new(start: Vec2, end: Vec2) -> Self;
    fn update_vars(&mut self, start: Vec2, end: Vec2) -> Vec2;
    fn get_state(&self, start: Vec2, end: Vec2, state: GetEntityState) -> Option<Vec2>;
    fn set_state(&mut self, start: Vec2, end: Vec2, state: SetEntityState);
    fn move_control_selected(&mut self, start: Vec2, end: Vec2, pos: Vec2) -> Option<Vec2>;
    fn toggle(&mut self);
    fn path_elements(&self, start: Vec2, end: Vec2) -> D1KindIter;
    fn get_pattern(&self) -> Pattern;
}
