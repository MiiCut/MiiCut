use crate::{
    canvas_core::Layer,
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
    pub fn add_shape(&mut self, cshape: CShape) {
        let cshid = cshape.get_id();
        self.cshapes.insert(cshid, cshape);
    }
    pub fn delete_shape(&mut self, cshid: CShid) -> Option<CShape> {
        if let Some(cshape) = self.cshapes.remove(&cshid) {
            Some(cshape)
        } else {
            for (cshid, shapes) in self.cshapes.iter_mut() {
                return shapes.get_children_mut().delete_shape(*cshid);
            }
            None
        }
    }
    pub fn get_shape(&self, target_cshid: CShid) -> Option<&CShape> {
        if let Some(cshape) = self.cshapes.get(&target_cshid) {
            return Some(cshape);
        }
        for (_key, shape) in self.cshapes.iter() {
            if let Some(child_shape) = shape.get_children().get_shape(target_cshid) {
                return Some(child_shape);
            }
        }
        None
    }
    pub fn get_shape_mut(&mut self, target_cshid: CShid) -> Option<&mut CShape> {
        // Check if the target shape exists directly
        if self.cshapes.contains_key(&target_cshid) {
            return self.cshapes.get_mut(&target_cshid);
        }
        for (_key, shape) in self.cshapes.iter_mut() {
            if let Some(child_shape) = shape.get_children_mut().get_shape_mut(target_cshid) {
                return Some(child_shape);
            }
        }
        None
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

    pub fn save_positions(&mut self) {
        self.cshapes.values_mut().for_each(|cs| {
            cs.save_pos();
            cs.get_children_mut()
                .values_mut()
                .for_each(|csc| csc.save_pos());
        });
    }

    // Selections
    pub fn select_object(&mut self, pos: Vec2, precision: f64) {
        self.cshapes.values_mut().for_each(|cs| {
            cs.select_object(pos, precision);
            let selected = cs.is_selected();
            cs.get_children_mut().values_mut().for_each(|csc| {
                csc.set_selection(selected);
                if !selected {
                    csc.select_object(pos, precision);
                }
            });
        });
    }
    pub fn clear_selections(&mut self) {
        self.cshapes.values_mut().for_each(|cs| {
            cs.clear_selection();
            cs.get_children_mut().values_mut().for_each(|csc| {
                csc.clear_selection();
            });
        });
    }
    pub fn clear_selections_all(&mut self) {
        self.cshapes.values_mut().for_each(|cs| {
            cs.clear_selection_all();
            cs.get_children_mut().values_mut().for_each(|csc| {
                csc.clear_selection_all();
            });
        });
    }
    pub fn move_selection(&mut self, pos_dwn: Vec2, cursor_pos: Vec2) {
        self.cshapes.values_mut().for_each(|cs| {
            cs.move_selection(pos_dwn, cursor_pos);
            cs.get_children_mut()
                .values_mut()
                .for_each(|csc| csc.move_selection(pos_dwn, cursor_pos));
        });
    }
    pub fn move_cshid_selection(&mut self, cshid: CShid, pos_dwn: Vec2, cursor_pos: Vec2) {
        if let Some(cshape) = self.get_shape_mut(cshid) {
            cshape.move_selection(pos_dwn, cursor_pos);
        }
    }
    pub fn toggle_op(&mut self, cshid: CShid) {
        if let Some(cshape) = self.get_shape_mut(cshid) {
            if cshape.get_parent().is_some() {
                log!("toggle_op: {:?}", cshid);
                cshape.toggle_op();
            }
        }
    }
    pub fn get_selection(&self) -> Vec<CShid> {
        let mut selection = vec![];
        self.cshapes.values().for_each(|cs| {
            if cs.is_selected() {
                selection.push(cs.get_id());
            }
        });
        selection
    }
    pub fn delete_objects_selected(&mut self) {
        self.cshapes.retain(|_, v| !v.is_selected());
        self.cshapes.values_mut().for_each(|cs| {
            cs.get_children_mut()
                .cshapes
                .retain(|_, v| !v.is_selected());
        });
    }
    // Highlighting
    pub fn highlight_object(&mut self, pos: Vec2, precision: f64) {
        for cshape in self.cshapes.values_mut() {
            cshape.highlight_object(pos, precision);
            let highlight = cshape.is_highlighted();
            for cshape_child in cshape.get_children_mut().values_mut() {
                cshape_child.set_highlight(highlight);
                if !highlight {
                    cshape_child.highlight_object(pos, precision);
                }
            }
        }
    }
    pub fn get_first_highlighted(&self) -> Option<CShid> {
        for cshape in self.cshapes.values() {
            if cshape.is_highlighted() {
                return Some(cshape.get_id());
            }
            for cshape_child in cshape.get_children().values() {
                if cshape_child.is_highlighted() {
                    return Some(cshape_child.get_id());
                }
            }
        }
        None
    }
    pub fn get_highlighted(&self) -> Vec<CShid> {
        let mut highlight = vec![];
        for cshape in self.cshapes.values() {
            if cshape.is_highlighted() {
                highlight.push(cshape.get_id());
            }
        }
        highlight
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
