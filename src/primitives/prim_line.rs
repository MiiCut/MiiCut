use super::primitives::{PrimitiveControls, PrimitiveKindIter};
use crate::{
    canvas::{CanvasText, Pattern},
    dimensions::{DimKind, Dimension},
    math::*,
    pools::HS,
    GetEntityState, KeysStates, Pointer, Position, SetEntityState, SetEntityStateFromPos, Status,
};
use kurbo::{BezPath, Line, Shape, Size, Vec2};

#[derive(Debug, Clone)]
pub struct PrimLine {
    state: Status,
}
impl PrimLine {
    pub fn new() -> Self {
        PrimLine {
            state: Status::default(),
        }
    }
}
impl PrimitiveControls for PrimLine {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 5.;

    fn toggle_prop(&mut self) {
        ()
    }
    fn save_vars(&mut self) {
        ()
    }
    fn restore_vars(&mut self) {
        ()
    }
    fn update_primitives_vars(&mut self, start: Position, end: Position) -> Vec2 {
        (start.pos + end.pos) / 2.
    }
    fn get_state(&self, start: Vec2, end: Vec2, state: GetEntityState) -> Option<Vec2> {
        use GetEntityState::*;
        match state {
            IsHS(hs) => self.state.is_hs(hs).then(|| (start + end) / 2.),
            IsAnyControlHS(_) => None,
        }
    }
    fn set_state(&mut self, _start: Vec2, _end: Vec2, state: SetEntityState) {
        use SetEntityState::*;
        match state {
            SetHS(hs, value) => self.state.set_hs(hs, value),
            _ => (),
        }
    }
    fn set_state_from_pos(
        &mut self,
        start: Vec2,
        end: Vec2,
        pointer: &mut Pointer,
        state: SetEntityStateFromPos,
    ) {
        use SetEntityStateFromPos::*;
        match state {
            SetHSFromPos(hs) => self.state.set_hs(
                hs,
                distance_to_segment(start, end, pointer.pos(), Self::GRAB) < Self::GRAB,
            ),
            _ => (),
        }
    }
    fn move_control_selected(
        &mut self,
        _start: Vec2,
        _end: Vec2,
        _pointer: &Pointer,
        _keys_states: KeysStates,
    ) -> bool {
        false
    }
    fn get_all_controls_positions(&self, _: Vec2, _: Vec2) -> Vec<Vec2> {
        vec![]
    }

    fn path_elements(&self, start: Vec2, end: Vec2) -> PrimitiveKindIter {
        PrimitiveKindIter::Line(
            Line::new(start.to_point(), end.to_point()).path_elements(Self::TOLERANCE),
        )
    }
    fn get_paths_and_patterns(&self, start: Vec2, end: Vec2, _das: &Size) -> (BezPath, Pattern) {
        use HS::*;
        (
            self.path_elements(start, end).collect(),
            self.get_pattern(self.state.is_hs(Select), self.state.is_hs(Highlight)),
        )
    }
    fn get_mod_paths_and_patterns(
        &self,
        _start: Vec2,
        _end: Vec2,
        _das: &Size,
    ) -> Vec<(BezPath, Pattern)> {
        vec![]
    }
    fn get_dimensions_paths_and_patterns(
        &self,
        start: Vec2,
        end: Vec2,
        _das: &Size,
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        let mut res = vec![];
        let length = (start - end).hypot();

        let dim = Dimension::new(DimKind::Linear, start, end, length);
        let dim = dim.get_path_and_pattern();
        res.push(dim);

        let dim = Dimension::new(DimKind::Angle, start, end, 0.);
        let dim = dim.get_path_and_pattern();
        res.push(dim);
        res
    }
}
