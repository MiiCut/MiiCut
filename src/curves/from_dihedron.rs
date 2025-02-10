use super::curves::{CurveControls, PrimitiveKindIter};
use crate::{
    canvas::{CanvasText, Pattern},
    dimensions::{DimKind, Dimension},
    math::*,
    pools::HS,
    positions::{Status, Value, ValueBool},
    KeysStates, Pointer, Position,
};
use kurbo::{BezPath, Shape, Size, Vec2};
use std::{f64::consts::PI, vec};

#[derive(Copy, Debug, Clone, PartialEq)]
pub enum CurveFromDihedronKind {
    Line,
    Arc,
    Point,
}

/// A dihedron (wedge)
///
/// The wedge is defined by:
/// - an apex point,
/// - a central direction (a unit vector),
/// - a half‑angle (in radians) that determines the wedge’s opening (full angle = 2*half_angle).
#[derive(Copy, Debug, Clone, PartialEq)]
pub struct Dihedron {
    pub apex: Position,
    pub u_dir: Position,
    pub half_angle: Value, // in radians; must be in ]0, PI[
}
impl Dihedron {
    /// Creates a new dihedron. Returns None if:
    /// - the direction vector is zero,
    /// - or the half_angle is not in the range ]0, PI/2[
    pub fn new(apex: Vec2, direction: Vec2, half_angle: f64) -> Option<Self> {
        if half_angle < EPSILON || half_angle > PI / 2. - EPSILON {
            return None;
        }
        if direction.hypot() < EPSILON {
            return None;
        }
        Some(Self {
            apex: Position::new(apex),
            u_dir: Position::new(direction.normalize()),
            half_angle: Value::new(half_angle),
        })
    }
    pub fn from_three_points(a: Vec2, b: Vec2, c: Vec2) -> Option<Self> {
        // Compute vectors from the apex to the other two points.
        let v1 = a - b;
        let v2 = c - b;
        // Normalize the boundary vectors.
        if v1.hypot() < EPSILON || v2.hypot() < EPSILON {
            return None;
        }
        let v1_norm = v1.normalize();
        let v2_norm = v2.normalize();
        // Compute the angle between v1_norm and v2_norm using the dot product.
        let dot = (v1_norm.x * v2_norm.x + v1_norm.y * v2_norm.y).clamp(-1.0, 1.0);
        let angle = dot.acos();
        // Exclude the cases where the angle is zero or PI.
        if angle < EPSILON || angle > PI - EPSILON {
            return None;
        }
        let half_angle = angle / 2.0;
        let bisector = (v1_norm + v2_norm).normalize();
        Dihedron::new(b, bisector, half_angle)
    }
    pub fn is_good(&self) -> Option<(Vec2, Vec2, f64)> {
        if self.u_dir.pos.hypot() < EPSILON {
            return None;
        }
        if self.half_angle.value < EPSILON || self.half_angle.value > PI / 2. - EPSILON {
            return None;
        }
        Some((self.apex.pos, -self.u_dir.pos, self.half_angle.value))
    }
    pub fn save_vars(&mut self) {
        self.apex.saved_pos = self.apex.pos;
        self.u_dir.saved_pos = self.u_dir.pos;
        self.half_angle.saved_val = self.half_angle.value;
    }
    pub fn restore_vars(&mut self) {
        self.apex.pos = self.apex.saved_pos;
        self.u_dir.pos = self.u_dir.saved_pos;
        self.half_angle.value = self.half_angle.saved_val;
    }
}

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct CurveFromDihedron {
    dih: Dihedron,
    // For Arc
    radius: Value,
    concavity: ValueBool, // false if the arc near apex
    // For Line
    offset: Value,
    curve_kind: CurveFromDihedronKind,
    apex_state: Status,
    state: Status,
}
impl CurveFromDihedron {
    const ANGLE_GUARD: f64 = 0.02;
    const MIN_ARC_RADIUS: f64 = 5.;
    const MIN_OFFSET: f64 = Self::MIN_ARC_RADIUS;

    pub fn new(dih: Dihedron) -> Option<Self> {
        dih.is_good().and_then(|(..)| {
            Some(CurveFromDihedron {
                dih,
                curve_kind: CurveFromDihedronKind::Point,
                radius: Value::new(Self::MIN_ARC_RADIUS),
                concavity: ValueBool::new(false),
                offset: Value::new(Self::MIN_OFFSET),
                state: Status::default(),
                apex_state: Status::default(),
            })
        })
    }
    pub fn toogle(&mut self) {
        use CurveFromDihedronKind::*;
        self.curve_kind = match self.curve_kind {
            Point => Line,
            Line => Arc,
            Arc => Point,
        }
    }
    pub fn get_apex(&self) -> &Position {
        &self.dih.apex
    }
    pub fn get_apex_mut(&mut self) -> &mut Position {
        &mut self.dih.apex
    }

    pub fn get_start(&self) -> Vec2 {
        use CurveFromDihedronKind::*;
        match self.curve_kind {
            Line => self
                .get_line_three_points()
                .and_then(|(s, ..)| Some(s))
                .unwrap_or_else(|| self.dih.apex.pos),
            Arc => self
                .get_circle_three_points()
                .and_then(|(s, ..)| Some(s))
                .unwrap_or_else(|| self.dih.apex.pos),
            Point => self.dih.apex.pos,
        }
    }
    pub fn get_end(&self) -> Vec2 {
        use CurveFromDihedronKind::*;
        match self.curve_kind {
            Line => self
                .get_line_three_points()
                .and_then(|(.., e)| Some(e))
                .unwrap_or_else(|| self.dih.apex.pos),
            Arc => self
                .get_circle_three_points()
                .and_then(|(.., e)| Some(e))
                .unwrap_or_else(|| self.dih.apex.pos),
            Point => self.dih.apex.pos,
        }
    }
    pub fn get_apex_state(&self, hs: HS) -> Option<Vec2> {
        self.apex_state.is_hs(hs).then(|| Some(self.dih.apex.pos))?
    }
    pub fn set_apex_state(&mut self, hs: HS, state: bool) {
        self.apex_state.set_hs(hs, state);
    }

    fn get_line_three_points(&self) -> Option<(Vec2, Vec2, Vec2)> {
        self.dih.is_good().and_then(|(apex, u_dir, ha)| {
            let mid_pt = apex - u_dir * self.offset.value;
            let half_len = self.offset.value * ha.tan();
            Some((
                mid_pt - Vec2::new(-u_dir.y, u_dir.x) * half_len,
                mid_pt,
                mid_pt + Vec2::new(-u_dir.y, u_dir.x) * half_len,
            ))
        })
    }
    fn get_circle_three_points(&self) -> Option<(Vec2, Vec2, Vec2)> {
        self.dih.is_good().and_then(|(apex, u_dir, ha)| {
            let start = apex - rotate_vector(u_dir, ha) * self.radius.value / ha.tan();
            let end = apex - rotate_vector(u_dir, -ha) * self.radius.value / ha.tan();
            if self.concavity.value {
                Some((
                    start,
                    apex - u_dir * self.radius.value * (1. / ha.sin() + 1.),
                    end,
                ))
            } else {
                Some((
                    start,
                    apex - u_dir * self.radius.value * (1. / ha.sin() - 1.),
                    end,
                ))
            }
        })
    }
}
impl CurveControls for CurveFromDihedron {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 5.;

    fn toggle_prop(&mut self) {
        log!("toggle");
        self.concavity.value = !self.concavity.value;
    }
    fn save_vars(&mut self) {
        self.dih.save_vars();
        self.radius.saved_val = self.radius.value;
        self.concavity.saved_val = self.concavity.value;
        self.offset.saved_val = self.offset.value;
    }
    fn restore_vars(&mut self) {
        self.dih.restore_vars();
        self.radius.value = self.radius.saved_val;
        self.concavity.value = self.concavity.saved_val;
        self.offset.value = self.offset.saved_val;
    }

    fn update_from_apices(&mut self, apex_prev: Vec2, apex_next: Vec2) -> Option<Vec2> {
        use CurveFromDihedronKind::*;
        let dih = Dihedron::from_three_points(apex_prev, self.dih.apex.pos, apex_next)?;
        dih.is_good().and_then(|(_, u_dir, ha)| {
            self.dih.u_dir.pos = -u_dir;
            self.dih.half_angle.value = ha;
            match self.curve_kind {
                Line => self.get_line_three_points().map(|(_, mid_pt, _)| mid_pt),
                Arc => self
                    .get_circle_three_points()
                    .map(|(_, third_pt, _)| third_pt),
                Point => Some(self.dih.apex.pos),
            }
        })
    }

    fn get_state(&self, hs: HS) -> Option<Vec2> {
        use CurveFromDihedronKind::*;
        self.state.is_hs(hs).then(|| {
            self.dih.is_good().and_then(|(..)| match self.curve_kind {
                Line => self.get_line_three_points().map(|(_, mid_pt, _)| mid_pt),
                Arc => self
                    .get_circle_three_points()
                    .map(|(_, third_pt, _)| third_pt),
                Point => Some(self.dih.apex.pos),
            })
        })?
    }
    fn set_state(&mut self, hs: HS, state: bool) {
        self.state.set_hs(hs, state);
    }
    fn get_dist_from_pos(&self, pointer_pos: Vec2) -> Option<(f64, Vec2)> {
        use CurveFromDihedronKind::*;
        match self.curve_kind {
            Line => {
                let (s, _, e) = self.get_line_three_points()?;
                distance_and_projection_to_segment(s, e, pointer_pos, Self::ANGLE_GUARD)
            }
            Arc => {
                let (s, a, e) = self.get_circle_three_points()?;
                let arc = arc_from_three_points(s, a, e)?;
                distance_and_projection_to_arc(&arc, pointer_pos, Self::ANGLE_GUARD)
            }
            Point => Some(((self.dih.apex.pos - pointer_pos).hypot(), self.dih.apex.pos)),
        }
    }

    fn move_control_selected(
        &mut self,
        _start: Vec2,
        _end: Vec2,
        pointer: &Pointer,
        _keys_states: KeysStates,
    ) -> bool {
        use CurveFromDihedronKind::*;
        self.state
            .is_hs(HS::Select)
            .then(|| {
                self.dih
                    .is_good()
                    .and_then(|(_, u_dir, ha)| match self.curve_kind {
                        Line => {
                            let dpos_proj = -u_dir.dot(pointer.dpos());
                            self.offset.value = self.offset.saved_val + dpos_proj;
                            Some(())
                        }
                        Arc => {
                            let dpos_proj = -u_dir.dot(pointer.dpos());
                            self.radius.value = self.radius.saved_val + dpos_proj * ha.sin();
                            Some(())
                        }

                        Point => None,
                    })
            })
            .is_some()
    }

    fn path_elements(&self) -> PrimitiveKindIter {
        use CurveFromDihedronKind::*;
        match self.curve_kind {
            Line => {
                // If the dihedron is good, return the path elements of the line.
                self.get_line_three_points()
                    .map(|(s, _, e)| {
                        PrimitiveKindIter::Line(
                            kurbo::Line::new(s.to_point(), e.to_point())
                                .path_elements(Self::TOLERANCE),
                        )
                    })
                    .unwrap_or_else(|| PrimitiveKindIter::None)
            }
            Arc => self
                .get_circle_three_points()
                .and_then(|(s, a, e)| {
                    arc_from_three_points(s, a, e)
                        .map(|arc| PrimitiveKindIter::Arc(arc.path_elements(Self::TOLERANCE)))
                })
                .unwrap_or_else(|| PrimitiveKindIter::None),
            Point => PrimitiveKindIter::None,
        }
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
        use CurveFromDihedronKind::*;
        match self.curve_kind {
            Line => self
                .get_line_three_points()
                .map(|(s, _, e)| Dimension::new(DimKind::Linear, s, e, 0.).get_path_and_pattern())
                // Convert the Option into an iterator (None yields an empty iterator)
                .into_iter()
                .collect(),
            Arc => {
                self.get_circle_three_points()
                    .and_then(|(s, a, e)| arc_from_three_points(s, a, e))
                    .map(|arc| {
                        // Compute an offset based on the arc's x-radius and the square root of 2.
                        let offset = arc.radii.x / 2_f64.sqrt();
                        // Determine an "end" point by shifting the arc's center.
                        let end = arc.center + Vec2::new(offset, -offset);
                        // Create a Dimension of type Radius, then retrieve its path and pattern.
                        Dimension::new(
                            DimKind::Radius,
                            arc.center.to_vec2(),
                            end.to_vec2(),
                            arc.radii.x,
                        )
                        .get_path_and_pattern()
                    })
                    // Convert the Option into an iterator (None yields an empty iterator)
                    .into_iter()
                    .collect()
            }
            Point => vec![],
        }
    }
}
