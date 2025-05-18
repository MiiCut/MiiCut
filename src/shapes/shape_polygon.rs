use crate::{
    canvas::{CanvasText, Colors, Pattern},
    curves::half_edge::{EdgeKind, HEProps, HalfEdge, VertexKind},
    dimensions::{dim_linear, dim_linear_angle},
    math::*,
    prefab::*,
    shapes::drawable::HS,
    types::{Status, Value, VecRing},
    KeysStates, Pointer,
};
use geo::{LineString, Polygon};
use kurbo::{BezPath, PathEl, Point, Rect, Shape, Size, Vec2};
use std::{fmt::Display, vec};

use super::drawable::Drawable;

#[derive(Debug, Clone)]
enum PolyKind {
    Rectangle,
    RectangleFilleted,
    Oblong,
    Custom,
}
#[derive(Debug, Clone)]
pub struct ShapePolygon {
    kind: PolyKind,
    hes_prim: Option<VecRing<HalfEdge>>,
    hes: VecRing<HalfEdge>,
    state: Status,
    segs: BezPath,
    polygon: Polygon<f64>,
}

impl ShapePolygon {
    pub const MIN_OBLONG_WIDTH: f64 = 2.;
    pub const START_OBLONG_WIDTH: f64 = 40.;

    pub fn new_rectangle(hes_prim: VecRing<HalfEdge>) -> Option<ShapePolygon> {
        (hes_prim.len() == 2).then(|| {
            let tmp1 = hes_prim.get(0).get_vertex().pos;
            let tmp2 = hes_prim.get(1).get_vertex().pos;
            // Always counter clockwize to have a positive area
            let pt1 = Vec2::new(tmp1.x.min(tmp2.x), tmp2.y.min(tmp1.y));
            let pt3 = Vec2::new(tmp2.x.max(tmp1.x), tmp1.y.max(tmp2.y));
            let pt4 = Vec2::new(tmp1.x.min(tmp2.x), tmp1.y.max(tmp2.y));
            let pt2 = Vec2::new(tmp2.x.max(tmp1.x), tmp2.y.min(tmp1.y));
            let props = HEProps::default();
            let mut hes = VecRing::from_element(HalfEdge::new(pt1, props));
            hes.push(HalfEdge::new(pt2, props));
            hes.push(HalfEdge::new(pt3, props));
            hes.push(HalfEdge::new(pt4, props));
            let mut shape_polygon = ShapePolygon {
                kind: PolyKind::Rectangle,
                hes_prim: None,
                hes,
                state: Status::default(),
                segs: BezPath::new(),
                polygon: Polygon::new(LineString::new(vec![]), vec![]),
            };
            shape_polygon.update_all();
            log!("area: {}", shape_polygon.area());
            log!("new_rectangle");
            shape_polygon
        })
    }
    pub fn new_rectangle_filleted(hes_prim: VecRing<HalfEdge>) -> Option<ShapePolygon> {
        (hes_prim.len() == 2).then(|| {
            let tmp1 = hes_prim.get(0).get_vertex().pos;
            let tmp2 = hes_prim.get(1).get_vertex().pos;
            // Always counter clockwize to have a positive area
            let pt1 = Vec2::new(tmp1.x.min(tmp2.x), tmp2.y.min(tmp1.y));
            let pt3 = Vec2::new(tmp2.x.max(tmp1.x), tmp1.y.max(tmp2.y));
            let pt4 = Vec2::new(tmp1.x.min(tmp2.x), tmp1.y.max(tmp2.y));
            let pt2 = Vec2::new(tmp2.x.max(tmp1.x), tmp2.y.min(tmp1.y));
            let props = HEProps::default();
            let mut hes = VecRing::from_element(HalfEdge::new(pt1, props));
            hes.push(HalfEdge::new(pt2, props));
            hes.push(HalfEdge::new(pt3, props));
            hes.push(HalfEdge::new(pt4, props));
            let mut shape_polygon = ShapePolygon {
                kind: PolyKind::RectangleFilleted,
                hes_prim: None,
                hes,
                state: Status::default(),
                segs: BezPath::new(),
                polygon: Polygon::new(LineString::new(vec![]), vec![]),
            };
            shape_polygon.update_all();
            log!("area: {}", shape_polygon.area());
            log!("new_rectangle filleted");
            shape_polygon
        })
    }
    pub fn new_oblong(mut hes_prim: VecRing<HalfEdge>) -> Option<ShapePolygon> {
        hes_prim.len().eq(&2).then(|| {
            // Create the width control
            SegBundle::new(
                hes_prim.get(0).get_vertex().pos,
                hes_prim.get(1).get_vertex().pos,
            )
            .and_then(|bdl| {
                log!("bdl len: {:?}", bdl.len());
                let width_pt = bdl.m() + bdl.n() * Self::START_OBLONG_WIDTH / 2.;
                hes_prim.push(HalfEdge::new(width_pt, HEProps::default()));
                Some(())
            })?;

            let mut props = HEProps::default();
            // props.vertex_selectable = true;
            props.vertex_movable = false;
            props.vertex_selectable = false;
            // Create hes dummy
            let mut hes = VecRing::from_element(HalfEdge::new(Vec2::ZERO, props));
            hes.push(HalfEdge::new(Vec2::ZERO, props));
            hes.push(HalfEdge::new(Vec2::ZERO, props));
            hes.push(HalfEdge::new(Vec2::ZERO, props));
            let mut shape_polygon = ShapePolygon {
                kind: PolyKind::Oblong,
                hes_prim: Some(hes_prim),
                hes,
                state: Status::default(),
                segs: BezPath::new(),
                polygon: Polygon::new(LineString::new(vec![]), vec![]),
            };
            shape_polygon.hes.get_mut(1).set_edge_kind(EdgeKind::Arc {
                sag_rel: Value::new(0.5),
            });
            shape_polygon.hes.get_mut(3).set_edge_kind(EdgeKind::Arc {
                sag_rel: Value::new(0.5),
            });

            // Update hes from hes_prim
            shape_polygon.update_oblong_edges().then(|| {
                shape_polygon.update_all();
                log!("area: {}", shape_polygon.area());
                log!("new_oblong");
                shape_polygon
            })
        })?
    }
    pub fn new_custom(mut hes: VecRing<HalfEdge>) -> Option<ShapePolygon> {
        if hes.len() < 3 {
            return None;
        }
        log!("new_polygon");
        // Always counter clockwize to have a positive area
        if area_from_hes(&hes) < 0. {
            hes.vec.reverse();
        }
        let mut shape_polygon = ShapePolygon {
            kind: PolyKind::Custom,
            hes_prim: None,
            hes,
            state: Status::default(),
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        };
        log!("area: {}", shape_polygon.area());
        shape_polygon.update_all();
        Some(shape_polygon)
    }

    fn update_oblong_edges(&mut self) -> bool {
        self.hes_prim
            .as_ref()
            .and_then(|hes_prim| {
                let v = hes_prim.get(0).get_vertex().pos;
                let v_next = hes_prim.get(1).get_vertex().pos;
                let bdl = SegBundle::new(v, v_next)?;
                let width2 = (hes_prim.get(2).get_vertex().pos - bdl.m()).hypot();
                match hes_prim.get(0).get_edge_kind() {
                    EdgeKind::Segment { dum: _ } => {
                        self.hes.get_mut(0).set_vertex_pos(v - bdl.n() * width2);
                        self.hes.get_mut(3).set_vertex_pos(v + bdl.n() * width2);
                        self.hes
                            .get_mut(1)
                            .set_vertex_pos(v_next - bdl.n() * width2);
                        self.hes
                            .get_mut(2)
                            .set_vertex_pos(v_next + bdl.n() * width2);
                    }
                    EdgeKind::Arc { sag_rel } => {
                        let sagitta_pt = bdl.m() - bdl.n() * bdl.len() * sag_rel.value;
                        circle_from_three_points(v, sagitta_pt, v_next).and_then(
                            |(center, _radius)| {
                                let n_v = (v - center).normalize()
                                    * (sagitta_pt - v).cross(bdl.u()).signum();
                                let n_v_next = (v_next - center).normalize()
                                    * (sagitta_pt - v_next).cross(bdl.u()).signum();
                                self.hes.get_mut(0).set_vertex_pos(v + n_v * width2);
                                self.hes
                                    .get_mut(1)
                                    .set_vertex_pos(v_next + n_v_next * width2);
                                self.hes
                                    .get_mut(2)
                                    .set_vertex_pos(v_next - n_v_next * width2);
                                self.hes.get_mut(3).set_vertex_pos(v - n_v * width2);
                                Some(())
                            },
                        );
                    }
                }

                Some(())
            })
            .is_some()
    }

    // fn update_hes_edges_from_hes_prim_edges(&mut self) -> bool {
    //     self.hes_prim
    //         .as_ref()
    //         .and_then(|(hes_prim, _width)| {
    //             // Update the edge kind
    //             let edge_kind = *hes_prim.get(0).get_edge_kind();
    //             match edge_kind {
    //                 EdgeKind::Segment { dum } => {
    //                     self.hes.get_mut(0).set_edge_kind(EdgeKind::Segment { dum });
    //                     self.hes.get_mut(2).set_edge_kind(EdgeKind::Segment {
    //                         dum: Value::new(-dum.value),
    //                     });
    //                 }
    //                 EdgeKind::Arc { sag_rel } => {
    //                     self.hes
    //                         .get_mut(0)
    //                         .set_edge_kind(EdgeKind::Arc { sag_rel: sag_rel });
    //                     self.hes.get_mut(2).set_edge_kind(EdgeKind::Arc {
    //                         sag_rel: Value::new(-sag_rel.value),
    //                     });
    //                 }
    //             }
    //             Some(())
    //         })
    //         .is_some()
    // }

    fn get_hes_len(&self) -> usize {
        self.hes.len()
    }
    fn get_he(&self, idx: i64) -> &HalfEdge {
        self.hes.get(idx)
    }
    fn get_he_mut(&mut self, idx: i64) -> &mut HalfEdge {
        self.hes.get_mut(idx)
    }
    fn set_near_c(&mut self, pointer: &mut Pointer, keys_states: KeysStates, hs: HS) -> bool {
        for idx_he in 0..self.hes.len() as i64 {
            if self
                .hes
                .get(idx_he)
                .get_distance_to_c(pointer.pos())
                .and_then(|(dist, v)| {
                    if dist < Self::GRAB_RADIUS / pointer.get_draw_scale() {
                        if !keys_states.alt_pressed {
                            pointer.set_pos(v);
                            pointer.set_magnetized(true);
                        }
                        self.hes.get_mut(idx_he).set_c_state(hs, true);
                        Some(true)
                    } else {
                        None
                    }
                })
                .is_some()
            {
                return true;
            }
        }
        false
    }
    fn set_near_vertex(&mut self, pointer: &mut Pointer, keys_states: KeysStates, hs: HS) -> bool {
        for idx_he in 0..self.hes.len() as i64 {
            if self.hes.get(idx_he).is_vertex_selectable() {
                let (dist, v) = self.hes.get(idx_he).get_distance_to_vertex(pointer.pos());
                if dist < Self::GRAB_RADIUS / pointer.get_draw_scale() {
                    if !keys_states.alt_pressed {
                        pointer.set_pos(v);
                        pointer.set_magnetized(true);
                    }
                    self.hes.get_mut(idx_he).set_vertex_state(hs, true);
                    return true;
                }
            };
        }
        if let Some(hes_prim) = self.hes_prim.as_mut() {
            for idx_he in 0..hes_prim.len() as i64 {
                if hes_prim.get(idx_he).is_vertex_selectable() {
                    let (dist, pos) = hes_prim.get(idx_he).get_distance_to_vertex(pointer.pos());
                    if dist < Self::GRAB_RADIUS / pointer.get_draw_scale() {
                        if !keys_states.alt_pressed {
                            pointer.set_pos(pos);
                            pointer.set_magnetized(true);
                        }
                        hes_prim.get_mut(idx_he).set_vertex_state(hs, true);
                        return true;
                    }
                }
            }
        }
        false
    }

    fn clear_selections(&mut self, hs: HS) {
        self.state.set_hs(hs, false);

        self.hes.iter_mut().for_each(|he| {
            he.set_vertex_state(hs, false);
            he.set_c_state(hs, false);
        });
        self.hes_prim.as_mut().and_then(|hes_prim| {
            hes_prim.iter_mut().for_each(|he| {
                he.set_vertex_state(hs, false);
            });
            Some(())
        });
    }
    fn update_all(&mut self) {
        // Update the half edges
        for idx_he in 0..self.get_hes_len() as i64 {
            let v_prev = self.get_he(idx_he - 1).get_vertex().pos;
            let v_next = self.get_he(idx_he + 1).get_vertex().pos;
            let edge_kind_prev = self.get_he(idx_he - 1).get_edge().clone();
            self.get_he_mut(idx_he)
                .update_data(v_prev, edge_kind_prev, v_next);
        }
        // Update the primitives half edges
        if let Some(hes_prim) = self.hes_prim.as_mut() {
            for idx_he in 0..hes_prim.len() as i64 {
                let v_prev = hes_prim.get(idx_he - 1).get_vertex().pos;
                let v_next = hes_prim.get(idx_he + 1).get_vertex().pos;
                let edge_kind_prev = hes_prim.get(idx_he - 1).get_edge().clone();
                hes_prim
                    .get_mut(idx_he)
                    .update_data(v_prev, edge_kind_prev, v_next);
            }
        }
        // Update the polygon
        self.segs = calc_segs(self.to_path(Self::TOLERANCE));
        self.polygon = calc_polygon(&self.segs);
    }

    pub fn get_hes(&self) -> &VecRing<HalfEdge> {
        &self.hes
    }
    pub fn get_hes_prim(&self) -> Option<&VecRing<HalfEdge>> {
        self.hes_prim.as_ref()
    }

    pub fn get_polygon(&self) -> Polygon<f64> {
        self.polygon.clone()
    }
}

impl Display for ShapePolygon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Polygon")
    }
}
impl Drawable for ShapePolygon {}

impl Shape for ShapePolygon {
    type PathElementsIter<'iter> = PolygonIter;

    fn path_elements(&self, _tolerance: f64) -> PolygonIter {
        let mut iter = vec![];
        let paths = self.get_paths_and_patterns(&Size::ZERO, (Rect::ZERO, 0., Vec2::ZERO));
        for (bez_path, ..) in paths.iter() {
            for el in bez_path.elements() {
                iter.push(*el);
            }
        }
        PolygonIter { idx: 0, iter }
    }
    #[inline]
    fn area(&self) -> f64 {
        let len = self.hes.len();
        let mut area = 0.0;
        for idx in 0..len as i64 {
            let vi = self.hes.get(idx).get_vertex().pos;
            let vj = self.hes.get((idx + 1) % len as i64).get_vertex().pos;
            area += vi.x * vj.y - vj.x * vi.y;
        }
        area * 0.5
    }
    #[inline]
    fn perimeter(&self, _accuracy: f64) -> f64 {
        0.
    }
    #[inline]
    fn winding(&self, pt: Point) -> i32 {
        compute_winding_number(&self.segs, pt.to_vec2())
    }
    #[inline]
    fn bounding_box(&self) -> Rect {
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for v_pos in self.hes.iter().map(|he| he.get_vertex().pos) {
            if v_pos.x < min_x {
                min_x = v_pos.x;
            }
            if v_pos.y < min_y {
                min_y = v_pos.y;
            }
            if v_pos.x > max_x {
                max_x = v_pos.x;
            }
            if v_pos.y > max_y {
                max_y = v_pos.y;
            }
        }
        Rect::new(min_x, min_y, max_x, max_y)
    }
    #[inline]
    fn as_rect(&self) -> Option<Rect> {
        None
    }
    #[inline]
    fn contains(&self, pt: Point) -> bool {
        self.winding(pt) != 0
    }
}
impl ObjectsFuncs for ShapePolygon {
    const TOLERANCE: f64 = 0.01;
    const GRAB_RADIUS: f64 = 10.;
    type Kindvars = ShapePolygon;

    fn tab(&mut self) -> bool {
        log!("tab");
        for idx_he in 0..self.get_hes_len() as i64 {
            let he = self.get_he_mut(idx_he);
            if he.get_vertex_state(HS::Select) {
                he.vertex_next_kind();
                self.update_all();
                return true;
            }
            if he.get_c_state(HS::Select) {
                he.vertex_next_kind2();
                self.update_all();
                return true;
            }
        }
        false
    }
    fn save_vars(&mut self) {
        for he in self.hes.iter_mut() {
            he.save_vars();
        }
        if let Some(hes_prim) = &mut self.hes_prim {
            for he_prim in hes_prim.iter_mut() {
                he_prim.save_vars();
            }
        }
    }
    fn restore_vars(&mut self) {
        for he in self.hes.iter_mut() {
            he.restore_vars();
        }
        if let Some(hes_prim) = &mut self.hes_prim {
            for he_prim in hes_prim.iter_mut() {
                he_prim.restore_vars();
            }
        }
    }
    fn get_vars(&self) -> ShapePolygon {
        ShapePolygon {
            kind: self.kind.clone(),
            hes_prim: self.hes_prim.clone(),
            hes: self.hes.clone(),
            state: Status::default(),
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        }
    }
    fn set_vars(&mut self, polygon: &ShapePolygon) {
        self.kind = polygon.kind.clone();
        self.hes = polygon.hes.clone();
        self.hes_prim = polygon.hes_prim.clone();
    }

    fn get_state(&self, get: GetEntityState) -> bool {
        match get {
            IsHS(hs) => self.state.is_hs(hs),
            IsAControlHS(hs) => {
                for he in self.hes.iter() {
                    if he.get_vertex_state(hs) {
                        return true;
                    }
                    if he.get_c_state(hs) {
                        return true;
                    }
                }
                if let Some(hes_prim) = self.hes_prim.as_ref() {
                    for he in hes_prim.iter() {
                        if he.get_vertex_state(hs) {
                            return true;
                        }
                    }
                }
                false
            }
        }
    }
    fn set_state(&mut self, set: SetEntityState) {
        match set {
            SetHS(hs, value) => self.state.set_hs(hs, value),
            SetAllControlsHS(hs, state) => {
                self.hes.iter_mut().for_each(|he| {
                    he.set_vertex_state(hs, state);
                });
                self.hes_prim.as_mut().and_then(|hes_prim| {
                    hes_prim.iter_mut().for_each(|he| {
                        he.set_vertex_state(hs, state);
                    });
                    Some(())
                });
            }
        }
    }
    fn set_state_from_pos(
        &mut self,
        pointer: &mut Pointer,
        keys_states: KeysStates,
        set: SetEntityStateFromPos,
    ) -> bool {
        match set {
            SetHSFromPos(hs) => {
                let state = self.contains(pointer.pos().to_point());
                self.state.set_hs(hs, state);
                state
            }
            SetControlHSFromPos(hs) => {
                // Clear all selections
                self.clear_selections(hs);

                // Check for the vertices
                if self.set_near_vertex(pointer, keys_states, hs) {
                    return true;
                };
                // Check for the c
                if self.set_near_c(pointer, keys_states, hs) {
                    return true;
                };

                // Check also for polygon center
                let centroid = self.get_centroid()[0];
                if (pointer.pos() - centroid).hypot() < Self::GRAB_RADIUS {
                    if !keys_states.alt_pressed {
                        pointer.set_pos(centroid);
                        pointer.set_magnetized(true);
                        // Don't return true here
                    }
                    self.state.set_hs(hs, true);
                }
                false
            }
        }
    }
    fn contains_pointer(&self, pointer: &Pointer) -> bool {
        self.contains(pointer.pos().to_point())
    }
    fn move_position(&mut self, pointer: &mut Pointer, _keys_states: KeysStates) -> bool {
        for idx_he in 0..self.hes.len() as i64 {
            self.hes.get_mut(idx_he).move_vertex(pointer.dpos());
        }
        self.hes_prim.as_mut().and_then(|hes_prim| {
            for idx_he in 0..hes_prim.len() as i64 {
                hes_prim.get_mut(idx_he).move_vertex(pointer.dpos());
            }
            Some(())
        });
        self.update_all();
        true
    }
    fn move_controls(&mut self, pointer: &Pointer, _keys_states: KeysStates) -> bool {
        use PolyKind::*;

        let snap = pointer.get_snap().val();
        let snap_angle = pointer.get_snap_angle().val();
        /*
           hes
        */
        for idx_he in 0..self.hes.len() as i64 {
            if self.hes.get(idx_he).get_vertex_state(Select)
                && self.hes.get(idx_he).is_vertex_movable()
            {
                match self.kind {
                    Rectangle | RectangleFilleted => {
                        let v_p = self.hes.get(idx_he - 1).get_vertex().pos;
                        let v = self.hes.get(idx_he).get_vertex().pos;
                        let v_n = self.hes.get(idx_he + 1).get_vertex().pos;
                        let v_nn = self.hes.get(idx_he + 2).get_vertex().pos;
                        let vs_p = self.hes.get(idx_he - 1).get_vertex().saved_pos;
                        let vs = self.hes.get(idx_he).get_vertex().saved_pos;
                        let vs_n = self.hes.get(idx_he + 1).get_vertex().saved_pos;
                        let len1 = (vs - vs_p).hypot();
                        let len2 = (vs_n - vs).hypot();
                        if let Some(bdl1) = SegBundle::new(vs, vs_p) {
                            if let Some(bdl2) = SegBundle::new(vs, vs_n) {
                                let dpos_1 = pointer.dpos().dot(-bdl1.u());
                                let dpos_2 = pointer.dpos().dot(-bdl2.u());
                                let (new_len1, new_len2) = if !pointer.is_magnetized() {
                                    (snap_val(len1 + dpos_1, snap), snap_val(len2 + dpos_2, snap))
                                } else {
                                    (len1 + dpos_1, len2 + dpos_2)
                                };

                                let new_vp = v_nn - bdl2.u() * new_len2;
                                let new_vn = v_nn - bdl1.u() * new_len1;
                                let new_v = if idx_he % 2 == 0 {
                                    Vec2::new(new_vp.x, new_vn.y)
                                } else {
                                    Vec2::new(new_vn.x, new_vp.y)
                                };
                                if (new_v - new_vp).hypot() > EPSILON
                                    && (new_vn - new_v).hypot() > EPSILON
                                {
                                    self.hes.get_mut(idx_he - 1).set_vertex_pos(new_vp);
                                    self.hes.get_mut(idx_he + 1).set_vertex_pos(new_vn);
                                    self.hes.get_mut(idx_he).set_vertex_pos(new_v);
                                    if self.area() <= 0. {
                                        self.hes.get_mut(idx_he).set_vertex_pos(v);
                                        self.hes.get_mut(idx_he - 1).set_vertex_pos(v_p);
                                        self.hes.get_mut(idx_he + 1).set_vertex_pos(v_n);
                                    }
                                    self.update_all();
                                    return true;
                                } else {
                                    return false;
                                }
                            }
                        }
                    }
                    Oblong => {}
                    Custom => {
                        let v_p = self.hes.get(idx_he - 1).get_vertex().pos;
                        let v = if !pointer.is_magnetized() {
                            snap_length_and_angle(v_p, pointer.pos(), snap, snap_angle)
                        } else {
                            pointer.pos()
                        };
                        self.hes.get_mut(idx_he).set_vertex_pos(v);
                        self.update_all();
                    }
                }
                return true;
            }
            if self.hes.get(idx_he).get_c_state(Select) {
                if self
                    .hes
                    .get(idx_he)
                    .get_c()
                    .and_then(|c| {
                        let v = self.hes.get(idx_he).get_vertex().pos;
                        SegBundle::new(v, c).and_then(|bdl| {
                            match self.hes.get_mut(idx_he).get_vertex_kind_mut() {
                                VertexKind::Chamfer { length } => {
                                    let dpos_proj = pointer.dpos().dot(bdl.u());
                                    length.value = (length.saved_val + dpos_proj)
                                        .max(HalfEdge::MIN_FILLET_RADIUS);
                                    self.update_all();
                                    Some(())
                                }
                                VertexKind::Fillet { radius } => {
                                    let dpos_proj = pointer.dpos().dot(bdl.u());
                                    radius.value = (radius.saved_val + dpos_proj)
                                        .max(HalfEdge::MIN_FILLET_RADIUS);
                                    self.update_all();
                                    Some(())
                                }
                                VertexKind::Point { dummy: _ } => None,
                            }
                        })
                    })
                    .is_some()
                {
                    return true;
                }
            };
        }

        /*
           hes primitives
        */
        if let Some(hes_prim) = self.hes_prim.as_mut() {
            // Check prim vertices, move the first selected found and return
            for idx_he in 0..hes_prim.len() as i64 {
                if hes_prim.get(idx_he).get_vertex_state(Select) {
                    if hes_prim.get(idx_he).is_vertex_movable() {
                        //
                        match self.kind {
                            Oblong => {
                                if idx_he == 0 || idx_he == 1 {
                                    SegBundle::new(
                                        hes_prim.get(0).get_vertex().saved_pos,
                                        hes_prim.get(1).get_vertex().saved_pos,
                                    )
                                    .and_then(|bdl_saved| {
                                        let start = if idx_he == 1 {
                                            hes_prim.get(0).get_vertex().saved_pos
                                        } else {
                                            hes_prim.get(1).get_vertex().saved_pos
                                        };
                                        let end = if !pointer.is_magnetized() {
                                            snap_length_and_angle(
                                                start,
                                                pointer.pos(),
                                                snap,
                                                snap_angle,
                                            )
                                        } else {
                                            pointer.pos()
                                        };
                                        hes_prim.get_mut(idx_he).set_vertex_pos(end);

                                        // Update the width control
                                        let saved_width2 = (bdl_saved.m()
                                            - hes_prim.get(2).get_vertex().saved_pos)
                                            .hypot();
                                        SegBundle::new(
                                            hes_prim.get(0).get_vertex().pos,
                                            hes_prim.get(1).get_vertex().pos,
                                        )
                                        .and_then(|bdl| {
                                            let width_pt = bdl.m() + bdl.n() * saved_width2;
                                            hes_prim.get_mut(2).set_vertex_pos(width_pt);
                                            Some(())
                                        })
                                    });
                                } else {
                                    if idx_he == 2 {
                                        SegBundle::new(
                                            hes_prim.get(0).get_vertex().pos,
                                            hes_prim.get(1).get_vertex().pos,
                                        )
                                        .and_then(|bdl| {
                                            let width_pt_saved =
                                                hes_prim.get(2).get_vertex().saved_pos;
                                            let dpos_proj =
                                                snap_val(pointer.dpos().dot(bdl.n()), snap);
                                            let width_pt = width_pt_saved + bdl.n() * dpos_proj;
                                            if (width_pt - bdl.m()).hypot()
                                                >= Self::MIN_OBLONG_WIDTH
                                            {
                                                hes_prim.get_mut(2).set_vertex_pos(width_pt);
                                            }
                                            Some(())
                                        });
                                    }
                                }
                                self.update_oblong_edges();
                                self.update_all();
                            }
                            _ => (),
                        }
                        return true;
                    }
                }
            }
        }
        false
    }
    fn get_position(&self) -> Vec2 {
        self.get_centroid()[0]
    }
    fn get_centroid(&self) -> Vec<Vec2> {
        let mut sum = Vec2::ZERO;
        for he in self.hes.iter() {
            sum += he.get_vertex().pos;
        }
        vec![sum / self.hes.len() as f64]
    }
    fn get_controls_paths_and_patterns(
        &self,
        _das: &Size,
        canvas_infos: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, Colors)> {
        let scale = canvas_infos.1;
        let mut paths_patterns = vec![];
        // Centroid
        paths_patterns.push((
            centroid_path(self.get_centroid()[0], scale, Self::GRAB_RADIUS),
            Pattern::Point,
            get_centroids_colors(self.state),
        ));
        // VERTICES HES
        for idx_he in 0..self.hes.len() as i64 {
            let he = self.hes.get(idx_he);
            if he.is_vertex_selectable() {
                paths_patterns.push((
                    point_path(he.get_vertex().pos, scale),
                    Pattern::Point,
                    get_controls_colors(he.vertex_state()),
                ));
            }
        }
        // VERTEX KIND CONTROL HES
        for idx_he in 0..self.hes.len() as i64 {
            let he = self.hes.get(idx_he);
            if he.is_vertex_selectable() {
                paths_patterns.push((
                    he.get_c_control_paths_and_patterns(scale).0,
                    Pattern::Point,
                    get_controls_colors(he.c_state()),
                ));
            }
        }
        // VERTICES HES PRIM
        self.hes_prim.as_ref().and_then(|hes_prim| {
            for idx_he in 0..hes_prim.len() as i64 {
                let he = hes_prim.get(idx_he);
                if he.is_vertex_selectable() {
                    paths_patterns.push((
                        point_path(he.get_vertex().pos, scale),
                        Pattern::Point,
                        get_controls_colors(he.vertex_state()),
                    ));
                }
            }
            Some(())
        });
        // POLYGON CENTER
        paths_patterns.push((
            centroid_path(self.get_centroid()[0], scale, Self::GRAB_RADIUS),
            Pattern::Point,
            get_centroids_colors(self.state),
        ));
        paths_patterns
    }
    fn get_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, Colors)> {
        let mut paths_patterns = vec![];

        for idx_he in 0..self.hes.len() as i64 {
            let s_next = self.hes.get(idx_he + 1).get_s();
            let v_next = self.hes.get(idx_he + 1).get_vertex().pos;
            // Vertex kind
            let vertex_kind_path_pattern =
                self.hes.get(idx_he).get_vertex_kind_paths_and_patterns();
            paths_patterns.push((
                vertex_kind_path_pattern.0,
                vertex_kind_path_pattern.1,
                get_shapes_colors(self.state),
            ));
            // EDGES
            let edge_path_pattern = self
                .hes
                .get(idx_he)
                .get_edge_paths_and_patterns(s_next, v_next);
            paths_patterns.push((
                edge_path_pattern.0,
                edge_path_pattern.1,
                get_shapes_colors(self.state),
            ));
        }
        paths_patterns
    }
    fn get_prim_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, Colors)> {
        let mut paths_patterns = vec![];
        if let Some(hes_prim) = self.hes_prim.as_ref() {
            let s_next = hes_prim.get(1).get_s();
            let v_next = hes_prim.get(1).get_vertex().pos;
            let prim_path_pattern = hes_prim
                .get(0)
                .get_prim_edge_paths_and_patterns(s_next, v_next);
            paths_patterns.push((
                prim_path_pattern.0,
                prim_path_pattern.1,
                get_prims_colors(self.state),
            ));
        };
        paths_patterns
    }
    fn get_dimensions_paths_and_patterns(
        &self,
        _size: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, Colors, Vec<CanvasText>)> {
        use PolyKind::*;
        use HS::*;
        let mut res = vec![];
        match self.kind {
            Rectangle => {
                let display = self.state.is_hs(Select)
                    || self.state.is_hs(Highlight)
                    || self.get_state(IsAControlHS(Select))
                    || self.get_state(IsAControlHS(Highlight));

                (0..2).for_each(|idx| {
                    let he = self.hes.get(idx);
                    display.then(|| {
                        SegBundle::new(he.get_vertex().pos, self.hes.get(idx + 1).get_vertex().pos)
                            .and_then(|bdl| {
                                res.push(dim_linear(bdl, cinfo, self.state));
                                Some(())
                            });
                    });
                });
            }
            RectangleFilleted => {
                let display = self.state.is_hs(Select)
                    || self.state.is_hs(Highlight)
                    || self.get_state(IsAControlHS(Select))
                    || self.get_state(IsAControlHS(Highlight));

                (0..2).for_each(|idx| {
                    let he = self.hes.get(idx);
                    display.then(|| {
                        SegBundle::new(he.get_vertex().pos, self.hes.get(idx + 1).get_vertex().pos)
                            .and_then(|bdl| {
                                res.push(dim_linear(bdl, cinfo, self.state));
                                Some(())
                            });
                    });
                });
            }
            Oblong => {
                let display = self.state.is_hs(Select)
                    || self.state.is_hs(Highlight)
                    || self.get_state(IsAControlHS(Select))
                    || self.get_state(IsAControlHS(Highlight));

                let v0 = self.hes.get(0).get_vertex().pos;
                let v1 = self.hes.get(1).get_vertex().pos;
                let v2 = self.hes.get(2).get_vertex().pos;
                display.then(|| {
                    SegBundle::new(v0, v1).and_then(|bdl1| {
                        res.push(dim_linear_angle(bdl1, cinfo, self.state));
                        let width = (v1 - v2).hypot();
                        let (vm1, vm2) = (
                            v1 + bdl1.u() * width * 3. / 5.,
                            v2 + bdl1.u() * width * 3. / 5.,
                        );
                        SegBundle::new(vm1, vm2).and_then(|bdl2| {
                            res.push(dim_linear(bdl2, cinfo, self.state));
                            Some(())
                        })
                    });
                });
            }
            Custom => {
                let display = self.state.is_hs(Select)
                    || self.state.is_hs(Highlight)
                    || self.get_state(IsAControlHS(Select))
                    || self.get_state(IsAControlHS(Highlight));

                for he_idx in 0..self.hes.len() as i64 {
                    let he = self.hes.get(he_idx);
                    display.then(|| {
                        SegBundle::new(
                            he.get_vertex().pos,
                            self.hes.get(he_idx + 1).get_vertex().pos,
                        )
                        .and_then(|bdl| {
                            res.push(dim_linear_angle(bdl, cinfo, self.state));
                            Some(())
                        });
                    });
                }
            }
        }
        res
    }
}

pub struct PolygonIter {
    idx: usize,
    iter: Vec<PathEl>,
}
impl Iterator for PolygonIter {
    type Item = PathEl;

    fn next(&mut self) -> Option<Self::Item> {
        self.idx += 1;
        self.iter.get(self.idx - 1).cloned()
    }
}
