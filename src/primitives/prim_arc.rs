use super::primitives::{
    GetPrimitiveState, PrimitiveControls, PrimitiveKindIter, SetPrimitiveState,
    SetPrimitiveStateFromPos, VertexChange,
};
use crate::{
    canvas::{CanvasText, Pattern},
    dimensions::{DimKind, Dimension},
    math::*,
    pools::HS,
    prefab::*,
    KeysStates, Pointer, Position, Value,
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
            concavity: false,
            concavity_saved: false,
            highlighted: false,
            selected: false,
        }
    }

    pub fn get_radius(&self) -> f64 {
        self.radius.value
    }
    pub fn get_concavity(&self) -> bool {
        self.concavity
    }
    pub fn validate_radius(&mut self, start: Vec2, end: Vec2) {
        if self.radius.value.signum() > 0. {
            if self.radius.value < (start - end).hypot() / 2.0 {
                self.radius.value = (start - end).hypot() / 2.0 + EPSILON;
            }
        } else {
            if self.radius.value > -(start - end).hypot() / 2.0 {
                self.radius.value = -(start - end).hypot() / 2.0 - EPSILON;
            }
        }
    }
}
impl PrimitiveControls for PrimArc {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 5.;

    fn toggle(&mut self) {
        log!("toggle");
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
    fn update_primitives_vars(
        &mut self,
        start: Position,
        end: Position,
        vertex_changed: VertexChange,
    ) -> Vec2 {
        match vertex_changed {
            VertexChange::StartChanged | VertexChange::EndChanged => {
                let old_diam = (start.saved_pos - end.saved_pos).hypot();
                let new_diam = (start.pos - end.pos).hypot();
                if old_diam > EPSILON {
                    self.radius.value = self.radius.saved_val * new_diam / old_diam;
                }
                self.validate_radius(start.pos, end.pos);
            }
            VertexChange::Nope => {
                self.validate_radius(start.pos, end.pos);
            }
        }
        find_circle_center(start.pos, end.pos, self.radius.value, self.concavity)
    }
    fn is_selected(&self) -> bool {
        self.selected
    }
    fn is_highlighted(&self) -> bool {
        self.highlighted
    }
    fn get_state(&self, start: Vec2, end: Vec2, state: GetPrimitiveState) -> Option<Vec2> {
        use GetPrimitiveState::*;
        match state {
            IsSelected => self.selected.then(|| (start + end) / 2.),
            IsHighligh => self.highlighted.then(|| (start + end) / 2.),
            IsOtherModifiersSelected => self
                .radius
                .selected
                .then(|| find_circle_center(start, end, self.radius.value, self.concavity)),
            IsOtherModifiersHighligh => self
                .radius
                .highlighted
                .then(|| find_circle_center(start, end, self.radius.value, self.concavity)),
            _ => None,
        }
    }
    fn set_state(&mut self, _start: Vec2, _end: Vec2, state: SetPrimitiveState) {
        use SetPrimitiveState::*;
        match state {
            SetHS(hs, value) => match hs {
                HS::Select => self.selected = value,
                HS::Highlight => self.highlighted = value,
            },

            SetAllOtherModifiersHS(hs, value) => match hs {
                HS::Select => self.radius.selected = value,
                HS::Highlight => self.radius.highlighted = value,
            },
            _ => (),
        }
    }
    fn set_state_from_pos(
        &mut self,
        start: Vec2,
        end: Vec2,
        pointer: &mut Pointer,
        state: SetPrimitiveStateFromPos,
    ) {
        use SetPrimitiveStateFromPos::*;
        let res_state = || -> bool {
            is_point_near_arc(
                &create_arc_from_radius_and_concavity(
                    start,
                    end,
                    self.radius.value,
                    self.get_concavity(),
                ),
                pointer.pos(),
                Self::GRAB,
            )
        };
        match state {
            SetHSFromPos(hs) => match hs {
                HS::Select => self.selected = res_state(),
                HS::Highlight => self.highlighted = res_state(),
            },
            SetOthersModifiersHSFromPos(hs) => {
                let center = find_circle_center(start, end, self.radius.value, self.concavity);
                let state = (pointer.pos() - center).hypot() < Self::GRAB;
                match hs {
                    HS::Select => {
                        self.radius.selected = state;
                        if self.radius.selected {
                            pointer.set_pos(center);
                            pointer.save_pos();
                        }
                    }
                    HS::Highlight => {
                        self.radius.highlighted = state;
                        if self.radius.highlighted {
                            pointer.set_pos(center);
                        }
                    }
                }
            }
            _ => (),
        }
    }

    fn move_control_selected(
        &mut self,
        start: Vec2,
        end: Vec2,
        pointer: &Pointer,
        _keys_states: KeysStates,
    ) -> bool {
        if self.radius.selected {
            let center = find_circle_center(start, end, self.radius.saved_val, self.concavity);

            let pos = pointer.pos();
            let dpos = pointer.dpos();
            let dpos_proj = perpendicular_point_with_projection(
                (start - end) / 2.,
                (end - start) / 2.,
                dpos,
                self.concavity,
            );
            let sign = if self.concavity {
                -(pos - start).cross(end - start).signum()
            } else {
                (pos - start).cross(end - start).signum()
            };

            self.radius.value = snap_val(
                sign * (center + dpos_proj - start).hypot(),
                pointer.get_snap().val(),
            );
            self.validate_radius(start, end);
            true
        } else {
            false
        }
    }
    fn get_all_controls_positions(&self, start: Vec2, end: Vec2) -> Vec<Vec2> {
        vec![find_circle_center(
            start,
            end,
            self.radius.value,
            self.concavity,
        )]
    }

    // NOT USED, SEE shape_custom
    fn path_elements(&self, start: Vec2, end: Vec2) -> PrimitiveKindIter {
        let f = create_arc_from_radius_and_concavity(start, end, self.radius.value, self.concavity);
        PrimitiveKindIter::Arc(f.path_elements(Self::TOLERANCE))
    }
    // NOT USED, SEE shape_custom
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
    fn get_dimensions_paths_and_patterns(
        &self,
        start: Vec2,
        end: Vec2,
        _das: &Size,
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        let mut res = vec![];
        let center = find_circle_center(start, end, self.radius.saved_val, self.concavity);
        let offset = self.radius.value / 2_f64.sqrt();
        let end = center + Vec2::new(offset, -offset);
        let dim =
            Dimension::new(DimKind::Radius, center, end, self.radius.value).get_path_and_pattern();
        res.push(dim);
        res
    }
}
