use crate::{
    canvas::{CanvasText, Pattern},
    dimensions::{DimKind, Dimension},
    math::*,
    KeysStates, Pointer,
};

use super::primitives::{
    GetPrimitiveState, PrimitiveControls, PrimitiveKindIter, SetPrimitiveState,
    SetPrimitiveStateFromPos, VertexChange,
};
use kurbo::{BezPath, Line, Shape, Size, Vec2};

#[derive(Debug, Clone)]
pub struct PrimLine {
    highlighted: bool,
    selected: bool,
}
impl PrimLine {
    pub fn new() -> Self {
        PrimLine {
            highlighted: false,
            selected: false,
        }
    }
}
impl PrimitiveControls for PrimLine {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 5.;

    fn toggle(&mut self) {
        ()
    }
    fn save_vars(&mut self) {
        ()
    }
    fn restore_saved(&mut self) {
        ()
    }
    fn update_primitives_vars(&mut self, start: Vec2, end: Vec2, _changed: VertexChange) -> Vec2 {
        (start + end) / 2.
    }
    fn get_state(&self, start: Vec2, end: Vec2, state: GetPrimitiveState) -> Option<Vec2> {
        use GetPrimitiveState::*;
        match state {
            IsSelected => self.selected.then(|| (start + end) / 2.),
            IsHighligh => self.highlighted.then(|| (start + end) / 2.),
            IsOtherModifiersSelected => None,
            IsOtherModifiersHighligh => None,
            _ => None,
        }
    }
    fn is_selected(&self) -> bool {
        self.selected
    }
    fn is_highlighted(&self) -> bool {
        self.highlighted
    }
    fn set_state(&mut self, _start: Vec2, _end: Vec2, state: SetPrimitiveState) {
        use SetPrimitiveState::*;
        match state {
            SetSelect(value) => self.selected = value,
            SetHighli(value) => self.highlighted = value,
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
        match state {
            SelectFromPos => {
                self.selected =
                    distance_to_segment(start, end, pointer.pos(), Self::GRAB) < Self::GRAB
            }
            HighliFromPos => {
                self.highlighted =
                    distance_to_segment(start, end, pointer.pos(), Self::GRAB) < Self::GRAB
            }
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

    fn path_elements(&self, start: Vec2, end: Vec2) -> PrimitiveKindIter {
        PrimitiveKindIter::Line(
            Line::new(start.to_point(), end.to_point()).path_elements(Self::TOLERANCE),
        )
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
