use crate::{shape::ClosedShape, types::Value};

#[derive(Clone, Debug)]
pub struct Clipboard {
    item_copy: Option<(ClosedShape, Value)>,
    item_paste: Option<(ClosedShape, Value)>,
}
impl Clipboard {
    pub fn new() -> Self {
        Self {
            item_copy: None,
            item_paste: None,
        }
    }

    pub fn copy(&mut self, nodes: ClosedShape, pointer_copy: Value) {
        self.item_copy = Some((nodes, pointer_copy));
        self.item_paste = None;
    }

    pub fn paste(&mut self, pointer: Value) {
        if let Some(item_copy) = &self.item_copy {
            self.item_paste = Some((item_copy.0.clone(), pointer));
        }
    }

    pub fn move_paste(&mut self, pointer_paste: &mut Value) {
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
