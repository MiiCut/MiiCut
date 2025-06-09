use super::{
    curves::{CurveControls, PrimitiveKindIter},
    curves_wedge::Wedge,
};
use crate::{
    canvas::Pattern, math::*, pools::HS, positions::Status, KeysStates, Pointer, Position,
};
use kurbo::{BezPath, Shape, Vec2};

#[derive(Copy, Debug, Clone, PartialEq)]
pub enum EdgeKind {
    Line,
    Arc,
}
impl EdgeKind {
    pub fn next(&mut self) {
        use EdgeKind::*;
        match self {
            Line => *self = Arc,
            Arc => *self = Line,
        };
    }
    pub fn prev(&mut self) {
        use EdgeKind::*;
        match self {
            Line => *self = Arc,
            Arc => *self = Line,
        };
    }
}

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct SegInfo {
    start: Position,
    end: Position,
    third_pt: Position,
}
impl SegInfo {
    pub fn new(start: Vec2, end: Vec2) -> Option<Self> {
        // Vector representing the segment
        let v = end - start;
        if v.hypot() < EPSILON {
            log!("Error: Degenerate case SegInfo::new()");
            return None;
        }
        let clockwise = false;
        // Compute the perpendicular vector
        let n_dir = if clockwise {
            Vec2::new(-v.y, v.x).normalize() // Clockwise
        } else {
            Vec2::new(v.y, -v.x).normalize() // Counterclockwise
        };
        let third_pt = Position::new((start + end) / 2. + n_dir * (start - end).hypot() / 2.);
        Some(SegInfo {
            start: Position::new(start),
            end: Position::new(end),
            third_pt,
        })
    }
    pub fn start(&self) -> Vec2 {
        self.start.pos
    }
    pub fn end(&self) -> Vec2 {
        self.end.pos
    }
    pub fn third_pt(&self) -> Vec2 {
        self.third_pt.pos
    }
    pub fn third_pt_len(&self) -> f64 {
        (self.third_pt.pos - self.mid()).hypot()
    }
    pub fn u_dir(&self) -> Vec2 {
        (self.end.pos - self.start.pos).normalize()
    }
    pub fn n_dir(&self) -> Vec2 {
        let clockwise = false;
        // Vector representing the segment
        let v = self.end.pos - self.start.pos;
        // Compute the perpendicular vector
        let perp = if clockwise {
            Vec2::new(-v.y, v.x) // Clockwise
        } else {
            Vec2::new(v.y, -v.x) // Counterclockwise
        };
        perp.normalize()
    }
    pub fn mid(&self) -> Vec2 {
        (self.start.pos + self.end.pos) / 2.
    }
    pub fn len(&self) -> f64 {
        (self.start.pos - self.end.pos).hypot()
    }

    fn new_n_dir(&self, start: Vec2, end: Vec2) -> Vec2 {
        let clockwise = false;
        // Vector representing the segment
        let v = end - start;
        // Compute the perpendicular vector
        let perp = if clockwise {
            Vec2::new(-v.y, v.x) // Clockwise
        } else {
            Vec2::new(v.y, -v.x) // Counterclockwise
        };
        perp.normalize()
    }
    pub fn try_set_start(&mut self, start: Vec2) -> bool {
        if (self.end.pos - start).hypot() < EPSILON {
            return false;
        }
        // Calculate the new third point
        let new_n_dir = self.new_n_dir(start, self.end.pos);
        let new_seg_len = (start - self.end.pos).hypot();
        let keep_sign = (self.third_pt.pos - self.mid())
            .cross(self.mid() - self.start.pos)
            .signum();
        self.third_pt = Position::new(
            (start + self.end.pos) / 2.
                + new_n_dir * self.third_pt_len() * new_seg_len / self.len() * keep_sign,
        );
        // Save new position
        self.start = Position::new(start);
        true
    }
    pub fn try_set_end(&mut self, end: Vec2) -> bool {
        if (end - self.start.pos).hypot() < EPSILON {
            return false;
        }
        // Calculate the new third point
        let new_n_dir = self.new_n_dir(self.start.pos, end);
        let new_seg_len = (self.start.pos - end).hypot();
        let keep_sign = (self.third_pt.pos - self.mid())
            .cross(self.mid() - self.start.pos)
            .signum();
        self.third_pt = Position::new(
            (self.start.pos + end) / 2.
                + new_n_dir * self.third_pt_len() * new_seg_len / self.len() * keep_sign,
        );
        // Save new position
        self.end = Position::new(end);
        true
    }
    pub fn save_vars(&mut self) {
        self.start.saved_pos = self.start.pos;
        self.end.saved_pos = self.end.pos;
        self.third_pt.saved_pos = self.third_pt.pos;
    }
    pub fn restore_vars(&mut self) {
        self.start.pos = self.start.saved_pos;
        self.end.pos = self.end.saved_pos;
        self.third_pt.pos = self.third_pt.saved_pos;
    }
}

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct Edge {
    seg_info: Option<SegInfo>,
    edge_kind: EdgeKind,
    state: Status,
}
impl Edge {
    const ANGLE_GUARD: f64 = 0.02;

    pub fn new(start: Vec2, end: Vec2) -> Self {
        Self {
            seg_info: SegInfo::new(start, end),
            edge_kind: EdgeKind::Line,
            state: Status::default(),
        }
    }
    pub fn try_set_start(&mut self, start: Vec2) -> bool {
        if let Some(seg) = self.seg_info.as_mut() {
            seg.try_set_start(start)
        } else {
            false
        }
    }
    pub fn try_set_end(&mut self, end: Vec2) -> bool {
        if let Some(seg) = self.seg_info.as_mut() {
            seg.try_set_end(end)
        } else {
            false
        }
    }
    pub fn prev(&mut self) {
        self.edge_kind.prev();
    }
    pub fn next(&mut self) {
        self.edge_kind.next();
    }
    pub fn get_seg_info(&self) -> Option<SegInfo> {
        self.seg_info
    }
    pub fn get_curve_kind(&self) -> EdgeKind {
        self.edge_kind
    }

    pub fn move_control_selected(&mut self, pointer: &Pointer, _keys_states: KeysStates) -> bool {
        use EdgeKind::*;
        self.state
            .is_hs(HS::Select)
            .then(|| {
                if let Some(seg) = self.seg_info.as_mut() {
                    match &mut self.edge_kind {
                        Arc => {
                            let dpos_proj = seg.n_dir().dot(pointer.dpos());
                            seg.third_pt.pos = seg.third_pt.saved_pos + seg.n_dir() * dpos_proj;
                            Some(())
                        }
                        Line => None,
                    }
                } else {
                    None
                }
            })
            .is_some()
    }
    pub fn get_dist_from_pos(&self, pointer_pos: Vec2) -> Option<(f64, Vec2)> {
        use EdgeKind::*;
        self.seg_info.and_then(|seg| match self.edge_kind {
            Line => distance_and_projection_to_segment(
                seg.start.pos,
                seg.end.pos,
                pointer_pos,
                2. * Self::GRAB,
            ),
            Arc => {
                let arc = arc_from_three_points(seg.start.pos, seg.third_pt.pos, seg.end.pos)?;
                distance_and_projection_to_arc(&arc, pointer_pos, Self::ANGLE_GUARD)
            }
        })
    }

    fn path_elements(&self, wedge_prev: Wedge, wedge_next: Wedge) -> PrimitiveKindIter {
        use EdgeKind::*;

        match self.edge_kind {
            Line => PrimitiveKindIter::Line(
                kurbo::Line::new(start.to_point(), end.to_point()).path_elements(Self::TOLERANCE),
            ),
            Arc => {
                if let Some(seg) = self.seg_info {
                    if let Some(arc) =
                        arc_from_three_points(seg.start.pos, seg.third_pt.pos, seg.end.pos)
                    {
                        if let Some(sub_arc) = sub_arc(arc, start, end) {
                            PrimitiveKindIter::Arc(sub_arc.path_elements(Self::TOLERANCE))
                        } else {
                            PrimitiveKindIter::None
                        }
                    } else {
                        PrimitiveKindIter::None
                    }
                } else {
                    PrimitiveKindIter::None
                }
            }
        }
    }
    pub fn get_paths_and_patterns(
        &self,
        wedge_prev: Wedge,
        wedge_next: Wedge,
        parent_selected: bool,
        parent_highlighted: bool,
    ) -> (BezPath, Pattern) {
        use HS::*;
        (
            self.path_elements(wedge_prev, wedge_next).collect(),
            self.get_pattern(
                self.state.is_hs(Select) || parent_selected,
                self.state.is_hs(Highlight) || parent_highlighted,
            ),
        )
    }
    // pub fn get_dimensions_paths_and_patterns(
    //     &self,
    //     start_apex: Vec2,
    //     end_apex: Vec2,
    //     _das: &Size,
    // ) -> Vec<(BezPath, Pattern, CanvasText)> {
    //     use CurveEdgeKind::*;
    //     match self.curve_kind {
    //         Line(_) => {
    //             vec![Dimension::new(DimKind::Linear, start_apex, end_apex, 0.)
    //                 .get_path_and_pattern()]
    //         }
    //         Arc(third_pt) => {
    //             unit_perpendicular(self.start.pos, self.end.pos, false)
    //                 .and_then(|(..)| {
    //                     arc_from_three_points(self.start.pos, third_pt.pos, self.end.pos)
    //                         .map(|arc| (arc, third_pt.pos))
    //                 })
    //                 .map(|(arc, a)| {
    //                     // Compute an offset based on the arc's x-radius and the square root of 2.
    //                     let _ = arc.radii.x / 2_f64.sqrt();
    //                     // Determine an "end" point by shifting the arc's center.
    //                     // let end = arc.center + Vec2::new(offset, -offset);
    //                     let end = a + (a - arc.center.to_vec2()).normalize() * 20.;
    //                     // Create a Dimension of type Radius, then retrieve its path and pattern.
    //                     Dimension::new(DimKind::Radius, a, end, arc.radii.x).get_path_and_pattern()
    //                 })
    //                 // Convert the Option into an iterator (None yields an empty iterator)
    //                 .into_iter()
    //                 .collect()
    //         }
    //     }
    // }
}
impl CurveControls for Edge {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 5.;

    fn save_vars(&mut self) {
        self.seg_info.as_mut().map(|seg| seg.save_vars());
    }
    fn restore_vars(&mut self) {
        self.seg_info.as_mut().map(|seg| seg.restore_vars());
    }
    fn get_state(&self, hs: HS) -> Option<Vec2> {
        use EdgeKind::*;
        self.state.is_hs(hs).then(|| {
            self.seg_info.and_then(|seg| match self.edge_kind {
                Line => Some((seg.start() + seg.end()) / 2.),
                Arc => Some(seg.third_pt()),
            })
        })?
    }
    fn set_state(&mut self, hs: HS, state: bool) {
        self.state.set_hs(hs, state);
    }
}
