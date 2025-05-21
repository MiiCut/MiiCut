use crate::{
    canvas::Pattern,
    math::{
        arc_from_center_and_points, bissector, circle_from_three_points,
        distance_and_projection_to_arc, distance_and_projection_to_segment, SegBundle,
    },
    prefab::point_path,
    types::{Status, Value, HS},
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
    pub vertex_movable: bool,
    pub vertex_magnetic: bool,
}
impl Default for HEProps {
    fn default() -> Self {
        Self {
            vertex_selectable: true,
            vertex_movable: true,
            vertex_magnetic: true,
        }
    }
}

// sagitta_rel:
// The vertex_kind and the next vertex_kind form a segment. In case the edge is an arc,
// this segment is a chord of an arc. The distance beween the mid point of this segment and the mid
// point of the arc is called the sagitta.
// We define the sag_rel that is the sagitta divided by the segment length.
// A value of 0.5 means the sagitta is egual to half the segment length (hence the chord is a diameter)
// We sign this value to indicate the side of the arc relative to the segment
#[derive(Copy, Debug, Clone)]
pub enum EdgeKind {
    Segment { dum: Value<f64> },
    Arc { sag_rel: Value<f64> },
}
impl EdgeKind {
    pub fn set_arc(&mut self) {
        use EdgeKind::*;
        match self {
            Segment { dum } => *self = Arc { sag_rel: *dum },
            _ => (),
        };
    }
    pub fn set_segment(&mut self) {
        use EdgeKind::*;
        match self {
            Arc { sag_rel } => *self = Segment { dum: *sag_rel },
            _ => (),
        };
    }
    pub fn save_vars(&mut self) {
        match self {
            EdgeKind::Segment { dum } => dum.saved = dum.curr,
            EdgeKind::Arc { sag_rel } => sag_rel.saved = sag_rel.curr,
        }
    }
    pub fn restore_vars(&mut self) {
        match self {
            EdgeKind::Segment { dum } => dum.curr = dum.saved,
            EdgeKind::Arc { sag_rel } => sag_rel.curr = sag_rel.saved,
        }
    }
}

#[derive(Copy, Debug, Clone)]
pub enum VertexKind {
    Chamfer { length: Value<f64> },
    Fillet { radius: Value<f64> },
    Point { dummy: Value<f64> },
}
impl VertexKind {
    pub fn next(&mut self) {
        use VertexKind::*;
        match self {
            Point { dummy } => *self = Fillet { radius: *dummy },
            Fillet { radius } => *self = Chamfer { length: *radius },
            Chamfer { length } => *self = Point { dummy: *length },
        };
    }
    pub fn next2(&mut self) {
        use VertexKind::*;
        match self {
            Fillet { radius } => *self = Chamfer { length: *radius },
            Chamfer { length } => *self = Fillet { radius: *length },
            _ => (),
        };
    }
    pub fn save_vars(&mut self) {
        match self {
            VertexKind::Point { dummy } => dummy.saved = dummy.curr,
            VertexKind::Chamfer { length } => length.saved = length.curr,
            VertexKind::Fillet { radius } => radius.saved = radius.curr,
        }
    }
    pub fn restore_vars(&mut self) {
        match self {
            VertexKind::Point { dummy } => dummy.curr = dummy.saved,
            VertexKind::Chamfer { length } => length.curr = length.saved,
            VertexKind::Fillet { radius } => radius.curr = radius.saved,
        }
    }
}

#[derive(Copy, Debug, Clone)]
pub struct HalfEdge {
    props: HEProps,
    saved_props: HEProps,
    vertex: Value<Vec2>,
    vertex_state: Status,
    vertex_kind: VertexKind,
    edge: EdgeKind,
    // Calculated data
    s: Vec2,
    e: Vec2,
    c: Option<Vec2>,
    c_state: Status,
}
impl HalfEdge {
    const ANGLE_GUARD: f64 = 0.02;
    const VERTEX_GUARD: f64 = 2.;
    // const GRAB: f64 = 5.;
    const TOLERANCE: f64 = 0.01;
    pub const MIN_FILLET_RADIUS: f64 = 2.;

    pub fn new(v: Vec2, props: HEProps) -> Self {
        // By default the vertex_kind is a point and the edge is a line segment
        Self {
            props,
            saved_props: props,
            vertex: Value::<Vec2>::new(v),
            vertex_state: Status::default(),
            vertex_kind: VertexKind::Point {
                dummy: Value::new(10. * Self::MIN_FILLET_RADIUS),
            },
            edge: EdgeKind::Segment {
                dum: Value::new(0.5), // The sagitta is half the segment length
            },
            s: v,
            e: v,
            c: None,
            c_state: Status::default(),
        }
    }
    pub fn set_vertex_state(&mut self, hs: HS, value: bool) {
        if self.props.vertex_selectable {
            self.vertex_state.set_hs(hs, value);
        }
    }
    pub fn get_vertex_state(&self, hs: HS) -> bool {
        self.vertex_state.is_hs(hs)
    }
    pub fn vertex_state(&self) -> Status {
        self.vertex_state
    }

    pub fn set_c_state(&mut self, hs: HS, value: bool) {
        self.c_state.set_hs(hs, value);
    }
    pub fn get_c_state(&self, hs: HS) -> bool {
        self.c_state.is_hs(hs)
    }
    pub fn c_state(&self) -> Status {
        self.c_state
    }

    pub fn is_vertex_selectable(&self) -> bool {
        self.props.vertex_selectable
    }
    pub fn is_vertex_movable(&self) -> bool {
        self.props.vertex_movable
    }
    pub fn is_vertex_magnetic(&self) -> bool {
        self.props.vertex_magnetic
    }
    pub fn vertex_next_kind(&mut self) {
        self.vertex_kind.next();
    }
    pub fn vertex_next_kind2(&mut self) {
        self.vertex_kind.next2();
    }
    pub fn edge_set_arc(&mut self) {
        self.edge.set_arc();
    }
    pub fn edge_set_segment(&mut self) {
        self.edge.set_segment();
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
        self.vertex_kind.save_vars();
        self.edge.save_vars();
        self.vertex.saved = self.vertex.curr;
    }
    pub fn restore_vars(&mut self) {
        self.props = self.saved_props;
        self.vertex_kind.restore_vars();
        self.edge.restore_vars();
        self.vertex.curr = self.vertex.saved;
    }
    pub fn get_s(&self) -> Vec2 {
        self.s
    }
    pub fn get_e(&self) -> Vec2 {
        self.e
    }
    pub fn get_c(&self) -> Option<Vec2> {
        self.c
    }

    // Update s, e, c data
    pub fn update_data(&mut self, vertex_prev: Vec2, edge_prev: EdgeKind, vertex_next: Vec2) {
        use EdgeKind::*;
        use VertexKind::*;
        match self.vertex_kind {
            Point { dummy: _ } => {
                self.s = self.vertex.curr;
                self.e = self.vertex.curr;
                self.c = None;
            }
            Chamfer { length } => match (edge_prev, self.edge) {
                (Segment { dum: _ }, Segment { dum: _ }) => {
                    bissector(vertex_prev, self.vertex.curr, vertex_next).map(
                        |(b_dir, angle2, u_p, u_n)| {
                            self.e = self.vertex.curr + u_n * length.curr / 2. / angle2.sin();
                            self.s = self.vertex.curr + u_p * length.curr / 2. / angle2.sin();
                            self.c = Some(self.vertex.curr + b_dir * length.curr / angle2.sin());
                        },
                    );
                }
                _ => (),
            },
            Fillet { radius } => match (edge_prev, self.edge) {
                (Segment { dum: _ }, Segment { dum: _ }) => {
                    bissector(vertex_prev, self.vertex.curr, vertex_next).map(
                        |(b_dir, angle2, u_p, u_n)| {
                            self.e = self.vertex.curr + u_n * radius.curr / angle2.tan();
                            self.s = self.vertex.curr + u_p * radius.curr / angle2.tan();
                            self.c = Some(self.vertex.curr + b_dir * radius.curr / angle2.sin());
                        },
                    );
                }
                _ => (),
            },
        }
    }
    pub fn get_vertex(&self) -> &Value<Vec2> {
        &self.vertex
    }
    pub fn get_vertex_kind(&self) -> &VertexKind {
        &self.vertex_kind
    }
    pub fn get_vertex_kind_mut(&mut self) -> &mut VertexKind {
        &mut self.vertex_kind
    }
    pub fn get_edge(&self) -> &EdgeKind {
        &self.edge
    }

    pub fn get_k_vertex_kind(&self) -> KShape {
        use VertexKind::*;
        match self.vertex_kind {
            Point { dummy: _ } => KShape::KPoint(self.vertex.curr.to_point()),
            Chamfer { length: _ } => {
                KShape::KLine(kurbo::Line::new(self.s.to_point(), self.e.to_point()))
            }
            Fillet { radius: _ } => self
                .c
                .and_then(|c| {
                    arc_from_center_and_points(c, self.s, self.e)
                        .and_then(|arc| Some(KShape::KArc(arc)))
                })
                .unwrap_or(KShape::KPoint(self.vertex.curr.to_point())),
        }
    }
    pub fn get_sagitta(&self, v_next: Vec2) -> Option<Vec2> {
        use EdgeKind::*;
        match self.edge {
            Segment { dum: _ } => None,
            Arc { sag_rel } => SegBundle::new(self.vertex.curr, v_next)
                .and_then(|sb| Some(sb.m() - sb.n() * sb.len() * sag_rel.curr)),
        }
    }
    pub fn get_sagitta_rel(&self) -> f64 {
        use EdgeKind::*;
        match self.edge {
            Segment { dum: sag_rel } => sag_rel.curr,
            Arc { sag_rel } => sag_rel.curr,
        }
    }
    pub fn set_sagitta_rel(&mut self, s_rel: f64) {
        use EdgeKind::*;
        match &mut self.edge {
            Segment { dum: sag_rel } => sag_rel.curr = s_rel,
            Arc { sag_rel } => sag_rel.curr = s_rel,
        }
    }
    pub fn get_k_edge(&self, s_next: Vec2, v_next: Vec2) -> KShape {
        use EdgeKind::*;
        SegBundle::new(self.vertex.curr, v_next)
            .and_then(|sb| match self.edge {
                Segment { dum: _ } => Some(KShape::KLine(kurbo::Line::new(
                    self.e.to_point(),
                    s_next.to_point(),
                ))),
                Arc { sag_rel } => {
                    let sagitta_pt = sb.m() - sb.n() * sb.len() * sag_rel.curr;
                    circle_from_three_points(self.vertex.curr, sagitta_pt, v_next).and_then(
                        |(center, radius)| {
                            let start_angle = (self.e - center).atan2();
                            let end_angle = (s_next - center).atan2();
                            let sweep_angle = if (v_next - sagitta_pt)
                                .cross(self.vertex.curr - sagitta_pt)
                                > 0.
                            {
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
                self.vertex.curr.to_point(),
                v_next.to_point(),
            )))
    }
    pub fn get_distance_to_vertex(&self, pos: Vec2) -> (f64, Vec2) {
        ((self.vertex.curr - pos).hypot(), self.get_vertex().curr)
    }
    pub fn get_distance_to_c(&self, pos: Vec2) -> Option<(f64, Vec2)> {
        self.c.and_then(|c| Some(((c - pos).hypot(), c)))
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
        self.vertex.curr = self.vertex.saved + dpos;
    }
    pub fn set_vertex_pos(&mut self, pos: Vec2) {
        self.vertex.curr = pos;
    }

    // pub fn move_vertex_kind(&mut self, dpos: Vec2) {
    //     use VertexKind::*;
    //     match &mut self.vertex_kind {
    //         Point { dummy: _ } => (),
    //         Chamfer { length } => {
    //             if let Some(sb) = SegBundle::new(self.s, self.e) {
    //                 let dpos_proj: f64 = dpos.dot(sb.n());
    //                 let mut new_length = length.saved_val + dpos_proj;
    //                 if new_length < Self::MIN_FILLET_RADIUS {
    //                     new_length = Self::MIN_FILLET_RADIUS;
    //                 }
    //                 length.value = new_length;
    //             } else {
    //                 return;
    //             }
    //         }
    //         Fillet { radius } => {
    //             if let Some(sb) = SegBundle::new(self.s, self.e) {
    //                 let dpos_proj: f64 = dpos.dot(sb.n());
    //                 let mut new_radius = radius.saved_val + dpos_proj;
    //                 if new_radius < Self::MIN_FILLET_RADIUS {
    //                     new_radius = Self::MIN_FILLET_RADIUS;
    //                 }
    //                 radius.value = new_radius;
    //             } else {
    //                 return;
    //             }
    //         }
    //     };
    // }

    pub fn get_c_control_paths_and_patterns(&self, scale: f64) -> (BezPath, Pattern) {
        use VertexKind::*;
        match self.vertex_kind {
            Chamfer { length: _ } | Fillet { radius: _ } => self
                .c
                .and_then(|c| Some((point_path(c, scale), Pattern::Point)))
                .unwrap_or((BezPath::new(), Pattern::Basic)),
            Point { dummy: _ } => (BezPath::new(), Pattern::Basic),
        }
    }
    pub fn get_vertex_kind_paths_and_patterns(&self) -> (BezPath, Pattern) {
        use KShape::*;
        use PrimitiveKindIter::*;
        match self.get_k_vertex_kind() {
            KLine(line) => (
                PLine(line.path_elements(Self::TOLERANCE)).collect(),
                Pattern::Basic,
            ),
            KArc(arc) => (
                PArc(arc.path_elements(Self::TOLERANCE)).collect(),
                Pattern::Basic,
            ),
            KPoint(_) => (PNone.collect(), Pattern::Basic),
        }
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
    pub fn get_edge_paths_and_patterns(&self, s_next: Vec2, v_next: Vec2) -> (BezPath, Pattern) {
        (
            self.edge_path_elements(s_next, v_next).collect(),
            Pattern::Basic,
        )
    }
    pub fn get_prim_edge_paths_and_patterns(
        &self,
        s_next: Vec2,
        v_next: Vec2,
    ) -> (BezPath, Pattern) {
        (
            self.edge_path_elements(s_next, v_next).collect(),
            Pattern::Basic,
        )
    }
}
