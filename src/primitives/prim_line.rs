use super::primitives::{PrimitiveControls, PrimitiveKindIter};
use crate::{
    canvas::{CanvasText, Pattern},
    dimensions::{DimKind, Dimension},
    math::*,
    pools::HS,
    KeysStates, Pointer, Position, Status,
};
use kurbo::{BezPath, Line, Shape, Size, Vec2};

#[derive(Copy, Debug, Clone)]
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
        if let Some((dist, pos)) =
            distance_and_projection_to_segment(start, end, pointer.pos(), Self::GRAB)
        {
            Some((dist, pos))
        } else {
            None
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
