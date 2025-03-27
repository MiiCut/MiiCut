// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use super::shape_disc::ShapeDisc;
use super::shape_polygon::PolygonIter;
use super::shape_polygon::ShapePolygon;
use super::shape_polygon::VecRing;
use super::shapes_pool::BSid;
use crate::canvas::CanvasText;
use crate::canvas::Colors;
use crate::canvas::Pattern;
use crate::curves::half_edge::HalfEdge;
use crate::pools::Pools;
use crate::pools::PoolsFunctions;
use crate::traits::*;
use crate::Action;
use crate::KeysStates;
use crate::Pointer;
use geo::{OpType, Polygon};
use kurbo::Circle;
use kurbo::CirclePathIter;
use kurbo::PathEl;
use kurbo::Point;
use kurbo::Rect;
use kurbo::Shape;
use kurbo::Size;
use kurbo::{BezPath, Vec2};
use std::fmt::Debug;
use std::fmt::Display;

pub struct MoveShapesAction {
    pub shids_vars: Vec<(BSid, ShapeKind)>,
}
impl Action for MoveShapesAction {
    fn undo(&self, pools: &mut Pools) {
        log!("Undoing last shapes move");
        for (shid, vars) in &self.shids_vars {
            if let Some(shape) = pools.shapes.get_mut(*shid) {
                shape.get_kind_mut().set_vars(vars);
                shape.get_kind_mut().restore_vars();
            }
        }
    }
    fn redo(&self, pools: &mut Pools) {
        log!("Redoing last shapes move");
        for (shid, vars) in &self.shids_vars {
            if let Some(shape) = pools.shapes.get_mut(*shid) {
                shape.get_kind_mut().set_vars(vars);
            }
        }
    }
}

pub struct ToogleBoolOpsShapesAction {
    pub shid_toogle: (BSid, BoolOps),
}
impl Action for ToogleBoolOpsShapesAction {
    fn undo(&self, pools: &mut Pools) {
        log!("Undoing last shapes toogle");
        if let Some(shape) = pools.shapes.get_mut(self.shid_toogle.0) {
            shape.set_boolean_op(self.shid_toogle.1);
        }
    }

    fn redo(&self, pools: &mut Pools) {
        log!("Redoing last shapes toogle");
        if let Some(shape) = pools.shapes.get_mut(self.shid_toogle.0) {
            let mut toogle = self.shid_toogle.1;
            toogle.toggle();
            shape.set_boolean_op(toogle);
        }
    }
}

#[derive(Debug, Clone)]
pub enum ShapeKind {
    KindDisc(ShapeDisc),
    KindPolygon(ShapePolygon),
}
impl ShapeKind {
    pub fn new_disc(center: Vec2, end: Vec2, boolean_op: BoolOps) -> Option<MiiShape> {
        let shape_kind = ShapeDisc::new(center, end)?;
        let shid = BSid::new();
        Some(MiiShape::new(shid, shape_kind, boolean_op))
    }
    pub fn new_rectangle(positions: VecRing<HalfEdge>, boolean_op: BoolOps) -> Option<MiiShape> {
        let shape_kind = ShapePolygon::new_rectangle(positions)?;
        let shid = BSid::new();
        Some(MiiShape::new(shid, shape_kind, boolean_op))
    }
    pub fn new_custom(positions: VecRing<HalfEdge>, boolean_op: BoolOps) -> Option<MiiShape> {
        let shape_kind = ShapePolygon::new_custom(positions)?;
        let shid = BSid::new();
        Some(MiiShape::new(shid, shape_kind, boolean_op))
    }
    pub fn new_oblong(positions: VecRing<HalfEdge>, boolean_op: BoolOps) -> Option<MiiShape> {
        let shape_kind = ShapePolygon::new_oblong(positions)?;
        let shid = BSid::new();
        Some(MiiShape::new(shid, shape_kind, boolean_op))
    }
    //
    pub fn get_geo_polygon(&self) -> Polygon<f64> {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.get_polygon(),
            KindPolygon(sh) => sh.get_polygon(),
        }
    }
}
impl Display for ShapeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => write!(f, "{sh}"),
            KindPolygon(sh) => write!(f, "{sh}"),
        }
    }
}
impl ObjectsFuncs for ShapeKind {
    const TOLERANCE: f64 = 0.01;
    const GRAB_RADIUS: f64 = 2.;
    type Kindvars = ShapeKind;

    fn save_vars(&mut self) {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.save_vars(),
            KindPolygon(sh) => sh.save_vars(),
        }
    }
    fn restore_vars(&mut self) {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.restore_vars(),
            KindPolygon(sh) => sh.restore_vars(),
        }
    }
    fn get_vars(&self) -> ShapeKind {
        use ShapeKind::*;
        match &self {
            KindDisc(sh) => sh.get_vars(),
            KindPolygon(sh) => sh.get_vars(),
        }
    }
    fn set_vars(&mut self, vars: &ShapeKind) {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.set_vars(vars),
            KindPolygon(sh) => sh.set_vars(vars),
        }
    }

    fn get_state(&self, get: GetEntityState) -> bool {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.get_state(get),
            KindPolygon(sh) => sh.get_state(get),
        }
    }
    fn set_state(&mut self, set: SetEntityState) {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.set_state(set),
            KindPolygon(sh) => sh.set_state(set),
        }
    }
    fn set_state_from_pos(
        &mut self,
        pointer: &mut Pointer,
        keys_states: KeysStates,
        set: SetEntityStateFromPos,
    ) -> bool {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.set_state_from_pos(pointer, keys_states, set),
            KindPolygon(sh) => sh.set_state_from_pos(pointer, keys_states, set),
        }
    }
    fn contains_pointer(&self, pointer: &Pointer) -> bool {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.contains_pointer(pointer),
            KindPolygon(sh) => sh.contains_pointer(pointer),
        }
    }

    fn move_position(&mut self, pointer: &mut Pointer, keys_states: KeysStates) -> bool {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.move_position(pointer, keys_states),
            KindPolygon(sh) => sh.move_position(pointer, keys_states),
        }
    }
    fn move_controls(&mut self, pointer: &Pointer, keys_states: KeysStates) -> bool {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.move_controls(pointer, keys_states),
            KindPolygon(sh) => sh.move_controls(pointer, keys_states),
        }
    }
    fn get_position(&self) -> Vec2 {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.get_position(),
            KindPolygon(sh) => sh.get_position(),
        }
    }

    fn get_paths_and_patterns(
        &self,
        das: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, Colors)> {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.get_paths_and_patterns(das, cinfo),
            KindPolygon(sh) => sh.get_paths_and_patterns(das, cinfo),
        }
    }
    fn get_prim_paths_and_patterns(
        &self,
        das: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, Colors)> {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.get_prim_paths_and_patterns(das, cinfo),
            KindPolygon(sh) => sh.get_prim_paths_and_patterns(das, cinfo),
        }
    }
    fn get_controls_paths_and_patterns(
        &self,
        das: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, Colors)> {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.get_controls_paths_and_patterns(das, cinfo),
            KindPolygon(sh) => sh.get_controls_paths_and_patterns(das, cinfo),
        }
    }
    fn get_dimensions_paths_and_patterns(
        &self,
        das: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, Colors, Vec<CanvasText>)> {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.get_dimensions_paths_and_patterns(das, cinfo),
            KindPolygon(sh) => sh.get_dimensions_paths_and_patterns(das, cinfo),
        }
    }
}
impl Shape for ShapeKind {
    type PathElementsIter<'iter> = ShapeIter;

    fn path_elements(&self, tolerance: f64) -> ShapeIter {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => ShapeIter::DiscIter(sh.path_elements(tolerance)),
            KindPolygon(sh) => ShapeIter::CustomIter(sh.path_elements(tolerance)),
        }
    }
    #[inline]
    fn area(&self) -> f64 {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.area(),
            KindPolygon(sh) => sh.area(),
        }
    }
    #[inline]
    fn perimeter(&self, accuracy: f64) -> f64 {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.perimeter(accuracy),
            KindPolygon(sh) => sh.perimeter(accuracy),
        }
    }
    #[inline]
    fn winding(&self, pt: Point) -> i32 {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.winding(pt),
            KindPolygon(sh) => sh.winding(pt),
        }
    }
    #[inline]
    fn bounding_box(&self) -> Rect {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.bounding_box(),
            KindPolygon(sh) => sh.bounding_box(),
        }
    }
    #[inline]
    fn as_circle(&self) -> Option<Circle> {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.as_circle(),
            KindPolygon(sh) => sh.as_circle(),
        }
    }
    #[inline]
    fn contains(&self, pt: Point) -> bool {
        use ShapeKind::*;
        match self {
            KindDisc(sh) => sh.contains(pt),
            KindPolygon(sh) => sh.contains(pt),
        }
    }
}
pub enum ShapeIter {
    DiscIter(CirclePathIter),
    CustomIter(PolygonIter),
}
impl Iterator for ShapeIter {
    type Item = PathEl;
    fn next(&mut self) -> Option<Self::Item> {
        use ShapeIter::*;
        log!("MiiShapeIter::next");
        match self {
            DiscIter(sh) => sh.next(),
            CustomIter(sh) => sh.next(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum BoolOps {
    Union,
    UnionForced,
    Difference,
}
impl Display for BoolOps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoolOps::Union => write!(f, "Add"),
            BoolOps::UnionForced => write!(f, "Add to top"),
            BoolOps::Difference => write!(f, "Substract"),
        }
    }
}
impl BoolOps {
    pub fn toggle(&mut self) {
        match self {
            BoolOps::Union => *self = BoolOps::UnionForced,
            BoolOps::UnionForced => *self = BoolOps::Difference,
            BoolOps::Difference => *self = BoolOps::Union,
        }
    }
    pub fn union(&mut self) {
        *self = BoolOps::Union;
    }
    pub fn force_union(&mut self) {
        *self = BoolOps::UnionForced;
    }
    pub fn difference(&mut self) {
        *self = BoolOps::Difference;
    }
    pub fn get_op(&self) -> OpType {
        match self {
            BoolOps::Union => OpType::Union,
            BoolOps::Difference => OpType::Difference,
            BoolOps::UnionForced => OpType::Union,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MiiShape {
    shid: BSid,
    shape_kind: ShapeKind,
    boolean_op: BoolOps,
}
impl MiiShape {
    pub fn new(shid: BSid, shape_kind: ShapeKind, boolean_op: BoolOps) -> MiiShape {
        MiiShape {
            shid,
            shape_kind,
            boolean_op,
        }
    }
    pub fn get_boolean_op(&self) -> BoolOps {
        self.boolean_op
    }
    pub fn toggle_boolean_op(&mut self) {
        self.boolean_op.toggle();
    }
    pub fn set_boolean_op(&mut self, bool_ops: BoolOps) {
        self.boolean_op = bool_ops;
    }
}

impl ObjectOps for MiiShape {
    type Id = BSid;
    type Kind = ShapeKind;

    fn get_id(&self) -> BSid {
        self.shid
    }
    fn get_kind(&self) -> &ShapeKind {
        &self.shape_kind
    }
    fn get_kind_mut(&mut self) -> &mut ShapeKind {
        &mut self.shape_kind
    }
    fn set_new_id(&mut self, new_id: BSid) {
        self.shid = new_id;
    }
}
