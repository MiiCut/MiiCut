use super::shapes::ShapeKind;
use crate::{
    canvas::{CanvasText, Pattern},
    math::*,
    pools::HS,
    positions::{Position, Value, ValueBool},
    prefab::*,
    primitives::primitives::{Primitive, PrimitiveControls, PrimitiveCurve},
    GetEntityState, KeysStates, ObjectsFuncs, Pointer, SetEntityState, SetEntityStateFromPos,
};
use geo::{LineString, Polygon};
use kurbo::{BezPath, Line, PathEl, Point, Rect, Shape, Size, Vec2};
use std::{f64::consts::PI, fmt::Display};

#[derive(Copy, Debug, Clone)]
pub enum HalfEdgeProperty {
    RectangleLike,
    Nope,
}
impl Display for HalfEdgeProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use HalfEdgeProperty::*;
        match self {
            RectangleLike => write!(f, "Rectangle"),
            Nope => write!(f, "Polygon"),
        }
    }
}
#[derive(Copy, Debug, Clone)]
pub enum VertexModifier {
    // Fillet radius or Chamfer distance, fillet concavity
    Chamfer(Value, ValueBool, bool, bool),
    Fillet(Value, ValueBool, bool, bool),
    Nope(Value, ValueBool, bool, bool),
}
impl VertexModifier {
    pub fn toogle(&self) -> Self {
        use VertexModifier::*;
        match self {
            Chamfer(radius, concavity, selected, highlighted) => {
                Fillet(*radius, *concavity, *selected, *highlighted)
            }
            Fillet(radius, concavity, selected, highlighted) => {
                Nope(*radius, *concavity, *selected, *highlighted)
            }
            Nope(radius, concavity, selected, highlighted) => {
                Chamfer(*radius, *concavity, *selected, *highlighted)
            }
        }
    }
    pub fn get_offset(&self) -> &f64 {
        use VertexModifier::*;
        match self {
            Chamfer(offset, ..) | Fillet(offset, ..) | Nope(offset, ..) => &offset.value,
        }
    }
    pub fn get_concavity(&self) -> &bool {
        use VertexModifier::*;
        match self {
            Chamfer(_, concavity, ..) | Fillet(_, concavity, ..) | Nope(_, concavity, ..) => {
                &concavity.value
            }
        }
    }
    pub fn get_offset_saved(&self) -> &f64 {
        use VertexModifier::*;
        match self {
            Chamfer(offset, ..) | Fillet(offset, ..) | Nope(offset, ..) => &offset.saved_val,
        }
    }
    pub fn get_concavity_saved(&self) -> &bool {
        use VertexModifier::*;
        match self {
            Chamfer(_, concavity, ..) | Fillet(_, concavity, ..) | Nope(_, concavity, ..) => {
                &concavity.saved_val
            }
        }
    }
    pub fn set_offset(&mut self, offset: f64) {
        use VertexModifier::*;
        match self {
            Chamfer(ref mut off, ..) => off.value = offset,
            Fillet(ref mut off, ..) => off.value = offset,
            Nope(ref mut off, ..) => off.value = offset,
        }
    }
    pub fn set_concavity(&mut self, concavity: bool) {
        use VertexModifier::*;
        match self {
            Chamfer(_, ref mut conc, ..) => conc.value = concavity,
            Fillet(_, ref mut conc, ..) => conc.value = concavity,
            Nope(_, ref mut conc, ..) => conc.value = concavity,
        }
    }
    pub fn set_offset_saved(&mut self, offset: f64) {
        use VertexModifier::*;
        match self {
            Chamfer(ref mut off, ..) => off.saved_val = offset,
            Fillet(ref mut off, ..) => off.saved_val = offset,
            Nope(ref mut off, ..) => off.saved_val = offset,
        }
    }
    pub fn set_concavity_saved(&mut self, concavity: bool) {
        use VertexModifier::*;
        match self {
            Chamfer(_, ref mut conc, ..) => conc.saved_val = concavity,
            Fillet(_, ref mut conc, ..) => conc.saved_val = concavity,
            Nope(_, ref mut conc, ..) => conc.saved_val = concavity,
        }
    }
}

#[derive(Copy, Debug, Clone)]
pub struct Vertex {
    pos: Position,
    modifier: VertexModifier,
}
impl Vertex {
    const MIN_OFFSET: f64 = 30.;
    pub fn new(pos: Vec2) -> Self {
        Self {
            pos: Position::new(pos, true),
            modifier: VertexModifier::Nope(
                Value::new(Self::MIN_OFFSET),
                ValueBool::new(false),
                false,
                false,
            ),
        }
    }
    pub fn get_pos(&self) -> &Position {
        &self.pos
    }
    pub fn get_pos_mut(&mut self) -> &mut Position {
        &mut self.pos
    }
    pub fn set_pos(&mut self, pos: Vec2) {
        self.pos.pos = pos;
    }
    pub fn save_pos(&mut self) {
        self.pos.saved_pos = self.pos.pos;
    }
    pub fn restore_pos(&mut self) {
        self.pos.pos = self.pos.saved_pos;
    }
    pub fn save_vars(&mut self) {
        self.save_pos();
        self.save_modifier();
    }
    pub fn restore_vars(&mut self) {
        self.restore_pos();
        self.restore_modifier();
    }
    pub fn get_modifier(&self) -> VertexModifier {
        self.modifier
    }
    pub fn set_modifier(&mut self, modifier: VertexModifier) {
        self.modifier = modifier;
    }
    pub fn save_modifier(&mut self) {
        use VertexModifier::*;
        match &mut self.modifier {
            Nope(offset, concavity, ..) => {
                offset.saved_val = offset.value;
                concavity.saved_val = concavity.value;
            }
            Chamfer(offset, concavity, ..) => {
                offset.saved_val = offset.value;
                concavity.saved_val = concavity.value;
            }
            Fillet(offset, concavity, ..) => {
                offset.saved_val = offset.value;
                concavity.saved_val = concavity.value;
            }
        }
    }
    pub fn restore_modifier(&mut self) {
        use VertexModifier::*;
        match &mut self.modifier {
            Nope(offset, concavity, ..) => {
                offset.value = offset.saved_val;
                concavity.value = concavity.saved_val;
            }
            Chamfer(offset, concavity, ..) => {
                offset.value = offset.saved_val;
                concavity.value = concavity.saved_val;
            }
            Fillet(offset, concavity, ..) => {
                offset.value = offset.saved_val;
                concavity.value = concavity.saved_val;
            }
        }
    }

    pub fn toogle_modifier(&mut self) {
        self.modifier = self.modifier.toogle();
    }

    pub fn get_offset(&self) -> f64 {
        use VertexModifier::*;
        match self.modifier {
            Nope(..) => 0.,
            Chamfer(offset, ..) | Fillet(offset, ..) => offset.value,
        }
    }
    pub fn set_modifier_offset(&mut self, offset: f64) {
        use VertexModifier::*;
        match self.modifier {
            Nope(_, concavity, s, h) => self.modifier = Nope(Value::new(offset), concavity, s, h),
            Chamfer(_, concavity, s, h) => {
                self.modifier = Chamfer(Value::new(offset), concavity, s, h)
            }
            Fillet(_, concavity, s, h) => {
                self.modifier = Fillet(Value::new(offset), concavity, s, h)
            }
        }
    }
    pub fn get_modifier_offset_saved(&self) -> f64 {
        use VertexModifier::*;
        match self.modifier {
            Nope(offset, ..) | Chamfer(offset, ..) | Fillet(offset, ..) => offset.saved_val,
        }
    }
    pub fn move_pos(&mut self, dpos: Vec2) {
        self.pos.pos = self.pos.saved_pos + dpos;
    }

    pub fn get_pattern(&self) -> Pattern {
        use HS::*;
        match (self.pos.is_hs(Select), self.pos.is_hs(Highlight)) {
            (false, false) => Pattern::BasicNormal,
            (false, true) => Pattern::BasicHighlighted,
            (true, false) => Pattern::BasicSelected,
            (true, true) => Pattern::BasicSelected,
        }
    }
}

#[derive(Copy, Debug, Clone)]
pub struct HalfEdge {
    vertex: Vertex,
    primitive: Primitive,
}
impl HalfEdge {
    fn new(vertex: Vertex, primitive: Primitive) -> Self {
        Self { vertex, primitive }
    }
    pub fn get_vertex(&self) -> &Vertex {
        &self.vertex
    }
    pub fn get_vertex_mut(&mut self) -> &mut Vertex {
        &mut self.vertex
    }
    pub fn get_primitive(&self) -> &Primitive {
        &self.primitive
    }
    pub fn get_primitive_mut(&mut self) -> &mut Primitive {
        &mut self.primitive
    }
    pub fn save_vars(&mut self) {
        self.vertex.save_vars();
        self.primitive.save_vars();
    }
    pub fn restore_vars(&mut self) {
        self.vertex.restore_vars();
        self.primitive.restore_vars();
    }
}

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
    hes_property: HalfEdgeProperty,

    on_creation: Option<Vertex>,

    highlighted: bool,
    selected: bool,

    segs: BezPath,
    polygon: Polygon<f64>,
}

impl ShapePolygon {
    const GRAB: f64 = 5.;
    pub const MIN_RECT_SIZE: f64 = 10.;

    /// Start an empty polygon (but always with a defined half edge)
    pub fn with_first_half_edge(start_pos: Vec2, he_property: HalfEdgeProperty) -> ShapeKind {
        use HalfEdgeProperty::*;
        use PrimitiveCurve::*;
        use ShapeKind::*;
        match he_property {
            Nope => {
                let v = Vertex::new(start_pos);
                let p = Primitive::new(CurveLine);

                KindPolygon(ShapePolygon {
                    hes: VecRing::from_element(HalfEdge::new(v, p)),
                    hes_property: he_property,

                    on_creation: Some(Vertex::new(start_pos)),

                    highlighted: false,
                    selected: false,

                    segs: BezPath::new(),
                    polygon: Polygon::new(LineString::new(vec![]), vec![]),
                })
            }
            RectangleLike => {
                let end_pos = Vec2::new(start_pos.x + Self::GRAB, start_pos.y + Self::GRAB);

                let he1 = HalfEdge::new(
                    Vertex::new(Vec2::new(start_pos.x, start_pos.y)),
                    Primitive::new(CurveLine),
                );
                let he2 = HalfEdge::new(
                    Vertex::new(Vec2::new(end_pos.x, start_pos.y)),
                    Primitive::new(CurveLine),
                );
                let mut he3 = HalfEdge::new(
                    Vertex::new(Vec2::new(end_pos.x, end_pos.y)),
                    Primitive::new(CurveLine),
                );
                he3.get_vertex_mut().get_pos_mut().set_hs(HS::Select, true);
                let he4 = HalfEdge::new(
                    Vertex::new(Vec2::new(start_pos.x, end_pos.y)),
                    Primitive::new(CurveLine),
                );
                let mut hes = VecRing::from_element(he1);
                hes.push(he2);
                hes.push(he3);
                hes.push(he4);

                KindRectangle(ShapePolygon {
                    hes,
                    hes_property: he_property,

                    on_creation: Some(Vertex::new(Vec2::new(end_pos.x, end_pos.y))),

                    highlighted: false,
                    selected: false,

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
                Primitive::new(PrimitiveCurve::CurveLine),
            ));
            v.set_pos(new_pos);
            v.save_pos();
        }
    }

    pub fn get_hes_len(&self) -> usize {
        self.hes.len()
    }
    pub fn get_hes_iter(&self) -> impl Iterator<Item = &HalfEdge> {
        self.hes.iter()
    }
    pub fn get_hes_iter_mut(&mut self) -> impl Iterator<Item = &mut HalfEdge> {
        self.hes.iter_mut()
    }
    pub fn get_he(&self, idx: i64) -> &HalfEdge {
        self.hes.get(idx)
    }
    pub fn get_he_mut(&mut self, idx: i64) -> &mut HalfEdge {
        self.hes.get_mut(idx)
    }
    pub fn get_primitive_control_state(&self, he_idx: i64, hs: HS) -> Option<Vec2> {
        let he = self.hes.get(he_idx);
        let s = he.get_vertex().get_pos().pos;
        let e = self.hes.get(he_idx + 1).get_vertex().get_pos().pos;
        he.get_primitive().get_control_state(s, e, hs)
    }

    ///
    /// Other methods
    ///
    pub fn get_hes_property(&self) -> HalfEdgeProperty {
        self.hes_property
    }
    fn get_centroid(&self) -> Vec2 {
        let mut sum = Vec2::ZERO;
        for he in self.hes.iter() {
            sum += he.get_vertex().get_pos().pos;
        }
        sum / self.hes.len() as f64
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
    pub fn update_primitives_vars(&mut self) {
        for idx_he in 0..self.hes.len() as i64 {
            let s = self.hes.get(idx_he).get_vertex().get_pos().clone();
            let s_next = self.hes.get(idx_he + 1).get_vertex().get_pos().clone();
            self.hes
                .get_mut(idx_he)
                .get_primitive_mut()
                .update_primitives_vars(s.clone(), s_next.clone());
        }
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
            hes_property: self.hes_property,
            on_creation: None,
            highlighted: self.highlighted,
            selected: self.selected,
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
    fn set_vars(&mut self, mii_shape_kind: &ShapeKind) {
        if let ShapeKind::KindPolygon(shape_polygon) = mii_shape_kind {
            self.hes = shape_polygon.hes.clone();
            self.hes_property = shape_polygon.hes_property;
            self.on_creation = None;
            self.update_polygon();
        }
    }
    fn good_size(&self) -> bool {
        match self.hes_property {
            HalfEdgeProperty::Nope => {
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
        match self.hes_property {
            HalfEdgeProperty::Nope => {
                if self.hes.len() > 2 {
                    self.on_creation = None;
                    self.update_polygon();
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
                        self.update_polygon();
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
        use HS::*;
        match get {
            IsHS(Select) => self.selected.then(|| self.get_position()),
            IsHS(Highlight) => self.highlighted.then(|| self.get_position()),
            GetFirstControlHS(hs) => {
                // Return the position of the first vertex found
                for he in self.hes.iter() {
                    let v = he.get_vertex().get_pos();
                    if v.is_hs(hs) {
                        return Some(v.pos);
                    }
                }
                // Return the position of the first control found
                for he_idx in 0..self.hes.len() as i64 {
                    if let Some(pos) = self.get_primitive_control_state(he_idx, hs) {
                        return Some(pos);
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
            SetAllControlsHS(hs, state) => {
                for he_idx in 0..self.hes.len() as i64 {
                    self.hes
                        .get_mut(he_idx)
                        .get_primitive_mut()
                        .set_all_controls_state(hs, state);
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
                // For all vertices and vertices modifiers, retrieve the nearest from the pointer, if any
                let mut nearest_vertex: Option<(i64, f64, Vec2)> = None;
                for he_idx in 0..self.hes.len() as i64 {
                    // Vertex
                    let v_pos = self.get_he(he_idx).get_vertex().get_pos();
                    let dist = v_pos.get_dist_from_pos(pointer.pos());
                    if let Some((_, nearerst_dist, _)) = nearest_vertex {
                        if dist < nearerst_dist {
                            nearest_vertex = Some((he_idx, dist, v_pos.pos));
                        }
                    } else {
                        if dist < Self::GRAB {
                            nearest_vertex = Some((he_idx, dist, v_pos.pos));
                        }
                    }
                    // // Modifier
                    // match self.get_he(he_idx).get_vertex().get_modifier() {
                    //     VertexModifier::Chamfer(_, _, selected, _)
                    //     | VertexModifier::Fillet(_, _, selected, _) => {
                    //         if selected {
                    //             let dist = v_pos.get_dist_from_pos(pointer.pos());
                    //             if let Some((_, nearerst_dist, _)) = nearest_vertex {
                    //                 if dist < nearerst_dist {
                    //                     nearest_vertex = Some((he_idx, dist, v_pos.pos));
                    //                 }
                    //             } else {
                    //                 if dist < Self::GRAB {
                    //                     nearest_vertex = Some((he_idx, dist, v_pos.pos));
                    //                 }
                    //             }
                    //         }
                    //     }
                    //     _ => (),
                    // }
                }

                // For all primitives, retrieve their nearest control from the pointer, if any
                let mut nearest_control: Option<(i64, f64, Vec2)> = None;
                for he_idx in 0..self.hes.len() as i64 {
                    let s = self.hes.get(he_idx).get_vertex().get_pos().pos;
                    let e = self.hes.get(he_idx + 1).get_vertex().get_pos().pos;
                    if let Some((dist, pos)) = self
                        .hes
                        .get(he_idx)
                        .get_primitive()
                        .get_dist_from_control(s, e, pointer)
                    {
                        if let Some((_, nearerst_dist, _)) = nearest_control {
                            if dist < nearerst_dist {
                                nearest_control = Some((he_idx, dist, pos));
                            }
                        } else {
                            if dist < Self::GRAB {
                                nearest_control = Some((he_idx, dist, pos));
                            }
                        }
                    }
                }

                // Clear all hs
                for he in self.hes.iter_mut() {
                    he.get_vertex_mut().get_pos_mut().set_hs(hs, false);
                    he.get_primitive_mut().set_all_controls_state(hs, false);
                }

                // Compare the nearest vertex and the nearest control
                match (nearest_vertex, nearest_control) {
                    (Some((nv_he_idx, nv_dist, nv_pos)), Some((nc_he_idx, nc_dist, nc_pos))) => {
                        if nv_dist < nc_dist {
                            self.hes
                                .get_mut(nv_he_idx)
                                .get_vertex_mut()
                                .get_pos_mut()
                                .set_hs(hs, true);
                            pointer.set_pos(nv_pos);
                            pointer.save_pos();
                        } else {
                            self.hes
                                .get_mut(nc_he_idx)
                                .get_primitive_mut()
                                .set_all_controls_state(hs, true);
                            pointer.set_pos(nc_pos);
                            pointer.save_pos();
                        }
                    }
                    (Some((nv_he_idx, _, nv_pos)), None) => {
                        self.hes
                            .get_mut(nv_he_idx)
                            .get_vertex_mut()
                            .get_pos_mut()
                            .set_hs(hs, true);
                        pointer.set_pos(nv_pos);
                        pointer.save_pos();
                    }
                    (None, Some((nc_he_idx, _, nc_pos))) => {
                        self.hes
                            .get_mut(nc_he_idx)
                            .get_primitive_mut()
                            .set_all_controls_state(hs, true);
                        pointer.set_pos(nc_pos);
                        pointer.save_pos();
                    }
                    (None, None) => {}
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
        self.update_primitives_vars();
        self.update_polygon();
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
        match self.hes_property {
            Nope => {
                // Check if we are in creation mode
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

                // Move the first vertex found
                for idx_he in 0..self.hes.len() as i64 {
                    let state = self.hes.get(idx_he).get_vertex().pos.is_hs(Select);
                    if state {
                        let s_prev = self.hes.get(idx_he - 1).get_vertex().get_pos().clone();
                        let s = self.hes.get(idx_he).get_vertex().get_pos().clone();
                        let s_next = self.hes.get(idx_he + 1).get_vertex().get_pos().clone();
                        let v = self.hes.get_mut(idx_he).get_vertex_mut();
                        // Move the vertex
                        if pointer.is_magnetized() {
                            v.set_pos(pointer.pos());
                        } else {
                            v.set_pos(move_b_with_snapping(
                                s_prev.pos,
                                v.get_pos().saved_pos,
                                s_next.pos,
                                dpos,
                                snap,
                            ));
                        }
                        // Update primitives variables
                        self.hes
                            .get_mut(idx_he - 1)
                            .get_primitive_mut()
                            .update_primitives_vars(s_prev.clone(), s.clone());
                        self.hes
                            .get_mut(idx_he)
                            .get_primitive_mut()
                            .update_primitives_vars(s.clone(), s_next.clone());
                        self.update_polygon();
                        return true;
                    }
                }
            }
            RectangleLike => {
                // Move the first vertex found and move adjacent vertices
                for he_idx in 0..self.hes.len() as i64 {
                    let v = self.hes.get(he_idx).get_vertex().get_pos().clone();
                    if v.is_hs(Select) {
                        let prev_v = self.hes.get(he_idx - 1).get_vertex().get_pos().clone();
                        let next_v = self.hes.get(he_idx + 1).get_vertex().get_pos().clone();
                        // Projection of dpos on the previous edge
                        let mut dpos_proj_prev =
                            project_on_vec(prev_v.saved_pos, v.saved_pos, dpos);
                        let mut dpos_proj_next =
                            project_on_vec(next_v.saved_pos, v.saved_pos, dpos);

                        if !pointer.is_magnetized() {
                            let prev_rel = v.saved_pos - prev_v.saved_pos;
                            dpos_proj_prev = snap_pt(prev_rel + dpos_proj_prev, snap) - prev_rel;

                            let next_rel = v.saved_pos - next_v.saved_pos;
                            dpos_proj_next = snap_pt(next_rel + dpos_proj_next, snap) - next_rel;
                        }

                        if (v.pos + dpos_proj_prev - prev_v.pos).hypot() < Self::MIN_RECT_SIZE
                            || (v.pos + dpos_proj_next - next_v.pos).hypot() < Self::MIN_RECT_SIZE
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

                        // Update primitives variables
                        self.hes
                            .get_mut(he_idx - 1)
                            .get_primitive_mut()
                            .update_primitives_vars(prev_v, v);
                        self.hes
                            .get_mut(he_idx)
                            .get_primitive_mut()
                            .update_primitives_vars(v, next_v);

                        self.update_polygon();
                        return true;
                    }
                }
            }
        }

        // Move the first control found on the primitive
        for idx_he in 0..self.hes.len() as i64 {
            let v = self.hes.get(idx_he).get_vertex().get_pos().clone();
            let v_next = self.hes.get(idx_he + 1).get_vertex().get_pos().clone();
            let p = self.hes.get_mut(idx_he).get_primitive_mut();
            if p.move_control_selected(v.pos, v_next.pos, pointer, keys_states) {
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

    // Only the vertices and control vertices are considered as modifiers
    // Other controls (like line and arc) are not considered as modifiers
    // They are drawn with get_paths_and_patterns
    fn get_mod_paths_and_patterns(
        &self,
        _das: &Size,
        canvas_infos: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        use HS::*;
        let scale = canvas_infos.1;
        let mut paths_patterns: Vec<(BezPath, Pattern)> = vec![];

        // Polygon vertices
        for he in self.hes.iter() {
            let v = he.get_vertex().get_pos();
            paths_patterns.push((
                modifiers_path(v.pos, scale, Self::GRAB),
                modifiers_pattern(v.is_hs(Select), v.is_hs(Highlight)),
            ));
        }

        // Polygon center
        let center = self.get_centroid();
        paths_patterns.push((
            center_path(center, scale, Self::GRAB),
            modifiers_pattern(self.selected, self.highlighted),
        ));
        paths_patterns
    }
    fn get_dimensions_paths_and_patterns(
        &self,
        size: &Size,
        _cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        use HS::*;
        let mut res = vec![];
        for idx in 0..self.get_hes_len() as i64 {
            let s = self.hes.get(idx).get_vertex().get_pos().pos;
            let s_next = self.hes.get(idx + 1).get_vertex().get_pos().pos;

            let p = self.hes.get(idx).get_primitive();
            let selected = p.get_control_state(s, s_next, Select).is_some() || self.selected;
            let highlighted =
                p.get_control_state(s, s_next, Highlight).is_some() || self.highlighted;

            if selected || highlighted {
                let dim = p.get_dimensions_paths_and_patterns(s, s_next, size);
                res.extend(dim);
            }
        }
        res
    }
    fn get_paths_and_patterns(&self, das: &Size, _: (Rect, f64, Vec2)) -> Vec<(BezPath, Pattern)> {
        use PrimitiveCurve::*;
        use VertexModifier::*;
        let mut paths_patterns = vec![];

        let len = self.hes.len();
        for idx in 0..len as i64 {
            let p: &Primitive = self.hes.get(idx).get_primitive();
            let v_prev = self.hes.get(idx - 1).get_vertex();
            let v = self.hes.get(idx).get_vertex();
            let v_next = self.hes.get(idx + 1).get_vertex();

            match p.get_curve() {
                CurveLine => {
                    let start_real =
                        point_from_start(v.get_pos().pos, v_next.get_pos().pos, v.get_offset());
                    let end_real =
                        point_from_end(v.get_pos().pos, v_next.get_pos().pos, v_next.get_offset());
                    let prev_end_real =
                        point_from_end(v_prev.get_pos().pos, v.get_pos().pos, v.get_offset());

                    // Vertices modifiers
                    match v.get_modifier() {
                        Nope(..) => (),
                        Chamfer(..) => {
                            paths_patterns.push((
                                Line::new(prev_end_real.to_point(), start_real.to_point())
                                    .into_path(Self::TOLERANCE),
                                v.get_pattern(),
                            ));
                        }
                        Fillet(_, mut concavity, _s, _h) => {
                            let angle =
                                (PI - angle_from(
                                    v.get_pos().pos - prev_end_real,
                                    start_real - v.get_pos().pos,
                                )) * 0.5;
                            let mut radius = -v.get_offset() * angle.tan();

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
                            paths_patterns.push((f.into_path(Self::TOLERANCE), v.get_pattern()));
                        }
                    }

                    // Polygon edges
                    paths_patterns.push(p.get_line().get_paths_and_patterns(
                        start_real,
                        end_real,
                        das,
                        self.selected,
                        self.highlighted,
                    ));
                }
                CurveArc => {
                    paths_patterns.push(p.get_arc().get_paths_and_patterns(
                        v.get_pos().pos,
                        v_next.get_pos().pos,
                        das,
                        self.selected,
                        self.highlighted,
                    ));
                }
            };
        }
        // If on creation, draw the current creation line
        if let HalfEdgeProperty::Nope = self.hes_property {
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
