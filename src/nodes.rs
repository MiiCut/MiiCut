use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::{
    fmt::Display,
    hash::Hasher,
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Debug)]
pub struct Tree<E: Clone> {
    pub nodes: HashMap<NodUId, Nod<E>>,
    pub nodes_state: NodeState,
}

impl<E: Clone> Tree<E> {
    pub fn new() -> Self {
        Tree {
            nodes: HashMap::new(),
            nodes_state: NodeState::new(),
        }
    }

    // Insert a new node into the tree
    pub fn insert(&mut self, parent_id: Option<NodUId>, new_node: Nod<E>) {
        if let Some(parent_node) = parent_id.and_then(|pid| self.get_mut(pid)) {
            parent_node.children.push(new_node.get_id());
        }
        self.nodes.insert(new_node.get_id(), new_node);
    }
    pub fn get(&self, id: NodUId) -> Option<&Nod<E>> {
        self.nodes.get(&id)
    }
    pub fn get_mut(&mut self, id: NodUId) -> Option<&mut Nod<E>> {
        self.nodes.get_mut(&id)
    }
    pub fn get_children(&self, id: NodUId) -> Option<&Vec<NodUId>> {
        self.get(id).map(|node| &node.children)
    }
}

#[derive(Debug)]
pub struct Nod<E: Clone> {
    pub id: NodUId,
    pub parent_id: Option<NodUId>,
    pub name: String,
    pub element: E,
    pub children: Vec<NodUId>,
}
impl<E: Clone> Clone for Nod<E> {
    fn clone(&self) -> Self {
        Nod {
            id: NodUId::new(),
            parent_id: self.parent_id,
            name: self.name.clone(),
            element: self.element.clone(),
            children: vec![],
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
impl<E: Clone> Nod<E> {
    pub fn new_element(parent_id: Option<NodUId>, name: &str, element: E) -> Self {
        Nod {
            id: NodUId::new(),
            parent_id,
            name: name.to_string(),
            element,
            children: vec![],
        }
    }
    pub fn get_id(&self) -> NodUId {
        self.id
    }
}

#[derive(Debug, Clone)]
pub struct NodeState {
    highlighted: HashSet<NodUId>, // Set of all highlighted nodes
    selected: HashSet<NodUId>,    // Set of all selected nodes
}
impl NodeState {
    pub fn new() -> Self {
        NodeState {
            highlighted: HashSet::new(),
            selected: HashSet::new(),
        }
    }

    pub fn highlight_node(&mut self, node: NodUId) {
        self.highlighted.insert(node);
    }
    pub fn unhighlight_node(&mut self, node: &NodUId) {
        self.highlighted.remove(node);
    }
    pub fn clear_highlighted(&mut self) {
        self.highlighted.clear();
    }

    pub fn select_node(&mut self, node: NodUId) {
        self.selected.insert(node);
    }
    pub fn unselect_node(&mut self, node: &NodUId) {
        self.selected.remove(node);
    }
    pub fn clear_selected(&mut self) {
        self.selected.clear();
    }

    // Get the list of highlighted nodes
    pub fn get_highlighted_nodes(&self) -> HashSet<NodUId> {
        self.highlighted.clone()
    }

    // Get the list of selected nodes
    pub fn get_selected_nodes(&self) -> HashSet<NodUId> {
        self.selected.clone()
    }
}

#[derive(Clone, Debug, PartialEq)]
struct NodeSelector {
    selectable_nodes: Vec<NodUId>, // IDs of selectable nodes
    current_index: usize,          // Current index in the list
}

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
            // Reset if the set of shapes changes
            self.selectable_nodes = new_nodes.into_iter().collect();
            self.current_index = 0;
        }
    }
    pub fn next_selection(&mut self) -> Option<NodUId> {
        if self.selectable_nodes.is_empty() {
            return None;
        }
        // Select the current shape and move to the next
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
