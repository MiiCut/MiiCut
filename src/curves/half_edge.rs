use crate::{
    canvas::{Color, Colors, Pattern},
    math::{
        arc_from_center_and_points, arc_from_three_points, bissector, circle_from_three_points,
        circle_line_intersection, distance_and_projection_to_arc,
        distance_and_projection_to_segment, nearest_circle_point, project_point_on_line, SegBundle,
    },
    pools::HS,
    positions::{Position, Status, Value},
};
use kurbo::{ArcAppendIter, BezPath, CubicBezIter, LinePathIter, PathEl, QuadBezIter, Shape, Vec2};
use std::f64::consts::PI;

pub enum PrimitiveKindIter {
    PNone,
    PLine(LinePathIter),
    PArc(std::iter::Chain<std::iter::Once<PathEl>, ArcAppendIter>),
    QBez(QuadBezIter),
    QBezSmooth(QuadBezIter),
    CBez(CubicBezIter),
    CBezSmooth(CubicBezIter),
}
impl Iterator for PrimitiveKindIter {
    type Item = PathEl;
    fn next(&mut self) -> Option<Self::Item> {
        use PrimitiveKindIter::*;
        match self {
            PNone => Option::None,
            PLine(sh) => sh.next(),
            PArc(sh) => sh.next(),
            QBez(sh) => sh.next(),
            QBezSmooth(sh) => sh.next(),
            CBez(sh) => sh.next(),
            CBezSmooth(sh) => sh.next(),
        }
    }
}

#[derive(Copy, Debug, Clone, PartialEq)]
pub enum KShape {
    KLine(kurbo::Line),
    KArc(kurbo::Arc),
    KPoint(kurbo::Point),
}

#[derive(Copy, Debug, Clone)]
pub struct HEProps {
    pub vertex_selectable: bool,
    pub vertex_changeable: bool,
    pub vertex_movable: bool,
    pub edge_selectable: bool,
    pub edge_changeable: bool,
    pub edge_movable: bool,
    pub corner_selectable: bool,
    pub corner_changeable: bool,
    pub corner_movable: bool,
}
impl Default for HEProps {
    fn default() -> Self {
        Self {
            vertex_selectable: true,
            vertex_changeable: true,
            vertex_movable: true,
            edge_selectable: true,
            edge_changeable: true,
            edge_movable: true,
            corner_selectable: true,
            corner_changeable: true,
            corner_movable: true,
        }
    }
}

// sagitta_rel:
// The corner vertex and the next corner vertex form a segment. In case the edge is an arc,
// this segment is a chord of an arc. The distance beween the mid point of this segment and the mid
// point of the arc is called the sagitta.
// We define the sag_rel that is the sagitta divided by the segment length.
// A value of 0.5 means the sagitta is egual to half the segment length (hence the chord is a diameter)
// We sign this value to indicate the side of the arc relative to the segment
#[derive(Copy, Debug, Clone, PartialEq)]
pub enum EdgeKind {
    Segment { dum: Value },
    Arc { sag_rel: Value },
}
impl EdgeKind {
    pub fn next(&mut self) {
        use EdgeKind::*;
        match self {
            Segment { dum } => *self = Arc { sag_rel: *dum },
            Arc { sag_rel } => *self = Segment { dum: *sag_rel },
        };
    }
    pub fn prev(&mut self) {
        use EdgeKind::*;
        match self {
            Segment { dum } => *self = Arc { sag_rel: *dum },
            Arc { sag_rel } => *self = Segment { dum: *sag_rel },
        };
    }
    pub fn save_vars(&mut self) {
        match self {
            EdgeKind::Segment { dum } => dum.saved_val = dum.value,
            EdgeKind::Arc { sag_rel } => sag_rel.saved_val = sag_rel.value,
        }
    }
    pub fn restore_vars(&mut self) {
        match self {
            EdgeKind::Segment { dum } => dum.value = dum.saved_val,
            EdgeKind::Arc { sag_rel } => sag_rel.value = sag_rel.saved_val,
        }
    }
}

#[derive(Copy, Debug, Clone, PartialEq)]
pub enum CornerKind {
    Chamfer { length: Value },
    Fillet { radius: Value },
    Point { dummy: Value },
}
impl CornerKind {
    pub fn next(&mut self) {
        use CornerKind::*;
        match self {
            Point { dummy } => *self = Chamfer { length: *dummy },
            Chamfer { length } => *self = Fillet { radius: *length },
            Fillet { radius } => *self = Point { dummy: *radius },
        };
    }
    pub fn save_vars(&mut self) {
        match self {
            CornerKind::Point { dummy } => dummy.saved_val = dummy.value,
            CornerKind::Chamfer { length } => length.saved_val = length.value,
            CornerKind::Fillet { radius } => radius.saved_val = radius.value,
        }
    }
    pub fn restore_vars(&mut self) {
        match self {
            CornerKind::Point { dummy } => dummy.value = dummy.saved_val,
            CornerKind::Chamfer { length } => length.value = length.saved_val,
            CornerKind::Fillet { radius } => radius.value = radius.saved_val,
        }
    }
}

#[derive(Copy, Debug, Clone)]
pub struct HalfEdge {
    props: HEProps,
    saved_props: HEProps,
    vertex: Position,
    vertex_state: Status,
    corner: CornerKind,
    corner_state: Status,
    edge: EdgeKind,
    edge_state: Status,
    // Calculated data
    s: Vec2,
    e: Vec2,
    c: Vec2,
}
impl HalfEdge {
    const ANGLE_GUARD: f64 = 0.02;
    const VERTEX_GUARD: f64 = 2.;
    // const GRAB: f64 = 5.;
    const TOLERANCE: f64 = 0.01;
    const MIN_FILLET_RADIUS: f64 = 2.;

    pub fn new(v: Vec2, props: HEProps) -> Self {
        // By default the corner is a point and the edge is a line segment
        Self {
            props,
            saved_props: props,
            vertex: Position::new(v),
            vertex_state: Status::default(),
            corner: CornerKind::Point {
                dummy: Value::new(10. * Self::MIN_FILLET_RADIUS),
            },
            corner_state: Status::default(),
            edge: EdgeKind::Segment {
                dum: Value::new(0.5), // The sagitta is half the segment length
            },
            edge_state: Status::default(),
            s: v,
            e: v,
            c: v,
        }
    }
    pub fn set_vertex_state(&mut self, hs: HS, value: bool) {
        if self.props.vertex_selectable {
            self.vertex_state.set_hs(hs, value);
        }
    }
    pub fn set_corner_state(&mut self, hs: HS, value: bool) {
        if self.props.corner_selectable {
            self.corner_state.set_hs(hs, value);
        }
    }
    pub fn set_edge_state(&mut self, hs: HS, value: bool) {
        if self.props.edge_selectable {
            self.edge_state.set_hs(hs, value);
        }
    }
    pub fn get_vertex_state(&self, hs: HS) -> bool {
        self.vertex_state.is_hs(hs)
    }
    pub fn vertex_state(&self) -> Status {
        self.vertex_state
    }
    pub fn get_corner_state(&self, hs: HS) -> bool {
        self.corner_state.is_hs(hs)
    }
    pub fn get_edge_state(&self, hs: HS) -> bool {
        self.edge_state.is_hs(hs)
    }
    pub fn edge_state(&self) -> Status {
        self.edge_state
    }

    pub fn is_vertex_selectable(&self) -> bool {
        self.props.vertex_selectable
    }
    pub fn is_vertex_changeable(&self) -> bool {
        self.props.vertex_changeable
    }
    pub fn is_vertex_movable(&self) -> bool {
        self.props.vertex_movable
    }

    pub fn is_edge_selectable(&self) -> bool {
        self.props.edge_selectable
    }
    pub fn is_edge_changeable(&self) -> bool {
        self.props.edge_changeable
    }
    pub fn is_edge_movable(&self) -> bool {
        self.props.edge_movable
    }

    pub fn is_corner_selectable(&self) -> bool {
        self.props.corner_selectable
    }
    pub fn is_corner_changeable(&self) -> bool {
        self.props.corner_changeable
    }
    pub fn is_corner_movable(&self) -> bool {
        self.props.corner_movable
    }

    pub fn corner_next_kind(&mut self) {
        if self.props.corner_changeable {
            self.corner.next();
        }
    }
    pub fn edge_next_kind(&mut self) {
        if self.props.edge_changeable {
            self.edge.next();
        }
    }
    pub fn get_edge_kind(&self) -> &EdgeKind {
        &self.edge
    }
    pub fn get_edge_kind_mut(&mut self) -> &mut EdgeKind {
        &mut self.edge
    }
    pub fn set_edge_kind(&mut self, edge: EdgeKind) {
        self.edge = edge;
    }

    pub fn save_vars(&mut self) {
        self.saved_props = self.props;
        self.corner.save_vars();
        self.edge.save_vars();
        self.vertex.saved_pos = self.vertex.pos;
    }
    pub fn restore_vars(&mut self) {
        self.props = self.saved_props;
        self.corner.restore_vars();
        self.edge.restore_vars();
        self.vertex.pos = self.vertex.saved_pos;
    }
    pub fn get_s(&self) -> Vec2 {
        self.s
    }
    pub fn get_e(&self) -> Vec2 {
        self.e
    }
    pub fn get_c(&self) -> Vec2 {
        self.c
    }

    // Update s, e, c data
    pub fn update_data(&mut self, vertex_prev: Vec2, edge_prev: EdgeKind, vertex_next: Vec2) {
        use CornerKind::*;
        use EdgeKind::*;
        match self.corner {
            Point { dummy: _ } => {
                self.s = self.vertex.pos;
                self.e = self.vertex.pos;
                self.c = self.vertex.pos;
            }
            Chamfer { length } => match (edge_prev, self.edge) {
                (Segment { dum: _ }, Segment { dum: _ }) => {
                    bissector(vertex_prev, self.vertex.pos, vertex_next).map(
                        |(_b_dir, angle2, u_p, u_n)| {
                            self.e = self.vertex.pos + u_n * length.value / 2. / angle2.sin();
                            self.s = self.vertex.pos + u_p * length.value / 2. / angle2.sin();
                        },
                    );
                }
                (Segment { dum: _ }, Arc { sag_rel: _ })
                | (Arc { sag_rel: _ }, Segment { dum: _ })
                | (Arc { sag_rel: _ }, Arc { sag_rel: _ }) => {
                    self.s = self.vertex.pos;
                    self.e = self.vertex.pos;
                    self.c = self.vertex.pos;
                }
            },
            Fillet { radius } => match (edge_prev, self.edge) {
                (Segment { dum: _ }, Segment { dum: _ }) => {
                    bissector(vertex_prev, self.vertex.pos, vertex_next).map(
                        |(b_dir, angle2, u_p, u_n)| {
                            self.e = self.vertex.pos + u_n * radius.value / angle2.tan();
                            self.s = self.vertex.pos + u_p * radius.value / angle2.tan();
                            self.c = self.vertex.pos + b_dir * radius.value / angle2.sin();
                        },
                    );
                }
                (Segment { dum: _ }, Arc { sag_rel: _ })
                | (Arc { sag_rel: _ }, Segment { dum: _ })
                | (Arc { sag_rel: _ }, Arc { sag_rel: _ }) => {
                    self.s = self.vertex.pos;
                    self.e = self.vertex.pos;
                    self.c = self.vertex.pos;
                }
            },
        }
    }
    pub fn get_vertex(&self) -> &Position {
        &self.vertex
    }
    pub fn get_corner(&self) -> &CornerKind {
        &self.corner
    }
    pub fn get_edge(&self) -> &EdgeKind {
        &self.edge
    }

    pub fn get_k_corner(&self) -> KShape {
        use CornerKind::*;
        match self.corner {
            Point { dummy: _ } => KShape::KPoint(self.vertex.pos.to_point()),
            Chamfer { length: _ } => {
                KShape::KLine(kurbo::Line::new(self.s.to_point(), self.e.to_point()))
            }
            Fillet { radius: _ } => arc_from_center_and_points(self.c, self.s, self.e)
                .and_then(|arc| Some(KShape::KArc(arc)))
                .unwrap_or(KShape::KPoint(self.vertex.pos.to_point())),
        }
    }
    pub fn get_sagitta(&self, v_next: Vec2) -> Option<Vec2> {
        use EdgeKind::*;
        match self.edge {
            Segment { dum: _ } => None,
            Arc { sag_rel } => SegBundle::new(self.vertex.pos, v_next)
                .and_then(|sb| Some(sb.m() - sb.n() * sb.len() * sag_rel.value)),
        }
    }
    pub fn get_sagitta_rel(&self) -> f64 {
        use EdgeKind::*;
        match self.edge {
            Segment { dum: sag_rel } => sag_rel.value,
            Arc { sag_rel } => sag_rel.value,
        }
    }
    pub fn set_sagitta_rel(&mut self, s_rel: f64) {
        use EdgeKind::*;
        match &mut self.edge {
            Segment { dum: sag_rel } => sag_rel.value = s_rel,
            Arc { sag_rel } => sag_rel.value = s_rel,
        }
    }
    pub fn get_k_edge(&self, s_next: Vec2, v_next: Vec2) -> KShape {
        use EdgeKind::*;
        SegBundle::new(self.vertex.pos, v_next)
            .and_then(|sb| match self.edge {
                Segment { dum: _ } => Some(KShape::KLine(kurbo::Line::new(
                    self.e.to_point(),
                    s_next.to_point(),
                ))),
                Arc { sag_rel } => {
                    let sagitta_pt = sb.m() - sb.n() * sb.len() * sag_rel.value;
                    circle_from_three_points(self.vertex.pos, sagitta_pt, v_next).and_then(
                        |(center, radius)| {
                            let start_angle = (self.e - center).atan2();
                            let end_angle = (s_next - center).atan2();
                            let sweep_angle =
                                if (v_next - sagitta_pt).cross(self.vertex.pos - sagitta_pt) > 0. {
                                    (end_angle - start_angle).rem_euclid(PI * 2.)
                                } else {
                                    (end_angle - start_angle).rem_euclid(PI * 2.) - 2. * PI
                                };
                            let arc = kurbo::Arc {
                                center: center.to_point(),
                                radii: Vec2::new(radius, radius),
                                start_angle,
                                sweep_angle,
                                x_rotation: 0.0,
                            };
                            Some(KShape::KArc(arc))
                        },
                    )
                }
            })
            .unwrap_or(KShape::KLine(kurbo::Line::new(
                self.vertex.pos.to_point(),
                v_next.to_point(),
            )))
    }
    pub fn get_distance_to_vertex(&self, pos: Vec2) -> (f64, Vec2) {
        ((self.get_vertex().pos - pos).hypot(), self.get_vertex().pos)
    }
    pub fn get_distance_to_corner(&self, pos: Vec2) -> Option<(f64, Vec2)> {
        use KShape::*;
        match self.get_k_corner() {
            KPoint(_) => None,
            KLine(line) => {
                distance_and_projection_to_segment(line.p0.to_vec2(), line.p1.to_vec2(), pos, 0.)
            }
            KArc(arc) => distance_and_projection_to_arc(&arc, pos, Self::ANGLE_GUARD),
        }
    }
    pub fn get_distance_to_edge(
        &self,
        s_next: Vec2,
        v_next: Vec2,
        pos: Vec2,
    ) -> Option<(f64, Vec2)> {
        match self.get_k_edge(s_next, v_next) {
            KShape::KLine(line) => distance_and_projection_to_segment(
                line.p0.to_vec2(),
                line.p1.to_vec2(),
                pos,
                Self::VERTEX_GUARD,
            ),
            KShape::KArc(arc) => distance_and_projection_to_arc(&arc, pos, Self::ANGLE_GUARD),
            KShape::KPoint(_) => None,
        }
    }
    pub fn move_vertex(&mut self, dpos: Vec2) {
        self.vertex.pos = self.vertex.saved_pos + dpos;
    }
    pub fn set_vertex_pos(&mut self, pos: Vec2) {
        self.vertex.pos = pos;
    }
    pub fn move_corner(&mut self, dpos: Vec2) {
        use CornerKind::*;
        match &mut self.corner {
            Point { dummy: _ } => (),
            Chamfer { length } => {
                if let Some(sb) = SegBundle::new(self.s, self.e) {
                    let dpos_proj: f64 = dpos.dot(sb.n());
                    let mut new_length = length.saved_val + dpos_proj;
                    if new_length < Self::MIN_FILLET_RADIUS {
                        new_length = Self::MIN_FILLET_RADIUS;
                    }
                    length.value = new_length;
                } else {
                    return;
                }
            }
            Fillet { radius } => {
                if let Some(sb) = SegBundle::new(self.s, self.e) {
                    let dpos_proj: f64 = dpos.dot(sb.n());
                    let mut new_radius = radius.saved_val + dpos_proj;
                    if new_radius < Self::MIN_FILLET_RADIUS {
                        new_radius = Self::MIN_FILLET_RADIUS;
                    }
                    radius.value = new_radius;
                } else {
                    return;
                }
            }
        };
    }

    fn corner_path_elements(&self) -> PrimitiveKindIter {
        use KShape::*;
        use PrimitiveKindIter::*;
        match self.get_k_corner() {
            KLine(line) => PLine(line.path_elements(Self::TOLERANCE)),
            KArc(arc) => PArc(arc.path_elements(Self::TOLERANCE)),
            KPoint(_) => PNone,
        }
    }
    pub fn get_corner_paths_and_patterns(&self) -> (BezPath, Pattern, Colors) {
        (
            self.corner_path_elements().collect(),
            Pattern::Basic,
            self.get_colors(self.corner_state),
        )
    }
    fn edge_path_elements(&self, s_next: Vec2, v_next: Vec2) -> PrimitiveKindIter {
        use KShape::*;
        use PrimitiveKindIter::*;
        match self.get_k_edge(s_next, v_next) {
            KLine(line) => PLine(line.path_elements(Self::TOLERANCE)),
            KArc(arc) => PArc(arc.path_elements(Self::TOLERANCE)),
            KPoint(_) => PNone,
        }
    }
    pub fn get_edge_paths_and_patterns(
        &self,
        s_next: Vec2,
        v_next: Vec2,
    ) -> (BezPath, Pattern, Colors) {
        (
            self.edge_path_elements(s_next, v_next).collect(),
            Pattern::Basic,
            self.get_colors(self.edge_state),
        )
    }
    pub fn get_prim_edge_paths_and_patterns(
        &self,
        s_next: Vec2,
        v_next: Vec2,
    ) -> (BezPath, Pattern, Colors) {
        (
            self.edge_path_elements(s_next, v_next).collect(),
            Pattern::Basic,
            self.get_colors(self.edge_state),
        )
    }
    fn get_colors(&self, state: Status) -> Colors {
        use HS::*;
        match (state.is_hs(Select), state.is_hs(Highlight)) {
            (true, _) => Colors {
                color: Color::Gray,
                fill_color: Color::Gray,
            },
            (false, false) => Colors {
                color: Color::Gray,
                fill_color: Color::Gray,
            },
            (false, true) => Colors {
                color: Color::Gray,
                fill_color: Color::Gray,
            },
        }
    }
}
