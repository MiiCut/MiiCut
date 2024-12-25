use crate::{canvas_core::Pattern, prefab};
use kurbo::{BezPath, Vec2};

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
    pub fn is_selected(&self) -> bool {
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
    pub fn get_path(&self, scale: f64) -> BezPath {
        match self.kind {
            HandleKind::Grab => prefab::handle_grab_path(self.pos, scale),
            HandleKind::Modify => prefab::handle_modify_path(self.pos, scale),
        }
    }
    pub fn get_pattern(&self) -> Pattern {
        match self.kind {
            HandleKind::Grab => match (self.selected, self.highlighted) {
                (false, false) => Pattern::HandleNormal(true),
                (false, true) => Pattern::HandleHighlighted(true),
                (true, false) => Pattern::HandleSelected(true),
                (true, true) => Pattern::HandleSelected(true),
            },
            HandleKind::Modify => {
                if self.highlighted {
                    Pattern::HandleHighlighted(true)
                } else {
                    Pattern::HandleNormal(true)
                }
            }
        }
    }
}
