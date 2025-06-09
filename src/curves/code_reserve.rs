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
            (Segment { dum: _ }, Arc { sag_rel }) => {
                SegBundle::new(vertex_prev, self.vertex.pos).and_then(|sb_p| {
                    SegBundle::new(self.vertex.pos, vertex_next).and_then(|sb_n| {
                        let sagitta_pt = sb_n.m() - sb_n.n() * sb_n.len() * sag_rel.value;
                        let v_apex = self.vertex.pos;
                        arc_from_three_points(v_apex, sagitta_pt, vertex_next).map(|arc| {
                            let ca = arc.center.to_vec2() - v_apex;
                            let sa = sagitta_pt - v_apex;
                            let sa_ca = sa.cross(ca).signum() > 0.;
                            let sbpn_ca = sb_p.n().cross(ca).signum() > 0.;

                            // log!(
                            //     "sa_ca && !sbpn_ca: {}, sbpn_ca && sa_ca: {}",
                            //     sa_ca && !sbpn_ca,
                            //     sbpn_ca && sa_ca
                            // );

                            let (r, line_pt) = match (sbpn_ca && sa_ca, sa_ca && !sbpn_ca) {
                                (true, true) => {
                                    (arc.radii.x - radius.value, v_apex - sb_p.n() * radius.value)
                                }
                                (true, false) => {
                                    (arc.radii.x - radius.value, v_apex + sb_p.n() * radius.value)
                                }
                                (false, true) => {
                                    (arc.radii.x + radius.value, v_apex - sb_p.n() * radius.value)
                                }
                                (false, false) => {
                                    (arc.radii.x + radius.value, v_apex + sb_p.n() * radius.value)
                                }
                            };

                            circle_line_intersection(arc.center.to_vec2(), r, line_pt, sb_p.u())
                                .and_then(|(pt1, o_pt2)| {
                                    Some(match o_pt2 {
                                        Some(pt2)
                                            if (pt1 - v_apex).hypot() < (pt2 - v_apex).hypot() =>
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
                                        self.s =
                                            project_point_on_line(fillet_center, v_apex, sb_p.u());
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
