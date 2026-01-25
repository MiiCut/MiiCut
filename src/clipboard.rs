use crate::{shape::GeneralShape, type_vertex::Vertex};

#[derive(Clone, Debug)]
pub struct Clipboard {
    item_copy: Option<(GeneralShape, Vertex)>,
    item_paste: Option<(GeneralShape, Vertex)>,
}
impl Clipboard {
    pub fn new() -> Self {
        Self {
            item_copy: None,
            item_paste: None,
        }
    }

    pub fn copy(&mut self, nodes: GeneralShape, pointer_copy: Vertex) {
        self.item_copy = Some((nodes, pointer_copy));
        self.item_paste = None;
    }

    pub fn paste(&mut self, pointer: Vertex) {
        if let Some(item_copy) = &self.item_copy {
            self.item_paste = Some((item_copy.0.clone(), pointer));
        }
    }

    pub fn make_paste(&self, pointer: &Vertex) -> Option<GeneralShape> {
        let (shape, copy_pointer) = self.item_copy.as_ref()?;
        let delta = pointer.curr() - copy_pointer.curr();
        let mut pasted = shape.clone();
        pasted.move_shape(delta);
        Some(pasted)
    }

    pub fn move_paste(&mut self, pointer_paste: &mut Vertex) {
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
