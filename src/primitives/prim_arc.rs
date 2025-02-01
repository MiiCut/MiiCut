use super::primitives::{PrimitiveControls, PrimitiveKindIter};
use crate::{
    canvas::{CanvasText, Pattern},
    dimensions::{DimKind, Dimension},
    math::*,
    pools::HS,
    positions::Status,
    KeysStates, Pointer, Position,
};
use kurbo::{BezPath, Line, Shape, Size, Vec2};

#[derive(Copy, Debug, Clone)]
pub struct PrimArc {
    start: Position,
    end: Position,
    pt3: Position,
    status: Status,
    init_done: bool,
}
impl PrimArc {
    const ANGLE_GUARD: f64 = 0.02;

    pub fn new() -> Self {
        PrimArc {
            start: Position::new(Vec2::ZERO, false),
            end: Position::new(Vec2::ZERO, false),
            pt3: Position::new(Vec2::ZERO, false),
            status: Status::default(),
            init_done: false,
        }
    }
    pub fn get_radius(&self) -> f64 {
        if let Some((_center, radius)) =
            circle_from_three_points(self.start.pos, self.end.pos, self.pt3.pos)
        {
            radius
        } else {
            0.
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
        self.start.saved_pos = self.start.pos;
        self.end.saved_pos = self.end.pos;
        self.pt3.saved_pos = self.pt3.pos;
    }
    fn restore_vars(&mut self) {
        self.start.pos = self.start.saved_pos;
        self.end.pos = self.end.saved_pos;
        self.pt3.pos = self.pt3.saved_pos;
    }
    fn update_primitives_vars(&mut self, start: Position, end: Position) -> Vec2 {
        if !self.init_done {
            if let Some(unit) = unit_perpendicular(start.pos, end.pos, false) {
                self.pt3.pos =
                    (start.pos + end.pos) / 2. + unit * (end.pos - start.pos).hypot() / 4.;
                self.init_done = true;
            }
        } else {
            // The two points have moved together (move polygon)
            if self.start.pos != start.pos && self.end.pos != end.pos {
                self.pt3.pos += start.pos - self.start.pos;
            }
        }
        self.start = start;
        self.end = end;
        self.pt3.pos
    }

    fn get_control_state(&self, _start: Vec2, _end: Vec2, hs: HS) -> Option<Vec2> {
        self.status
            .is_hs(hs)
            .then(|| (self.start.pos + self.end.pos) / 2.)
    }
    fn set_all_controls_state(&mut self, hs: HS, state: bool) {
        self.status.set_hs(hs, state);
    }
    fn get_dist_from_control(
        &self,
        _start: Vec2,
        _end: Vec2,
        pointer: &Pointer,
    ) -> Option<(f64, Vec2)> {
        if let Some(arc) = arc_from_three_points(self.start.pos, self.pt3.pos, self.end.pos) {
            if let Some((dist, pos)) =
                distance_and_projection_to_arc(&arc, pointer.pos(), Self::ANGLE_GUARD)
            {
                Some((dist, pos))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn move_control_selected(
        &mut self,
        _start: Vec2,
        _end: Vec2,
        pointer: &Pointer,
        _keys_states: KeysStates,
    ) -> bool {
        if self.status.is_hs(HS::Select) {
            self.pt3.pos = pointer.pos();
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

    fn path_elements(&self, _start: Vec2, _end: Vec2) -> PrimitiveKindIter {
        if let Some(arc) = arc_from_three_points(self.start.pos, self.pt3.pos, self.end.pos) {
            PrimitiveKindIter::Arc(arc.path_elements(Self::TOLERANCE))
        } else {
            PrimitiveKindIter::Line(
                Line::new(self.start.pos.to_point(), self.end.pos.to_point())
                    .path_elements(Self::TOLERANCE),
            )
        }
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
                self.status.is_hs(Select) || parent_selected,
                self.status.is_hs(Highlight) || parent_highlighted,
            ),
        )
    }
    fn get_dimensions_paths_and_patterns(
        &self,
        _start: Vec2,
        _end: Vec2,
        _das: &Size,
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        let mut res = vec![];
        if let Some(arc) = arc_from_three_points(self.start.pos, self.pt3.pos, self.end.pos) {
            let offset = arc.radii.x / 2_f64.sqrt();
            let end = arc.center + Vec2::new(offset, -offset);
            let dim = Dimension::new(
                DimKind::Radius,
                arc.center.to_vec2(),
                end.to_vec2(),
                arc.radii.x,
            )
            .get_path_and_pattern();
            res.push(dim);
        }

        res
    }
}
