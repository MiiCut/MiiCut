use super::shapes::ShapeKind;
use crate::{
    canvas::{CanvasText, Pattern},
    curves::{curves::CurveControls, curves_edge::Edge, curves_wedge::CurveWedge},
    math::*,
    pools::HS,
    positions::{HalfEdge, Minimum, Status},
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
    hes: VecRing<HalfEdge>,
    state: Status,
    segs: BezPath,
    polygon: Polygon<f64>,
}

impl ShapePolygon {
    const GRAB: f64 = 5.;
    pub const MIN_RECT_SIZE: f64 = 10.;

    fn vec_to_he(vec: VecRing<Vec2>) -> Option<VecRing<HalfEdge>> {
        // First value is a dummy that we replace in the for loop
        let dum_edge = Edge::new(Vec2::new(-1., -1.), Vec2::new(0., 0.));
        let dum_wedge = CurveWedge::new(Vec2::ZERO);
        let mut hes = VecRing::from_element(HalfEdge::new(dum_wedge, dum_edge));

        for i in 0..vec.len() as i64 {
            let apex = *vec.get(i);
            let apex_next = *vec.get(i + 1);
            let he = HalfEdge::new(CurveWedge::new(apex), Edge::new(apex, apex_next));
            if i == 0 {
                hes.replace_first(he);
            } else {
                hes.push(he);
            }
        }
        Some(hes)
    }
    pub fn new_polygon(mut points: VecRing<Vec2>) -> Option<ShapeKind> {
        use ShapeKind::*;
        log!("new_polygon");
        // Alwas counter clockwize to have a positive area
        if area_from_points(&points) < 0. {
            points.vec.reverse();
        }
        let hes = ShapePolygon::vec_to_he(points)?;
        let mut shape_polygon = ShapePolygon {
            hes,
            state: Status::default(),
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        };
        log!("area: {}", shape_polygon.area());
        shape_polygon.update_geo_polygon();
        Some(KindPolygon(shape_polygon))
    }
    pub fn get_hes_len(&self) -> usize {
        self.hes.len()
    }
    pub fn get_he(&self, idx: i64) -> &HalfEdge {
        self.hes.get(idx)
    }
    pub fn get_he_mut(&mut self, idx: i64) -> &mut HalfEdge {
        self.hes.get_mut(idx)
    }

    fn near_apex(&self, pointer: &mut Pointer) -> Option<(f64, i64, Vec2)> {
        let mut minimum = Minimum::new();
        for idx_he in 0..self.hes.len() as i64 {
            let apex = self.hes.get(idx_he).get_wedge().get_apex().pos;
            let dist = (apex - pointer.pos()).hypot();
            if dist < Self::GRAB_RADIUS {
                minimum.update(dist, idx_he, apex);
            }
        }
        minimum.get_min()
    }
    fn near_edge(&self, pointer: &mut Pointer) -> Option<(f64, i64, Vec2)> {
        let mut minimum = Minimum::new();
        for idx_he in 0..self.hes.len() as i64 {
            let dist = self
                .hes
                .get(idx_he)
                .get_edge()
                .get_dist_from_pos(pointer.pos());
            if let Some((dist, pos)) = dist {
                if dist < Self::GRAB_RADIUS {
                    minimum.update(dist, idx_he, pos);
                }
            }
        }
        minimum.get_min()
    }
    fn near_wedge(&self, pointer: &mut Pointer) -> Option<(f64, i64, Vec2)> {
        let mut minimum = Minimum::new();
        for idx_he in 0..self.hes.len() as i64 {
            let edge_prev = self.hes.get(idx_he - 1).get_edge();
            let edge_next = self.hes.get(idx_he).get_edge();
            let dist = self.hes.get(idx_he).get_wedge().get_dist_from_pos(
                edge_prev,
                edge_next,
                pointer.pos(),
            );
            if let Some((dist, pos)) = dist {
                if dist < Self::GRAB_RADIUS {
                    minimum.update(dist, idx_he, pos);
                }
            }
        }
        minimum.get_min()
    }

    pub fn magnet_to_point(&self, pointer: &mut Pointer, keys_states: KeysStates) -> bool {
        if !keys_states.alt_pressed {
            // Check for the edges
            if let Some((.., pos)) = self.near_edge(pointer) {
                log!("Magnet to edge");
                pointer.set_pos(pos);
                pointer.set_magnetized(true);
                return true;
            }
            // Check for the wedges
            if let Some((.., pos)) = self.near_wedge(pointer) {
                log!("Magnet to wedge");
                pointer.set_pos(pos);
                pointer.set_magnetized(true);
                return true;
            }
            // Check for apices
            if let Some((.., pos)) = self.near_apex(pointer) {
                log!("Magnet to apex");
                pointer.set_pos(pos);
                pointer.set_magnetized(true);
                return true;
            }
            // Check also for polygon center
            let centroid = self.get_centroid();
            if (pointer.pos() - centroid).hypot() < Self::GRAB_RADIUS {
                pointer.set_pos(centroid);
                pointer.set_magnetized(true);
                return true;
            }
        }
        false
    }
    pub fn get_polygon(&self) -> Polygon<f64> {
        self.polygon.clone()
    }

    pub fn update_geo_polygon(&mut self) {
        self.segs = calc_segs(self.to_path(Self::TOLERANCE));
        self.polygon = calc_polygon(&self.segs);
    }
    pub fn change_polygon_wedge_or_edge(&mut self, keys_states: &KeysStates) {
        use HS::*;
        for idx_he in 0..self.get_hes_len() as i64 {
            let he = self.get_he_mut(idx_he);
            // A. Change first primitive selected and break if found
            let edge = he.get_edge_mut();
            if edge.get_state(Select).is_some() {
                if keys_states.shift_pressed {
                    edge.next();
                } else {
                    edge.prev();
                }
                break;
            }
            // B. Change first apex selected and break if found
            if he.get_wedge().get_apex_state(Select).is_some()
                || he.get_wedge().get_state(Select).is_some()
            {
                if keys_states.shift_pressed {
                    he.next_wedge_curve();
                } else {
                    he.prev_wedge_curve();
                }
                break;
            }
        }
        log!("change_polygon_wedge_or_edge");
        self.update_geo_polygon();
    }
    fn get_centroid(&self) -> Vec2 {
        let mut sum = Vec2::ZERO;
        for he in self.hes.iter() {
            sum += he.get_wedge().get_apex().pos;
        }
        sum / self.hes.len() as f64
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
            let vi = self.hes.get(idx).get_wedge().get_apex().pos;
            let vj = self
                .hes
                .get((idx + 1) % len as i64)
                .get_wedge()
                .get_apex()
                .pos;
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
        for v_pos in self.hes.iter().map(|he| he.get_wedge().get_apex().pos) {
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
    }
    fn restore_vars(&mut self) {
        for he in self.hes.iter_mut() {
            he.restore_vars();
        }
    }
    fn get_vars(&self) -> ShapeKind {
        ShapeKind::KindPolygon(ShapePolygon {
            hes: self.hes.clone(),
            state: Status::default(),
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
    fn set_vars(&mut self, shape_kind: &ShapeKind) {
        if let ShapeKind::KindPolygon(shape_polygon) = shape_kind {
            self.hes = shape_polygon.hes.clone();
        }
    }

    fn get_state(&self, get: GetEntityState) -> Option<Vec2> {
        use GetEntityState::*;
        match get {
            IsHS(hs) => self.state.is_hs(hs).then(|| self.get_position()),
            GetFirstControlHS(hs) => self
                .hes
                .iter()
                .find_map(|he| he.get_wedge().get_apex_state(hs))
                .or_else(|| self.hes.iter().find_map(|he| he.get_wedge().get_state(hs)))
                .or_else(|| self.hes.iter().find_map(|he| he.get_edge().get_state(hs))),
        }
    }
    fn set_state(&mut self, set: SetEntityState) {
        use SetEntityState::*;
        match set {
            SetHS(hs, value) => self.state.set_hs(hs, value),
            SetAllControlsHS(hs, state) => {
                self.hes
                    .iter_mut()
                    .for_each(|he| he.get_edge_mut().set_state(hs, state));
                self.hes.iter_mut().for_each(|he| {
                    he.get_wedge_mut().set_state(hs, state);
                    he.get_wedge_mut().set_apex_state(hs, state)
                });
            }
        }
    }
    fn set_state_from_pos(
        &mut self,
        pointer: &mut Pointer,
        _keys_states: KeysStates,
        set: SetEntityStateFromPos,
    ) {
        use SetEntityStateFromPos::*;
        match set {
            SetHSFromPos(hs) => {
                self.state
                    .set_hs(hs, self.contains(pointer.pos().to_point()));
            }
            SetControlHSFromPos(hs) => {
                // Clear all selections
                self.hes.iter_mut().for_each(|he| {
                    he.get_wedge_mut().set_state(hs, false);
                    he.get_wedge_mut().set_apex_state(hs, false);
                    he.get_edge_mut().set_state(hs, false);
                });
                // 1. Check for the edges
                log!("hs to edge");
                if let Some((_, idx, _)) = self.near_edge(pointer) {
                    self.hes
                        .get_mut(idx as i64)
                        .get_edge_mut()
                        .set_state(hs, true);
                    return;
                };
                // 2. Check for the wedges
                log!("hs to wedge");
                if let Some((_, idx, _)) = self.near_wedge(pointer) {
                    self.hes
                        .get_mut(idx as i64)
                        .get_wedge_mut()
                        .set_state(hs, true);
                    return;
                };
                // 3. Check for the apices
                log!("hs to apex");
                if let Some((_, idx, _)) = self.near_apex(pointer) {
                    self.hes
                        .get_mut(idx as i64)
                        .get_wedge_mut()
                        .set_apex_state(hs, true);
                    return;
                };
            }
        }
    }

    fn move_position(&mut self, pointer: &mut Pointer, _keys_states: KeysStates) -> bool {
        for idx_he in 0..self.hes.len() as i64 {
            // Move the wedge apex
            self.hes
                .get_mut(idx_he)
                .get_wedge_mut()
                .move_apex(pointer.dpos());
            let apex = self.hes.get(idx_he).get_wedge().get_apex().pos;
            // Update the adjacent edges
            self.hes
                .get_mut(idx_he - 1)
                .get_edge_mut()
                .try_set_end(apex);
            self.hes.get_mut(idx_he).get_edge_mut().try_set_start(apex);
        }
        self.update_geo_polygon();
        true
    }
    fn move_controls(&mut self, pointer: &Pointer, keys_states: KeysStates) -> bool {
        use HS::*;
        let snap = pointer.get_snap().val();

        // Check apices, move the first selected found and return
        for idx_he in 0..self.hes.len() as i64 {
            if self
                .hes
                .get(idx_he)
                .get_wedge()
                .get_apex_state(Select)
                .is_some()
            {
                let apex_prev = self.hes.get(idx_he - 1).get_wedge().get_apex().pos;
                let apex_next = self.hes.get(idx_he + 1).get_wedge().get_apex().pos;
                let saved_apex = self.hes.get(idx_he).get_wedge().get_apex().saved_pos;

                // Try move the wedge apex (None if snapping not found)
                let o_apex = if pointer.is_magnetized() {
                    Some(pointer.pos())
                } else {
                    move_apex_with_snapping(apex_prev, saved_apex, apex_next, pointer.dpos(), snap)
                };

                if let Some(apex) = o_apex {
                    // We set the new apex position only if leads to valid edges
                    if self
                        .hes
                        .get_mut(idx_he - 1)
                        .get_edge_mut()
                        .try_set_end(apex)
                    {
                        if self.hes.get_mut(idx_he).get_edge_mut().try_set_start(apex) {
                            // All good, update wedge apex
                            self.hes.get_mut(idx_he).get_wedge_mut().set_apex(apex);
                            self.update_geo_polygon();
                            return true;
                        } else {
                            // Cancel the edge_prev update
                            self.hes
                                .get_mut(idx_he - 1)
                                .get_edge_mut()
                                .try_set_end(saved_apex);
                        }
                    }
                }
                return false;
            }
        }

        // Move control on wedge: move the first control selected found and return
        for idx_he in 0..self.hes.len() as i64 {
            let edge_prev = *self.hes.get(idx_he - 1).get_edge();
            let edge_next = *self.hes.get(idx_he).get_edge();
            let wedge = self.hes.get_mut(idx_he).get_wedge_mut();
            if wedge.move_control_selected(&edge_prev, &edge_next, pointer, keys_states) {
                self.update_geo_polygon();
                return true;
            }
        }

        // Move edges: move the first control selected found and return
        for idx_he in 0..self.hes.len() as i64 {
            let edge = self.hes.get_mut(idx_he).get_edge_mut();
            if edge.move_control_selected(pointer, keys_states) {
                self.update_geo_polygon();
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

        // Polygon vertices
        for he in self.hes.iter() {
            let wedge = he.get_wedge();

            // APICES
            paths_patterns.push((
                modifiers_path(wedge.get_apex().pos, scale, Self::GRAB),
                modifiers_pattern(
                    wedge.get_apex_state(Select).is_some(),
                    wedge.get_apex_state(Highlight).is_some(),
                ),
            ));

            // DEBUG
            if let Some(third_pt) = he
                .get_edge()
                .get_seg_info()
                .and_then(|seg| Some(seg.third_pt()))
            {
                // log!("third_pt: {:?}", third_pt);
                paths_patterns.push((
                    modifiers_path(third_pt, scale, Self::GRAB),
                    modifiers_pattern(false, false),
                ));
            }
        }

        // Polygon center
        paths_patterns.push((
            center_path(self.get_centroid(), scale, Self::GRAB),
            modifiers_pattern(self.state.is_hs(Select), self.state.is_hs(Highlight)),
        ));
        paths_patterns
    }

    fn get_paths_and_patterns(&self, _: &Size, _: (Rect, f64, Vec2)) -> Vec<(BezPath, Pattern)> {
        use HS::*;
        let mut wedges_infos = VecRing::from_element(None);
        for idx_he in 0..self.hes.len() as i64 {
            let edge_prev = *self.hes.get(idx_he - 1).get_edge();
            let edge_next = *self.hes.get(idx_he).get_edge();
            let wedge_fillet = self
                .hes
                .get(idx_he)
                .get_wedge()
                .get_fillet(&edge_prev, &edge_next);
            if idx_he == 0 {
                wedges_infos.replace_first(wedge_fillet);
            } else {
                wedges_infos.push(wedge_fillet);
            }
        }

        let mut paths_patterns = vec![];
        for idx_he in 0..wedges_infos.len() as i64 {
            // WEDGES
            if let Some((center, start, end)) = wedges_infos.get(idx_he).clone() {
                paths_patterns.push(self.hes.get(idx_he).get_wedge().get_paths_and_patterns(
                    center,
                    start,
                    end,
                    self.state.is_hs(Select),
                    self.state.is_hs(Highlight),
                ));

                if let Some((_, start_next, end_next)) = wedges_infos.get(idx_he + 1).clone() {
                    // log!(
                    //     "start: ({:.2},{:.2}), end: ({:.2},{:.2}), start_next: ({:.2},{:.2}), end_next: ({:.2},{:.2})",
                    //     start.x,
                    //     start.y,
                    //     end.x,
                    //     end.y,
                    //     start_next.x,
                    //     start_next.y,
                    //     end_next.x,
                    //     end_next.y
                    // );
                    // EDGES
                    paths_patterns.push(self.hes.get(idx_he).get_edge().get_paths_and_patterns(
                        end,
                        start_next,
                        self.state.is_hs(Select),
                        self.state.is_hs(Highlight),
                    ));
                }
            }
        }
        paths_patterns
    }

    fn get_dimensions_paths_and_patterns(
        &self,
        size: &Size,
        _cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        use HS::*;
        let mut res = vec![];
        // for he_idx in 0..self.hes.len() as i64 {
        //     let edge = self.hes.get(he_idx).get_edge();
        //     let start_apex = self.hes.get(he_idx).get_wedge().get_apex().pos;
        //     let end_apex = self.hes.get(he_idx + 1).get_wedge().get_apex().pos;
        //     let selected = edge.get_state(Select).is_some() || self.state.is_hs(Select);
        //     let highlighted = edge.get_state(Highlight).is_some() || self.state.is_hs(Highlight);
        //     if selected || highlighted {
        //         let dim = edge.get_dimensions_paths_and_patterns(start_apex, end_apex, size);
        //         res.extend(dim);
        //     }
        // }
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
