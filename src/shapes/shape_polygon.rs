use super::shapes::ShapeKind;
use crate::{
    canvas::{CanvasText, Pattern},
    curves::half_edge::HalfEdge,
    math::*,
    pools::HS,
    positions::{Minimum, Status},
    prefab::*,
    GetEntityState, KeysStates, ObjectsFuncs, Pointer, SetEntityState, SetEntityStateFromPos,
};
use geo::{LineString, Polygon};
use kurbo::{BezPath, PathEl, Point, Rect, Shape, Size, Vec2};
use std::{fmt::Display, vec};

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
pub struct ShapePolygon {
    hes_prim: Option<VecRing<HalfEdge>>,
    hes: VecRing<HalfEdge>,
    state: Status,
    segs: BezPath,
    polygon: Polygon<f64>,
}

impl ShapePolygon {
    const GRAB: f64 = 5.;
    pub const MIN_OBLONG_WIDTH: f64 = 10.;

    pub fn new_polygon(mut hes: VecRing<HalfEdge>) -> Option<ShapeKind> {
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
    pub fn new_oblong(hes_prim: VecRing<HalfEdge>) -> Option<ShapeKind> {
        use ShapeKind::*;
        (hes_prim.len() >= 2).then(|| {
            // Construct hes from hes_prim
            let mut shape_polygon = ShapePolygon {
                hes_prim: Some(hes_prim),
                hes: VecRing::from_element(HalfEdge::new(Vec2::ZERO, false, false)),
                state: Status::default(),
                segs: BezPath::new(),
                polygon: Polygon::new(LineString::new(vec![]), vec![]),
            };
            shape_polygon.construct_hes_from_hes_prim().then(|| {
                shape_polygon.update_all();
                log!("area: {}", shape_polygon.area());
                log!("new_oblong");
                KindPolygon(shape_polygon)
            })
        })?
    }
    fn construct_hes_from_hes_prim(&mut self) -> bool {
        let mut hes_up = vec![];
        let mut hes_down = vec![];
        self.hes_prim
            .as_ref()
            .and_then(|hes_prim| {
                // Here we are sure that prim_len >= 2
                let prim_len = hes_prim.len() as i64;
                for idx in 0..prim_len {
                    match idx {
                        0 => {
                            // Code for the first element
                            let v = hes_prim.get(idx).get_vertex().pos;
                            let v_next = hes_prim.get(idx + 1).get_vertex().pos;
                            let bdl = get_seg_bdle(v, v_next)?;
                            // Continue processing...
                            hes_up.push(v + bdl.n * Self::MIN_OBLONG_WIDTH);
                            hes_down.push(v - bdl.n * Self::MIN_OBLONG_WIDTH);
                        }
                        _ if idx == prim_len - 1 => {
                            // Code for the last element
                            let v_prev = hes_prim.get(idx - 1).get_vertex().pos;
                            let v = hes_prim.get(idx).get_vertex().pos;
                            let bdl = get_seg_bdle(v_prev, v)?;
                            // Continue processing...
                            hes_up.push(v + bdl.n * Self::MIN_OBLONG_WIDTH);
                            hes_down.push(v - bdl.n * Self::MIN_OBLONG_WIDTH);
                        }
                        _ => {
                            // Code for all other elements
                            let v_prev = hes_prim.get(idx - 1).get_vertex().pos;
                            let v = hes_prim.get(idx).get_vertex().pos;
                            let v_next = hes_prim.get(idx + 1).get_vertex().pos;
                            let bdl_prev = get_seg_bdle(v_prev, v)?;
                            let bdl_next = get_seg_bdle(v, v_next)?;
                            // Continue processing...
                            let n = bdl_prev.n + bdl_next.n;
                            hes_up.push(v + n * Self::MIN_OBLONG_WIDTH);
                            hes_down.push(v - n * Self::MIN_OBLONG_WIDTH);
                        }
                    }
                }
                // Construct the half edges
                let mut hes = VecRing::from_element(HalfEdge::new(hes_down[0], false, false));
                for idx in 1..hes_down.len() {
                    hes.push(HalfEdge::new(hes_down[idx], false, false));
                }
                hes_up.reverse();
                for idx in 0..hes_up.len() {
                    hes.push(HalfEdge::new(hes_up[idx], false, false));
                }
                self.hes = hes;
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
    fn near_vertex(&self, pointer: &mut Pointer) -> Option<(f64, i64, Vec2)> {
        let mut minimum = Minimum::new();
        for idx_he in 0..self.hes.len() as i64 {
            let (dist, v) = self.hes.get(idx_he).get_distance_to_vertex(pointer.pos());
            if dist < Self::GRAB_RADIUS {
                minimum.update(dist, idx_he, v);
            }
        }
        minimum.get_min()
    }
    fn near_edge(&self, pointer: &mut Pointer) -> Option<(f64, i64, Vec2)> {
        let mut minimum = Minimum::new();
        for idx_he in 0..self.hes.len() as i64 {
            let s_next = self.hes.get(idx_he + 1).get_s();
            let v_next = self.hes.get(idx_he + 1).get_vertex().pos;
            let dist = self
                .hes
                .get(idx_he)
                .get_distance_to_edge(s_next, v_next, pointer.pos());
            if let Some((dist, pos)) = dist {
                if dist < Self::GRAB_RADIUS {
                    minimum.update(dist, idx_he, pos);
                }
            }
        }
        minimum.get_min()
    }
    fn near_corner(&self, pointer: &mut Pointer) -> Option<(f64, i64, Vec2)> {
        let mut minimum = Minimum::new();
        for idx_he in 0..self.hes.len() as i64 {
            let dist = self.hes.get(idx_he).get_distance_to_corner(pointer.pos());
            if let Some((dist, pos)) = dist {
                if dist < Self::GRAB_RADIUS {
                    minimum.update(dist, idx_he, pos);
                }
            }
        }
        minimum.get_min()
    }
    fn clear_selections(&mut self, hs: HS) {
        self.hes.iter_mut().for_each(|he| {
            he.set_vertex_state(hs, false);
            he.set_corner_state(hs, false);
            he.set_edge_state(hs, false);
        });
        self.state.set_hs(hs, false);
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
        // Update the polygon
        self.segs = calc_segs(self.to_path(Self::TOLERANCE));
        self.polygon = calc_polygon(&self.segs);
    }

    pub fn change_polygon_wedge_or_edge(&mut self, keys_states: &KeysStates) {
        use HS::*;
        for idx_he in 0..self.get_hes_len() as i64 {
            let he = self.get_he_mut(idx_he);
            // A. Change first primitive selected and break if found
            if he.get_edge_state(Select) {
                if keys_states.shift_pressed {
                    he.edge_next_kind();
                } else {
                    he.edge_prev_kind();
                }
                break;
            }
            // B. Change first corner selected and break if found
            if he.get_corner_state(Select) || he.get_vertex_state(Select) {
                if keys_states.shift_pressed {
                    he.corner_next_kind();
                } else {
                    he.corner_prev_kind();
                }
                break;
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
    const GRAB_RADIUS: f64 = 5.;
    type Kindvars = ShapeKind;

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
    fn get_vars(&self) -> ShapeKind {
        ShapeKind::KindPolygon(ShapePolygon {
            hes_prim: self.hes_prim.clone(),
            hes: self.hes.clone(),
            state: Status::default(),
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
    fn set_vars(&mut self, shape_kind: &ShapeKind) {
        if let ShapeKind::KindPolygon(shape_polygon) = shape_kind {
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
                false
            }
        }
    }
    fn set_state(&mut self, set: SetEntityState) {
        use SetEntityState::*;
        match set {
            SetHS(hs, value) => self.state.set_hs(hs, value),
            SetAllControlsHS(hs, state) => {
                self.hes
                    .iter_mut()
                    .for_each(|he| he.set_vertex_state(hs, state));
                self.hes.iter_mut().for_each(|he| {
                    he.set_corner_state(hs, state);
                    he.set_edge_state(hs, state);
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
                // 1. Check for the edges
                if let Some((_, idx, pos)) = self.near_edge(pointer) {
                    if !keys_states.alt_pressed {
                        pointer.set_pos(pos);
                        pointer.set_magnetized(true);
                    }
                    self.hes.get_mut(idx as i64).set_edge_state(hs, true);
                    return true;
                };
                // 2. Check for the apices
                if let Some((_, idx, pos)) = self.near_corner(pointer) {
                    if !keys_states.alt_pressed {
                        pointer.set_pos(pos);
                        pointer.set_magnetized(true);
                    }
                    self.hes.get_mut(idx as i64).set_corner_state(hs, true);
                    return true;
                }
                // 3. Check for the vertices
                if let Some((_, idx, pos)) = self.near_vertex(pointer) {
                    if !keys_states.alt_pressed {
                        pointer.set_pos(pos);
                        pointer.set_magnetized(true);
                    }
                    self.hes.get_mut(idx as i64).set_vertex_state(hs, true);
                    return true;
                };
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
        self.update_all();
        true
    }
    fn move_controls(&mut self, pointer: &Pointer, _keys_states: KeysStates) -> bool {
        use HS::*;
        let snap = pointer.get_snap().val();
        // 1. Check vertices, move the first selected found and return
        for idx_he in 0..self.hes.len() as i64 {
            if self.hes.get(idx_he).get_vertex_state(Select) {
                // We need prev and next vertices for the snapping
                let v_prev = self.hes.get(idx_he - 1).get_vertex().pos;
                let v_next = self.hes.get(idx_he + 1).get_vertex().pos;
                let v_saved = self.hes.get(idx_he).get_vertex().saved_pos;

                // Try move the vertex (None if snapping not found)
                let o_v_new = if pointer.is_magnetized() {
                    Some(pointer.pos())
                } else {
                    move_vertex_with_snapping(v_prev, v_saved, v_next, pointer.dpos(), snap)
                };
                if let Some(v_new) = o_v_new {
                    self.hes.get_mut(idx_he).set_vertex_pos(v_new);
                    self.update_all();
                    return true;
                }
                return false;
            }
        }

        // 2. Move apices (chamfer/fillet): move the first control selected found and return
        for idx_he in 0..self.hes.len() as i64 {
            if self.hes.get(idx_he).get_corner_state(Select) {
                self.hes.get_mut(idx_he).move_corner(pointer.dpos());
                self.update_all();
                return true;
            }
        }

        // 3. Move edges (third point): move the first control selected found and return
        for idx_he in 0..self.hes.len() as i64 {
            if self.hes.get(idx_he).get_edge_state(Select) {
                let v_next = self.hes.get(idx_he + 1).get_vertex().pos;
                self.hes.get_mut(idx_he).move_edge(v_next, pointer.dpos());
                self.update_all();
                return true;
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
            if he.is_vertex_editable() {
                paths_patterns.push((
                    modifiers_path(he.get_vertex().pos, scale, Self::GRAB),
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
            center_path(self.get_centroid(), scale, Self::GRAB),
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
    fn get_dimensions_paths_and_patterns(
        &self,
        _size: &Size,
        _cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        // use HS::*;
        let mut _res = vec![];
        // for he_idx in 0..self.hes.len() as i64 {
        //     let edge = self.hes.get(he_idx).get_edge();
        //     let start_corner = self.hes.get(he_idx).get_wedge().get_corner().pos;
        //     let end_corner = self.hes.get(he_idx + 1).get_wedge().get_corner().pos;
        //     let selected = edge.get_state(Select).is_some() || self.state.is_hs(Select);
        //     let highlighted = edge.get_state(Highlight).is_some() || self.state.is_hs(Highlight);
        //     if selected || highlighted {
        //         vec![Dimension::new(DimKind::Linear, start_apex, end_apex, 0.)
        //             .get_path_and_pattern()];
        //         let dim = edge.get_dimensions_paths_and_patterns(start_corner, end_corner, size);
        //         res.extend(dim);
        //     }
        // }
        _res
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
