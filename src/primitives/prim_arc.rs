use super::primitives::{PrimitiveControls, PrimitiveKindIter};
use crate::{
    canvas::{CanvasText, Pattern},
    dimensions::{DimKind, Dimension},
    math::*,
    pools::HS,
    KeysStates, Pointer, Position, Status, Value,
};
use kurbo::{BezPath, Shape, Size, Vec2};

#[derive(Copy, Debug, Clone)]
pub struct PrimArc {
    radius: Value,
    concavity: bool,
    concavity_saved: bool,
    state: Status,
}
impl PrimArc {
    const MIN_RADIUS: f64 = 10.;
    const ANGLE_GUARD: f64 = 0.02;

    pub fn new() -> Self {
        PrimArc {
            radius: Value::new(Self::MIN_RADIUS),
            concavity: false,
            concavity_saved: false,
            state: Status::default(),
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

    fn toggle_prop(&mut self) {
        log!("toggle");
        ()
    }
    fn save_vars(&mut self) {
        self.radius.saved_val = self.radius.value;
        self.concavity_saved = self.concavity;
    }
    fn restore_vars(&mut self) {
        self.radius.value = self.radius.saved_val;
        self.concavity = self.concavity_saved;
    }
    fn update_primitives_vars(&mut self, start: Position, end: Position) -> Vec2 {
        let old_diam = (start.saved_pos - end.saved_pos).hypot();
        let new_diam = (start.pos - end.pos).hypot();
        if old_diam > EPSILON {
            self.radius.value = self.radius.saved_val * new_diam / old_diam;
        }
        self.validate_radius(start.pos, end.pos);

        find_circle_center(start.pos, end.pos, self.radius.value, self.concavity)
    }

    fn get_control_state(&self, start: Vec2, end: Vec2, hs: HS) -> Option<Vec2> {
        self.state.is_hs(hs).then(|| (start + end) / 2.)
    }
    fn set_all_controls_state(&mut self, hs: HS, state: bool) {
        self.state.set_hs(hs, state);
    }
    fn get_dist_from_control(
        &self,
        start: Vec2,
        end: Vec2,
        pointer: &Pointer,
    ) -> Option<(f64, Vec2)> {
        if let Some((dist, pos)) = distance_and_projection_to_arc(
            &create_arc_from_radius_and_concavity(
                start,
                end,
                self.radius.value,
                self.get_concavity(),
            ),
            pointer.pos(),
            Self::ANGLE_GUARD,
        ) {
            Some((dist, pos))
        } else {
            None
        }
    }

    fn move_control_selected(
        &mut self,
        start: Vec2,
        end: Vec2,
        pointer: &Pointer,
        _keys_states: KeysStates,
    ) -> bool {
        if self.radius.is_hs(HS::Select) {
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
    fn get_all_controls_positions(&self, _start: Vec2, _end: Vec2) -> Vec<Vec2> {
        // vec![find_circle_center(
        //     start,
        //     end,
        //     self.radius.value,
        //     self.concavity,
        // )]
        vec![]
    }

    fn path_elements(&self, start: Vec2, end: Vec2) -> PrimitiveKindIter {
        let f = create_arc_from_radius_and_concavity(start, end, self.radius.value, self.concavity);
        PrimitiveKindIter::Arc(f.path_elements(Self::TOLERANCE))
    }
    fn get_paths_and_patterns(
        &self,
        start: Vec2,
        end: Vec2,
        _das: &Size,
        parent_selected: bool,
        parent_highlighted: bool,
    ) -> (BezPath, Pattern) {
        use HS::*;
        (
            self.path_elements(start, end).collect(),
            self.get_pattern(
                self.state.is_hs(Select) || parent_selected,
                self.state.is_hs(Highlight) || parent_highlighted,
            ),
        )
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
