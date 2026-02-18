use crate::{canvas::NotesData, shapes::DataSet};
use kurbo::Vec2;

#[derive(Clone, Debug)]
pub struct DrawState {
    pub dataset: DataSet,
    pub notes: NotesData,
    pub scale: f64,
    pub offset: Vec2,
}

#[derive(Clone, Debug)]
struct HistoryEntry {
    before: DrawState,
    after: DrawState,
}

#[derive(Debug)]
pub struct UndoRedo {
    undo_stack: Vec<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
    in_progress: Option<DrawState>,
    max_entries: usize,
}

impl Default for UndoRedo {
    fn default() -> Self {
        Self::new()
    }
}

impl UndoRedo {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            in_progress: None,
            max_entries: 256,
        }
    }

    pub fn begin(&mut self, before: DrawState) {
        self.in_progress = Some(before);
    }

    pub fn commit(&mut self, after: DrawState) {
        if let Some(before) = self.in_progress.take() {
            self.push_entry(before, after);
        }
    }

    pub fn abort(&mut self) {
        self.in_progress = None;
    }

    pub fn push(&mut self, before: DrawState, after: DrawState) {
        self.in_progress = None;
        self.push_entry(before, after);
    }

    fn push_entry(&mut self, before: DrawState, after: DrawState) {
        self.undo_stack.push(HistoryEntry { before, after });
        if self.undo_stack.len() > self.max_entries {
            let excess = self.undo_stack.len() - self.max_entries;
            self.undo_stack.drain(0..excess);
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, current: DrawState) -> Option<DrawState> {
        let entry = self.undo_stack.pop()?;
        self.redo_stack.push(HistoryEntry {
            before: entry.before.clone(),
            after: current,
        });
        Some(entry.before)
    }

    pub fn redo(&mut self, current: DrawState) -> Option<DrawState> {
        let entry = self.redo_stack.pop()?;
        self.undo_stack.push(HistoryEntry {
            before: current,
            after: entry.after.clone(),
        });
        Some(entry.after)
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.in_progress = None;
    }
}
