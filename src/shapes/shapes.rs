// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use super::shape_custom::ShapeCustom;
use super::shape_custom::ShapeCustomIter;
use super::shape_disc::ShapeDisc;
use super::shape_oblong::ShapeOblong;
use super::shape_oblong::ShapeOblongIter;
use super::shape_rectangle::ShapeRectangle;
use super::shape_rectangle::ShapeRectangleIter;
use super::shape_rectangle_rounded::ShapeRectRounded;
use super::shape_rectangle_rounded::ShapeRectRoundedIter;
use super::shapes_pool::BSid;
use crate::canvas::CanvasText;
use crate::canvas::Pattern;
use crate::pools::Pools;
use crate::pools::PoolsFunctions;
use crate::primitives::primitives::Primitive;
use crate::traits::*;
use crate::Action;
use crate::KeysStates;
use crate::Pointer;
use crate::Position;
use crate::Value;
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
    pub shids_vars: Vec<(BSid, BSKindvars)>,
}
impl Action for MoveShapesAction {
    fn undo(&self, pools: &mut Pools) {
        log!("Undoing last shapes move");
        for (shid, vars) in &self.shids_vars {
            if let Some(shape) = pools.shapes.get_mut(*shid) {
                shape.get_kind_mut().set_vars(vars);
                shape.get_kind_mut().restore_saved();
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

pub enum BSKindvars {
    Rectangle(Position, Position),
    RectangleRounded(Position, Position, Value, Value, Value, Value),
    Disc(Position, Value),
    Oblong(Position, Position, Value),
    Custom(Vec<Primitive>),
}

#[derive(Debug, Clone)]
pub enum BSKind {
    Rectangle(ShapeRectangle),
    RectangleRounded(ShapeRectRounded),
    Disc(ShapeDisc),
    Oblong(ShapeOblong),
    Custom(ShapeCustom),
}
impl BSKind {
    pub fn get_polygon(&self) -> Polygon<f64> {
        use BSKind::*;
        match self {
            Rectangle(sh) => sh.get_polygon(),
            RectangleRounded(sh) => sh.get_polygon(),
            Disc(sh) => sh.get_polygon(),
            Oblong(sh) => sh.get_polygon(),
            Custom(sh) => sh.get_polygon(),
        }
    }
}
impl Display for BSKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use BSKind::*;
        match self {
            Rectangle(sh) => write!(f, "{sh}"),
            RectangleRounded(sh) => write!(f, "{sh}"),
            Disc(sh) => write!(f, "{sh}"),
            Oblong(sh) => write!(f, "{sh}"),
            Custom(sh) => write!(f, "{sh}"),
        }
    }
}
impl ObjectsFuncs for BSKind {
    const TOLERANCE: f64 = 0.01;
    const GRAB_RADIUS: f64 = 2.;
    type Kindvars = BSKindvars;

    fn save_vars(&mut self) {
        use BSKind::*;
        match self {
            Rectangle(sh) => sh.save_vars(),
            RectangleRounded(sh) => sh.save_vars(),
            Disc(sh) => sh.save_vars(),
            Oblong(sh) => sh.save_vars(),
            Custom(sh) => sh.save_vars(),
        }
    }
    fn restore_saved(&mut self) {
        use BSKind::*;
        match self {
            Rectangle(sh) => sh.restore_saved(),
            RectangleRounded(sh) => sh.restore_saved(),
            Disc(sh) => sh.restore_saved(),
            Oblong(sh) => sh.restore_saved(),
            Custom(sh) => sh.restore_saved(),
        }
    }
    fn get_vars(&self) -> BSKindvars {
        use BSKind::*;
        match &self {
            Rectangle(sh) => sh.get_vars(),
            RectangleRounded(sh) => sh.get_vars(),
            Disc(sh) => sh.get_vars(),
            Oblong(sh) => sh.get_vars(),
            Custom(sh) => sh.get_vars(),
        }
    }
    fn set_vars(&mut self, vars: &BSKindvars) {
        use BSKind::*;
        match self {
            Rectangle(sh) => sh.set_vars(vars),
            RectangleRounded(sh) => sh.set_vars(vars),
            Disc(sh) => sh.set_vars(vars),
            Oblong(sh) => sh.set_vars(vars),
            Custom(sh) => sh.set_vars(vars),
        }
    }
    fn good_size(&self) -> bool {
        use BSKind::*;
        match &self {
            Rectangle(sh) => sh.good_size(),
            RectangleRounded(sh) => sh.good_size(),
            Disc(sh) => sh.good_size(),
            Oblong(sh) => sh.good_size(),
            Custom(sh) => sh.good_size(),
        }
    }

    fn get_state(&self, get: GetEntityState) -> Option<Vec2> {
        use BSKind::*;
        match self {
            Rectangle(sh) => sh.get_state(get),
            RectangleRounded(sh) => sh.get_state(get),
            Disc(sh) => sh.get_state(get),
            Oblong(sh) => sh.get_state(get),
            Custom(sh) => sh.get_state(get),
        }
    }
    fn set_state(&mut self, set: SetEntityState) {
        use BSKind::*;
        match self {
            Rectangle(sh) => sh.set_state(set),
            RectangleRounded(sh) => sh.set_state(set),
            Disc(sh) => sh.set_state(set),
            Oblong(sh) => sh.set_state(set),
            Custom(sh) => sh.set_state(set),
        }
    }
    fn set_state_from_pos(&mut self, pointer: &mut Pointer, set: SetEntityStateFromPos) {
        use BSKind::*;
        match self {
            Rectangle(sh) => sh.set_state_from_pos(pointer, set),
            RectangleRounded(sh) => sh.set_state_from_pos(pointer, set),
            Disc(sh) => sh.set_state_from_pos(pointer, set),
            Oblong(sh) => sh.set_state_from_pos(pointer, set),
            Custom(sh) => sh.set_state_from_pos(pointer, set),
        }
    }

    fn toggle_prop(&mut self) {
        use BSKind::*;
        match self {
            Rectangle(sh) => sh.toggle_prop(),
            RectangleRounded(sh) => sh.toggle_prop(),
            Disc(sh) => sh.toggle_prop(),
            Oblong(sh) => sh.toggle_prop(),
            Custom(sh) => sh.toggle_prop(),
        }
    }

    fn move_position(&mut self, pointer: &mut Pointer, keys_states: KeysStates) -> bool {
        use BSKind::*;
        match self {
            Rectangle(sh) => sh.move_position(pointer, keys_states),
            RectangleRounded(sh) => sh.move_position(pointer, keys_states),
            Disc(sh) => sh.move_position(pointer, keys_states),
            Oblong(sh) => sh.move_position(pointer, keys_states),
            Custom(sh) => sh.move_position(pointer, keys_states),
        }
    }
    fn move_modifier(&mut self, pointer: &Pointer, keys_states: KeysStates) -> bool {
        use BSKind::*;
        match self {
            Rectangle(sh) => sh.move_modifier(pointer, keys_states),
            RectangleRounded(sh) => sh.move_modifier(pointer, keys_states),
            Disc(sh) => sh.move_modifier(pointer, keys_states),
            Oblong(sh) => sh.move_modifier(pointer, keys_states),
            Custom(sh) => sh.move_modifier(pointer, keys_states),
        }
    }
    fn get_position(&self) -> Vec2 {
        use BSKind::*;
        match self {
            Rectangle(sh) => sh.get_position(),
            RectangleRounded(sh) => sh.get_position(),
            Disc(sh) => sh.get_position(),
            Oblong(sh) => sh.get_position(),
            Custom(sh) => sh.get_position(),
        }
    }

    fn get_paths_and_patterns(
        &self,
        das: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        use BSKind::*;
        match self {
            Rectangle(sh) => sh.get_paths_and_patterns(das, cinfo),
            RectangleRounded(sh) => sh.get_paths_and_patterns(das, cinfo),
            Disc(sh) => sh.get_paths_and_patterns(das, cinfo),
            Oblong(sh) => sh.get_paths_and_patterns(das, cinfo),
            Custom(sh) => sh.get_paths_and_patterns(das, cinfo),
        }
    }
    fn get_mod_paths_and_patterns(
        &self,
        das: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        use BSKind::*;
        match self {
            Rectangle(sh) => sh.get_mod_paths_and_patterns(das, cinfo),
            RectangleRounded(sh) => sh.get_mod_paths_and_patterns(das, cinfo),
            Disc(sh) => sh.get_mod_paths_and_patterns(das, cinfo),
            Oblong(sh) => sh.get_mod_paths_and_patterns(das, cinfo),
            Custom(sh) => sh.get_mod_paths_and_patterns(das, cinfo),
        }
    }
    fn get_dimensions_paths_and_patterns(
        &self,
        das: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        use BSKind::*;
        match self {
            Rectangle(sh) => sh.get_dimensions_paths_and_patterns(das, cinfo),
            RectangleRounded(sh) => sh.get_dimensions_paths_and_patterns(das, cinfo),
            Disc(sh) => sh.get_dimensions_paths_and_patterns(das, cinfo),
            Oblong(sh) => sh.get_dimensions_paths_and_patterns(das, cinfo),
            Custom(sh) => sh.get_dimensions_paths_and_patterns(das, cinfo),
        }
    }
}
impl Shape for BSKind {
    type PathElementsIter<'iter> = BSKindIter;

    fn path_elements(&self, tolerance: f64) -> BSKindIter {
        use BSKind::*;
        match self {
            Rectangle(sh) => BSKindIter::RectangleIter(sh.path_elements(tolerance)),
            RectangleRounded(sh) => BSKindIter::RectangleRoundedIter(sh.path_elements(tolerance)),
            Disc(sh) => BSKindIter::DiscIter(sh.path_elements(tolerance)),
            Oblong(sh) => BSKindIter::OblongIter(sh.path_elements(tolerance)),
            Custom(sh) => BSKindIter::CustomIter(sh.path_elements(tolerance)),
        }
    }
    #[inline]
    fn area(&self) -> f64 {
        use BSKind::*;
        match self {
            Rectangle(sh) => sh.area(),
            RectangleRounded(sh) => sh.area(),
            Disc(sh) => sh.area(),
            Oblong(sh) => sh.area(),
            Custom(sh) => sh.area(),
        }
    }
    #[inline]
    fn perimeter(&self, accuracy: f64) -> f64 {
        use BSKind::*;
        match self {
            Rectangle(sh) => sh.perimeter(accuracy),
            RectangleRounded(sh) => sh.perimeter(accuracy),
            Disc(sh) => sh.perimeter(accuracy),
            Oblong(sh) => sh.perimeter(accuracy),
            Custom(sh) => sh.perimeter(accuracy),
        }
    }
    #[inline]
    fn winding(&self, pt: Point) -> i32 {
        use BSKind::*;
        match self {
            Rectangle(sh) => sh.winding(pt),
            RectangleRounded(sh) => sh.winding(pt),
            Disc(sh) => sh.winding(pt),
            Oblong(sh) => sh.winding(pt),
            Custom(sh) => sh.winding(pt),
        }
    }
    #[inline]
    fn bounding_box(&self) -> Rect {
        use BSKind::*;
        match self {
            Rectangle(sh) => sh.bounding_box(),
            RectangleRounded(sh) => sh.bounding_box(),
            Disc(sh) => sh.bounding_box(),
            Oblong(sh) => sh.bounding_box(),
            Custom(sh) => sh.bounding_box(),
        }
    }
    #[inline]
    fn as_circle(&self) -> Option<Circle> {
        use BSKind::*;
        match self {
            Rectangle(sh) => sh.as_circle(),
            RectangleRounded(sh) => sh.as_circle(),
            Disc(sh) => sh.as_circle(),
            Oblong(sh) => sh.as_circle(),
            Custom(sh) => sh.as_circle(),
        }
    }
    #[inline]
    fn contains(&self, pt: Point) -> bool {
        use BSKind::*;
        match self {
            Rectangle(sh) => sh.contains(pt),
            RectangleRounded(sh) => sh.contains(pt),
            Disc(sh) => sh.contains(pt),
            Oblong(sh) => sh.contains(pt),
            Custom(sh) => sh.contains(pt),
        }
    }
}
pub enum BSKindIter {
    RectangleIter(ShapeRectangleIter),
    RectangleRoundedIter(ShapeRectRoundedIter),
    DiscIter(CirclePathIter),
    OblongIter(ShapeOblongIter),
    CustomIter(ShapeCustomIter),
}
impl Iterator for BSKindIter {
    type Item = PathEl;
    fn next(&mut self) -> Option<Self::Item> {
        use BSKindIter::*;
        log!("BSKindIter::next");
        match self {
            RectangleIter(sh) => sh.next(),
            RectangleRoundedIter(sh) => sh.next(),
            DiscIter(sh) => sh.next(),
            OblongIter(sh) => sh.next(),
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
pub struct BasicShape {
    shid: BSid,
    shape_kind: BSKind,
    boolean_op: BoolOps,
}
impl BasicShape {
    pub fn new(shid: BSid, shape_kind: BSKind, boolean_op: BoolOps) -> BasicShape {
        BasicShape {
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

impl ObjectOps for BasicShape {
    type Id = BSid;
    type Kind = BSKind;

    fn get_id(&self) -> BSid {
        self.shid
    }
    fn get_kind(&self) -> &BSKind {
        &self.shape_kind
    }
    fn get_kind_mut(&mut self) -> &mut BSKind {
        &mut self.shape_kind
    }
    fn set_new_id(&mut self, new_id: BSid) {
        self.shid = new_id;
    }
}
