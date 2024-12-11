// A macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

use crate::{
    dom::Pattern, math::*, prefab, shape_hole::CShapeHole, shape_oblong::CShapeOblong,
    shape_rectangle::CShapeRectangle, shape_rectangle_rounded::CShapeRectRounded,
};
use kurbo::{BezPath, Vec2};
use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum COperation {
    Add,
    Sub,
    And,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HandleKind {
    Grab,
    Modify,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Handle {
    saved_pos: Vec2,
    old_pos: Vec2,
    pos: Vec2,
    kind: HandleKind,
    highlighted: bool,
    selected: bool,
}
impl Handle {
    pub fn new(pos: Vec2, kind: HandleKind, selected: bool) -> Handle {
        Handle {
            saved_pos: pos,
            old_pos: pos,
            pos,
            kind,
            highlighted: false,
            selected,
        }
    }
    pub fn is_highlighted(&self) -> bool {
        self.highlighted
    }
    pub fn set_highlighted(&mut self, highlighted: bool) {
        self.highlighted = highlighted;
    }
    pub fn get_selection(&self) -> bool {
        self.selected
    }
    pub fn set_selection(&mut self, selected: bool) {
        self.selected = selected;
    }
    pub fn get_pos(&self) -> Vec2 {
        self.pos
    }
    pub fn get_last_pos(&self) -> Vec2 {
        self.old_pos
    }
    pub fn get_saved_pos(&self) -> Vec2 {
        self.saved_pos
    }
    pub fn set_pos(&mut self, pos: Vec2) {
        self.old_pos = self.pos;
        self.pos = pos;
    }
    pub fn save_pos(&mut self) {
        self.saved_pos = self.pos;
        self.old_pos = self.pos;
    }
    pub fn get_kind(&self) -> HandleKind {
        self.kind
    }
    pub fn get_path(&self, scale: f64) -> (Pattern, BezPath) {
        let grab_path = prefab::handle_grab_path(self.pos, scale);
        let modify_path = prefab::handle_modify_path(self.pos, scale);

        match self.kind {
            HandleKind::Grab => match (self.selected, self.highlighted) {
                (false, false) => (Pattern::Normal, grab_path),
                (false, true) => (Pattern::Highlighted, grab_path),
                (true, false) => (Pattern::Selected, grab_path),
                (true, true) => (Pattern::Highlighted, grab_path),
            },
            HandleKind::Modify => {
                if self.highlighted {
                    (Pattern::Highlighted, modify_path)
                } else {
                    (Pattern::Light, modify_path)
                }
            }
        }
    }
}

pub trait CShapes {
    const TOLERANCE: f64;

    fn new(cshid: ClosedShapeId, pos1: Vec2, pos2: Vec2) -> CShape;
    fn get_id(&self) -> ClosedShapeId;
    fn get_op(&self) -> COperation;
    fn save_pos(&mut self);
    fn toggle_prop(&mut self);
    fn is_near_cursor(&self, pos: Vec2, precision: f64) -> bool;
    fn get_shape_path(&self) -> BezPath;
    fn highlight_object(&mut self, pos: Vec2, precision: f64);
    fn select_object(&mut self, pos: Vec2, precision: f64);
    fn is_selected(&self) -> bool;
    fn clear_selection(&mut self);
    fn clear_selection_all(&mut self);
    fn move_position(&mut self, pos_init: Vec2, pos: Vec2);
    fn get_handles(&self) -> Vec<Handle>;
    // Trait implementation must call this function after each move
    fn update_handles_pos(&mut self);
    // Return the first handle selected found or None
    fn get_handle_selected(&self) -> Option<(Handle, usize)>;
    // Return the first handle highlighted found or None
    fn get_handle_highlighted(&self) -> Option<(Handle, usize)>;
}
#[derive(Clone, Debug, PartialEq)]
pub enum CShape {
    CRectangle(CShapeRectangle),
    CRectangleRounded(CShapeRectRounded),
    CHole(CShapeHole),
    COblong(CShapeOblong),
}
impl Display for CShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use CShape::*;
        match self {
            CRectangle(sh) => write!(f, "{sh}"),
            CRectangleRounded(sh) => write!(f, "{sh}"),
            CHole(sh) => write!(f, "{sh}"),
            COblong(sh) => write!(f, "{sh}"),
        }
    }
}
impl CShape {
    pub fn new_rectangle(pos1: Vec2, pos2: Vec2) -> (CShape, ClosedShapeId) {
        let cshid = ClosedShapeId::new();
        (CShapeRectangle::new(cshid, pos1, pos2), cshid)
    }
    pub fn new_rectangle_rounded(pos1: Vec2, pos2: Vec2) -> (CShape, ClosedShapeId) {
        let cshid = ClosedShapeId::new();
        (CShapeRectRounded::new(cshid, pos1, pos2), cshid)
    }
    pub fn new_circle(pos1: Vec2, pos2: Vec2) -> (CShape, ClosedShapeId) {
        let cshid = ClosedShapeId::new();
        (CShapeHole::new(cshid, pos1, pos2), cshid)
    }
    pub fn new_oblong(pos1: Vec2, pos2: Vec2) -> (CShape, ClosedShapeId) {
        let cshid = ClosedShapeId::new();
        (CShapeOblong::new(cshid, pos1, pos2), cshid)
    }

    pub fn get_id(&self) -> ClosedShapeId {
        use CShape::*;
        match self {
            CRectangle(sh) => sh.get_id(),
            CRectangleRounded(sh) => sh.get_id(),
            CHole(sh) => sh.get_id(),
            COblong(sh) => sh.get_id(),
        }
    }
    pub fn get_op(&self) -> COperation {
        use CShape::*;
        match self {
            CRectangle(sh) => sh.get_op(),
            CRectangleRounded(sh) => sh.get_op(),
            CHole(sh) => sh.get_op(),
            COblong(sh) => sh.get_op(),
        }
    }
    pub fn save_pos(&mut self) {
        use CShape::*;
        match self {
            CRectangle(sh) => sh.save_pos(),
            CRectangleRounded(sh) => sh.save_pos(),
            CHole(sh) => sh.save_pos(),
            COblong(sh) => sh.save_pos(),
        }
    }
    pub fn toggle_prop(&mut self) {
        ()
    }
    pub fn is_near_cursor(&self, pos: Vec2, precision: f64) -> bool {
        use CShape::*;
        match self {
            CRectangle(sh) => sh.is_near_cursor(pos, precision),
            CRectangleRounded(sh) => sh.is_near_cursor(pos, precision),
            CHole(sh) => sh.is_near_cursor(pos, precision),
            COblong(sh) => sh.is_near_cursor(pos, precision),
        }
    }
    pub fn get_shape_path(&self) -> BezPath {
        use CShape::*;
        match self {
            CRectangle(sh) => sh.get_shape_path(),
            CRectangleRounded(sh) => sh.get_shape_path(),
            CHole(sh) => sh.get_shape_path(),
            COblong(sh) => sh.get_shape_path(),
        }
    }
    pub fn highlight_object(&mut self, pos: Vec2, precision: f64) {
        use CShape::*;
        match self {
            CRectangle(sh) => sh.highlight_object(pos, precision),
            CRectangleRounded(sh) => sh.highlight_object(pos, precision),
            CHole(sh) => sh.highlight_object(pos, precision),
            COblong(sh) => sh.highlight_object(pos, precision),
        }
    }
    pub fn set_selection(&mut self, pos: Vec2, precision: f64) {
        use CShape::*;
        match self {
            CRectangle(sh) => sh.select_object(pos, precision),
            CRectangleRounded(sh) => sh.select_object(pos, precision),
            CHole(sh) => sh.select_object(pos, precision),
            COblong(sh) => sh.select_object(pos, precision),
        }
    }
    pub fn is_selected(&self) -> bool {
        use CShape::*;
        match self {
            CRectangle(sh) => sh.is_selected(),
            CRectangleRounded(sh) => sh.is_selected(),
            CHole(sh) => sh.is_selected(),
            COblong(sh) => sh.is_selected(),
        }
    }
    pub fn clear_selection(&mut self) {
        use CShape::*;
        match self {
            CRectangle(sh) => sh.clear_selection(),
            CRectangleRounded(sh) => sh.clear_selection(),
            CHole(sh) => sh.clear_selection(),
            COblong(sh) => sh.clear_selection(),
        }
    }
    pub fn clear_selection_all(&mut self) {
        use CShape::*;
        match self {
            CRectangle(sh) => sh.clear_selection_all(),
            CRectangleRounded(sh) => sh.clear_selection_all(),
            CHole(sh) => sh.clear_selection_all(),
            COblong(sh) => sh.clear_selection_all(),
        }
    }
    pub fn move_selection(&mut self, pos_init: Vec2, pos: Vec2) {
        use CShape::*;
        match self {
            CRectangle(sh) => sh.move_position(pos_init, pos),
            CRectangleRounded(sh) => sh.move_position(pos_init, pos),
            CHole(sh) => sh.move_position(pos_init, pos),
            COblong(sh) => sh.move_position(pos_init, pos),
        }
    }
    pub fn get_handles(&self) -> Vec<Handle> {
        use CShape::*;
        match self {
            CRectangle(sh) => sh.get_handles(),
            CRectangleRounded(sh) => sh.get_handles(),
            CHole(sh) => sh.get_handles(),
            COblong(sh) => sh.get_handles(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClosedShapesPool {
    cshapes: HashMap<ClosedShapeId, CShape>,
    pub on_creation: Option<ClosedShapeId>,
}
impl ClosedShapesPool {
    pub fn new() -> ClosedShapesPool {
        ClosedShapesPool {
            cshapes: HashMap::new(),
            on_creation: None,
        }
    }
    pub fn create_bundle_rectangle(&mut self, pos: Vec2) -> ClosedShapeId {
        let (cshape, cshid) = CShape::new_rectangle(pos, pos);
        self.cshapes.insert(cshid, cshape);
        cshid
    }
    pub fn create_bundle_rectangle_rounded(&mut self, pos: Vec2) -> ClosedShapeId {
        let (cshape, cshid) = CShape::new_rectangle_rounded(pos, pos);
        self.cshapes.insert(cshid, cshape);
        cshid
    }
    pub fn create_bundle_circle(&mut self, pos: Vec2) -> ClosedShapeId {
        let (cshape, cshid) = CShape::new_circle(pos, pos);
        self.cshapes.insert(cshid, cshape);
        cshid
    }
    pub fn create_bundle_oblong(&mut self, pos: Vec2) -> ClosedShapeId {
        let (cshape, cshid) = CShape::new_oblong(pos, pos);
        self.cshapes.insert(cshid, cshape);
        cshid
    }

    pub fn get(&self, cshid: ClosedShapeId) -> Result<&CShape, MyError> {
        self.cshapes
            .get(&cshid)
            .ok_or(MyError::NoClosedShapeForCShid(cshid))
    }
    pub fn get_mut(&mut self, cshid: ClosedShapeId) -> Result<&mut CShape, MyError> {
        self.cshapes
            .get_mut(&cshid)
            .ok_or(MyError::NoClosedShapeForCShid(cshid))
    }
    pub fn remove(&mut self, cshid: ClosedShapeId) -> Result<CShape, MyError> {
        log!("Removing cshid {}", cshid);
        self.cshapes
            .remove(&cshid)
            .ok_or(MyError::NoClosedShapeForCShid(cshid))
    }
    pub fn iter(&self) -> impl Iterator<Item = (&ClosedShapeId, &CShape)> {
        self.cshapes.iter()
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&ClosedShapeId, &mut CShape)> {
        self.cshapes.iter_mut()
    }
    pub fn values(&self) -> impl Iterator<Item = &CShape> {
        self.cshapes.values()
    }
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut CShape> {
        self.cshapes.values_mut()
    }
    pub fn get_cshapes_mut(&mut self) -> &mut HashMap<ClosedShapeId, CShape> {
        &mut self.cshapes
    }

    pub fn delete_shape(&mut self, cshid: ClosedShapeId) -> Result<CShape, MyError> {
        self.remove(cshid)
    }
    pub fn set_selection(&mut self, pos: Vec2, precision: f64) {
        for cshape in self.cshapes.values_mut() {
            cshape.set_selection(pos, precision);
        }
    }
    pub fn clear_selection(&mut self, cshid: ClosedShapeId) -> Result<(), MyError> {
        self.get_mut(cshid)?.clear_selection();
        Ok(())
    }
    pub fn clear_selection_all(&mut self, cshid: ClosedShapeId) -> Result<(), MyError> {
        self.get_mut(cshid)?.clear_selection_all();
        Ok(())
    }
    pub fn clear_selections(&mut self) {
        for cshape in self.cshapes.values_mut() {
            cshape.clear_selection();
        }
    }
    pub fn clear_selections_all(&mut self) {
        for cshape in self.cshapes.values_mut() {
            cshape.clear_selection_all();
        }
    }
    pub fn save_positions(&mut self) {
        for cshape in self.cshapes.values_mut() {
            cshape.save_pos();
        }
    }
    pub fn highlight_object(&mut self, pos: Vec2, precision: f64) {
        for cshape in self.cshapes.values_mut() {
            cshape.highlight_object(pos, precision);
        }
    }
    pub fn move_selection(&mut self, pos_dwn: Vec2, cursor_pos: Vec2) -> Result<(), MyError> {
        for cshape in self.cshapes.values_mut() {
            cshape.move_selection(pos_dwn, cursor_pos);
        }
        Ok(())
    }
    pub fn delete_object_selected(&mut self) -> Result<(), MyError> {
        self.cshapes.retain(|_, v| !v.is_selected());
        Ok(())
    }
}

static COUNTER_CLOSED_SHAPES: AtomicUsize = AtomicUsize::new(0);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd)]
pub struct ClosedShapeId {
    id: usize,
}
impl Deref for ClosedShapeId {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.id
    }
}
impl DerefMut for ClosedShapeId {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.id
    }
}
impl Display for ClosedShapeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
impl ClosedShapeId {
    pub fn new() -> ClosedShapeId {
        ClosedShapeId {
            id: COUNTER_CLOSED_SHAPES.fetch_add(1, Ordering::Relaxed),
        }
    }
}
