use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::{
    fmt::Display,
    hash::Hasher,
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Debug)]
pub struct Set<E: Clone> {
    pub nodes: HashMap<NodUId, Nod<E>>,
    pub nodes_highlighted: HashSet<NodUId>,
    pub nodes_selected: HashSet<NodUId>,
    pub node_selector: NodeSelector,
}
impl<E: Clone> Set<E> {
    pub fn new() -> Self {
        Set {
            nodes: HashMap::new(),
            nodes_highlighted: HashSet::new(),
            nodes_selected: HashSet::new(),
            node_selector: NodeSelector::new(),
        }
    }
    // Insert a new node into the tree
    pub fn push(&mut self, new_node: Nod<E>) {
        self.nodes.insert(new_node.id, new_node);
    }
    // Remove a node from the tree
    pub fn pop(&mut self, id: NodUId) -> Option<Nod<E>> {
        self.nodes_highlighted.remove(&id);
        self.nodes_selected.remove(&id);
        self.nodes.remove(&id)
    }
    pub fn get(&self, id: NodUId) -> Option<&Nod<E>> {
        self.nodes.get(&id)
    }
    pub fn get_mut(&mut self, id: NodUId) -> Option<&mut Nod<E>> {
        self.nodes.get_mut(&id)
    }
}

#[derive(Debug)]
pub struct Nod<E: Clone> {
    pub id: NodUId,
    pub name: String,
    pub element: E,
}
impl<E: Clone> Nod<E> {
    pub fn new_element(name: &str, element: E) -> Self {
        Nod {
            id: NodUId::new(),
            name: name.to_string(),
            element,
        }
    }
}
impl<E: Clone> Clone for Nod<E> {
    fn clone(&self) -> Self {
        Nod {
            id: NodUId::new(),
            name: self.name.clone(),
            element: self.element.clone(),
        }
    }
}
impl<E: Clone> Hash for Nod<E> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
impl<E: Clone> PartialEq for Nod<E> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<E: Clone> Eq for Nod<E> {}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct NodeSelector {
    selectable_nodes: Vec<NodUId>, // IDs of selectable nodes
    current_index: usize,          // Current index in the list
}
#[allow(dead_code)]
impl NodeSelector {
    pub fn new() -> Self {
        Self {
            selectable_nodes: Vec::new(),
            current_index: 0,
        }
    }
    pub fn refresh_selectable_nodes(&mut self, new_nodes: HashSet<NodUId>) {
        let current_set: HashSet<_> = self.selectable_nodes.iter().cloned().collect();
        // Compare the sets, ignoring order
        if current_set != new_nodes {
            // Reset if the set of nodes changes
            self.selectable_nodes = new_nodes.into_iter().collect();
            self.current_index = 0;
        }
    }
    pub fn next_selection(&mut self) -> Option<NodUId> {
        if self.selectable_nodes.is_empty() {
            return None;
        }
        // Select the current node and move to the next
        let selected = self.selectable_nodes[self.current_index];
        self.current_index = (self.current_index + 1) % self.selectable_nodes.len();
        Some(selected)
    }
}

static COUNTER_NODE: AtomicUsize = AtomicUsize::new(0);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd)]
pub struct NodUId {
    id: usize,
}
impl Display for NodUId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
impl NodUId {
    pub fn new() -> Self {
        let id = COUNTER_NODE.fetch_add(1, Ordering::SeqCst);
        NodUId { id }
    }
}
