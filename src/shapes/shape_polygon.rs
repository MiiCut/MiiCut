use super::shapes::ShapeKind;
use crate::{
    canvas::{CanvasText, Pattern},
    curves::curves::{Curve, CurveControls},
    math::*,
    pools::HS,
    positions::{HalfEdge, HalfEdgeElement, HalfEdgeProperty, Status, Vertex},
    prefab::*,
    GetEntityState, KeysStates, ObjectsFuncs, Pointer, SetEntityState, SetEntityStateFromPos,
};
use geo::{LineString, Polygon};
use kurbo::{BezPath, Line, PathEl, Point, Rect, Shape, Size, Vec2};
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
    on_creation: Option<Vertex>,
    state: Status,
    segs: BezPath,
    polygon: Polygon<f64>,
}

impl ShapePolygon {
    const GRAB: f64 = 5.;
    pub const MIN_RECT_SIZE: f64 = 10.;

    /// Start an empty polygon (but always with a defined half edge)
    pub fn with_first_half_edge(start_pos: Vec2, he_property: HalfEdgeProperty) -> ShapeKind {
        use HalfEdgeProperty::*;
        use ShapeKind::*;
        match he_property {
            General => {
                let v = Vertex::new(start_pos);
                let c = Curve::new_line();

                KindPolygon(ShapePolygon {
                    hes: VecRing::from_element(HalfEdge::new(v, c)),
                    he_property,
                    on_creation: Some(Vertex::new(start_pos)),
                    state: Status::default(),
                    segs: BezPath::new(),
                    polygon: Polygon::new(LineString::new(vec![]), vec![]),
                })
            }
            RectangleLike => {
                let end_pos = Vec2::new(start_pos.x + Self::GRAB, start_pos.y + Self::GRAB);

                let he1 = HalfEdge::new(
                    Vertex::new(Vec2::new(start_pos.x, start_pos.y)),
                    Curve::new_line(),
                );
                let he2 = HalfEdge::new(
                    Vertex::new(Vec2::new(end_pos.x, start_pos.y)),
                    Curve::new_line(),
                );
                let mut he3 = HalfEdge::new(
                    Vertex::new(Vec2::new(end_pos.x, end_pos.y)),
                    Curve::new_line(),
                );
                he3.get_vertex_mut().set_state(HS::Select, true);
                let he4 = HalfEdge::new(
                    Vertex::new(Vec2::new(start_pos.x, end_pos.y)),
                    Curve::new_line(),
                );
                let mut hes = VecRing::from_element(he1);
                hes.push(he2);
                hes.push(he3);
                hes.push(he4);

                KindRectangle(ShapePolygon {
                    hes,
                    he_property,
                    on_creation: Some(Vertex::new(Vec2::new(end_pos.x, end_pos.y))),
                    state: Status::default(),
                    segs: BezPath::new(),
                    polygon: Polygon::new(LineString::new(vec![]), vec![]),
                })
            }
        }
    }
    /// Add a new vertex at pos
    pub fn add_half_edge(&mut self, new_pos: Vec2) {
        if let Some(v) = self.on_creation.as_mut() {
            self.hes.push(HalfEdge::new(
                Vertex::new(v.get_pos().pos),
                Curve::new_line(),
            ));
            v.set_pos(new_pos);
            v.save_pos();
        }
        self.update_half_edges_vars_from_vertices();
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
    pub fn get_magnet_points(&self) -> Vec<Vec2> {
        let mut points = vec![];
        for he in self.hes.iter() {
            points.push(he.get_vertex().get_pos().pos);
        }
        points.push(self.get_centroid());
        points
    }
    pub fn get_polygon(&self) -> Polygon<f64> {
        self.polygon.clone()
    }
    pub fn update_half_edges_vars_from_vertices(&mut self) {
        for idx_he in 0..self.hes.len() as i64 {
            let s_prev = self.hes.get(idx_he - 1).get_vertex().get_pos().clone();
            let s = self.hes.get(idx_he).get_vertex().get_pos().clone();
            let s_next = self.hes.get(idx_he + 1).get_vertex().get_pos().clone();
            self.hes
                .get_mut(idx_he)
                .get_vertex_curve_mut()
                .update_vars(s_prev, s, s_next);
        }
        for idx_he in 0..self.hes.len() as i64 {
            let edge_start = self.hes.get(idx_he).get_vertex_curve().get_end();
            let edge_end = self.hes.get(idx_he + 1).get_vertex_curve().get_start();
            self.hes
                .get_mut(idx_he)
                .get_edge_mut()
                .set_from_start_end(edge_start, edge_end);
        }
        // update_geo_polygon
        self.segs = calc_segs(self.to_path(Self::TOLERANCE));
        self.polygon = calc_polygon(&self.segs);
    }

    fn get_centroid(&self) -> Vec2 {
        let mut sum = Vec2::ZERO;
        for he in self.hes.iter() {
            sum += he.get_vertex().get_pos().pos;
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
            let vi = self.hes.get(idx).get_vertex().get_pos().pos;
            let vj = self
                .hes
                .get((idx + 1) % len as i64)
                .get_vertex()
                .get_pos()
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
        for v_pos in self.hes.iter().map(|he| he.get_vertex().get_pos().pos) {
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
            on_creation: None,
            state: Status::default(),
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
    fn set_vars(&mut self, mii_shape_kind: &ShapeKind) {
        if let ShapeKind::KindPolygon(shape_polygon) = mii_shape_kind {
            self.hes = shape_polygon.hes.clone();
            self.he_property = shape_polygon.he_property;
            self.on_creation = None;
            self.update_half_edges_vars_from_vertices();
        }
    }
    fn good_size(&self) -> bool {
        match self.he_property {
            HalfEdgeProperty::General => {
                let len = self.hes.len();
                len > 2
            }
            HalfEdgeProperty::RectangleLike => {
                let len = self.hes.len();
                if len != 4 {
                    return false;
                }
                let start = self.hes.get(0).get_vertex().get_pos().pos;
                let end = self.hes.get(2).get_vertex().get_pos().pos;
                let width = (end.x - start.x).abs();
                let height = (end.y - start.y).abs();
                width >= Self::MIN_RECT_SIZE && height >= Self::MIN_RECT_SIZE
            }
        }
    }
    fn finish_draw(&mut self) -> bool {
        match self.he_property {
            HalfEdgeProperty::General => {
                if self.hes.len() > 2 {
                    self.on_creation = None;
                    self.update_half_edges_vars_from_vertices();
                    if self.area() < 0. {
                        self.hes.vec.reverse();
                        self.update_half_edges_vars_from_vertices();
                    }
                    log!("area: {}", self.area());
                    true
                } else {
                    false
                }
            }
            HalfEdgeProperty::RectangleLike => {
                if self.hes.len() == 4 {
                    let start = self.hes.get(0).get_vertex().get_pos().pos;
                    let end = self.hes.get(2).get_vertex().get_pos().pos;
                    let width = (end.x - start.x).abs();
                    let height = (end.y - start.y).abs();
                    if width >= Self::MIN_RECT_SIZE && height >= Self::MIN_RECT_SIZE {
                        self.on_creation = None;
                        self.update_half_edges_vars_from_vertices();
                        log!("Rectangle finished!");
                        true
                    } else {
                        log!("Rectangle bad size");
                        false
                    }
                } else {
                    log!("Rectangle not finished");
                    return false;
                }
            }
        }
    }
    fn get_state(&self, get: GetEntityState) -> Option<Vec2> {
        use GetEntityState::*;
        match get {
            IsHS(hs) => self.state.is_hs(hs).then(|| self.get_position()),
            GetFirstControlHS(hs) => self
                .hes
                .iter()
                .find_map(|he| he.get_vertex().get_state(hs))
                .or_else(|| self.hes.iter().find_map(|he| he.get_edge().get_state(hs)))
                .or_else(|| {
                    self.hes
                        .iter()
                        .find_map(|he| he.get_vertex_curve().get_state(hs))
                }),
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
                    he.get_vertex_mut().set_state(hs, false);
                    he.get_vertex_curve_mut().set_state(hs, false);
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

                        // Vertex candidate.
                        if let Some(elem) = Elem::new(
                            Vertex,
                            he_idx,
                            Some(he.get_vertex().get_dist_from_pos(pointer.pos())),
                        ) {
                            elems.push(elem);
                        }

                        // Modifier candidate.
                        if let Some(elem) = Elem::new(
                            Modifier,
                            he_idx,
                            he.get_vertex_curve().get_dist_from_pos(pointer.pos()),
                        ) {
                            elems.push(elem);
                        }

                        // Curve candidate.
                        if let Some(elem) = Elem::new(
                            Curve,
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
                        Vertex => {
                            self.hes
                                .get_mut(nearest.idx as i64)
                                .get_vertex_mut()
                                .set_state(hs, true);
                        }
                        Modifier => {
                            self.hes
                                .get_mut(nearest.idx as i64)
                                .get_vertex_curve_mut()
                                .set_state(hs, true);
                        }
                        Curve => {
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
        ()
    }
    fn move_position(&mut self, pointer: &mut Pointer, _keys_states: KeysStates) -> bool {
        self.hes
            .iter_mut()
            .map(|he| he.get_vertex_mut())
            .for_each(|vertex| {
                vertex.move_pos(pointer.dpos());
            });
        self.update_half_edges_vars_from_vertices();

        true
    }
    fn move_controls(&mut self, pointer: &Pointer, keys_states: KeysStates) -> bool {
        use HalfEdgeProperty::*;
        use HS::*;

        // Move the first polygon vertex found in case of multiples (normally not the case)
        // Also, since each primitive has start/end vertices, we need to move the end vertex
        // of the previous primitive
        let snap = pointer.get_snap().val();
        let dpos = pointer.dpos();
        match self.he_property {
            General => {
                // Check if we are in creation mode, if yes process it and return
                if let Some(v) = self.on_creation.as_mut() {
                    if pointer.is_magnetized() {
                        v.set_pos(pointer.pos());
                    } else {
                        let len = self.hes.len();
                        let last_pos = self
                            .hes
                            .get(len as i64 - 1)
                            .get_vertex()
                            .get_pos()
                            .saved_pos;

                        let depl = if len == 1 {
                            // There is only one vertex, so we snap from it
                            snap_end_to_multiple_of(last_pos, v.get_pos().saved_pos, dpos, snap)
                        } else {
                            let first_pos = self.hes.get(0).get_vertex().get_pos().saved_pos;
                            // There are more than one vertex, so snap two vectors:
                            // first - v, last - v
                            if pointer.is_magnetized() {
                                pointer.pos()
                            } else {
                                move_b_with_snapping(
                                    first_pos,
                                    v.get_pos().saved_pos,
                                    last_pos,
                                    dpos,
                                    snap,
                                )
                            }
                        };
                        v.set_pos(depl);
                    };
                    return true;
                }
                // Check veritices, if any selected, move the first found and return
                for idx_he in 0..self.hes.len() as i64 {
                    // get the vertex and check if it is selected
                    if self
                        .hes
                        .get(idx_he)
                        .get_vertex()
                        .get_state(Select)
                        .is_some()
                    {
                        let p_prev = self.hes.get(idx_he - 1).get_vertex().get_pos().clone();
                        let p_next = self.hes.get(idx_he + 1).get_vertex().get_pos().clone();
                        let v = self.hes.get_mut(idx_he).get_vertex_mut();
                        // Move the vertex
                        if pointer.is_magnetized() {
                            v.set_pos(pointer.pos());
                        } else {
                            v.set_pos(move_b_with_snapping(
                                p_prev.pos,
                                v.get_pos().saved_pos,
                                p_next.pos,
                                dpos,
                                snap,
                            ));
                        }
                        self.update_half_edges_vars_from_vertices();
                        return true;
                    }
                }
            }
            RectangleLike => {
                // Move the first vertex found and move adjacent vertices
                for he_idx in 0..self.hes.len() as i64 {
                    let v = self.hes.get(he_idx).get_vertex().clone();
                    if v.get_state(Select).is_some() {
                        let prev_v = self.hes.get(he_idx - 1).get_vertex().get_pos().clone();
                        let next_v = self.hes.get(he_idx + 1).get_vertex().get_pos().clone();
                        // Projection of dpos on the previous edge
                        let mut dpos_proj_prev =
                            project_on_vec(prev_v.saved_pos, v.get_pos().saved_pos, dpos);
                        let mut dpos_proj_next =
                            project_on_vec(next_v.saved_pos, v.get_pos().saved_pos, dpos);

                        if !pointer.is_magnetized() {
                            let prev_rel = v.get_pos().saved_pos - prev_v.saved_pos;
                            dpos_proj_prev = snap_pt(prev_rel + dpos_proj_prev, snap) - prev_rel;

                            let next_rel = v.get_pos().saved_pos - next_v.saved_pos;
                            dpos_proj_next = snap_pt(next_rel + dpos_proj_next, snap) - next_rel;
                        }

                        if (v.get_pos().pos + dpos_proj_prev - prev_v.pos).hypot()
                            < Self::MIN_RECT_SIZE
                            || (v.get_pos().pos + dpos_proj_next - next_v.pos).hypot()
                                < Self::MIN_RECT_SIZE
                        {
                            log!("Too small");
                            return false;
                        }
                        // Move the vertices
                        self.hes
                            .get_mut(he_idx - 1)
                            .get_vertex_mut()
                            .move_pos(dpos_proj_next);
                        self.hes
                            .get_mut(he_idx + 1)
                            .get_vertex_mut()
                            .move_pos(dpos_proj_prev);
                        self.hes
                            .get_mut(he_idx)
                            .get_vertex_mut()
                            .move_pos(dpos_proj_prev + dpos_proj_next);
                        self.update_half_edges_vars_from_vertices();
                        return true;
                    }
                }
            }
        }

        // Move the first control found on the curve
        for idx_he in 0..self.hes.len() as i64 {
            let v = self.hes.get(idx_he).get_vertex().get_pos().clone();
            let v_next = self.hes.get(idx_he + 1).get_vertex().get_pos().clone();
            let p = self.hes.get_mut(idx_he).get_edge_mut();
            if p.move_control_selected(v.pos, v_next.pos, pointer, keys_states) {
                self.update_half_edges_vars_from_vertices();
                return true;
            }
        }

        // for idx in 0..self.get_primitives_len() as i64 {
        //     let p = self.primitives.get(idx);
        //     let s = self.vertices.get(p.get_start());
        //     let e = self.vertices.get(p.get_end());
        //     if s.get_pos().is_hs(Select) {
        //         if keys_states.crtl_cmd_pressed {
        //             // Vertex modification (size of chamfer or fillet)
        //             match s.get_modifier() {
        //                 VertexModifier::Nope(..) => {
        //                     continue;
        //                 }
        //                 // VertexModifier::Chamfer(len, _) | VertexModifier::Fillet(len, _) => {
        //                 //     if let Some(radius) = self.modify_vertex(idx, dpos, snap) {
        //                 //         vertex_modified = Some((radius, start_mod));
        //                 //         self.update_polygon();
        //                 //         break;
        //                 //     }
        //                 // }
        //                 _ => {
        //                     continue;
        //                 }
        //             }
        //         } else {
        //             // Vertex move
        //             s.move_pos(pointer);
        //             self.update_polygon();
        //         }
        //         return true;
        //     }
        // }
        // if keys_states.crtl_cmd_pressed && keys_states.shift_pressed {
        //     for prim in self.prims.iter_mut() {
        //         prim.set_start_modifier(start_mod);
        //         prim.set_start_modifier_offset(radius);
        //     }
        //     self.update_polygon();
        //     return true;
        // }
        // // Move prim modifiers if selected
        // let mut moved = false;
        // for idx in 0..self.get_primitives_len() as i64 {
        //     let p = self.primitives.get(idx);
        //     let s = self.vertices.get(p.get_start());
        //     let e = self.vertices.get(p.get_end());
        //     if p.move_control_selected(pointer, keys_states) {
        //         self.update_polygon();
        //         moved = true;
        //         break;
        //     }
        // }
        // moved
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
            let v = he.get_vertex();
            paths_patterns.push((
                modifiers_path(v.get_pos().pos, scale, Self::GRAB),
                modifiers_pattern(
                    v.get_state(Select).is_some(),
                    v.get_state(Highlight).is_some(),
                ),
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
        use Curve::*;
        use HS::*;
        let mut paths_patterns = vec![];

        for idx in 0..self.hes.len() as i64 {
            let he = self.hes.get(idx);
            // Modifiers
            paths_patterns.push(he.get_vertex_curve().get_paths_and_patterns(
                das,
                self.state.is_hs(Select),
                self.state.is_hs(Highlight),
            ));

            // Polygon edges
            match he.get_edge().get() {
                CLine(l, _) => {
                    paths_patterns.push(l.get_paths_and_patterns(
                        das,
                        self.state.is_hs(Select),
                        self.state.is_hs(Highlight),
                    ));
                }
                CArc(_, a) => {
                    paths_patterns.push(a.get_paths_and_patterns(
                        das,
                        self.state.is_hs(Select),
                        self.state.is_hs(Highlight),
                    ));
                }
            };
        }
        // If on creation, draw the current creation line
        if let HalfEdgeProperty::General = self.he_property {
            if let Some(v) = self.on_creation {
                paths_patterns.push((
                    Line::new(
                        self.hes
                            .get(self.hes.len() as i64 - 1)
                            .get_vertex()
                            .get_pos()
                            .pos
                            .to_point(),
                        v.get_pos().pos.to_point(),
                    )
                    .path_elements(Self::TOLERANCE)
                    .collect(),
                    Pattern::BasicSelected,
                ));
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
