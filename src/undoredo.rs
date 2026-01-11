use crate::shape::GeneralShape;

#[derive(Debug)]
pub struct UndoRedo {
    undo_stack: Vec<GeneralShape>,
    redo_stack: Vec<GeneralShape>,
}

impl UndoRedo {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn undo(&mut self, nodes: &mut GeneralShape) {
        if let Some(popped_nodes) = self.undo_stack.pop() {
            self.redo_stack.push(nodes.clone());
            *nodes = popped_nodes;
        }
    }

    pub fn redo(&mut self, nodes: &mut GeneralShape) {
        if let Some(popped_nodes) = self.redo_stack.pop() {
            self.undo_stack.push(nodes.clone());
            *nodes = popped_nodes;
        }
    }

    pub fn push(&mut self, nodes: &mut GeneralShape) {
        self.undo_stack.push(nodes.clone());
        self.redo_stack.clear();
    }
}
