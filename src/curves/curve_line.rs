use std::f64::consts::PI;

use super::curves::{CurveControls, PrimitiveKindIter};
use crate::{
    canvas::{CanvasText, Pattern},
    dimensions::{DimKind, Dimension},
    math::*,
    pools::HS,
    positions::Value,
    KeysStates, Pointer, Position, Status,
};
use kurbo::{BezPath, Line, Shape, Size, Vec2};

#[derive(Copy, Debug, Clone, PartialEq, Default)]
pub struct CurveLine {
    start: Position,
    end: Position,
    offset: Value,
    state: Status,
    init_done: bool,
}
impl CurveLine {
    pub fn get_start(&self) -> Position {
        self.start
    }
    pub fn get_end(&self) -> Position {
        self.end
    }
    const MIN_OFFSET: f64 = 10.;
}
impl CurveControls for CurveLine {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 5.;

    fn toggle_prop(&mut self) {
        ()
    }
    fn save_vars(&mut self) {
        self.start.saved_pos = self.start.pos;
        self.end.saved_pos = self.end.pos;
    }
    fn restore_vars(&mut self) {
        self.start.pos = self.start.saved_pos;
        self.end.pos = self.end.saved_pos;
    }
    /// Sets the start and end positions from given `start` and `end` positions,
    /// updates the internal state accordingly, and returns the midpoint between them.
    ///
    /// # Parameters
    /// - `start`: A `Position` struct representing the starting point.
    /// - `end`: A `Position` struct representing the ending point.
    ///
    /// # Returns
    /// Returns an `Option<Vec2>` containing the midpoint of the two positions if they differ,
    /// or `None` if the start and end positions are equal (indicating a degenerate configuration).
    fn set_from_start_end(&mut self, start: Position, end: Position) -> Option<Vec2> {
        // Check if the positions are equal using the `eq` method.
        // If they are equal, return None as the configuration is degenerate.
        start.pos.eq(&end.pos).then(|| {
            return None::<Vec2>;
        });

        // Mark the internal state as initialized.
        self.init_done = true;

        // Update the internal start and end positions.
        self.start = start;
        self.end = end;

        // Reset the offset to its default value.
        self.offset = Value::default();

        // Calculate and return the midpoint between start and end positions.
        Some((self.start.pos + self.end.pos) / 2.)
    }

    /// Sets internal geometry based on a dihedral configuration defined by three consecutive positions:
    /// the previous position (`p_prev`), the current position (`p`), and the next position (`p_next`).
    ///
    /// The function updates internal state values (`start.pos` and `end.pos`) based on the dihedral angle
    /// at point `p`. It computes two offset positions along the directions from `p` to `p_prev` and from `p`
    /// to `p_next`, adjusted by a cosine factor of the half-angle between these vectors. Finally, it returns
    /// the midpoint between the two computed positions, if everything is well defined.
    ///
    /// # Parameters
    /// - `p_prev`: The position before the current position.
    /// - `p`: The current (vertex) position.
    /// - `p_next`: The position after the current position.
    ///
    /// # Returns
    /// Returns `Some(Vec2)` representing the midpoint between `start.pos` and `end.pos` if the configuration
    /// is valid, or `None` if a degenerate case is detected (for example, if one of the segments is too short
    /// or the half-angle is nearly ±π/2).
    fn set_from_dihedron(
        &mut self,
        p_prev: Position,
        p: Position,
        p_next: Position,
    ) -> Option<Vec2> {
        // If this instance has not been initialized yet, set the offset to a minimal value and mark it as initialized.
        if !self.init_done {
            self.offset = Value::new(Self::MIN_OFFSET);
            self.init_done = true;
        }

        // Calculate vectors from the current position `p` to the previous and next positions.
        let v1 = p_prev.pos - p.pos; // Vector from p to p_prev.
        let v2 = p_next.pos - p.pos; // Vector from p to p_next.

        // Check if either vector is nearly zero-length using a small tolerance (EPSILON).
        // If either is too short, the dihedral (and thus the offset positions) is undefined.
        (v1.hypot() < EPSILON || v2.hypot() < EPSILON).then(|| return None::<Vec2>);

        // Compute the half-angle between the two vectors.
        // The function `angle_from` (assumed defined elsewhere) computes the full angle between v1 and v2.
        // Multiplying by 0.5 yields the half-angle.
        let angle = angle_from(v1, v2) * 0.5;

        // Check if the half-angle is nearly ±π/2.
        // If it is, then the computed positions along the offset will be degenerate (or extremely sensitive)
        // and the configuration is considered invalid.
        ((angle - PI / 2.).abs() < EPSILON || (angle + PI / 2.).abs() < EPSILON)
            .then(|| return None::<Vec2>);

        // Calculate the start position:
        // Move from `p` in the normalized direction of v1, scaled by the offset adjusted by the cosine of the angle.
        self.start.pos = p.pos + v1.normalize() * self.offset.value / angle.cos();
        // Calculate the end position:
        // Move from `p` in the normalized direction of v2, scaled similarly.
        self.end.pos = p.pos + v2.normalize() * self.offset.value / angle.cos();

        // Return the midpoint between the computed start and end positions.
        Some((self.start.pos + self.end.pos) / 2.)
    }

    fn get_state(&self, hs: HS) -> Option<Vec2> {
        self.state
            .is_hs(hs)
            .then(|| (self.start.pos + self.end.pos) / 2.)
    }
    fn set_state(&mut self, hs: HS, state: bool) {
        self.state.set_hs(hs, state);
    }
    fn get_dist_from_pos(&self, pointer_pos: Vec2) -> Option<(f64, Vec2)> {
        if let Some((dist, pos)) = distance_and_projection_to_segment(
            self.start.pos,
            self.end.pos,
            pointer_pos,
            Self::GRAB,
        ) {
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

    fn path_elements(&self) -> PrimitiveKindIter {
        PrimitiveKindIter::Line(
            Line::new(self.start.pos.to_point(), self.end.pos.to_point())
                .path_elements(Self::TOLERANCE),
        )
    }
    fn get_paths_and_patterns(
        &self,
        _das: &Size,
        parent_selected: bool,
        parent_highlighted: bool,
    ) -> (BezPath, Pattern) {
        use HS::*;
        (
            self.path_elements().collect(),
            self.get_pattern(
                self.state.is_hs(Select) || parent_selected,
                self.state.is_hs(Highlight) || parent_highlighted,
            ),
        )
    }
    fn get_dimensions_paths_and_patterns(
        &self,
        _das: &Size,
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        let mut res = vec![];
        let length = (self.start.pos - self.end.pos).hypot();

        let dim = Dimension::new(DimKind::Linear, self.start.pos, self.end.pos, length);
        let dim = dim.get_path_and_pattern();
        res.push(dim);

        let dim = Dimension::new(DimKind::Angle, self.start.pos, self.end.pos, 0.);
        let dim = dim.get_path_and_pattern();
        res.push(dim);
        res
    }
}
