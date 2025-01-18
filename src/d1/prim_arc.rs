use super::{
    d1::{D1KindIter, VertexChange},
    primitives::PrimitiveControls,
};
use crate::{
    canvas::Pattern, math::*, prefab::modifiers_path, GetEntityState, SetEntityState, Value,
};
use kurbo::{BezPath, Shape, Size, Vec2};

#[derive(Debug, Clone)]
pub struct PrimArc {
    // center_rel is relative to start,
    // hence it's norm is the radius
    radius: Value,
    concavity: bool,
    concavity_saved: bool,
    highlighted: bool,
    selected: bool,
}
impl PrimArc {
    const MIN_RADIUS: f64 = 10.;

    pub fn new() -> Self {
        PrimArc {
            radius: Value::new(Self::MIN_RADIUS),
            concavity: true,
            concavity_saved: true,
            highlighted: false,
            selected: false,
        }
    }
    pub fn get_center(&mut self, start: Vec2, end: Vec2) -> Vec2 {
        // Validate the radius
        if self.radius.value < (start - end).hypot() / 2.0 {
            self.radius.value = (start - end).hypot() / 2.0 + EPSILON;
        }
        find_circle_center(start, end, self.radius.value, self.concavity)
    }
}
impl PrimitiveControls for PrimArc {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 5.;

    fn toggle(&mut self) {
        self.concavity = !self.concavity;
    }
    fn save_vars(&mut self) {
        self.radius.saved_val = self.radius.value;
        self.concavity_saved = self.concavity;
    }
    fn restore_saved(&mut self) {
        self.radius.value = self.radius.saved_val;
        self.concavity = self.concavity_saved;
    }
    fn update_vars(&mut self, start: Vec2, end: Vec2, _changed: VertexChange) -> Vec2 {
        // Validate the radius
        if self.radius.value < (start - end).hypot() / 2.0 {
            self.radius.value = (start - end).hypot() / 2.0 + EPSILON;
        }
        find_circle_center(start, end, self.radius.value, self.concavity)
    }
    fn get_state(&self, start: Vec2, end: Vec2, state: GetEntityState) -> Option<Vec2> {
        use GetEntityState::*;
        match state {
            IsAnyModifierHighligh | IsHighligh => {
                if self.highlighted || self.radius.highlighted {
                    Some(find_circle_center(
                        start,
                        end,
                        self.radius.value,
                        self.concavity,
                    ))
                } else {
                    None
                }
            }
            IsAnyModifierSelected | IsSelected => {
                if self.selected || self.radius.selected {
                    Some(find_circle_center(
                        start,
                        end,
                        self.radius.value,
                        self.concavity,
                    ))
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
            SelectAllModifiers(value) => self.radius.selected = value,
            SetHighli(value) => self.highlighted = value,
            HighliAllModifiers(value) => self.radius.highlighted = value,

            SelectFromPos(pos, _, _) => {
                self.selected = is_point_near_arc(
                    &create_arc_from_center(
                        start,
                        end,
                        find_circle_center(start, end, self.radius.value, self.concavity),
                        self.concavity,
                    ),
                    pos,
                    Self::GRAB,
                );
            }
            HighliFromPos(pos, ..) => {
                self.highlighted = is_point_near_arc(
                    &create_arc_from_center(
                        start,
                        end,
                        find_circle_center(start, end, self.radius.value, self.concavity),
                        self.concavity,
                    ),
                    pos,
                    Self::GRAB,
                );
            }

            SelectModifierFromPos(pos, ..) => {
                let center = find_circle_center(start, end, self.radius.value, self.concavity);
                self.radius.selected = (pos - center).hypot() < Self::GRAB;
            }
            HighliModifierFromPos(pos, ..) => {
                let center = find_circle_center(start, end, self.radius.value, self.concavity);
                self.radius.highlighted = (pos - center).hypot() < Self::GRAB;
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
        if self.radius.selected {
            let center = find_circle_center(start, end, self.radius.saved_val, self.concavity);
            let dpos = pos - pos_init;
            let dpos_proj = perpendicular_point_with_projection(
                (start - end) / 2.,
                (end - start) / 2.,
                dpos,
                self.concavity,
            );
            log!("dpos_proj: ({:.2},{:.2})", dpos_proj.x, dpos_proj.y);
            self.radius.value = (center + dpos_proj - start).hypot();
            log!("radius: {:.2}", self.radius.value);
            Some(find_circle_center(
                start,
                end,
                self.radius.value,
                self.concavity,
            ))
        } else {
            None
        }
    }
    fn path_elements(&self, start: Vec2, end: Vec2) -> D1KindIter {
        let f = create_arc_from_center(
            start,
            end,
            find_circle_center(start, end, self.radius.value, self.concavity),
            self.concavity,
        );
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
        start: Vec2,
        end: Vec2,
        _das: &Size,
    ) -> Vec<(BezPath, Pattern)> {
        let mut paths_patterns: Vec<(BezPath, Pattern)> = vec![];
        paths_patterns.push((
            modifiers_path(
                find_circle_center(start, end, self.radius.value, self.concavity),
                1.,
                Self::GRAB,
            ),
            self.get_mod_pattern(self.radius.selected, self.radius.highlighted),
        ));
        paths_patterns
    }
}
