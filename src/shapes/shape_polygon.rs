use super::shapes::ShapeKind;
use crate::{
    canvas::{CanvasText, Pattern},
    curves::{
        curves::CurveControls,
        from_dihedron::{CurveFromDihedron, Dihedron},
        from_segment::{CurveFromSegment, Segment},
    },
    math::*,
    pools::HS,
    positions::{HalfEdge, HalfEdgeElement, HalfEdgeProperty, Status},
    prefab::*,
    GetEntityState, KeysStates, ObjectsFuncs, Pointer, SetEntityState, SetEntityStateFromPos,
};
use geo::{LineString, Polygon};
use kurbo::{BezPath, PathEl, Point, Rect, Shape, Size, Vec2};
use std::fmt::Display;

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
    he_property: HalfEdgeProperty,
    state: Status,
    segs: BezPath,
    polygon: Polygon<f64>,
}

impl ShapePolygon {
    const GRAB: f64 = 5.;
    pub const MIN_RECT_SIZE: f64 = 10.;

    fn vec_to_he(vec: VecRing<Vec2>) -> Option<VecRing<HalfEdge>> {
        // First value is a dummy that we replace in the for loop
        let dum_dihedron = CurveFromDihedron::new(Dihedron::from_three_points(
            Vec2::new(0., -1.),
            Vec2::new(0., 0.),
            Vec2::new(1., 0.),
        )?)?;
        let dum_edge =
            CurveFromSegment::new(Segment::new(Vec2::new(-1., -1.), Vec2::new(0., 0.))?)?;
        let mut hes = VecRing::from_element(HalfEdge::new(
            // Vertex::new(Vec2::ZERO),
            dum_dihedron,
            dum_edge,
        ));

        for i in 0..vec.len() as i64 {
            let p_prev = *vec.get(i - 1);
            let p = *vec.get(i);
            let p_next = *vec.get(i + 1);
            let dih = CurveFromDihedron::new(Dihedron::from_three_points(p_prev, p, p_next)?)?;
            let seg = CurveFromSegment::new(Segment::new(p, p_next)?)?;
            // let v = Vertex::new(p);
            let he = HalfEdge::new(dih, seg);
            if i == 0 {
                hes.replace_first(he);
            } else {
                hes.push(he);
            }
        }
        Some(hes)
    }
    pub fn new_rectangle(start: Vec2, end: Vec2) -> Option<ShapeKind> {
        use ShapeKind::*;
        log!("new_rectangle");
        let tl = Vec2::new(start.x, start.y);
        let tr = Vec2::new(end.x, start.y);
        let br = Vec2::new(end.x, end.y);
        let bl = Vec2::new(start.x, end.y);
        let mut points = VecRing::from_element(tl);
        points.push(tr);
        points.push(br);
        points.push(bl);

        // Alwas counter clockwize to have a positive area
        if area_from_points(&points) < 0. {
            points.vec.reverse();
        }
        let hes = ShapePolygon::vec_to_he(points)?;

        let mut shape_rectangle = ShapePolygon {
            hes,
            he_property: HalfEdgeProperty::RectangleLike,
            state: Status::default(),
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        };
        log!("area: {}", shape_rectangle.area());
        shape_rectangle.update_geo_polygon();
        Some(KindRectangle(shape_rectangle))
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
            he_property: HalfEdgeProperty::General,
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
    pub fn magnet_to_point(&self, pointer: &mut Pointer, keys_states: KeysStates) -> bool {
        use HS::*;
        if !keys_states.alt_pressed {
            // Check for the curves
            for he in self.hes.iter() {
                let curve = he.get_edge();
                if curve.get_state(Select).is_none() {
                    if let Some((dist, pos)) = curve.get_dist_from_pos(pointer.pos()) {
                        if dist < Self::GRAB_RADIUS {
                            pointer.set_pos(pos);
                            pointer.set_magnetized(true);
                            return true;
                        }
                    }
                }
            }
            // Check for vertices
            for he in self.hes.iter() {
                let v = he.get_wedge().get_apex().pos;
                if (pointer.pos() - v).hypot() < Self::GRAB_RADIUS {
                    pointer.set_pos(v);
                    pointer.set_magnetized(true);
                    return true;
                }
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
    pub fn update_adjacent_wedges(&mut self, idx_he: i64) {
        log!("update_adjacent_wedges");
        let apex_prev_prev = self.hes.get(idx_he - 2).get_wedge().get_apex().pos;
        let apex_prev = self.hes.get(idx_he - 1).get_wedge().get_apex().pos;
        let apex = self.hes.get(idx_he).get_wedge().get_apex().pos;
        let apex_next = self.hes.get(idx_he + 1).get_wedge().get_apex().pos;
        let apex_next_next = self.hes.get(idx_he + 2).get_wedge().get_apex().pos;

        self.hes
            .get_mut(idx_he - 1)
            .get_wedge_mut()
            .update_from_apices(apex_prev_prev, apex);

        self.hes
            .get_mut(idx_he + 1)
            .get_wedge_mut()
            .update_from_apices(apex, apex_next_next);

        // let edge_start = self.hes.get(idx_he - 1).get_wedge().get_end();
        // let edge_end = self.hes.get(idx_he).get_wedge().get_start();
        // Segment::new(edge_start, edge_end).and_then(|seg| {
        //     self.hes
        //         .get_mut(idx_he - 1)
        //         .get_edge_mut()
        //         .update_from_segment(&seg)
        // });

        // let edge_start = self.hes.get(idx_he).get_wedge().get_end();
        // let edge_end = self.hes.get(idx_he + 1).get_wedge().get_start();
        // Segment::new(edge_start, edge_end).and_then(|seg| {
        //     self.hes
        //         .get_mut(idx_he)
        //         .get_edge_mut()
        //         .update_from_segment(&seg)
        // });
        // self.update_geo_polygon();
    }
    pub fn update_he_edges(&mut self) {
        for idx_he in 0..self.hes.len() as i64 {
            let edge_start = self.hes.get(idx_he).get_wedge().get_end();
            let edge_end = self.hes.get(idx_he + 1).get_wedge().get_start();
            Segment::new(edge_start, edge_end).and_then(|seg| {
                self.hes
                    .get_mut(idx_he)
                    .get_edge_mut()
                    .update_from_segment(&seg)
            });
        }
        self.update_geo_polygon();
    }

    fn update_geo_polygon(&mut self) {
        self.segs = calc_segs(self.to_path(Self::TOLERANCE));
        self.polygon = calc_polygon(&self.segs);
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
            he_property: self.he_property,
            state: Status::default(),
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
    fn set_vars(&mut self, shape_kind: &ShapeKind) {
        if let ShapeKind::KindPolygon(shape_polygon) = shape_kind {
            self.hes = shape_polygon.hes.clone();
            self.he_property = shape_polygon.he_property;
            self.update_he_edges();
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
        use HalfEdgeElement::*;
        use SetEntityStateFromPos::*;

        // A candidate element that we might want to select.
        #[derive(Debug)]
        struct Elem {
            he_elem: HalfEdgeElement,
            idx: usize,
            dist: f64, // we'll convert Option<f64> to a f64 by filtering out invalid cases
        }
        impl Elem {
            const GRAB_RADIUS: f64 = ShapePolygon::GRAB_RADIUS;

            fn new(he_elem: HalfEdgeElement, idx: usize, dist: Option<f64>) -> Option<Self> {
                // Only create a candidate if a distance exists and is within our threshold.
                dist.filter(|&d| d < Self::grab_threshold()).map(|d| Self {
                    he_elem,
                    idx,
                    dist: d,
                })
            }
            fn grab_threshold() -> f64 {
                Self::GRAB_RADIUS
            }
        }

        match set {
            SetHSFromPos(hs) => {
                self.state
                    .set_hs(hs, self.contains(pointer.pos().to_point()));
            }
            SetControlHSFromPos(hs) => {
                // Clear all selections
                self.hes.iter_mut().for_each(|he| {
                    he.get_wedge_mut().set_state(hs, false);
                    he.get_edge_mut().set_state(hs, false);
                });

                // Build an iterator of candidate elements for all half-edges.
                let candidate = self
                    .hes
                    .iter()
                    .enumerate()
                    .flat_map(|(he_idx, he)| {
                        // Create candidates for each element type.
                        let mut elems = Vec::with_capacity(3);

                        // Apex candidate.
                        if let Some(elem) = Elem::new(
                            Apex,
                            he_idx,
                            Some((he.get_wedge().get_apex().pos - pointer.pos()).hypot()),
                        ) {
                            elems.push(elem);
                        }

                        // Wedge candidate.
                        if let Some(elem) = Elem::new(
                            Dihedron,
                            he_idx,
                            he.get_wedge()
                                .get_dist_from_pos(pointer.pos())
                                .and_then(|(dist, _)| Some(dist)),
                        ) {
                            elems.push(elem);
                        }

                        // Edge candidate.
                        if let Some(elem) = Elem::new(
                            Edge,
                            he_idx,
                            he.get_edge()
                                .get_dist_from_pos(pointer.pos())
                                .map(|(dist, _)| dist),
                        ) {
                            elems.push(elem);
                        }

                        elems.into_iter()
                    })
                    // Select the candidate with the smallest distance.
                    .min_by(|a, b| {
                        a.dist
                            .partial_cmp(&b.dist)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });

                if let Some(nearest) = candidate {
                    match nearest.he_elem {
                        Apex => {
                            self.hes
                                .get_mut(nearest.idx as i64)
                                .get_wedge_mut()
                                .set_apex_state(hs, true);
                        }
                        Dihedron => {
                            self.hes
                                .get_mut(nearest.idx as i64)
                                .get_wedge_mut()
                                .set_state(hs, true);
                        }
                        Edge => {
                            self.hes
                                .get_mut(nearest.idx as i64)
                                .get_edge_mut()
                                .set_state(hs, true);
                        }
                    }
                }
            }
        }
    }

    fn toggle_selected_prop(&mut self) {
        for idx_he in 0..self.hes.len() as i64 {
            let wedge = self.hes.get_mut(idx_he).get_wedge_mut();
            if wedge.get_state(HS::Select).is_some() {
                wedge.toggle_prop();
                self.update_adjacent_wedges(idx_he);
                return;
            }
        }
    }
    fn move_position(&mut self, pointer: &mut Pointer, _keys_states: KeysStates) -> bool {
        self.hes
            .iter_mut()
            .for_each(|he| he.get_wedge_mut().get_apex_mut().move_pos(pointer.dpos()));
        self.update_he_edges();
        true
    }
    fn move_controls(&mut self, pointer: &Pointer, keys_states: KeysStates) -> bool {
        use HalfEdgeProperty::*;
        use HS::*;
        let snap = pointer.get_snap().val();
        let dpos = pointer.dpos();

        // Move apices
        match self.he_property {
            General => {
                log!("Move apex");
                // Check apices, move the first selected found and return
                for idx_he in 0..self.hes.len() as i64 {
                    if self
                        .hes
                        .get(idx_he)
                        .get_wedge()
                        .get_apex_state(Select)
                        .is_some()
                    {
                        let p_prev = *self.hes.get(idx_he - 1).get_wedge().get_apex();
                        let p_next = *self.hes.get(idx_he + 1).get_wedge().get_apex();
                        let apex = self.hes.get_mut(idx_he).get_wedge_mut().get_apex_mut();
                        // Move the vertex
                        if pointer.is_magnetized() {
                            apex.pos = pointer.pos();
                        } else {
                            apex.pos = move_apex_with_snapping(
                                p_prev.pos,
                                apex.saved_pos,
                                p_next.pos,
                                dpos,
                                snap,
                            );
                        }
                        self.hes
                            .get_mut(idx_he)
                            .get_wedge_mut()
                            .update_from_apices(p_prev.pos, p_next.pos);
                        self.update_adjacent_wedges(idx_he);
                        self.update_he_edges();
                        return true;
                    }
                }
            }
            RectangleLike => {
                // Check vertices, move the first selected vertex found, move adjacent vertices and return
                for he_idx in 0..self.hes.len() as i64 {
                    let wedge = *self.hes.get(he_idx).get_wedge();
                    if wedge.get_apex_state(Select).is_some() {
                        let prev_a = *self.hes.get(he_idx - 1).get_wedge().get_apex();
                        let next_a = *self.hes.get(he_idx + 1).get_wedge().get_apex();
                        // Projection of dpos on the previous edge
                        let mut dpos_proj_prev =
                            project_on_vec(prev_a.saved_pos, wedge.get_apex().saved_pos, dpos);
                        let mut dpos_proj_next =
                            project_on_vec(next_a.saved_pos, wedge.get_apex().saved_pos, dpos);

                        if !pointer.is_magnetized() {
                            let prev_rel = wedge.get_apex().saved_pos - prev_a.saved_pos;
                            dpos_proj_prev = snap_pt(prev_rel + dpos_proj_prev, snap) - prev_rel;

                            let next_rel = wedge.get_apex().saved_pos - next_a.saved_pos;
                            dpos_proj_next = snap_pt(next_rel + dpos_proj_next, snap) - next_rel;
                        }

                        if (wedge.get_apex().pos + dpos_proj_prev - prev_a.pos).hypot()
                            < Self::MIN_RECT_SIZE
                            || (wedge.get_apex().pos + dpos_proj_next - next_a.pos).hypot()
                                < Self::MIN_RECT_SIZE
                        {
                            log!("Too small");
                            return false;
                        }
                        // Move the vertices
                        self.hes
                            .get_mut(he_idx - 1)
                            .get_wedge_mut()
                            .get_apex_mut()
                            .move_pos(dpos_proj_next);
                        self.hes
                            .get_mut(he_idx + 1)
                            .get_wedge_mut()
                            .get_apex_mut()
                            .move_pos(dpos_proj_prev);
                        self.hes
                            .get_mut(he_idx)
                            .get_wedge_mut()
                            .get_apex_mut()
                            .move_pos(dpos_proj_prev + dpos_proj_next);
                        self.update_he_edges();
                        return true;
                    }
                }
            }
        }

        // Move dihedron: move the first control selected found and return
        log!("Move wedge control");
        for idx_he in 0..self.hes.len() as i64 {
            let apex = *self.hes.get(idx_he).get_wedge().get_apex();
            let next_apex = *self.hes.get(idx_he + 1).get_wedge().get_apex();
            let wedge = self.hes.get_mut(idx_he).get_wedge_mut();
            if wedge.move_control_selected(apex.pos, next_apex.pos, pointer, keys_states) {
                self.update_he_edges();
                self.update_geo_polygon();
                return true;
            }
        }

        // Move edges: move the first control selected found and return
        log!("Move edge control");
        for idx_he in 0..self.hes.len() as i64 {
            let apex = *self.hes.get(idx_he).get_wedge().get_apex();
            let next_apex = *self.hes.get(idx_he + 1).get_wedge().get_apex();
            let edge = self.hes.get_mut(idx_he).get_edge_mut();
            if edge.move_control_selected(apex.pos, next_apex.pos, pointer, keys_states) {
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
            paths_patterns.push((
                modifiers_path(wedge.get_apex().pos, scale, Self::GRAB),
                modifiers_pattern(
                    wedge.get_apex_state(Select).is_some(),
                    wedge.get_apex_state(Highlight).is_some(),
                ),
            ));

            // DEBUG
            let tp = he.get_edge().get_third_pt();
            paths_patterns.push((
                modifiers_path(tp, scale, Self::GRAB),
                modifiers_pattern(false, false),
            ));
        }

        // Polygon center
        paths_patterns.push((
            center_path(self.get_centroid(), scale, Self::GRAB),
            modifiers_pattern(self.state.is_hs(Select), self.state.is_hs(Highlight)),
        ));
        paths_patterns
    }
    fn get_paths_and_patterns(&self, das: &Size, _: (Rect, f64, Vec2)) -> Vec<(BezPath, Pattern)> {
        use HS::*;
        let mut paths_patterns = vec![];

        for idx in 0..self.hes.len() as i64 {
            let he = self.hes.get(idx);
            // Polygon dihedron
            paths_patterns.push(he.get_wedge().get_paths_and_patterns(
                das,
                self.state.is_hs(Select),
                self.state.is_hs(Highlight),
            ));

            // Polygon edges
            paths_patterns.push(he.get_edge().get_paths_and_patterns(
                das,
                self.state.is_hs(Select),
                self.state.is_hs(Highlight),
            ));
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
        for he in self.hes.iter() {
            let p = he.get_edge();
            let selected = p.get_state(Select).is_some() || self.state.is_hs(Select);
            let highlighted = p.get_state(Highlight).is_some() || self.state.is_hs(Highlight);
            if selected || highlighted {
                let dim = p.get_dimensions_paths_and_patterns(size);
                res.extend(dim);
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
