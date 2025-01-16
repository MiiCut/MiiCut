use crate::{canvas::Pattern, math::*, GetEntityState, SetEntityState};

use super::{d1::D1KindIter, primitives::PrimitiveControls};
use kurbo::{Line, Shape, Vec2};

#[derive(Debug, Clone)]
pub struct PrimLine {
    highlighted: bool,
    selected: bool,
}

impl PrimitiveControls for PrimLine {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 5.;

    fn new(_: Vec2, _: Vec2) -> Self {
        PrimLine {
            highlighted: false,
            selected: false,
        }
    }
    fn toggle(&mut self) {
        ()
    }
    fn update_vars(&mut self, start: Vec2, end: Vec2) -> Vec2 {
        (start + end) / 2.
    }
    fn get_state(&self, start: Vec2, end: Vec2, state: GetEntityState) -> Option<Vec2> {
        use GetEntityState::*;
        match state {
            IsAnyModifierHighligh => None,
            IsAnyModifierSelected => None,
            IsHighligh => {
                if self.highlighted {
                    Some((start + end) / 2.)
                } else {
                    None
                }
            }
            IsSelected => {
                if self.selected {
                    Some((start + end) / 2.)
                } else {
                    None
                }
            }
        }
    }
    fn set_state(&mut self, start: Vec2, end: Vec2, state: SetEntityState) {
        use SetEntityState::*;
        match state {
            SetSelect(value) => self.selected = value,
            SelectFromPos(pos, _, _) => {
                self.selected = distance_to_segment(start, end, pos) < 2. * Self::GRAB
            }
            SetHighli(bool) => self.highlighted = bool,
            HighliFromPos(pos, _, _) => {
                self.highlighted = distance_to_segment(start, end, pos) < 2. * Self::GRAB
            }

            SelectAllModifiers(..) => (),
            SelectModifierFromPos(..) => (),

            HighliAllModifiers(..) => (),
            HighliModifierFromPos(..) => (),
        }
    }
    fn move_control_selected(&mut self, _start: Vec2, _end: Vec2, _pos: Vec2) -> Option<Vec2> {
        None
    }

    fn path_elements(&self, start: Vec2, end: Vec2) -> D1KindIter {
        D1KindIter::Line(Line::new(start.to_point(), end.to_point()).path_elements(Self::TOLERANCE))
    }
    fn get_pattern(&self) -> Pattern {
        match (self.selected, self.highlighted) {
            (false, false) => Pattern::BasicNormal,
            (false, true) => Pattern::BasicHighlighted,
            (true, false) => Pattern::BasicSelected,
            (true, true) => Pattern::BasicSelected,
        }
    }
}
