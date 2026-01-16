use crate::{shape::GeneralShape, types::Value};

#[derive(Clone, Debug)]
pub struct Clipboard {
    item_copy: Option<(GeneralShape, Value)>,
    item_paste: Option<(GeneralShape, Value)>,
}
impl Clipboard {
    pub fn new() -> Self {
        Self {
            item_copy: None,
            item_paste: None,
        }
    }

    pub fn copy(&mut self, nodes: GeneralShape, pointer_copy: Value) {
        self.item_copy = Some((nodes, pointer_copy));
        self.item_paste = None;
    }

    pub fn paste(&mut self, pointer: Value) {
        if let Some(item_copy) = &self.item_copy {
            self.item_paste = Some((item_copy.0.clone(), pointer));
        }
    }

    pub fn make_paste(&self, pointer: &Value) -> Option<GeneralShape> {
        let (shape, copy_pointer) = self.item_copy.as_ref()?;
        let delta = pointer.curr() - copy_pointer.curr();
        let mut pasted = shape.clone();
        pasted.move_shape(delta);
        for (_, value) in pasted.get_vertices_mut().iter_mut() {
            value.property_remove_all_binds();
        }
        Some(pasted)
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
