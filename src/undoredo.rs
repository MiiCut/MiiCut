use crate::{nodes::Elem, shapes::drawable::Drawable};

pub struct UndoRedo<E: Drawable> {
    undo_stack: Vec<Elem<E>>,
    redo_stack: Vec<Elem<E>>,
}

impl<E: Drawable> UndoRedo<E> {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn undo(&mut self, nodes: &mut Elem<E>) {
        if let Some(popped_nodes) = self.undo_stack.pop() {
            self.redo_stack.push(nodes.clone());
            *nodes = popped_nodes;
        }
    }

    pub fn redo(&mut self, nodes: &mut Elem<E>) {
        if let Some(popped_nodes) = self.redo_stack.pop() {
            self.undo_stack.push(nodes.clone());
            *nodes = popped_nodes;
        }
    }

    pub fn push(&mut self, nodes: &mut Elem<E>) {
        self.undo_stack.push(nodes.clone());
        self.redo_stack.clear();
    }
}
