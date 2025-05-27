use kurbo::Vec2;

use crate::nodes::Elem;
use crate::shapes::drawable::Drawable;
use crate::types::Value;

#[derive(Clone, Debug)]
pub struct Clipboard<E: Drawable> {
    item_copy: Option<(Elem<E>, Value<Vec2>)>,
    item_paste: Option<(Elem<E>, Value<Vec2>)>,
}
impl<E: Drawable> Clipboard<E> {
    pub fn new() -> Self {
        Self {
            item_copy: None,
            item_paste: None,
        }
    }

    pub fn copy(&mut self, nodes: Elem<E>, pointer_copy: Value<Vec2>) {
        self.item_copy = Some((nodes, pointer_copy));
        self.item_paste = None;
    }

    pub fn paste(&mut self, pointer: Value<Vec2>) {
        if let Some(item_copy) = &self.item_copy {
            self.item_paste = Some((item_copy.0.clone(), pointer));
        }
    }

    pub fn move_paste(&mut self, pointer_paste: &mut Value<Vec2>) {
        if let Some(item_paste) = self.item_paste.as_mut() {
            item_paste.1 = pointer_paste.clone();
        }
    }
    // Clear the clipboard
    pub fn clear(&mut self) {
        self.clear_copy();
        self.clear_paste();
    }
    pub fn clear_copy(&mut self) {
        self.item_copy = None;
    }
    pub fn clear_paste(&mut self) {
        self.item_paste = None;
    }
    // Check if the clipboard is empty
    pub fn is_copy_empty(&self) -> bool {
        self.item_copy.is_none()
    }
    pub fn is_paste_empty(&self) -> bool {
        self.item_paste.is_none()
    }
}
