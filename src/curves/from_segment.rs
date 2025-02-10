use super::curves::{CurveControls, PrimitiveKindIter};
use crate::{
    canvas::{CanvasText, Pattern},
    dimensions::{DimKind, Dimension},
    math::*,
    pools::HS,
    positions::Status,
    KeysStates, Pointer, Position,
};
use kurbo::{BezPath, Shape, Size, Vec2};

#[derive(Copy, Debug, Clone, PartialEq)]
pub enum CurveFromSegmentKind {
    Line,
    Arc,
}
#[derive(Copy, Debug, Clone, PartialEq)]
pub struct Segment {
    start: Position,
    end: Position,
}
impl Segment {
    pub fn new(start: Vec2, end: Vec2) -> Option<Self> {
        ((start - end).hypot() > EPSILON).then(|| Segment {
            start: Position::new(start),
            end: Position::new(end),
        })
    }
    pub fn is_good(&self) -> Option<(Vec2, Vec2)> {
        unit_perpendicular(self.start.pos, self.end.pos, false)
    }
    pub fn save_vars(&mut self) {
        self.start.saved_pos = self.start.pos;
        self.end.saved_pos = self.end.pos;
    }
    pub fn restore_vars(&mut self) {
        self.start.pos = self.start.saved_pos;
        self.end.pos = self.end.saved_pos;
    }
    pub fn s(self) -> Vec2 {
        self.start.pos
    }
    pub fn e(self) -> Vec2 {
        self.end.pos
    }
    pub fn set_s(&mut self, s: Vec2) {
        self.start.pos = s;
    }
    pub fn set_e(&mut self, e: Vec2) {
        self.end.pos = e;
    }
}

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct CurveFromSegment {
    seg: Segment,
    third_pt: Position,
    curve_kind: CurveFromSegmentKind,
    state: Status,
}
impl CurveFromSegment {
    const ANGLE_GUARD: f64 = 0.02;
    // const MIN_ARC_RADIUS: f64 = 5.;
    // const MIN_LINE_LEN: f64 = 2. * Self::MIN_ARC_RADIUS;

    pub fn new(seg: Segment) -> Option<Self> {
        seg.is_good().and_then(|(p_unit, mid_pt)| {
            let half_len = (seg.s() - seg.e()).hypot() / 2.;
            let third_pt = mid_pt + p_unit * half_len;
            Some(CurveFromSegment {
                seg,
                third_pt: Position::new(third_pt),
                curve_kind: CurveFromSegmentKind::Line,
                state: Status::default(),
            })
        })
    }
    pub fn get_third_pt(&self) -> Vec2 {
        self.third_pt.pos
    }
    pub fn next(&mut self) {
        use CurveFromSegmentKind::*;
        self.curve_kind = match self.curve_kind {
            Line => Arc,
            Arc => Line,
        };
    }
    pub fn prev(&mut self) {
        use CurveFromSegmentKind::*;
        self.curve_kind = match self.curve_kind {
            Line => Arc,
            Arc => Line,
        };
    }
}
impl CurveControls for CurveFromSegment {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 5.;

    fn toggle_prop(&mut self) {
        log!("toggle");
        ()
    }
    fn save_vars(&mut self) {
        self.seg.save_vars();
        self.third_pt.saved_pos = self.third_pt.pos;
    }
    fn restore_vars(&mut self) {
        self.seg.restore_vars();
        self.third_pt.pos = self.third_pt.saved_pos;
    }

    fn update_from_segment(&mut self, seg: &Segment) -> Option<Vec2> {
        use CurveFromSegmentKind::*;
        match (self.seg.is_good(), seg.is_good()) {
            (Some((_, old_mid_pt)), Some((new_p_unit, new_mid_pt))) => match self.curve_kind {
                Line => {
                    self.seg = *seg;
                    let old_len = (self.third_pt.pos - old_mid_pt).hypot();
                    let old_seg_len = (self.seg.s() - self.seg.e()).hypot();
                    let sign = (self.third_pt.pos - old_mid_pt)
                        .cross(old_mid_pt - self.seg.s())
                        .signum();
                    let new_seg_len = (seg.s() - seg.e()).hypot();
                    self.third_pt.pos =
                        new_mid_pt + new_p_unit * old_len * new_seg_len / old_seg_len * sign;
                    Some(new_mid_pt)
                }
                Arc => {
                    let old_len = (self.third_pt.pos - old_mid_pt).hypot();
                    let old_seg_len = (self.seg.s() - self.seg.e()).hypot();
                    let sign = (self.third_pt.pos - old_mid_pt)
                        .cross(old_mid_pt - self.seg.s())
                        .signum();
                    let new_seg_len = (seg.s() - seg.e()).hypot();
                    self.third_pt.pos =
                        new_mid_pt + new_p_unit * old_len * new_seg_len / old_seg_len * sign;

                    self.seg = *seg;
                    Some(self.third_pt.pos)
                }
            },
            (Some((..)), None) => Some(self.third_pt.pos),
            _ => None,
        }
    }

    fn get_state(&self, hs: HS) -> Option<Vec2> {
        use CurveFromSegmentKind::*;
        self.seg.is_good().and_then(|(_, mid_pt)| {
            self.state.is_hs(hs).then(|| match self.curve_kind {
                Line => Some(mid_pt),
                Arc => Some(self.third_pt.pos),
            })
        })?
    }
    fn set_state(&mut self, hs: HS, state: bool) {
        self.state.set_hs(hs, state);
    }
    fn get_dist_from_pos(&self, pointer_pos: Vec2) -> Option<(f64, Vec2)> {
        use CurveFromSegmentKind::*;
        self.seg.is_good().and_then(|(..)| match self.curve_kind {
            Line => distance_and_projection_to_segment(
                self.seg.s(),
                self.seg.e(),
                pointer_pos,
                Self::ANGLE_GUARD,
            ),
            Arc => {
                let arc = arc_from_three_points(self.seg.s(), self.third_pt.pos, self.seg.e())?;
                distance_and_projection_to_arc(&arc, pointer_pos, Self::ANGLE_GUARD)
            }
        })
    }

    fn move_control_selected(
        &mut self,
        _start: Vec2,
        _end: Vec2,
        pointer: &Pointer,
        _keys_states: KeysStates,
    ) -> bool {
        use CurveFromSegmentKind::*;
        self.state
            .is_hs(HS::Select)
            .then(|| {
                self.seg
                    .is_good()
                    .and_then(|(p_unit, _)| match self.curve_kind {
                        Arc => {
                            let dpos_proj = p_unit.dot(pointer.dpos());
                            self.third_pt.pos = self.third_pt.saved_pos + p_unit * dpos_proj;
                            Some(())
                        }
                        Line => None,
                    })
            })
            .is_some()
    }

    fn path_elements(&self) -> PrimitiveKindIter {
        use CurveFromSegmentKind::*;
        match self.curve_kind {
            Line =>
            // PrimitiveKindIter::Line(ExtLinePathIter {
            //     line: kurbo::Line::new(self.seg.s().to_point(), self.seg.e().to_point()),
            //     ix: 0,
            // }),
            {
                PrimitiveKindIter::Line(
                    kurbo::Line::new(self.seg.s().to_point(), self.seg.e().to_point())
                        .path_elements(Self::TOLERANCE),
                )
            }
            Arc => self
                .seg
                .is_good()
                .and_then(|(..)| {
                    arc_from_three_points(self.seg.s(), self.third_pt.pos, self.seg.e())
                        .map(|arc| PrimitiveKindIter::Arc(arc.path_elements(Self::TOLERANCE)))
                })
                .unwrap_or_else(|| {
                    PrimitiveKindIter::Line(
                        kurbo::Line::new(self.seg.s().to_point(), self.seg.e().to_point())
                            .path_elements(Self::TOLERANCE),
                    )
                    // PrimitiveKindIter::Line(ExtLinePathIter {
                    //     line: kurbo::Line::new(self.seg.s().to_point(), self.seg.e().to_point()),
                    //     ix: 0,
                    // })
                }),
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
        use CurveFromSegmentKind::*;
        match self.curve_kind {
            Line => {
                vec![
                    Dimension::new(DimKind::Linear, self.seg.s(), self.seg.e(), 0.)
                        .get_path_and_pattern(),
                ]
            }
            Arc => {
                self.seg
                    .is_good()
                    .and_then(|(..)| {
                        arc_from_three_points(self.seg.s(), self.third_pt.pos, self.seg.e())
                    })
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
    }
}
