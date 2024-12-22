use crate::{
    canvas_core::Layer,
    math::*,
    shape_hole::CShapeHole,
    shape_oblong::CShapeOblong,
    shape_rectangle::CShapeRectangle,
    shape_rectangle_rounded::CShapeRectRounded,
    shapes::{CShape, CShapes, GlobalCompositeOperation},
};
use kurbo::Vec2;
use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Clone, Debug, PartialEq)]
pub enum CShapeKind {
    CRectangle(CShapeRectangle),
    CRectangleRounded(CShapeRectRounded),
    CHole(CShapeHole),
    COblong(CShapeOblong),
}
impl Display for CShapeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use CShapeKind::*;
        match self {
            CRectangle(sh) => write!(f, "{sh}"),
            CRectangleRounded(sh) => write!(f, "{sh}"),
            CHole(sh) => write!(f, "{sh}"),
            COblong(sh) => write!(f, "{sh}"),
        }
    }
}
pub struct CShapeBuilder;
impl CShapeBuilder {
    fn new(
        parent: Option<CShid>,
        cshape_kind: CShapeKind,
        layer: Layer,
        op: GlobalCompositeOperation,
    ) -> CShape {
        let cshid = CShid::new();
        CShape::new(cshid, cshape_kind, parent, layer, op)
    }
    pub fn new_rectangle(
        pos1: Vec2,
        pos2: Vec2,
        parent: Option<CShid>,
        layer: Layer,
        op: GlobalCompositeOperation,
    ) -> CShape {
        let cshape_kind = CShapeRectangle::new(pos1, pos2);
        CShapeBuilder::new(parent, cshape_kind, layer, op)
    }
    pub fn new_rectangle_rounded(
        pos1: Vec2,
        pos2: Vec2,
        parent: Option<CShid>,
        layer: Layer,
        op: GlobalCompositeOperation,
    ) -> CShape {
        let cshape_kind = CShapeRectRounded::new(pos1, pos2);
        CShapeBuilder::new(parent, cshape_kind, layer, op)
    }
    pub fn new_circle(
        pos1: Vec2,
        pos2: Vec2,
        parent: Option<CShid>,
        layer: Layer,
        op: GlobalCompositeOperation,
    ) -> CShape {
        let cshape_kind = CShapeHole::new(pos1, pos2);
        CShapeBuilder::new(parent, cshape_kind, layer, op)
    }
    pub fn new_oblong(
        pos1: Vec2,
        pos2: Vec2,
        parent: Option<CShid>,
        layer: Layer,
        op: GlobalCompositeOperation,
    ) -> CShape {
        let cshape_kind = CShapeOblong::new(pos1, pos2);
        CShapeBuilder::new(parent, cshape_kind, layer, op)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CSPool {
    cshapes: HashMap<CShid, CShape>,
}
impl CSPool {
    pub fn new() -> CSPool {
        CSPool {
            cshapes: HashMap::new(),
        }
    }
    pub fn add_shape(&mut self, cshape: CShape) -> CShid {
        let cshid = cshape.get_id();
        self.cshapes.insert(cshid, cshape);
        cshid
    }
    pub fn delete_shape(&mut self, cshid: CShid) -> Option<CShape> {
        self.cshapes.remove(&cshid)
    }
    pub fn get_shape(&self, cshid: CShid) -> Option<&CShape> {
        self.cshapes.get(&cshid)
    }
    pub fn get_shape_mut(&mut self, cshid: CShid) -> Option<&mut CShape> {
        self.cshapes.get_mut(&cshid)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&CShid, &CShape)> {
        self.cshapes.iter()
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&CShid, &mut CShape)> {
        self.cshapes.iter_mut()
    }
    pub fn values(&self) -> impl Iterator<Item = &CShape> {
        self.cshapes.values()
    }
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut CShape> {
        self.cshapes.values_mut()
    }

    pub fn set_selection(&mut self, pos: Vec2, precision: f64) {
        for cshape in self.cshapes.values_mut() {
            cshape.set_selection(pos, precision);
        }
    }
    pub fn clear_selection_all(&mut self, cshid: CShid) -> Result<(), MyError> {
        self.cshapes
            .get_mut(&cshid)
            .ok_or(MyError::NoClosedShapeForCShid(cshid))?
            .clear_selection_all();
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
    pub fn get_selected(&self) -> Option<&CShape> {
        for cshape in self.cshapes.values() {
            if cshape.is_selected() {
                return Some(cshape);
            }
        }
        None
    }
    pub fn get_highlighted(&self) -> Option<CShid> {
        for cshape in self.cshapes.values() {
            if cshape.is_highlighted() {
                return Some(cshape.get_id());
            }
        }
        None
    }
    pub fn delete_object_selected(&mut self) {
        self.cshapes.retain(|_, v| !v.is_selected());
    }
}

static COUNTER_CLOSED_SHAPES: AtomicUsize = AtomicUsize::new(0);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd)]
pub struct CShid {
    id: usize,
}
impl Deref for CShid {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.id
    }
}
impl DerefMut for CShid {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.id
    }
}
impl Display for CShid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
impl CShid {
    pub fn new() -> CShid {
        CShid {
            id: COUNTER_CLOSED_SHAPES.fetch_add(1, Ordering::Relaxed),
        }
    }
}
