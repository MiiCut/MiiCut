use super::curves::{CurveControls, PrimitiveKindIter};
use crate::{
    canvas::{CanvasText, Pattern},
    dimensions::{DimKind, Dimension},
    math::*,
    pools::HS,
    positions::{Status, Value, ValueBool},
    KeysStates, Pointer, Position,
};
use kurbo::{BezPath, Line, Shape, Size, Vec2};

#[derive(Copy, Debug, Clone, PartialEq, Default)]
enum CurveArcKind {
    FromDihedron(Vec2),
    FromStartEnd,
    #[default]
    NoInit,
}
#[derive(Copy, Debug, Clone, PartialEq, Default)]
pub struct CurveArc {
    start: Position,
    end: Position,
    radius: Value,
    // Dihedron: true: near the apex, false: far the apex
    concavity: ValueBool,
    kind: CurveArcKind,
    kind_saved: CurveArcKind,
    state: Status,
}
impl CurveArc {
    const ANGLE_GUARD: f64 = 0.02;
    const MIN_RADIUS: f64 = 5.;
    const _MAX_RADIUS: f64 = 1000.;

    pub fn get_start(&self) -> Position {
        self.start
    }
    pub fn get_end(&self) -> Position {
        self.end
    }
    pub fn get_radius(&self) -> f64 {
        self.radius.value
    }
    fn get_third_pt(&self) -> Option<Vec2> {
        use CurveArcKind::*;
        match self.kind {
            FromDihedron(apex) => {
                if let Some((bisector_dir, half_angle)) =
                    bisector_dir_and_angle(self.end.pos, apex, self.start.pos)
                {
                    if self.concavity.value {
                        Some(
                            apex + bisector_dir
                                * ((self.end.pos - apex).hypot() / half_angle.cos()
                                    + self.radius.value),
                        )
                    } else {
                        Some(
                            apex + bisector_dir
                                * ((self.end.pos - apex).hypot() / half_angle.cos()
                                    - self.radius.value),
                        )
                    }
                } else {
                    None
                }
            }
            FromStartEnd => {
                if let Some(p_unit) = unit_perpendicular(self.start.pos, self.end.pos, false) {
                    let r = self.radius.value;
                    let s = self.start.pos;
                    let e = self.end.pos;
                    let mid_pos = (s + e) / 2.;
                    // let mid_len = (s - e).hypot() / 2.;
                    if self.concavity.value {
                        Some(mid_pos - p_unit * 2. * r)
                    } else {
                        Some(mid_pos + p_unit * 2. * r)
                    }
                } else {
                    None
                }
            }
            NoInit => None,
        }
    }
}
impl CurveControls for CurveArc {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 5.;

    fn toggle_prop(&mut self) {
        log!("toggle");
        ()
    }
    fn save_vars(&mut self) {
        self.start.saved_pos = self.start.pos;
        self.end.saved_pos = self.end.pos;
        self.radius.saved_val = self.radius.value;
        self.kind_saved = self.kind;
        self.concavity.saved_val = self.concavity.value;
    }
    fn restore_vars(&mut self) {
        self.start.pos = self.start.saved_pos;
        self.end.pos = self.end.saved_pos;
        self.radius.value = self.radius.saved_val;
        self.kind = self.kind_saved;
        self.concavity.value = self.concavity.saved_val;
    }

    fn set_from_start_end(&mut self, start: Position, end: Position) -> Option<Vec2> {
        use CurveArcKind::*;
        if (start.pos - end.pos).hypot() < EPSILON {
            return None;
        }
        if let NoInit = self.kind {
            self.radius.value = (start.pos - end.pos).hypot() / 2.;
            self.kind = FromStartEnd;
        }
        self.start.pos = start.pos;
        self.end.pos = end.pos;
        self.get_third_pt()
    }
    fn set_from_dihedron(
        &mut self,
        p_prev: Position,
        p: Position,
        p_next: Position,
    ) -> Option<Vec2> {
        use CurveArcKind::*;
        if let Some((_dir, half_angle)) = bisector_dir_and_angle(p_next.pos, p.pos, p_prev.pos) {
            if let NoInit = self.kind {
                self.radius.value = Self::MIN_RADIUS;
            }

            // Update apex
            self.kind = FromDihedron(p.pos);

            // half_angle.sin() is never zero
            let center = self.radius.value / half_angle.sin();

            // normalize() is always safe
            self.start.pos = p.pos + (p_prev.pos - p.pos).normalize() * center * half_angle.cos();
            self.end.pos = p.pos + (p_next.pos - p.pos).normalize() * center * half_angle.cos();

            self.get_third_pt()
        } else {
            None
        }
    }

    fn get_state(&self, hs: HS) -> Option<Vec2> {
        if self.state.is_hs(hs) {
            self.get_third_pt()
        } else {
            None
        }
    }
    fn set_state(&mut self, hs: HS, state: bool) {
        self.state.set_hs(hs, state);
    }
    fn get_dist_from_pos(&self, pointer_pos: Vec2) -> Option<(f64, Vec2)> {
        let third_pt = self.get_third_pt()?;
        let arc = arc_from_three_points(self.start.pos, third_pt, self.end.pos)?;
        let (dist, pos) = distance_and_projection_to_arc(&arc, pointer_pos, Self::ANGLE_GUARD)?;
        Some((dist, pos))
    }

    fn move_control_selected(
        &mut self,
        _start: Vec2,
        _end: Vec2,
        pointer: &Pointer,
        _keys_states: KeysStates,
    ) -> bool {
        if self.state.is_hs(HS::Select) {
            use CurveArcKind::*;
            let s = self.start.pos;
            let e = self.end.pos;
            let mid: Vec2 = (s + e) / 2.;
            match self.kind {
                FromDihedron(apex) => {
                    let dpos_proj = (apex - mid).dot(pointer.dpos());
                    true
                }
                FromStartEnd => {
                    if let Some(p_unit) = unit_perpendicular(self.start.pos, self.end.pos, false) {
                        let mut center = if self.concavity.value {
                            mid - p_unit * self.radius.saved_val
                        } else {
                            mid + p_unit * self.radius.saved_val
                        };
                        center += p_unit * p_unit.dot(pointer.dpos());
                        self.radius.value = (center - s).hypot();
                    }
                    true
                }
                NoInit => false,
            }
        } else {
            false
        }
    }

    fn path_elements(&self) -> PrimitiveKindIter {
        self.get_third_pt()
            .and_then(|third_pt| arc_from_three_points(self.start.pos, third_pt, self.end.pos))
            .map(|arc| PrimitiveKindIter::Arc(arc.path_elements(Self::TOLERANCE)))
            .unwrap_or_else(|| {
                PrimitiveKindIter::Line(
                    Line::new(self.start.pos.to_point(), self.end.pos.to_point())
                        .path_elements(Self::TOLERANCE),
                )
            })
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
        self.get_third_pt()
            .and_then(|third_pt| arc_from_three_points(self.start.pos, third_pt, self.end.pos))
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
}
