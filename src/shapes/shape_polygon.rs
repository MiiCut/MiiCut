use super::shapes::ShapeKind;
use crate::{
    canvas::{CanvasText, Pattern},
    curves::half_edge::{EdgeKind, HEProps, HalfEdge},
    dimensions::dim_linear,
    math::*,
    pools::HS,
    positions::{Status, Value},
    prefab::*,
    GetEntityState, KeysStates, ObjectsFuncs, Pointer, SetEntityState, SetEntityStateFromPos,
};
use geo::{LineString, Polygon};
use kurbo::{BezPath, PathEl, Point, Rect, Shape, Size, Vec2};
use std::{f64::consts::PI, fmt::Display, vec};

#[derive(Debug, Clone)]
pub struct VecRing<T> {
    vec: Vec<T>,
}
impl<T> VecRing<T> {
    pub fn from_element(e: T) -> Self {
        Self { vec: vec![e] }
    }
    pub fn get(&self, idx: i64) -> &T {
        let len = self.vec.len() as i64;
        let i = idx.rem_euclid(len) as usize;
        &self.vec[i]
    }
    pub fn get_mut(&mut self, idx: i64) -> &mut T {
        let len = self.vec.len() as i64;
        let i = idx.rem_euclid(len) as usize;
        &mut self.vec[i]
    }
    pub fn push(&mut self, e: T) {
        self.vec.push(e);
    }
    pub fn replace_first(&mut self, e: T) {
        self.vec[0] = e;
    }
    pub fn last_mut(&mut self) -> &mut T {
        let len1 = self.vec.len() - 1;
        &mut self.vec[len1]
    }
    pub fn len(&self) -> usize {
        self.vec.len()
    }
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.vec.iter()
    }
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.vec.iter_mut()
    }
}

#[derive(Debug, Clone)]
enum PolyKind {
    Rectangle,
    Oblong,
    Custom,
}
#[derive(Debug, Clone)]
pub struct ShapePolygon {
    kind: PolyKind,
    hes_prim: Option<(VecRing<HalfEdge>, Value)>,
    hes: VecRing<HalfEdge>,
    state: Status,
    segs: BezPath,
    polygon: Polygon<f64>,
}

impl ShapePolygon {
    pub const MIN_OBLONG_WIDTH: f64 = 2.;

    pub fn new_rectangle(hes_prim: VecRing<HalfEdge>) -> Option<ShapeKind> {
        use ShapeKind::*;
        (hes_prim.len() == 2).then(|| {
            let tmp1 = hes_prim.get(0).get_vertex().pos;
            let tmp2 = hes_prim.get(1).get_vertex().pos;
            // Always counter clockwize to have a positive area
            let pt1 = Vec2::new(tmp1.x.min(tmp2.x), tmp2.y.min(tmp1.y));
            let pt3 = Vec2::new(tmp2.x.max(tmp1.x), tmp1.y.max(tmp2.y));
            let pt4 = Vec2::new(tmp1.x.min(tmp2.x), tmp1.y.max(tmp2.y));
            let pt2 = Vec2::new(tmp2.x.max(tmp1.x), tmp2.y.min(tmp1.y));
            let mut props = HEProps::default();
            props.vertex_selectable = true;
            props.vertex_movable = false;
            props.edge_changeable = false;
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
            KindPolygon(shape_polygon)
        })
    }
    pub fn new_oblong(hes_prim: VecRing<HalfEdge>) -> Option<ShapeKind> {
        use ShapeKind::*;
        (hes_prim.len() == 2).then(|| {
            let mut props = HEProps::default();
            // props.vertex_selectable = true;
            props.vertex_movable = false;
            // Create hes dummy
            let mut hes = VecRing::from_element(HalfEdge::new(Vec2::ZERO, props));
            hes.push(HalfEdge::new(Vec2::ZERO, props));
            hes.push(HalfEdge::new(Vec2::ZERO, props));
            hes.push(HalfEdge::new(Vec2::ZERO, props));
            let mut shape_polygon = ShapePolygon {
                kind: PolyKind::Oblong,
                hes_prim: Some((hes_prim, Value::new(10. * Self::MIN_OBLONG_WIDTH))),
                hes,
                state: Status::default(),
                segs: BezPath::new(),
                polygon: Polygon::new(LineString::new(vec![]), vec![]),
            };
            // Update hes from hes_prim
            shape_polygon
                .update_hes_vertices_from_hes_prim_vertices()
                .then(|| {
                    shape_polygon.update_all();
                    log!("area: {}", shape_polygon.area());
                    log!("new_oblong");
                    KindPolygon(shape_polygon)
                })
        })?
    }
    pub fn new_custom(mut hes: VecRing<HalfEdge>) -> Option<ShapeKind> {
        use ShapeKind::*;
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
        Some(KindPolygon(shape_polygon))
    }

    fn update_hes_vertices_from_hes_prim_vertices(&mut self) -> bool {
        self.hes_prim
            .as_ref()
            .and_then(|(hes_prim, width)| {
                let v = hes_prim.get(0).get_vertex().pos;
                let v_next = hes_prim.get(1).get_vertex().pos;
                let bdl = get_seg_bdle(v, v_next)?;
                match hes_prim.get(0).get_edge_kind() {
                    EdgeKind::Segment { dum: _ } => {
                        self.hes.get_mut(0).set_vertex_pos(v - bdl.n * width.value);
                        self.hes.get_mut(3).set_vertex_pos(v + bdl.n * width.value);
                        self.hes
                            .get_mut(1)
                            .set_vertex_pos(v_next - bdl.n * width.value);
                        self.hes
                            .get_mut(2)
                            .set_vertex_pos(v_next + bdl.n * width.value);
                    }
                    EdgeKind::Arc { sag_rel } => {
                        let sagitta_pt = bdl.m - bdl.n * bdl.len * sag_rel.value;
                        circle_from_three_points(v, sagitta_pt, v_next).and_then(
                            |(center, _radius)| {
                                let n_v = (v - center).normalize()
                                    * (sagitta_pt - v).cross(bdl.u).signum();
                                let n_v_next = (v_next - center).normalize()
                                    * (sagitta_pt - v_next).cross(bdl.u).signum();
                                self.hes.get_mut(0).set_vertex_pos(v + n_v * width.value);
                                self.hes
                                    .get_mut(1)
                                    .set_vertex_pos(v_next + n_v_next * width.value);
                                self.hes
                                    .get_mut(2)
                                    .set_vertex_pos(v_next - n_v_next * width.value);
                                self.hes.get_mut(3).set_vertex_pos(v - n_v * width.value);
                                Some(())
                            },
                        );
                    }
                }

                Some(())
            })
            .is_some()
    }
    fn update_hes_edges_from_hes_prim_edges(&mut self) -> bool {
        self.hes_prim
            .as_ref()
            .and_then(|(hes_prim, _width)| {
                // Update the edge kind
                let edge_kind = *hes_prim.get(0).get_edge_kind();
                match edge_kind {
                    EdgeKind::Segment { dum } => {
                        self.hes.get_mut(0).set_edge_kind(EdgeKind::Segment { dum });
                        self.hes.get_mut(2).set_edge_kind(EdgeKind::Segment {
                            dum: Value::new(-dum.value),
                        });
                    }
                    EdgeKind::Arc { sag_rel } => {
                        self.hes
                            .get_mut(0)
                            .set_edge_kind(EdgeKind::Arc { sag_rel: sag_rel });
                        self.hes.get_mut(2).set_edge_kind(EdgeKind::Arc {
                            sag_rel: Value::new(-sag_rel.value),
                        });
                    }
                }
                Some(())
            })
            .is_some()
    }
    fn get_hes_len(&self) -> usize {
        self.hes.len()
    }
    fn get_he(&self, idx: i64) -> &HalfEdge {
        self.hes.get(idx)
    }
    fn get_he_mut(&mut self, idx: i64) -> &mut HalfEdge {
        self.hes.get_mut(idx)
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
            }
        }
        if let Some((hes_prim, _width)) = self.hes_prim.as_mut() {
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
    fn set_near_edge(&mut self, pointer: &mut Pointer, keys_states: KeysStates, hs: HS) -> bool {
        for idx_he in 0..self.hes.len() as i64 {
            if self.hes.get(idx_he).is_edge_selectable() {
                let s_next = self.hes.get(idx_he + 1).get_s();
                let v_next = self.hes.get(idx_he + 1).get_vertex().pos;
                if let Some((dist, pos)) =
                    self.hes
                        .get(idx_he)
                        .get_distance_to_edge(s_next, v_next, pointer.pos())
                {
                    if dist < Self::GRAB_RADIUS / pointer.get_draw_scale() {
                        if !keys_states.alt_pressed {
                            pointer.set_pos(pos);
                            pointer.set_magnetized(true);
                        }
                        self.hes.get_mut(idx_he).set_edge_state(hs, true);
                        return true;
                    }
                }
            }
        }
        if let Some((hes_prim, _width)) = self.hes_prim.as_mut() {
            for idx_he in 0..(hes_prim.len() - 1) as i64 {
                if hes_prim.get(idx_he).is_edge_selectable() {
                    let s_next = hes_prim.get(idx_he + 1).get_s();
                    let v_next = hes_prim.get(idx_he + 1).get_vertex().pos;
                    if let Some((dist, pos)) =
                        hes_prim
                            .get(idx_he)
                            .get_distance_to_edge(s_next, v_next, pointer.pos())
                    {
                        if dist < Self::GRAB_RADIUS / pointer.get_draw_scale() {
                            if !keys_states.alt_pressed {
                                pointer.set_pos(pos);
                                pointer.set_magnetized(true);
                            }
                            hes_prim.get_mut(idx_he).set_edge_state(hs, true);
                            return true;
                        }
                    }
                }
            }
        };
        false
    }
    fn set_near_corner(&mut self, pointer: &mut Pointer, keys_states: KeysStates, hs: HS) -> bool {
        for idx_he in 0..self.hes.len() as i64 {
            if let Some((dist, pos)) = self.hes.get(idx_he).get_distance_to_corner(pointer.pos()) {
                if dist < Self::GRAB_RADIUS / pointer.get_draw_scale() {
                    if !keys_states.alt_pressed {
                        pointer.set_pos(pos);
                        pointer.set_magnetized(true);
                    }
                    self.hes.get_mut(idx_he).set_corner_state(hs, true);
                    return true;
                }
            }
        }
        false
    }
    fn clear_selections(&mut self, hs: HS) {
        self.state.set_hs(hs, false);

        self.hes.iter_mut().for_each(|he| {
            he.set_vertex_state(hs, false);
            he.set_corner_state(hs, false);
            he.set_edge_state(hs, false);
        });
        self.hes_prim.as_mut().and_then(|(hes_prim, _width)| {
            hes_prim.iter_mut().for_each(|he| {
                he.set_vertex_state(hs, false);
                he.set_corner_state(hs, false);
                he.set_edge_state(hs, false);
            });
            Some(())
        });
    }
    fn get_centroid(&self) -> Vec2 {
        let mut sum = Vec2::ZERO;
        for he in self.hes.iter() {
            sum += he.get_vertex().pos;
        }
        sum / self.hes.len() as f64
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
        if let Some((hes_prim, _width)) = self.hes_prim.as_mut() {
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

    pub fn change_polygon_wedge_or_edge(&mut self, _keys_states: &KeysStates) {
        use HS::*;
        for idx_he in 0..self.get_hes_len() as i64 {
            let he = self.get_he_mut(idx_he);
            // A. Change first primitive selected and break if found
            if he.get_edge_state(Select) {
                he.edge_next_kind();
                self.update_hes_edges_from_hes_prim_edges();
                self.update_hes_vertices_from_hes_prim_vertices();
                break;
            }
            // B. Change first corner selected and break if found
            if he.get_corner_state(Select) || he.get_vertex_state(Select) {
                he.corner_next_kind();
                break;
            }
        }
        // Prim edges
        if let Some((hes_prim, _width)) = self.hes_prim.as_mut() {
            for idx_he in 0..hes_prim.len() as i64 {
                let he = hes_prim.get_mut(idx_he);
                if he.get_edge_state(Select) {
                    he.edge_next_kind();
                    self.update_hes_edges_from_hes_prim_edges();
                    self.update_hes_vertices_from_hes_prim_vertices();
                    self.update_all();
                    break;
                }
            }
        }
        self.update_all();
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
impl Shape for ShapePolygon {
    type PathElementsIter<'iter> = PolygonIter;

    fn path_elements(&self, _tolerance: f64) -> PolygonIter {
        let mut iter = vec![];
        let paths = self.get_paths_and_patterns(&Size::ZERO, (Rect::ZERO, 0., Vec2::ZERO));
        for (bez_path, _) in paths.iter() {
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
    type Kindvars = ShapeKind;

    fn save_vars(&mut self) {
        for he in self.hes.iter_mut() {
            he.save_vars();
        }
        if let Some((hes_prim, width)) = &mut self.hes_prim {
            for he_prim in hes_prim.iter_mut() {
                he_prim.save_vars();
            }
            width.saved_val = width.value;
        }
    }
    fn restore_vars(&mut self) {
        for he in self.hes.iter_mut() {
            he.restore_vars();
        }
        if let Some((hes_prim, width)) = &mut self.hes_prim {
            for he_prim in hes_prim.iter_mut() {
                he_prim.restore_vars();
            }
            width.value = width.saved_val;
        }
    }
    fn get_vars(&self) -> ShapeKind {
        ShapeKind::KindPolygon(ShapePolygon {
            kind: self.kind.clone(),
            hes_prim: self.hes_prim.clone(),
            hes: self.hes.clone(),
            state: Status::default(),
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
    fn set_vars(&mut self, shape_kind: &ShapeKind) {
        if let ShapeKind::KindPolygon(shape_polygon) = shape_kind {
            self.kind = shape_polygon.kind.clone();
            self.hes = shape_polygon.hes.clone();
            self.hes_prim = shape_polygon.hes_prim.clone();
        }
    }

    fn get_state(&self, get: GetEntityState) -> bool {
        use GetEntityState::*;
        match get {
            IsHS(hs) => self.state.is_hs(hs),
            IsAControlHS(hs) => {
                for he in self.hes.iter() {
                    if he.get_vertex_state(hs) {
                        return true;
                    }
                    if he.get_corner_state(hs) {
                        return true;
                    }
                    if he.get_edge_state(hs) {
                        return true;
                    }
                }
                if let Some((hes_prim, _width)) = self.hes_prim.as_ref() {
                    for he in hes_prim.iter() {
                        if he.get_vertex_state(hs) {
                            return true;
                        }
                        if he.get_corner_state(hs) {
                            return true;
                        }
                        if he.get_edge_state(hs) {
                            return true;
                        }
                    }
                }
                false
            }
        }
    }
    fn set_state(&mut self, set: SetEntityState) {
        use SetEntityState::*;
        match set {
            SetHS(hs, value) => self.state.set_hs(hs, value),
            SetAllControlsHS(hs, state) => {
                self.hes.iter_mut().for_each(|he| {
                    he.set_vertex_state(hs, state);
                    he.set_corner_state(hs, state);
                    he.set_edge_state(hs, state);
                });
                self.hes_prim.as_mut().and_then(|(hes_prim, _width)| {
                    hes_prim.iter_mut().for_each(|he| {
                        he.set_vertex_state(hs, state);
                        he.set_corner_state(hs, state);
                        he.set_edge_state(hs, state);
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
        use SetEntityStateFromPos::*;
        match set {
            SetHSFromPos(hs) => {
                let state = self.contains(pointer.pos().to_point());
                self.state.set_hs(hs, state);
                state
            }
            SetControlHSFromPos(hs) => {
                // Clear all selections
                self.clear_selections(hs);

                // 1. Check for the vertices
                if self.set_near_vertex(pointer, keys_states, hs) {
                    return true;
                };
                // 2. Check for the edges
                if self.set_near_edge(pointer, keys_states, hs) {
                    return true;
                };
                // 3. Check for the corners
                if self.set_near_corner(pointer, keys_states, hs) {
                    return true;
                }

                // Check also for polygon center
                let centroid = self.get_centroid();
                if (pointer.pos() - centroid).hypot() < Self::GRAB_RADIUS {
                    if !keys_states.alt_pressed {
                        pointer.set_pos(centroid);
                        pointer.set_magnetized(true);
                    }
                    self.state.set_hs(hs, true);
                    return true;
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
        self.hes_prim.as_mut().and_then(|(hes_prim, _width)| {
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
        use HS::*;
        let snap = pointer.get_snap().val();
        // 1. Check vertices, move the first selected found and return
        for idx_he in 0..self.hes.len() as i64 {
            if self.hes.get(idx_he).get_vertex_state(Select) {
                if self.hes.get(idx_he).is_vertex_movable() {
                    // We need prev and next vertices for the snapping
                    // The snapping serves to keep the edges length as a round number
                    let v_prev = self.hes.get(idx_he - 1).get_vertex().pos;
                    let v_next = self.hes.get(idx_he + 1).get_vertex().pos;
                    let v_saved = self.hes.get(idx_he).get_vertex().saved_pos;

                    // Try move the vertex (None if snapping not found)
                    let o_v_new = if pointer.is_magnetized() {
                        Some(pointer.pos())
                    } else {
                        move_vertex_with_3v_snapping(v_prev, v_saved, v_next, pointer.dpos(), snap)
                    };
                    if let Some(v_new) = o_v_new {
                        self.hes.get_mut(idx_he).set_vertex_pos(v_new);
                        self.update_all();
                        return true;
                    }
                }
                return false;
            }
        }

        // 2. Move corner (chamfer/fillet): move the first control selected found and return
        for idx_he in 0..self.hes.len() as i64 {
            if self.hes.get(idx_he).get_corner_state(Select) {
                if self.hes.get(idx_he).is_corner_movable() {
                    self.hes.get_mut(idx_he).move_corner(pointer.dpos());
                    self.update_all();
                    return true;
                }
            }
        }

        // 3. Move edges: move the first control selected found and return
        for idx_he in 0..self.hes.len() as i64 {
            if self.hes.get(idx_he).get_edge_state(Select) {
                use EdgeKind::*;
                match *self.hes.get_mut(idx_he).get_edge_kind() {
                    Segment { dum: _ } => {
                        if self.hes.get(idx_he).is_edge_movable() {
                            match self.kind {
                                Rectangle => {
                                    self.hes.get_mut(idx_he).move_vertex(pointer.dpos());
                                    self.hes.get_mut(idx_he + 1).move_vertex(pointer.dpos());
                                }
                                Oblong => {
                                    if idx_he == 0 || idx_he == 2 {
                                        if let Some((hes_prim, width)) = self.hes_prim.as_mut() {
                                            let v = hes_prim.get(0).get_vertex().pos;
                                            let v_next = hes_prim.get(1).get_vertex().pos;
                                            if let Some(bdl) = get_seg_bdle(v, v_next) {
                                                let dpos_proj = pointer.dpos().dot(bdl.n);
                                                width.value = if idx_he == 0 {
                                                    width.saved_val - dpos_proj
                                                } else {
                                                    width.saved_val + dpos_proj
                                                };
                                                if width.value < Self::MIN_OBLONG_WIDTH {
                                                    width.value = Self::MIN_OBLONG_WIDTH;
                                                }
                                            }
                                        }
                                    }
                                }
                                Custom => {
                                    self.hes.get_mut(idx_he).move_vertex(pointer.dpos());
                                    self.hes.get_mut(idx_he + 1).move_vertex(pointer.dpos());
                                }
                            }
                        }
                    }
                    Arc { sag_rel } => {
                        let v = self.hes.get(idx_he).get_vertex().pos;
                        let v_next = self.hes.get(idx_he + 1).get_vertex().pos;
                        if let Some(sb) = get_seg_bdle(v, v_next) {
                            let mut new_sag_rel = Value::new(
                                (sb.len * sag_rel.saved_val - pointer.dpos().dot(sb.n)) / sb.len,
                            );
                            new_sag_rel.saved_val = sag_rel.saved_val;
                            self.hes.get_mut(idx_he).set_edge_kind(Arc {
                                sag_rel: new_sag_rel,
                            });
                        }
                    }
                }
                self.update_hes_edges_from_hes_prim_edges();
                self.update_hes_vertices_from_hes_prim_vertices();
                self.update_all();
                return true;
            }
        }

        /*
           hes primitives
        */
        if let Some((hes_prim, _width)) = self.hes_prim.as_mut() {
            // 4. Check prim vertices, move the first selected found and return
            for idx_he in 0..hes_prim.len() as i64 {
                if hes_prim.get(idx_he).get_vertex_state(Select) {
                    if hes_prim.get(idx_he).is_vertex_movable() {
                        hes_prim.get_mut(idx_he).set_vertex_pos(pointer.pos());
                        self.update_hes_vertices_from_hes_prim_vertices();
                        self.update_all();
                        return true;
                    }
                }
            }

            // 5. Move corner (chamfer/fillet): move the first control selected found and return
            for idx_he in 0..hes_prim.len() as i64 {
                if hes_prim.get(idx_he).get_corner_state(Select) {
                    if hes_prim.get(idx_he).is_corner_movable() {
                        hes_prim.get_mut(idx_he).move_corner(pointer.dpos());
                        self.update_all();
                        return true;
                    }
                }
            }

            // 6. Move edges: move the first control selected found and return
            for idx_he in 0..(hes_prim.len() - 1) as i64 {
                if hes_prim.get(idx_he).get_edge_state(Select) {
                    if hes_prim.get(idx_he).is_edge_movable() {
                        use EdgeKind::*;
                        match *hes_prim.get_mut(idx_he).get_edge_kind() {
                            Segment { dum: _ } => {
                                hes_prim.get_mut(idx_he).move_vertex(pointer.dpos());
                                hes_prim.get_mut(idx_he + 1).move_vertex(pointer.dpos());
                            }
                            Arc { sag_rel } => {
                                let v = hes_prim.get(idx_he).get_vertex().pos;
                                let v_next = hes_prim.get(idx_he + 1).get_vertex().pos;
                                if let Some(sb) = get_seg_bdle(v, v_next) {
                                    let mut new_sag_rel = Value::new(
                                        (sb.len * sag_rel.saved_val - pointer.dpos().dot(sb.n))
                                            / sb.len,
                                    );
                                    new_sag_rel.saved_val = sag_rel.saved_val;
                                    hes_prim.get_mut(idx_he).set_edge_kind(Arc {
                                        sag_rel: new_sag_rel,
                                    });
                                }
                            }
                        }
                        self.update_hes_edges_from_hes_prim_edges();
                        self.update_hes_vertices_from_hes_prim_vertices();
                        self.update_all();
                        return true;
                    }
                }
            }
        }
        false
    }

    fn get_position(&self) -> Vec2 {
        self.get_centroid()
    }

    fn get_controls_paths_and_patterns(
        &self,
        _das: &Size,
        canvas_infos: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        use HS::*;
        let scale = canvas_infos.1;
        let mut paths_patterns: Vec<(BezPath, Pattern)> = vec![];
        // VERTICES
        for idx_he in 0..self.hes.len() as i64 {
            let he = self.hes.get(idx_he);
            if he.is_vertex_selectable() {
                paths_patterns.push((
                    modifiers_path(he.get_vertex().pos, scale),
                    modifiers_pattern(he.get_vertex_state(Select), he.get_vertex_state(Highlight)),
                ));
            }
            // // DEBUG
            // paths_patterns.push((
            //     modifiers_path(he.get_c(), scale, Self::GRAB),
            //     modifiers_pattern(false, true),
            // ));
            // paths_patterns.push((
            //     modifiers_path(he.get_s(), scale, Self::GRAB),
            //     modifiers_pattern(false, true),
            // ));
            // paths_patterns.push((
            //     modifiers_path(he.get_e(), scale, Self::GRAB),
            //     modifiers_pattern(false, true),
            // ));
            // if let Some(sag_pt) = he.get_sagitta(self.hes.get(idx_he + 1).get_vertex().pos) {
            //     paths_patterns.push((
            //         modifiers_path(sag_pt, scale, Self::GRAB),
            //         modifiers_pattern(false, true),
            //     ));
            // }
        }
        self.hes_prim.as_ref().and_then(|(hes_prim, _width)| {
            for idx_he in 0..hes_prim.len() as i64 {
                let he = hes_prim.get(idx_he);
                if he.is_vertex_selectable() {
                    paths_patterns.push((
                        modifiers_path(he.get_vertex().pos, scale),
                        modifiers_pattern(
                            he.get_vertex_state(Select),
                            he.get_vertex_state(Highlight),
                        ),
                    ));
                }
            }
            Some(())
        });

        // DEBUG
        // paths_patterns.push((
        //     PrimitiveKindIter::PArc(
        //         kurbo::Arc {
        //             center: Vec2::ZERO.to_point(),
        //             radii: Vec2::new(100., 100.),
        //             start_angle: -3. * PI / 2.,
        //             sweep_angle: PI / 3.,
        //             x_rotation: 0.0,
        //         }
        //         .path_elements(0.01),
        //     )
        //     .collect(),
        //     modifiers_pattern(false, true),
        // ));

        // POLYGON CENTER
        paths_patterns.push((
            center_path(self.get_centroid(), scale, Self::GRAB_RADIUS),
            modifiers_pattern(self.state.is_hs(Select), self.state.is_hs(Highlight)),
        ));
        paths_patterns
    }
    fn get_paths_and_patterns(&self, _: &Size, _: (Rect, f64, Vec2)) -> Vec<(BezPath, Pattern)> {
        use HS::*;
        let mut paths_patterns = vec![];

        for idx_he in 0..self.hes.len() as i64 {
            let s_next = self.hes.get(idx_he + 1).get_s();
            let v_next = self.hes.get(idx_he + 1).get_vertex().pos;
            // CORNERS
            paths_patterns.push(self.hes.get(idx_he).get_corner_paths_and_patterns(
                self.state.is_hs(Select),
                self.state.is_hs(Highlight),
            ));
            // EDGES
            paths_patterns.push(self.hes.get(idx_he).get_edge_paths_and_patterns(
                s_next,
                v_next,
                self.state.is_hs(Select),
                self.state.is_hs(Highlight),
            ));
        }
        paths_patterns
    }
    fn get_prim_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        use HS::*;
        let mut paths_patterns = vec![];
        if let Some((hes_prim, _width)) = self.hes_prim.as_ref() {
            if hes_prim.get(0).is_edge_selectable() {
                let s_next = hes_prim.get(1).get_s();
                let v_next = hes_prim.get(1).get_vertex().pos;
                paths_patterns.push(hes_prim.get(0).get_prim_edge_paths_and_patterns(
                    s_next,
                    v_next,
                    self.state.is_hs(Select),
                    self.state.is_hs(Highlight),
                ));
            }
        };
        paths_patterns
    }
    fn get_dimensions_paths_and_patterns(
        &self,
        _size: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        use PolyKind::*;
        use HS::*;
        let mut res = vec![];
        match self.kind {
            Rectangle => {
                let display = (0..4).any(|idx| {
                    self.hes.get(idx).get_edge_state(Select)
                        || self.hes.get(idx).get_edge_state(Highlight)
                }) || self.state.is_hs(Select)
                    || self.state.is_hs(Highlight);

                (0..2).for_each(|idx| {
                    let he = self.hes.get(idx);
                    display.then(|| {
                        get_seg_bdle(he.get_vertex().pos, self.hes.get(idx + 1).get_vertex().pos)
                            .and_then(|bdl| {
                                res.push(dim_linear(bdl, cinfo));
                                Some(())
                            });
                    });
                });
            }
            Oblong => {
                let display = (0..4).any(|idx| {
                    self.hes.get(idx).get_edge_state(Select)
                        || self.hes.get(idx).get_edge_state(Highlight)
                }) || self.state.is_hs(Select)
                    || self.state.is_hs(Highlight);

                let v0 = self.hes.get(0).get_vertex().pos;
                let v1 = self.hes.get(1).get_vertex().pos;
                let v2 = self.hes.get(2).get_vertex().pos;
                let v3 = self.hes.get(3).get_vertex().pos;
                display.then(|| {
                    get_seg_bdle(v0, v1).and_then(|bdl1| {
                        res.push(dim_linear(bdl1, cinfo));
                        let (vm1, vm2) = if bdl1.a > -PI / 2. && bdl1.a < PI / 2. {
                            (
                                v3 + bdl1.u * bdl1.len * 3. / 4.,
                                v0 + bdl1.u * bdl1.len * 3. / 4.,
                            )
                        } else {
                            (v3 + bdl1.u * bdl1.len / 4., v0 + bdl1.u * bdl1.len / 4.)
                        };
                        get_seg_bdle(vm1, vm2).and_then(|bdl2| {
                            res.push(dim_linear(bdl2, cinfo));
                            Some(())
                        })
                    });
                });
            }
            Custom => {
                for he_idx in 0..self.hes.len() as i64 {
                    let he = self.hes.get(he_idx);
                    let selected = he.get_vertex_state(Select) || self.state.is_hs(Select);
                    let highlighted = he.get_vertex_state(Highlight) || self.state.is_hs(Highlight);
                    (selected || highlighted).then(|| {
                        get_seg_bdle(
                            he.get_vertex().pos,
                            self.hes.get(he_idx + 1).get_vertex().pos,
                        )
                        .and_then(|bdl| {
                            res.push(dim_linear(bdl, cinfo));
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
