use super::{
    curves::{CurveControls, PrimitiveKindIter},
    curves_edge::Edge,
};
use crate::{
    canvas::Pattern,
    curves::curves_edge::EdgeKind,
    math::*,
    pools::HS,
    positions::{Status, Value},
    KeysStates, Pointer, Position,
};
use kurbo::{BezPath, Shape, Vec2};

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct Fillet {
    pub center: Vec2,
    pub start: Vec2,
    pub end: Vec2,
}

#[derive(Copy, Debug, Clone, PartialEq)]
pub enum WedgeKind {
    Chamfer,
    Fillet,
    Point,
}
impl WedgeKind {
    pub fn next(&mut self) {
        use WedgeKind::*;
        match self {
            Chamfer => *self = Fillet,
            Fillet => *self = Point,
            Point => *self = Chamfer,
        };
    }
    pub fn prev(&mut self) {
        use WedgeKind::*;
        match self {
            Chamfer => *self = Point,
            Fillet => *self = Chamfer,
            Point => *self = Fillet,
        };
    }
}

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct Wedge {
    apex: Position,
    fillet_radius: Value,
    // Calculated fields
    apex_state: Status,
    state: Status,
}
impl Wedge {
    const _ANGLE_GUARD: f64 = 0.02;
    const MIN_ARC_RADIUS: f64 = 20.;

    pub fn new(apex: Vec2) -> Self {
        Wedge {
            apex: Position::new(apex),
            fillet_radius: Value::new(Self::MIN_ARC_RADIUS),
            curve_kind: WedgeKind::Point,
            apex_state: Status::default(),
            state: Status::default(),
        }
    }
    pub fn prev(&mut self) {
        self.curve_kind.prev();
    }
    pub fn next(&mut self) {
        self.curve_kind.next();
    }
    pub fn get_apex(&self) -> &Position {
        &self.apex
    }
    pub fn set_apex(&mut self, apex: Vec2) {
        self.apex.pos = apex;
    }
    pub fn move_apex(&mut self, dpos: Vec2) {
        self.apex.move_pos(dpos);
    }

    pub fn get_apex_state(&self, hs: HS) -> Option<Vec2> {
        self.apex_state.is_hs(hs).then(|| Some(self.apex.pos))?
    }
    pub fn set_apex_state(&mut self, hs: HS, state: bool) {
        self.apex_state.set_hs(hs, state);
    }

    pub fn update_fillet(&self, edge_prev: &Edge, edge_next: &Edge) {
        use EdgeKind::*;
        use WedgeKind::*;
        edge_prev.get_seg_info().and_then(|seg_prev| {});

        edge_prev.get_seg_info().and_then(|seg_prev| {
            edge_next.get_seg_info().and_then(|seg_next| {
                match edge_prev.get_curve_kind() {
                    Line => Some(EdgeCurve::Line {
                        pt: seg_prev.start(),
                    }),
                    Arc => {
                        arc_from_three_points(seg_prev.start(), seg_prev.third_pt(), seg_prev.end())
                            .and_then(|arc| Some(EdgeCurve::Arc { arc }))
                    }
                }
                .and_then(|curve_prev| {
                    match edge_next.get_curve_kind() {
                        Line => Some(EdgeCurve::Line { pt: seg_next.end() }),
                        Arc => arc_from_three_points(
                            seg_next.start(),
                            seg_next.third_pt(),
                            seg_next.end(),
                        )
                        .and_then(|arc| Some(EdgeCurve::Arc { arc })),
                    }
                    .and_then(|curve_next| match self.curve_kind {
                        Chamfer => Some(fillet_between(
                            self.apex.pos,
                            curve_prev,
                            curve_next,
                            self.fillet_radius.value,
                        ))
                        .and_then(|(_, start, end)| Some(((start + end) / 2., start, end))),
                        Fillet => Some(fillet_between(
                            self.apex.pos,
                            curve_prev,
                            curve_next,
                            self.fillet_radius.value,
                        )),
                        Point => Some((self.apex.pos, self.apex.pos, self.apex.pos)),
                    })
                })
            })
        })
    }

    pub fn move_control_selected(
        &mut self,
        _prev_edge: &Edge,
        _next_edge: &Edge,
        _pointer: &Pointer,
        _keys_states: KeysStates,
    ) -> bool {
        use WedgeKind::*;
        self.state.is_hs(HS::Select).then(|| match self.curve_kind {
            Chamfer => {
                // let dpos_proj = u_dir.dot(pointer.dpos());
                // self.offset.value = self.offset.saved_val + dpos_proj;
                Some(())
            }
            Fillet => {
                // let dpos_proj = u_dir.dot(pointer.dpos());
                // self.radius.value = self.radius.saved_val + dpos_proj * ha.sin();
                Some(())
            }

            Point => None,
        });
        false
    }
    pub fn get_dist_from_pos(
        &self,
        edge_prev: &Edge,
        edge_next: &Edge,
        pointer_pos: Vec2,
    ) -> Option<(f64, Vec2)> {
        use WedgeKind::*;
        match self.curve_kind {
            Chamfer => self
                .update_fillet(edge_prev, edge_next)
                .and_then(|(_, start, end)| {
                    distance_and_projection_to_segment(start, end, pointer_pos, 0.)
                }),
            Fillet => self
                .update_fillet(edge_prev, edge_next)
                .and_then(|(center, start, end)| {
                    distance_and_projection_to_arc(
                        &arc_from_center_and_points(center, start, end)?,
                        pointer_pos,
                        0.,
                    )
                }),
            Point => None,
        }
    }
    fn path_elements(&self, edge_prev: &Edge, edge_next: &Edge) -> PrimitiveKindIter {
        use WedgeKind::*;
        match self.curve_kind {
            Chamfer => self
                .update_fillet(edge_prev, edge_next)
                .and_then(|(_, start, end)| {
                    Some(PrimitiveKindIter::Line(
                        kurbo::Line::new(start.to_point(), end.to_point())
                            .path_elements(Self::TOLERANCE),
                    ))
                })
                .unwrap_or_else(|| PrimitiveKindIter::None),

            Fillet => self
                .update_fillet(edge_prev, edge_next)
                .and_then(|(center, start, end)| {
                    Some(
                        arc_from_center_and_points(center, start, end)
                            .map(|arc| PrimitiveKindIter::Arc(arc.path_elements(Self::TOLERANCE)))
                            .unwrap_or_else(|| PrimitiveKindIter::None),
                    )
                })
                .unwrap_or_else(|| PrimitiveKindIter::None),

            Point => PrimitiveKindIter::None,
        }
    }
    pub fn get_paths_and_patterns(
        &self,
        edge_prev: &Edge,
        edge_next: &Edge,
        parent_selected: bool,
        parent_highlighted: bool,
    ) -> (BezPath, Pattern) {
        use HS::*;
        (
            self.path_elements(edge_prev, edge_next).collect(),
            self.get_pattern(
                self.state.is_hs(Select) || parent_selected,
                self.state.is_hs(Highlight) || parent_highlighted,
            ),
        )
    }
    // pub fn get_dimensions_paths_and_patterns(
    //     &self,
    //     _das: &Size,
    // ) -> Vec<(BezPath, Pattern, CanvasText)> {
    //     use CurveWedgeKind::*;
    //     match self.curve_kind {
    //         Chamfer => vec![],
    //         Fillet => vec![],
    //         Point => vec![],
    //     }
    // }
}
impl CurveControls for Wedge {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 5.;

    fn save_vars(&mut self) {
        self.apex.saved_pos = self.apex.pos;
        self.fillet_radius.saved_val = self.fillet_radius.value;
    }
    fn restore_vars(&mut self) {
        self.apex.pos = self.apex.saved_pos;
        self.fillet_radius.value = self.fillet_radius.saved_val;
    }

    fn get_state(&self, hs: HS) -> Option<Vec2> {
        use WedgeKind::*;
        self.state.is_hs(hs).then(|| match self.curve_kind {
            Chamfer => Some(self.apex.pos), //Todo
            Fillet => Some(self.apex.pos),  //Todo
            Point => Some(self.apex.pos),
        })?
    }
    fn set_state(&mut self, hs: HS, state: bool) {
        self.state.set_hs(hs, state);
    }
}
