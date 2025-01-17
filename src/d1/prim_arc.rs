use super::{d1::D1KindIter, primitives::PrimitiveControls};
use crate::{
    canvas::Pattern, math::*, prefab::modifiers_path, GetEntityState, Position, SetEntityState,
};
use kurbo::{BezPath, Shape, Size, Vec2};

#[derive(Debug, Clone)]
pub struct PrimArc {
    pub center: Position,
    pub concavity: bool,
    pub concavity_saved: bool,
    pub highlighted: bool,
    pub selected: bool,
}
impl PrimArc {
    const _MIN_BEND_RADIUS: f64 = 10.;
}
impl PrimitiveControls for PrimArc {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 5.;

    fn toggle(&mut self) {
        self.concavity = !self.concavity;
    }
    fn save_vars(&mut self) {
        self.center.saved_pos = self.center.pos;
        self.concavity_saved = self.concavity;
    }
    fn restore_saved(&mut self) {
        self.center.pos = self.center.saved_pos;
        self.concavity = self.concavity_saved;
    }
    fn update_vars(&mut self, start: Vec2, end: Vec2) -> Vec2 {
        // Update center
        self.center.pos = (start + end) / 2.;
        self.center.pos
    }
    fn get_state(&self, _start: Vec2, _end: Vec2, state: GetEntityState) -> Option<Vec2> {
        use GetEntityState::*;
        match state {
            IsAnyModifierHighligh | IsHighligh => {
                if self.highlighted {
                    Some(self.center.pos)
                } else {
                    None
                }
            }
            IsAnyModifierSelected | IsSelected => {
                if self.selected {
                    Some(self.center.pos)
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
            SelectAllModifiers(value) => self.center.selected = value,
            SetHighli(value) => self.highlighted = value,
            HighliAllModifiers(value) => self.center.highlighted = value,

            SelectFromPos(pos, _, _) => {
                self.selected = is_point_near_arc(
                    &create_arc_from_center(start, end, self.center.pos, self.concavity),
                    pos,
                    Self::GRAB,
                );
            }
            HighliFromPos(pos, ..) => {
                self.highlighted = is_point_near_arc(
                    &create_arc_from_center(start, end, self.center.pos, self.concavity),
                    pos,
                    Self::GRAB,
                );
            }

            SelectModifierFromPos(pos, ..) => {
                self.center.selected = (pos - self.center.pos).hypot() < Self::GRAB;
            }
            HighliModifierFromPos(pos, ..) => {
                self.center.highlighted = (pos - self.center.pos).hypot() < Self::GRAB;
            }
        }
    }
    fn move_control_selected(
        &mut self,
        start: Vec2,
        end: Vec2,
        pos_init: Vec2,
        pos: Vec2,
        _snap: f64,
        _shift_pressed: bool,
    ) -> Option<Vec2> {
        if self.selected {
            let dpos = pos - pos_init;
            let dpos_proj = perpendicular_point_with_projection(start, end, dpos, self.concavity);
            self.center.pos = snap_pt(self.center.saved_pos + dpos_proj, Self::GRAB);
            Some(self.center.pos)
        } else {
            None
        }
    }

    fn path_elements(&self, start: Vec2, end: Vec2) -> D1KindIter {
        let f = create_arc_from_center(start, end, self.center.pos, self.concavity);
        D1KindIter::Arc(f.path_elements(Self::TOLERANCE))
    }

    fn get_paths_and_patterns(&self, start: Vec2, end: Vec2, _das: &Size) -> (BezPath, Pattern) {
        (
            self.path_elements(start, end).collect(),
            self.get_pattern(self.selected, self.highlighted),
        )
    }
    fn get_mod_paths_and_patterns(
        &self,
        _start: Vec2,
        _end: Vec2,
        _das: &Size,
    ) -> Vec<(BezPath, Pattern)> {
        let mut paths_patterns: Vec<(BezPath, Pattern)> = vec![];
        paths_patterns.push((
            modifiers_path(self.center.pos, 1., Self::GRAB),
            self.get_pattern(self.center.selected, self.center.highlighted),
        ));
        paths_patterns
    }
}
