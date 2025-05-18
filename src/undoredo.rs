use crate::{nodes::Nod, shapes::drawable::Drawable};

pub struct UndoRedo<E: Drawable> {
    undo_stack: Vec<Nod<E>>,
    redo_stack: Vec<Nod<E>>,
}

impl<E: Drawable> UndoRedo<E> {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn undo(&mut self, nodes: &mut Nod<E>) {
        if let Some(popped_nodes) = self.undo_stack.pop() {
            self.redo_stack.push(nodes.clone());
            *nodes = popped_nodes;
        }
    }

    pub fn redo(&mut self, nodes: &mut Nod<E>) {
        if let Some(popped_nodes) = self.redo_stack.pop() {
            self.undo_stack.push(nodes.clone());
            *nodes = popped_nodes;
        }
    }

    pub fn push(&mut self, nodes: &mut Nod<E>) {
        self.undo_stack.push(nodes.clone());
        self.redo_stack.clear();
    }
}
