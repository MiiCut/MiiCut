use crate::shapes_pool::ShapesPool;

pub struct UndoRedo {
    undo_stack: Vec<Box<dyn Action>>,
    redo_stack: Vec<Box<dyn Action>>,
}
pub trait Action {
    fn undo(&self, pool: &mut ShapesPool);
    fn redo(&self, pool: &mut ShapesPool);
}
impl UndoRedo {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }
    pub fn undo(&mut self, pool: &mut ShapesPool) {
        if let Some(action) = self.undo_stack.pop() {
            action.undo(pool);
            self.redo_stack.push(action);
        }
    }

    pub fn redo(&mut self, pool: &mut ShapesPool) {
        if let Some(action) = self.redo_stack.pop() {
            action.redo(pool);
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
