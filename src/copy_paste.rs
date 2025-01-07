use kurbo::Vec2;

use crate::{
    shapes::{Shape, ShapeKindFuncs},
    shapes_pool::ShapesPool,
    Action,
};

pub struct PasteShapesAction {
    pub shapes: Vec<Shape>,
}

impl Action for PasteShapesAction {
    fn undo(&self, pool: &mut ShapesPool) {
        log!("Undoing shapes creation");
        self.shapes.iter().for_each(|shape| {
            pool.delete_shape(shape.get_id());
        });
    }

    fn redo(&self, pool: &mut ShapesPool) {
        log!("Redoing shapes creation");
        self.shapes.iter().for_each(|shape| {
            pool.add_shape(shape.clone());
        });
    }
}

pub struct CopyPasteShape {
    copied: Vec<Shape>,
    to_paste: Vec<Shape>,
    cursor_on_copied: Vec2,
}
impl CopyPasteShape {
    pub fn new() -> Self {
        Self {
            copied: Vec::new(),
            to_paste: Vec::new(),
            cursor_on_copied: Vec2::new(0.0, 0.0),
        }
    }
    pub fn copy(&mut self, shapes: Vec<Shape>, cursor_pos: Vec2) {
        self.cursor_on_copied = cursor_pos;
        self.copied = shapes;
        self.to_paste.clear();
    }
    pub fn paste(&mut self, cursor_pos: Vec2) {
        self.copied.iter().for_each(|shape| {
            let mut new_shape = ShapesPool::new_shape_cloned_from(shape);
            new_shape
                .kind_mut()
                .move_position(cursor_pos - self.cursor_on_copied);
            self.to_paste.push(new_shape);
        });
    }
    pub fn move_shape_to_paste(
        &mut self,
        _pos_init: Vec2,
        cursor_pos: Vec2,
        _shift_pressed: bool,
    ) -> bool {
        if self.to_paste.is_empty() {
            return false;
        }
        self.to_paste.iter_mut().for_each(|shape| {
            shape
                .kind_mut()
                .move_position(cursor_pos - self.cursor_on_copied);
        });
        true
    }
    pub fn get_to_paste(&self) -> &Vec<Shape> {
        &self.to_paste
    }
    pub fn get_to_paste_mut(&mut self) -> &mut Vec<Shape> {
        &mut self.to_paste
    }
    pub fn clear_paste(&mut self) {
        self.to_paste.clear();
    }
}
