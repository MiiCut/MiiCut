use super::shapes::ShapeKind;
use crate::{
    canvas::{CanvasText, Pattern},
    math::*,
    pools::HS,
    prefab::*,
    primitives::primitives::{
        Primitive, PrimitiveControls, PrimitiveCurve, Vertex, VertexModifier, VertexProperty,
    },
    GetEntityState, KeysStates, ObjectsFuncs, Pointer, SetEntityState, SetEntityStateFromPos,
};
use geo::{LineString, Polygon};
use kurbo::{BezPath, Line, PathEl, Point, Rect, Shape, Size, Vec2};
use std::{f64::consts::PI, fmt::Display};

#[derive(Debug, Clone)]
pub struct VecRing<T> {
    vec: Vec<T>,
}
impl<T> VecRing<T> {
    pub fn from_element(e: T) -> Self {
        Self { vec: vec![e] }
    }
    pub fn from_two_elements(e1: T, e2: T) -> Self {
        Self { vec: vec![e1, e2] }
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
    vertices: VecRing<Vertex>,
    primitives: VecRing<Primitive>,
    vertices_property: VertexProperty,

    highlighted: bool,
    selected: bool,

    segs: BezPath,
    polygon: Polygon<f64>,
}

impl ShapePolygon {
    const GRAB: f64 = 5.;
    pub const MIN_RECT_SIZE: f64 = 10.;

    /// Start an empty polygon (but always with a defined primitive curve
    /// and its two vertices
    pub fn with_first_primitive(
        start_pos: Vec2,
        end_pos: Vec2,
        vertices_property: VertexProperty,
    ) -> ShapeKind {
        // There is a dummy primitive that will be replaced by the first real primitive
        // Hence we can use the VecRing type
        match vertices_property {
            VertexProperty::Nope => {
                let v1 = Vertex::new(start_pos, vertices_property);
                let mut v2 = Vertex::new(end_pos, vertices_property);
                v2.get_pos_mut().set_hs(HS::Select, true);

                ShapeKind::KindPolygon(ShapePolygon {
                    vertices: VecRing::from_two_elements(v1, v2),
                    primitives: VecRing::from_element(Primitive::new(
                        PrimitiveCurve::CurveLine,
                        0,
                        1,
                    )),
                    vertices_property,

                    highlighted: false,
                    selected: false,

                    segs: BezPath::new(),
                    polygon: Polygon::new(LineString::new(vec![]), vec![]),
                })
            }
            VertexProperty::RectangleLike => {
                let v1 = Vertex::new(Vec2::new(start_pos.x, start_pos.y), vertices_property);
                let v2 = Vertex::new(Vec2::new(end_pos.x, start_pos.y), vertices_property);
                let mut v3 = Vertex::new(Vec2::new(end_pos.x, end_pos.y), vertices_property);
                v3.get_pos_mut().set_hs(HS::Select, true);
                let v4 = Vertex::new(Vec2::new(start_pos.x, end_pos.y), vertices_property);

                let mut vertices = VecRing::from_two_elements(v1, v2);
                vertices.push(v3);
                vertices.push(v4);

                let mut primitives =
                    VecRing::from_element(Primitive::new(PrimitiveCurve::CurveLine, 0, 1));
                primitives.push(Primitive::new(PrimitiveCurve::CurveLine, 1, 2));
                primitives.push(Primitive::new(PrimitiveCurve::CurveLine, 2, 3));
                primitives.push(Primitive::new(PrimitiveCurve::CurveLine, 3, 0));

                ShapeKind::KindPolygon(ShapePolygon {
                    vertices,
                    primitives,
                    vertices_property,

                    highlighted: false,
                    selected: false,

                    segs: BezPath::new(),
                    polygon: Polygon::new(LineString::new(vec![]), vec![]),
                })
            }
        }
    }
    /// Add a new vertex at `pos`, connecting it to the previously added vertex
    pub fn add_vertex(&mut self, pos: Vec2) -> bool {
        match self.vertices_property {
            VertexProperty::Nope => {
                let new_index = self.vertices.len();
                // Create the new vertex
                self.vertices.push(Vertex::new(pos, self.vertices_property));
                // Connect the previous vertex to the new one.
                self.primitives.push(Primitive::new(
                    PrimitiveCurve::CurveLine,
                    new_index - 1,
                    new_index,
                ));
                true
            }
            VertexProperty::RectangleLike => false,
        }
    }

    ///
    /// Primitive related methods
    ///
    pub fn get_primitives_len(&self) -> usize {
        self.primitives.len()
    }
    pub fn primitives_pos(&self) -> impl Iterator<Item = (Vec2, Vec2)> + '_ {
        self.primitives.iter().map(move |p| {
            let p1 = self.vertices.get(p.get_start()).get_pos().pos;
            let p2 = self.vertices.get(p.get_end()).get_pos().pos;
            (p1, p2)
        })
    }
    pub fn get_primitives_iter(&self) -> impl Iterator<Item = &Primitive> {
        self.primitives.iter()
    }
    pub fn get_primitives_iter_mut(&mut self) -> impl Iterator<Item = &mut Primitive> {
        self.primitives.iter_mut()
    }
    pub fn get_primitive_start(&self, idx: i64) -> &Vertex {
        self.vertices.get(self.primitives.get(idx).get_start())
    }
    pub fn get_primitive_start_mut(&mut self, idx: i64) -> &mut Vertex {
        self.vertices.get_mut(self.primitives.get(idx).get_start())
    }
    pub fn primitive_next_curve(&mut self, prim_idx: i64) -> Vec2 {
        let p = self.primitives.get_mut(prim_idx);
        p.next_curve();
        let p1 = self.vertices.get(p.get_start()).get_pos().pos;
        let p2 = self.vertices.get(p.get_end()).get_pos().pos;
        use PrimitiveCurve::*;
        match p.get_curve() {
            CurveLine => (p1 + p2) / 2.,
            CurveArc => (p1 + p2) / 2.,
        }
    }
    pub fn primitive_prev_curve(&mut self, prim_idx: i64) -> Vec2 {
        let p = self.primitives.get_mut(prim_idx);
        p.prev_curve();
        let s_pos = self.vertices.get(p.get_start()).get_pos().pos;
        let e_pos = self.vertices.get(p.get_end()).get_pos().pos;
        use PrimitiveCurve::*;
        match p.get_curve() {
            CurveLine => (s_pos + e_pos) / 2.,
            CurveArc => (s_pos + e_pos) / 2.,
        }
    }
    pub fn primitive_selected_next_curve(&mut self) {
        use GetEntityState::*;
        for idx in 0..self.primitives.len() {
            if self.get_state(IsHS(HS::Select)).is_some() {
                self.primitive_next_curve(idx as i64);
                break;
            }
        }
    }
    pub fn primitive_selected_prev_curve(&mut self) {
        use GetEntityState::*;
        for idx in 0..self.primitives.len() {
            if self.get_state(IsHS(HS::Select)).is_some() {
                self.primitive_prev_curve(idx as i64);
                break;
            }
        }
    }
    pub fn get_primitive_controls_positions(&self, prim_idx: i64) -> Vec<Vec2> {
        let p = self.primitives.get(prim_idx);
        let s_pos = self.vertices.get(p.get_start()).get_pos().pos;
        let e_pos = self.vertices.get(p.get_end()).get_pos().pos;
        p.get_all_controls_positions(s_pos, e_pos)
    }

    pub fn get_primitive_controls_paths_and_patterns(
        &self,
        prim_idx: i64,
        das: &Size,
    ) -> Vec<(BezPath, Pattern)> {
        let p = self.primitives.get(prim_idx);
        let s = self.vertices.get(p.get_start());
        let e = self.vertices.get(p.get_end());
        let mut paths_patterns: Vec<(BezPath, Pattern)> = vec![];
        paths_patterns.push((
            modifiers_path(s.get_pos().pos, 1., Self::GRAB),
            s.get_pattern(),
        ));
        paths_patterns.extend(p.get_mod_paths_and_patterns(s.get_pos().pos, e.get_pos().pos, das));
        paths_patterns
    }
    pub fn get_primitive_dimensions_paths_and_patterns(
        &self,
        prim_idx: i64,
        das: &Size,
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        let p = self.primitives.get(prim_idx);
        let s_pos = self.vertices.get(p.get_start()).get_pos().pos;
        let e_pos = self.vertices.get(p.get_end()).get_pos().pos;
        p.get_dimensions_paths_and_patterns(s_pos, e_pos, das)
    }

    pub fn primitive_update_vars(&mut self, prim_idx: i64) -> Vec2 {
        let p = self.primitives.get_mut(prim_idx);
        let s = *self.vertices.get(p.get_start()).get_pos();
        let e = *self.vertices.get(p.get_end()).get_pos();
        p.update_primitives_vars(s, e)
    }
    pub fn primitive_get_state(&self, prim_idx: i64, get: GetEntityState) -> Option<Vec2> {
        let p = self.primitives.get(prim_idx);
        let s = self.vertices.get(p.get_start()).get_pos();
        let e = self.vertices.get(p.get_end()).get_pos();

        p.get_state(s.pos, e.pos, get)
    }
    pub fn primitive_set_state(&mut self, prim_idx: i64, set: SetEntityState) {
        let p = self.primitives.get_mut(prim_idx);
        let s_pos = self.vertices.get(p.get_start()).get_pos().pos;
        let e_pos = self.vertices.get(p.get_end()).get_pos().pos;
        p.set_state(s_pos, e_pos, set);
    }
    pub fn primitive_set_state_from_pos(
        &mut self,
        prim_idx: i64,
        pointer: &mut Pointer,
        set: SetEntityStateFromPos,
    ) {
        let p = self.primitives.get_mut(prim_idx);
        let s_pos = self.vertices.get(p.get_start()).get_pos().pos;
        let e_pos = self.vertices.get(p.get_end()).get_pos().pos;
        p.set_state_from_pos(s_pos, e_pos, pointer, set)
    }
    pub fn primitive_move_control_selected(
        &mut self,
        prim_idx: i64,
        pointer: &Pointer,
        keys_states: KeysStates,
    ) -> bool {
        let primitive = self.primitives.get_mut(prim_idx);
        let s = *self.vertices.get(primitive.get_start());
        let e = *self.vertices.get(primitive.get_end());
        primitive.move_control_selected(s.get_pos().pos, e.get_pos().pos, pointer, keys_states)
    }

    ///
    /// Other methods
    ///
    pub fn get_vertices_property(&self) -> VertexProperty {
        self.vertices_property
    }
    fn get_centroid(&self) -> Vec2 {
        let mut sum = Vec2::ZERO;
        for vertex in self.vertices.iter() {
            sum += vertex.get_pos().pos;
        }
        sum / self.vertices.len() as f64
    }
    fn line_to(&self, start: Vec2, end: Vec2) -> BezPath {
        Line::new(start.to_point(), end.to_point()).into_path(Self::TOLERANCE)
    }
    pub fn get_polygon(&self) -> Polygon<f64> {
        self.polygon.clone()
    }
    pub fn update_polygon(&mut self) {
        self.segs = calc_segs(self.to_path(Self::TOLERANCE));
        self.polygon = calc_polygon(&self.segs);
    }
}
impl Display for ShapePolygon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Polygon")
    }
}
impl kurbo::Shape for ShapePolygon {
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
        0.
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

        for (s_pos, _) in self.primitives_pos() {
            if s_pos.x < min_x {
                min_x = s_pos.x;
            }
            if s_pos.y < min_y {
                min_y = s_pos.y;
            }
            if s_pos.x > max_x {
                max_x = s_pos.x;
            }
            if s_pos.y > max_y {
                max_y = s_pos.y;
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
        for p in self.primitives.iter_mut() {
            self.vertices.get_mut(p.get_start()).save_vars();
            self.vertices.get_mut(p.get_end()).save_vars();
            p.save_vars();
        }
    }
    fn restore_vars(&mut self) {
        for p in self.primitives.iter_mut() {
            self.vertices.get_mut(p.get_start()).restore_vars();
            self.vertices.get_mut(p.get_end()).restore_vars();
            p.restore_vars();
        }
        self.update_polygon();
    }
    fn get_vars(&self) -> ShapeKind {
        ShapeKind::KindPolygon(ShapePolygon {
            vertices: self.vertices.clone(),
            primitives: self.primitives.clone(),
            vertices_property: self.vertices_property,
            highlighted: self.highlighted,
            selected: self.selected,
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
    fn set_vars(&mut self, mii_shape_kind: &ShapeKind) {
        if let ShapeKind::KindPolygon(shape_polygon) = mii_shape_kind {
            self.vertices = shape_polygon.vertices.clone();
            self.primitives = shape_polygon.primitives.clone();
            self.vertices_property = shape_polygon.vertices_property;

            self.update_polygon();
        }
    }
    fn good_size(&self) -> bool {
        match self.vertices_property {
            VertexProperty::Nope => {
                let len = self.primitives.len();
                len > 2
            }
            VertexProperty::RectangleLike => {
                let len = self.primitives.len();
                if len != 4 {
                    return false;
                }
                let start = self.vertices.get(0).get_pos().pos;
                let end = self.vertices.get(2).get_pos().pos;
                let width = (end.x - start.x).abs();
                let height = (end.y - start.y).abs();
                width >= Self::MIN_RECT_SIZE && height >= Self::MIN_RECT_SIZE
            }
        }
    }
    fn finish_draw(&mut self) -> bool {
        match self.vertices_property {
            VertexProperty::Nope => {
                if self.primitives.len() > 2 {
                    let new_index = self.vertices.len();
                    // Last primitive
                    // Connect the previous vertex to the new one.
                    self.primitives
                        .push(Primitive::new(PrimitiveCurve::CurveLine, new_index, 0));
                    self.update_polygon();
                    true
                } else {
                    false
                }
            }
            VertexProperty::RectangleLike => {
                let len = self.primitives.len();
                if len < 4 {
                    return false;
                }
                let start = self.vertices.get(0).get_pos().pos;
                let end = self.vertices.get(2).get_pos().pos;
                let width = (end.x - start.x).abs();
                let height = (end.y - start.y).abs();
                width >= Self::MIN_RECT_SIZE && height >= Self::MIN_RECT_SIZE
            }
        }
    }

    fn get_state(&self, get: GetEntityState) -> Option<Vec2> {
        use GetEntityState::*;
        use HS::*;
        match get {
            IsHS(Select) => self.selected.then(|| self.get_position()),
            IsHS(Highlight) => self.highlighted.then(|| self.get_position()),
            IsAnyControlHS(hs) => {
                // Return the position of the first control found
                for p in self.primitives.iter() {
                    let s = self.vertices.get(p.get_start()).get_pos();
                    let e = self.vertices.get(p.get_end()).get_pos();

                    if s.is_hs(hs) {
                        return Some(s.pos);
                    } else {
                        if let Some(pos) = p.get_state(s.pos, e.pos, IsHS(hs)) {
                            return Some(pos);
                        } else {
                            if let Some(pos) = p.get_state(s.pos, e.pos, IsAnyControlHS(hs)) {
                                return Some(pos);
                            }
                        }
                    }
                }
                None
            }
        }
    }
    fn set_state(&mut self, set: SetEntityState) {
        use SetEntityState::*;
        use HS::*;
        match set {
            SetHS(Highlight, value) => self.highlighted = value,
            SetHS(Select, value) => self.selected = value,
            SetAllControlsHS(hs, value) => {
                for p in self.primitives.iter_mut() {
                    self.vertices
                        .get_mut(p.get_start())
                        .get_pos_mut()
                        .set_hs(hs, value);
                    let s = self.vertices.get(p.get_start()).get_pos().pos;
                    let e = self.vertices.get(p.get_end()).get_pos().pos;
                    p.set_state(s, e, SetHS(hs, value));
                    p.set_state(s, e, SetAllControlsHS(hs, value));
                }
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
        use HS::*;
        match set {
            SetHSFromPos(Highlight) => self.highlighted = self.contains(pointer.pos().to_point()),
            SetHSFromPos(Select) => self.selected = self.contains(pointer.pos().to_point()),
            SetControlHSFromPos(hs) => {
                for p in self.primitives.iter_mut() {
                    self.vertices
                        .get_mut(p.get_start())
                        .get_pos_mut()
                        .set_hs_from_pos(hs, pointer);
                    let s = self.vertices.get(p.get_start()).get_pos().pos;
                    let e = self.vertices.get(p.get_end()).get_pos().pos;
                    p.set_state_from_pos(s, e, pointer, SetHSFromPos(hs));
                    p.set_state_from_pos(s, e, pointer, SetControlHSFromPos(hs));
                }
            }
        }
    }

    fn toggle_selected_prop(&mut self) {
        use GetEntityState::*;
        if self.get_state(IsHS(HS::Select)).is_some() {
            for p in self.primitives.iter_mut() {
                let s_pos = *self.vertices.get(p.get_start()).get_pos();
                let e_pos = *self.vertices.get(p.get_end()).get_pos();
                p.toggle_prop();
                p.update_primitives_vars(s_pos, e_pos);
            }
        }
    }

    fn move_position(&mut self, pointer: &mut Pointer, _keys_states: KeysStates) -> bool {
        let mut moved = false;
        self.vertices.iter_mut().for_each(|vertex| {
            moved |= vertex.move_pos(pointer);
        });
        self.update_polygon();
        moved
    }
    fn move_controls(&mut self, pointer: &Pointer, _keys_states: KeysStates) -> bool {
        use HS::*;
        // Check if we are in creation mode
        // if let Some(current_pos) = &mut self.current_creation_pos {
        //     match self.vertices_property {
        //         VertexProperty::Nope => {
        //             // Yes, update the last line
        //             if let Some(last_line) = self.primitives.last_mut() {
        //                 if let PrimitiveCurve::CurveLine = last_line.get_prim_curve() {
        //                     let start_pos = last_line.get_start_pos();
        //                     if !pointer.is_magnetized() {
        //                         current_pos.pos = current_pos.saved_pos + pointer.dpos();
        //                         current_pos.pos =
        //                             snap_pt(current_pos.pos - start_pos, pointer.get_snap().val())
        //                                 + start_pos;
        //                     } else {
        //                         current_pos.pos = current_pos.saved_pos + pointer.dpos();
        //                     }
        //                     last_line.set_end_pos(current_pos.pos);
        //                 }
        //             }
        //             self.update_polygon();
        //             true
        //         }
        //         VertexProperty::RectangleLike => {
        //             let snap = pointer.get_snap().val();
        //             let dpos = pointer.dpos();
        //             let magnetized = pointer.is_magnetized();
        //             // Vertex move
        //             self.move_vertex(2, dpos, snap, magnetized);
        //             self.update_polygon();
        //             return true;
        //         }
        //     }
        // } else {
        // Move the first polygon vertex found in case of multiples (normally not the case)
        // Also, since each primitive has start/end vertices, we need to move the end vertex
        // of the previous primitive
        let snap = pointer.get_snap().val();
        let dpos = pointer.dpos();
        let magnetized = pointer.is_magnetized();

        for v in self.vertices.iter_mut() {
            if v.get_pos().is_hs(Select) {
                v.move_pos(pointer);
                self.update_polygon();
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

    fn get_mod_paths_and_patterns(
        &self,
        das: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        let mut paths_patterns: Vec<(BezPath, Pattern)> = vec![];
        for idx in 0..self.primitives.len() {
            paths_patterns.extend(self.get_primitive_controls_paths_and_patterns(idx as i64, das));
        }
        paths_patterns.push((
            center_path(self.get_position(), 1., Self::GRAB_RADIUS),
            self.get_pattern_status(self.selected, self.highlighted),
        ));
        // Filets centers
        use VertexModifier::*;
        for idx in 0..self.primitives.len() as i64 {
            let prim_prev = self.primitives.get(idx - 1);
            let prim: &Primitive = self.primitives.get(idx);
            let prim_next = self.primitives.get(idx + 1);

            let start_mod = prim.get_start_modifier();
            let end_mod = prim_next.get_start_modifier();
            let start_modifier_offset = prim.get_start_modifier_offset();
            let end_modifier_offset = prim_next.get_start_modifier_offset();

            let start = prim.get_start_pos();
            let start_prev = prim_prev.get_start_pos();
            let end = prim.get_end_pos();
            let end_prev = prim_prev.get_end_pos();

            let prim_start_pattern = prim.get_pattern(
                prim.is_start_selected() || self.selected,
                prim.is_start_highlighted() || self.highlighted,
            );

            match prim.get_prim_curve() {
                PrimitiveCurve::CurveLine => {
                    let selected = prim.get_line().is_selected() || self.selected;
                    let highlighted = prim.get_line().is_highlighted() || self.highlighted;

                    let start_real = point_from_start(start, end, start_modifier_offset);
                    let end_real = point_from_end(start, end, end_modifier_offset);
                    let prev_end_real = point_from_end(start_prev, end_prev, start_modifier_offset);

                    match start_mod {
                        Nope(..) => (),
                        Chamfer(..) => (),
                        Fillet(mut concavity) => {
                            let angle =
                                (PI - angle_from(start - prev_end_real, start_real - start)) * 0.5;
                            let mut radius = -start_modifier_offset * angle.tan();

                            if radius > 0. {
                                concavity = !concavity;
                                radius = -radius;
                            }
                            let center = create_arc_from_radius_and_concavity(
                                prev_end_real,
                                start_real,
                                radius,
                                concavity,
                            )
                            .center
                            .to_vec2();
                            paths_patterns.push((
                                center_path(center, 1., Self::GRAB_RADIUS),
                                prim_start_pattern,
                            ));
                            let ee = if let Nope(..) = end_mod {
                                end
                            } else {
                                end_real
                            };
                            paths_patterns.push((
                                self.line_to(start_real, ee),
                                prim.get_pattern(selected, highlighted),
                            ));
                        }
                    }
                }
                _ => (),
            };
        }
        paths_patterns
    }

    fn get_dimensions_paths_and_patterns(
        &self,
        size: &Size,
        _cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        use GetEntityState::*;
        use HS::*;
        let mut res = vec![];
        for idx in 0..self.get_primitives_len() as i64 {
            let p = self.primitives.get(idx);
            let s = self.vertices.get(p.get_start());
            let e = self.vertices.get(p.get_end());
            let s_pos = s.get_pos().pos;
            let e_pos = e.get_pos().pos;

            let selected = p.get_state(s_pos, e_pos, IsHS(Select)).is_some() || self.selected;
            let highlighted =
                p.get_state(s_pos, e_pos, IsHS(Highlight)).is_some() || self.highlighted;

            if selected || highlighted {
                let dim = p.get_dimensions_paths_and_patterns(s_pos, e_pos, size);
                res.extend(dim);
            }
        }
        res
    }
    fn get_paths_and_patterns(&self, _: &Size, _: (Rect, f64, Vec2)) -> Vec<(BezPath, Pattern)> {
        use GetEntityState::*;
        use VertexModifier::*;
        use HS::*;
        let mut paths_patterns = vec![];
        let len = self.primitives.len();
        for idx in 0..len as i64 {
            let p = self.primitives.get(idx);
            let s = self.vertices.get(p.get_start());
            let e = self.vertices.get(p.get_end());
            let p_prev = self.primitives.get(idx - 1);
            let s_prev = self.vertices.get(p_prev.get_start());
            let e_prev = self.vertices.get(p_prev.get_end());
            match self.primitives.get(idx).get_curve() {
                PrimitiveCurve::CurveLine => {
                    let selected = p
                        .get_state(s.get_pos().pos, e.get_pos().pos, IsHS(Select))
                        .is_some()
                        || self.selected;
                    let highlighted = p
                        .get_state(s.get_pos().pos, e.get_pos().pos, IsHS(Highlight))
                        .is_some()
                        || self.highlighted;

                    let start_real =
                        point_from_start(s.get_pos().pos, e.get_pos().pos, s.get_modifier_offset());
                    let end_real =
                        point_from_end(s.get_pos().pos, e.get_pos().pos, e.get_modifier_offset());
                    let prev_end_real = point_from_end(
                        s_prev.get_pos().pos,
                        e_prev.get_pos().pos,
                        s.get_modifier_offset(),
                    );

                    match s.get_modifier() {
                        Nope(..) => {
                            let ee = if let Nope(..) = e.get_modifier() {
                                e.get_pos().pos
                            } else {
                                end_real
                            };
                            paths_patterns.push((
                                self.line_to(s.get_pos().pos, ee),
                                p.get_pattern(selected, highlighted),
                            ));
                        }
                        Chamfer(..) => {
                            paths_patterns
                                .push((self.line_to(prev_end_real, start_real), s.get_pattern()));
                            let ee = if let Nope(..) = e.get_modifier() {
                                e.get_pos().pos
                            } else {
                                end_real
                            };
                            paths_patterns.push((
                                self.line_to(start_real, ee),
                                p.get_pattern(selected, highlighted),
                            ));
                        }
                        Fillet(_, mut concavity) => {
                            let angle =
                                (PI - angle_from(
                                    s.get_pos().pos - prev_end_real,
                                    start_real - s.get_pos().pos,
                                )) * 0.5;
                            let mut radius = -s.get_modifier_offset() * angle.tan();

                            if radius > 0. {
                                concavity.value = !concavity.value;
                                radius = -radius;
                            }
                            let f = create_arc_from_radius_and_concavity(
                                prev_end_real,
                                start_real,
                                radius,
                                concavity.value,
                            );
                            paths_patterns.push((f.into_path(Self::TOLERANCE), s.get_pattern()));
                            let ee = if let Nope(..) = e.get_modifier() {
                                e.get_pos().pos
                            } else {
                                end_real
                            };
                            paths_patterns.push((
                                self.line_to(start_real, ee),
                                p.get_pattern(selected, highlighted),
                            ));
                        }
                    }
                }
                PrimitiveCurve::CurveArc => {
                    let selected = p
                        .get_state(s.get_pos().pos, e.get_pos().pos, IsHS(Select))
                        .is_some()
                        || self.selected;
                    let highlighted = p
                        .get_state(s.get_pos().pos, e.get_pos().pos, IsHS(Highlight))
                        .is_some()
                        || self.highlighted;
                    let radius = p.get_arc().get_radius();
                    let concavity = p.get_arc().get_concavity();
                    let f = create_arc_from_radius_and_concavity(
                        s.get_pos().pos,
                        e.get_pos().pos,
                        radius,
                        concavity,
                    );
                    paths_patterns.push((
                        f.into_path(Self::TOLERANCE),
                        p.get_pattern(selected, highlighted),
                    ));
                }
            };
        }
        paths_patterns
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
