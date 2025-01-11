use kurbo::Vec2;

use crate::helpers::helpers::Helper;
use crate::pools::Pools;
use crate::shapes::shapes::BasicShape;
use crate::traits::*;

#[derive(Clone, Debug)]
pub enum ClipboardItem {
    Shapes((Vec<BasicShape>, Vec2)),
    Helpers((Vec<Helper>, Vec2)),
}

#[derive(Clone, Debug)]
pub struct Clipboard {
    item_copy: Option<ClipboardItem>,
    item_paste: Option<ClipboardItem>,
}
impl Clipboard {
    pub fn new() -> Self {
        Self {
            item_copy: None,
            item_paste: None,
        }
    }

    pub fn copy_shapes(&mut self, shapes: Vec<BasicShape>, cursor_pos: Vec2) {
        self.item_copy = Some(ClipboardItem::Shapes((shapes, cursor_pos)));
        self.item_paste = None;
    }

    pub fn copy_helpers(&mut self, helpers: Vec<Helper>, cursor_pos: Vec2) {
        self.item_copy = Some(ClipboardItem::Helpers((helpers, cursor_pos)));
        self.item_paste = None;
    }

    pub fn move_paste(&mut self, cursor_pos: Vec2, snap: f64) {
        if let Some(item_paste) = self.item_paste.as_mut() {
            match item_paste {
                ClipboardItem::Shapes((shapes, pos_copy)) => {
                    shapes.iter_mut().for_each(|shape| {
                        shape
                            .get_kind_mut()
                            .move_position(cursor_pos - *pos_copy, snap);
                    });
                }
                ClipboardItem::Helpers((helpers, pos_copy)) => {
                    helpers.iter_mut().for_each(|helper| {
                        helper
                            .get_kind_mut()
                            .move_position(cursor_pos - *pos_copy, snap);
                    });
                }
            }
        }
    }

    pub fn paste_item(&mut self, cursor_pos: Vec2, snap: f64) {
        if let Some(item_copy) = &self.item_copy {
            let mut item_paste = item_copy.clone();
            match &mut item_paste {
                ClipboardItem::Shapes((shapes, pos_copy)) => {
                    shapes.iter_mut().for_each(|shape| {
                        shape
                            .get_kind_mut()
                            .move_position(cursor_pos - *pos_copy, snap);
                    });
                }
                ClipboardItem::Helpers((helpers, pos_copy)) => {
                    helpers.iter_mut().for_each(|helper| {
                        helper
                            .get_kind_mut()
                            .move_position(cursor_pos - *pos_copy, snap);
                    });
                }
            }
            self.item_paste = Some(item_paste);
        }
    }
    // For pasting shapes or helpers
    pub fn get_paste(&self) -> Option<&ClipboardItem> {
        self.item_paste.as_ref()
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

//  Paste action
pub struct PasteAction {
    pub clip_item: ClipboardItem,
}
// Implementing the Action trait for PasteAction
impl Action for PasteAction {
    fn undo(&self, pools: &mut Pools) {
        log!("Undoing shapes or helpers paste");
        match &self.clip_item {
            ClipboardItem::Shapes((shapes, ..)) => {
                shapes.iter().for_each(|shape| {
                    pools.sh.delete_shape(shape.get_id());
                });
            }
            ClipboardItem::Helpers((helpers, ..)) => {
                helpers.iter().for_each(|helper| {
                    pools.hp.delete_helper(helper.get_id());
                });
            }
        }
    }
    fn redo(&self, pools: &mut Pools) {
        log!("Redoing shapes or helpers paste");
        match &self.clip_item {
            ClipboardItem::Shapes((shapes, _)) => {
                shapes.iter().for_each(|shape| {
                    pools.sh.add_shape(shape.clone());
                });
            }
            ClipboardItem::Helpers((helpers, _)) => {
                helpers.iter().for_each(|helper| {
                    pools.hp.add_helper(helper.clone());
                });
            }
        }
    }
}

pub struct UndoRedo {
    undo_stack: Vec<Box<dyn Action>>,
    redo_stack: Vec<Box<dyn Action>>,
}

pub trait Action {
    fn undo(&self, pools: &mut Pools);
    fn redo(&self, pools: &mut Pools);
}

impl UndoRedo {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn undo(&mut self, pools: &mut Pools) {
        if let Some(action) = self.undo_stack.pop() {
            action.undo(pools);
            self.redo_stack.push(action);
        }
    }

    pub fn redo(&mut self, pools: &mut Pools) {
        if let Some(action) = self.redo_stack.pop() {
            action.redo(pools);
            self.undo_stack.push(action);
        }
    }

    pub fn push(&mut self, action: Box<dyn Action>) {
        self.undo_stack.push(action);
        self.redo_stack.clear();
    }
}

impl Default for UndoRedo {
    fn default() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }
}
