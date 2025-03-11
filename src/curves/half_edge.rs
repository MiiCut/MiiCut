use crate::{
    canvas::Pattern,
    math::{
        arc_from_center_and_points, arc_from_three_points, bissector, circle_from_three_points,
        circle_line_intersection, distance_and_projection_to_arc,
        distance_and_projection_to_segment, get_seg_bdle, nearest_circle_point,
        project_point_on_line,
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
    Chamfer { length: Value, dum_rad: Value },
    Fillet { radius: Value, dum_len: Value },
    Point { dum_rad: Value, dum_len: Value },
}
impl CornerKind {
    pub fn next(&mut self) {
        use CornerKind::*;
        match self {
            Point { dum_rad, dum_len } => {
                *self = Chamfer {
                    length: *dum_len,
                    dum_rad: *dum_rad,
                }
            }
            Chamfer { length, dum_rad } => {
                *self = Fillet {
                    radius: *dum_rad,
                    dum_len: *length,
                }
            }
            Fillet { radius, dum_len } => {
                *self = Point {
                    dum_rad: *radius,
                    dum_len: *dum_len,
                }
            }
        };
    }
    pub fn prev(&mut self) {
        use CornerKind::*;
        match self {
            Point { dum_rad, dum_len } => {
                *self = Chamfer {
                    length: *dum_len,
                    dum_rad: *dum_rad,
                }
            }
            Chamfer { length, dum_rad } => {
                *self = Fillet {
                    radius: *dum_rad,
                    dum_len: *length,
                }
            }
            Fillet { radius, dum_len } => {
                *self = Point {
                    dum_rad: *radius,
                    dum_len: *dum_len,
                }
            }
        };
    }
    pub fn save_vars(&mut self) {
        match self {
            CornerKind::Point {
                dum_rad: dum1,
                dum_len: dum2,
            } => {
                dum1.saved_val = dum1.value;
                dum2.saved_val = dum2.value;
            }
            CornerKind::Chamfer {
                length,
                dum_rad: dum,
            } => {
                length.saved_val = length.value;
                dum.saved_val = dum.value;
            }
            CornerKind::Fillet {
                radius,
                dum_len: dum,
            } => {
                radius.saved_val = radius.value;
                dum.saved_val = dum.value;
            }
        }
    }
    pub fn restore_vars(&mut self) {
        match self {
            CornerKind::Point {
                dum_rad: dum1,
                dum_len: dum2,
            } => {
                dum1.value = dum1.saved_val;
                dum2.value = dum2.saved_val;
            }
            CornerKind::Chamfer {
                length,
                dum_rad: dum,
            } => {
                length.value = length.saved_val;
                dum.value = dum.saved_val;
            }
            CornerKind::Fillet {
                radius,
                dum_len: dum,
            } => {
                radius.value = radius.saved_val;
                dum.value = dum.saved_val;
            }
        }
    }
}

#[derive(Copy, Debug, Clone)]
pub struct HalfEdge {
    vertex: Position,
    vertex_state: Status,
    corner: CornerKind,
    corner_state: Status,
    edge: EdgeKind,
    edge_editable: bool,
    edge_state: Status,
    // Calculated data
    s: Vec2,
    e: Vec2,
    c: Vec2,
}
impl HalfEdge {
    const ANGLE_GUARD: f64 = 0.02;
    const VERTEX_GUARD: f64 = 5.;
    // const GRAB: f64 = 5.;
    const TOLERANCE: f64 = 0.01;
    const MIN_FILLET_RADIUS: f64 = 10.;
    const MIN_CHAMFER_LENGTH: f64 = 10.;

    fn get_pattern(&self, selected: bool, highlighted: bool) -> Pattern {
        match (selected, highlighted) {
            (false, false) => Pattern::BasicNormal,
            (false, true) => Pattern::BasicHighlighted,
            (true, false) => Pattern::BasicSelected,
            (true, true) => Pattern::BasicSelected,
        }
    }
    pub fn new(v: Vec2, v_editable: bool, edge_editable: bool) -> Self {
        // By default the corner is a point and the edge is a line segment
        Self {
            vertex: Position::new(v, v_editable),
            vertex_state: Status::default(),
            corner: CornerKind::Point {
                dum_rad: Value::new(Self::MIN_FILLET_RADIUS),
                dum_len: Value::new(Self::MIN_CHAMFER_LENGTH),
            },
            corner_state: Status::default(),
            edge: EdgeKind::Segment {
                dum: Value::new(0.5), // The sagitta is half the segment length
            },
            edge_editable,
            edge_state: Status::default(),
            s: v,
            e: v,
            c: v,
        }
    }
    pub fn set_vertex_state(&mut self, hs: HS, value: bool) {
        if self.vertex.editable {
            self.vertex_state.set_hs(hs, value);
        }
    }
    pub fn set_corner_state(&mut self, hs: HS, value: bool) {
        self.corner_state.set_hs(hs, value);
    }
    pub fn set_edge_state(&mut self, hs: HS, value: bool) {
        if self.edge_editable {
            self.edge_state.set_hs(hs, value);
        }
    }
    pub fn get_vertex_state(&self, hs: HS) -> bool {
        self.vertex_state.is_hs(hs)
    }
    pub fn get_corner_state(&self, hs: HS) -> bool {
        self.corner_state.is_hs(hs)
    }
    pub fn get_edge_state(&self, hs: HS) -> bool {
        self.edge_state.is_hs(hs)
    }

    pub fn is_vertex_editable(&self) -> bool {
        self.vertex.editable
    }
    pub fn is_edge_editable(&self) -> bool {
        self.edge_editable
    }

    pub fn corner_next_kind(&mut self) {
        self.corner.next();
    }
    pub fn corner_prev_kind(&mut self) {
        self.corner.prev();
    }
    pub fn edge_next_kind(&mut self) {
        self.edge.next();
    }
    pub fn edge_prev_kind(&mut self) {
        self.edge.prev();
    }

    pub fn save_vars(&mut self) {
        self.corner.save_vars();
        self.edge.save_vars();
        self.vertex.saved_pos = self.vertex.pos;
    }
    pub fn restore_vars(&mut self) {
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
            Point {
                dum_rad: _,
                dum_len: _,
            } => {
                self.s = self.vertex.pos;
                self.e = self.vertex.pos;
                self.c = self.vertex.pos;
            }
            Chamfer { length, dum_rad: _ } => match (edge_prev, self.edge) {
                (Segment { dum: _ }, Segment { dum: _ }) => {
                    bissector(vertex_prev, self.vertex.pos, vertex_next).map(
                        |(_b_dir, angle2, u_p, u_n)| {
                            self.e = self.vertex.pos + u_n * length.value / 2. / angle2.sin();
                            self.s = self.vertex.pos + u_p * length.value / 2. / angle2.sin();
                        },
                    );
                }
                (Segment { dum: _ }, Arc { sag_rel: _ }) => {
                    self.s = self.vertex.pos;
                    self.e = self.vertex.pos;
                    self.c = self.vertex.pos;
                }
                (Arc { sag_rel: _ }, Segment { dum: _ }) => {
                    self.s = self.vertex.pos;
                    self.e = self.vertex.pos;
                    self.c = self.vertex.pos;
                }
                (Arc { sag_rel: _ }, Arc { sag_rel: _ }) => {
                    self.s = self.vertex.pos;
                    self.e = self.vertex.pos;
                    self.c = self.vertex.pos;
                }
            },
            Fillet { radius, dum_len: _ } => match (edge_prev, self.edge) {
                (Segment { dum: _ }, Segment { dum: _ }) => {
                    bissector(vertex_prev, self.vertex.pos, vertex_next).map(
                        |(b_dir, angle2, u_p, u_n)| {
                            self.e = self.vertex.pos + u_n * radius.value / angle2.tan();
                            self.s = self.vertex.pos + u_p * radius.value / angle2.tan();
                            self.c = self.vertex.pos + b_dir * radius.value / angle2.sin();
                        },
                    );
                }
                (Segment { dum: _ }, Arc { sag_rel }) => {
                    get_seg_bdle(vertex_prev, self.vertex.pos).and_then(|sb_p| {
                        get_seg_bdle(self.vertex.pos, vertex_next).and_then(|sb_n| {
                            let sagitta_pt = sb_n.m - sb_n.n * sb_n.len * sag_rel.value;
                            let v_apex = self.vertex.pos;
                            arc_from_three_points(v_apex, sagitta_pt, vertex_next).map(|arc| {
                                let ca = arc.center.to_vec2() - v_apex;
                                let sa = sagitta_pt - v_apex;
                                let sa_ca = sa.cross(ca).signum() > 0.;
                                let sbpn_ca = sb_p.n.cross(ca).signum() > 0.;

                                // log!(
                                //     "sa_ca && !sbpn_ca: {}, sbpn_ca && sa_ca: {}",
                                //     sa_ca && !sbpn_ca,
                                //     sbpn_ca && sa_ca
                                // );

                                let (r, line_pt) = match (sbpn_ca && sa_ca, sa_ca && !sbpn_ca) {
                                    (true, true) => {
                                        (arc.radii.x - radius.value, v_apex - sb_p.n * radius.value)
                                    }
                                    (true, false) => {
                                        (arc.radii.x - radius.value, v_apex + sb_p.n * radius.value)
                                    }
                                    (false, true) => {
                                        (arc.radii.x + radius.value, v_apex - sb_p.n * radius.value)
                                    }
                                    (false, false) => {
                                        (arc.radii.x + radius.value, v_apex + sb_p.n * radius.value)
                                    }
                                };

                                circle_line_intersection(arc.center.to_vec2(), r, line_pt, sb_p.u)
                                    .and_then(|(pt1, o_pt2)| {
                                        Some(match o_pt2 {
                                            Some(pt2)
                                                if (pt1 - v_apex).hypot()
                                                    < (pt2 - v_apex).hypot() =>
                                            {
                                                // log!("1 pt1");
                                                pt1
                                            }
                                            Some(pt2) => {
                                                // log!("pt2");
                                                pt2
                                            }
                                            None => {
                                                // log!("2 pt1");
                                                pt1
                                            }
                                        })
                                    })
                                    .map(|fillet_center| {
                                        if let Some(e) = nearest_circle_point(
                                            arc.center.to_vec2(),
                                            arc.radii.x,
                                            fillet_center,
                                        ) {
                                            self.c = fillet_center;
                                            self.s = project_point_on_line(
                                                fillet_center,
                                                v_apex,
                                                sb_p.u,
                                            );
                                            self.e = e;
                                        }
                                    });
                            })
                        })
                    });
                }
                (Arc { sag_rel: _ }, Segment { dum: _ }) => {}

                //     let v = self.vertex.pos;
                //     get_seg_bdle(vertex_prev, v).and_then(|sb_p| {
                //         get_seg_bdle(v, vertex_next).and_then(|sb_n| {
                //             let sagitta_pt = sb_n.m - sb_n.n * sb_n.len * sag_rel.value;
                //             arc_from_three_points(vertex_prev, sagitta_pt, v).map(|arc| {
                //                 let r = arc.center.to_vec2() - v;
                //                 let add_radiuses = Vec2::new(r.y, -r.x).cross(sb_n.u) < 0.;
                //                 let line_near_center = !add_radiuses
                //                     && !((sagitta_pt - v).cross(v - vertex_prev) < 0.);
                //                 let (r, line_pt) = match (add_radiuses, line_near_center) {
                //                     (true, true) => {
                //                         (arc.radii.x - radius.value, v - sb_p.n * radius.value)
                //                     }
                //                     (true, false) => {
                //                         (arc.radii.x - radius.value, v + sb_p.n * radius.value)
                //                     }
                //                     (false, true) => {
                //                         (arc.radii.x + radius.value, v - sb_p.n * radius.value)
                //                     }
                //                     (false, false) => {
                //                         (arc.radii.x + radius.value, v + sb_p.n * radius.value)
                //                     }
                //                 };
                //                 let line_dir = sb_n.u;
                //                 circle_line_intersection(
                //                     arc.center.to_vec2(),
                //                     r,
                //                     line_pt,
                //                     line_dir,
                //                 )
                //                 .and_then(|(pt1, o_pt2)| {
                //                     Some(match o_pt2 {
                //                         Some(pt2) if (pt1 - v).hypot() < (pt2 - v).hypot() => pt1,
                //                         Some(pt2) => pt2,
                //                         None => pt1,
                //                     })
                //                 })
                //                 .map(|fillet_center| {
                //                     if let Some(e) = nearest_circle_point(
                //                         arc.center.to_vec2(),
                //                         arc.radii.x,
                //                         fillet_center,
                //                     ) {
                //                         self.c = fillet_center;
                //                         self.s = project_point_on_line(fillet_center, v, line_dir);
                //                         self.e = e;
                //                     }
                //                 });
                //             })
                //         })
                //     });
                // }
                (Arc { sag_rel: _s_rel_p }, Arc { sag_rel: _s_rel_n }) => (),
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
            Point {
                dum_rad: _,
                dum_len: _,
            } => KShape::KPoint(self.vertex.pos.to_point()),
            Chamfer {
                length: _,
                dum_rad: _,
            } => KShape::KLine(kurbo::Line::new(self.s.to_point(), self.e.to_point())),
            Fillet {
                radius: _,
                dum_len: _,
            } => arc_from_center_and_points(self.c, self.s, self.e)
                .and_then(|arc| Some(KShape::KArc(arc)))
                .unwrap_or(KShape::KPoint(self.vertex.pos.to_point())),
        }
    }
    pub fn get_sagitta(&self, v_next: Vec2) -> Option<Vec2> {
        use EdgeKind::*;
        match self.edge {
            Segment { dum: _ } => None,
            Arc { sag_rel } => get_seg_bdle(self.vertex.pos, v_next)
                .and_then(|sb| Some(sb.m - sb.n * sb.len * sag_rel.value)),
        }
    }
    pub fn get_k_edge(&self, s_next: Vec2, v_next: Vec2) -> KShape {
        use EdgeKind::*;
        get_seg_bdle(self.vertex.pos, v_next)
            .and_then(|sb| match self.edge {
                Segment { dum: _ } => Some(KShape::KLine(kurbo::Line::new(
                    self.e.to_point(),
                    s_next.to_point(),
                ))),
                Arc { sag_rel } => {
                    let sagitta_pt = sb.m - sb.n * sb.len * sag_rel.value;
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
            Point {
                dum_rad: _,
                dum_len: _,
            } => (),
            Chamfer { length, dum_rad: _ } => {
                if let Some(sb) = get_seg_bdle(self.s, self.e) {
                    let dpos_proj: f64 = dpos.dot(sb.n);
                    let mut new_length = length.saved_val + dpos_proj;
                    if new_length < Self::MIN_CHAMFER_LENGTH {
                        new_length = Self::MIN_CHAMFER_LENGTH;
                    }
                    length.value = new_length;
                } else {
                    return;
                }
            }
            Fillet { radius, dum_len: _ } => {
                if let Some(sb) = get_seg_bdle(self.s, self.e) {
                    let dpos_proj: f64 = dpos.dot(sb.n);
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
    pub fn move_edge(&mut self, v_next: Vec2, dpos: Vec2) {
        use EdgeKind::*;
        match &mut self.edge {
            Segment { dum: _ } => (),
            Arc { sag_rel } => {
                let v = self.vertex.pos;
                if let Some(sb) = get_seg_bdle(v, v_next) {
                    sag_rel.value = (sb.len * sag_rel.saved_val - dpos.dot(sb.n)) / sb.len;
                }
            }
        }
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
    pub fn get_corner_paths_and_patterns(
        &self,
        parent_selected: bool,
        parent_highlighted: bool,
    ) -> (BezPath, Pattern) {
        use HS::*;
        (
            self.corner_path_elements().collect(),
            self.get_pattern(
                self.corner_state.is_hs(Select) || parent_selected,
                self.corner_state.is_hs(Highlight) || parent_highlighted,
            ),
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
        parent_selected: bool,
        parent_highlighted: bool,
    ) -> (BezPath, Pattern) {
        use HS::*;
        (
            self.edge_path_elements(s_next, v_next).collect(),
            self.get_pattern(
                self.edge_state.is_hs(Select) || parent_selected,
                self.edge_state.is_hs(Highlight) || parent_highlighted,
            ),
        )
    }
}
