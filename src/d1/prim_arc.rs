use super::{d1::D1KindIter, primitives::PrimitiveControls};
use crate::{canvas::Pattern, math::*, GetEntityState, SetEntityState};
use kurbo::{Shape, Vec2};

#[derive(Debug, Clone)]
pub struct PrimArc {
    pub radius: f64,
    pub concavity: bool,
    pub concavity_saved: bool,
    pub highlighted: bool,
    pub selected: bool,
}
impl PrimArc {
    const MIN_BEND_RADIUS: f64 = 10.;
    fn get_middle_of_arc(&self, start: Vec2, end: Vec2) -> Vec2 {
        let arc = create_arc(start, end, self.radius, self.concavity);
        middle_point_of_arc(&arc)
    }
}
impl PrimitiveControls for PrimArc {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 5.;

    fn new(start: Vec2, end: Vec2) -> Self {
        PrimArc {
            radius: (start - end).hypot() + Self::MIN_BEND_RADIUS,
            concavity: true,
            concavity_saved: true,
            highlighted: false,
            selected: false,
        }
    }
    fn toggle(&mut self) {
        self.concavity = !self.concavity;
    }
    fn update_vars(&mut self, start: Vec2, end: Vec2) -> Vec2 {
        // Update the radius
        log!("update_vars concavity: {}", self.concavity);
        self.radius = (start - end).hypot() + Self::MIN_BEND_RADIUS;
        self.get_middle_of_arc(start, end)
    }

    fn get_state(&self, start: Vec2, end: Vec2, state: GetEntityState) -> Option<Vec2> {
        use GetEntityState::*;
        match state {
            IsAnyModifierHighligh | IsHighligh => {
                if self.highlighted {
                    Some(self.get_middle_of_arc(start, end))
                } else {
                    None
                }
            }
            IsAnyModifierSelected | IsSelected => {
                if self.selected {
                    Some(self.get_middle_of_arc(start, end))
                } else {
                    None
                }
            }
        }
    }
    fn set_state(&mut self, start: Vec2, end: Vec2, state: SetEntityState) {
        use SetEntityState::*;
        match state {
            SetSelect(value) | SelectAllModifiers(value) => self.selected = value,
            SelectFromPos(pos, _, _) | SelectModifierFromPos(pos, ..) => {
                self.selected = is_point_near_arc(
                    &create_arc(start, end, self.radius, self.concavity),
                    pos,
                    Self::GRAB,
                )
            }
            SetHighli(value) | HighliAllModifiers(value) => self.highlighted = value,
            HighliFromPos(pos, ..) | HighliModifierFromPos(pos, ..) => {
                self.highlighted = is_point_near_arc(
                    &create_arc(start, end, self.radius, self.concavity),
                    pos,
                    Self::GRAB,
                )
            }
        }
    }
    fn move_control_selected(&mut self, start: Vec2, end: Vec2, pos: Vec2) -> Option<Vec2> {
        if self.selected {
            // Calculate the new radius
            let mut new_radius = get_arc_radius(start, end, self.radius, pos, self.concavity);
            if new_radius < (start - end).hypot() + Self::MIN_BEND_RADIUS {
                new_radius = (start - end).hypot() + Self::MIN_BEND_RADIUS;
            }
            self.radius = new_radius;
            Some(self.get_middle_of_arc(start, end))
        } else {
            None
        }
    }

    fn path_elements(&self, start: Vec2, end: Vec2) -> D1KindIter {
        let f = create_arc(start, end, self.radius, self.concavity);
        D1KindIter::Arc(f.path_elements(Self::TOLERANCE))
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
