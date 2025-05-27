use crate::shapes::drawable::{Drawable, ValueUId};
use crate::types::{Binding, Value};
use kurbo::Vec2;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::{
    fmt::Display,
    hash::Hasher,
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Debug)]
pub struct Set<E: Clone> {
    pub elems: HashMap<ElemUId, Elem<E>>,
    pub elems_binded: Binding<ElemUId>,
    pub elems_highlighted: HashSet<ElemUId>,
    pub elems_selected: HashSet<ElemUId>,
    pub elem_selector: ElemSelector,
    pub elem_vertex_selected: Option<(ElemUId, ValueUId)>,
    pub elem_vertex_highlighted: Option<(ElemUId, ValueUId)>,
}
impl<E: Clone> Set<E> {
    pub fn new() -> Self {
        Set {
            elems: HashMap::new(),
            elems_binded: Binding::new(),
            elems_highlighted: HashSet::new(),
            elems_selected: HashSet::new(),
            elem_selector: ElemSelector::new(),
            elem_vertex_selected: None,
            elem_vertex_highlighted: None,
        }
    }
    pub fn push_element(&mut self, new_node: Elem<E>) {
        self.elems.insert(new_node.id, new_node);
    }
    pub fn pop_element(&mut self, id: ElemUId) -> Option<Elem<E>> {
        self.elems_highlighted.remove(&id);
        self.elems_selected.remove(&id);
        self.elems.remove(&id)
    }
    pub fn get_element(&self, id: ElemUId) -> Option<&Elem<E>> {
        self.elems.get(&id)
    }
    pub fn get_element_mut(&mut self, id: ElemUId) -> Option<&mut Elem<E>> {
        self.elems.get_mut(&id)
    }
}
impl<E: Drawable> Set<E> {
    pub fn element_select_vertex(&mut self, position: Value<Vec2>) -> bool {
        self.elem_vertex_selected = None;
        for (eid, element) in self.elems.iter_mut() {
            if let Some(vid_sel) = element.elem.select_vertex(position) {
                self.elems_selected.clear();
                self.elem_vertex_selected = Some((*eid, vid_sel));
            }
        }
        return self.elem_vertex_selected.is_some();
    }
    pub fn element_highlight_vertex(&mut self, position: Value<Vec2>) -> bool {
        self.elem_vertex_highlighted = None;
        for (eid, element) in self.elems.iter_mut() {
            if let Some(vid_sel) = element.elem.highlight_vertex(position) {
                self.elem_vertex_highlighted = Some((*eid, vid_sel));
            }
        }
        return self.elem_vertex_highlighted.is_some();
    }
    pub fn select_elements(&mut self, position: Value<Vec2>, shift_pressed: bool) {
        // Select the nodes whose element contains the position
        if !shift_pressed {
            let mut nodes_selected = HashSet::new();
            for (id, node) in &self.elems {
                if node.elem.contains(position.curr) {
                    nodes_selected.insert(*id);
                }
            }
            if nodes_selected.len() > 0 {
                self.elem_selector
                    .refresh_selectable_elems(nodes_selected.clone());
                if let Some(id) = self.elem_selector.next_selection() {
                    self.elems_selected.clear();
                    self.elems_selected.insert(id);
                }
            } else {
                self.elems_selected.clear();
            }
        } else {
            // Shift pressed, add to selection
            for (id, node) in &self.elems {
                if node.elem.contains(position.curr) {
                    self.elems_selected.insert(*id);
                }
            }
        }
    }
    pub fn highlight_elements(&mut self, position: Value<Vec2>) {
        // Select the nodes whose element contains the position
        self.elems_highlighted.clear();
        for (id, node) in &self.elems {
            if node.elem.contains(position.curr) {
                self.elems_highlighted.insert(*id);
            }
        }
    }
    pub fn save_elements_positions(&mut self) {
        for (_, node) in self.elems.iter_mut() {
            node.elem.save_vertices_positions();
        }
    }
    pub fn move_elements(&mut self, position: Value<Vec2>) -> bool {
        let delta = position.curr - position.saved;
        let mut bind_and_sel = self.elems_selected.clone();
        for id in &self.elems_selected {
            if let Some(elem) = self.elems.get(id) {
                for (_, v) in elem.elem.get_vertices() {
                    if let Some((eid_bind, _)) = v.bind {
                        bind_and_sel.insert(eid_bind);
                    }
                }
            }
        }
        let mut moved = false;
        for id in bind_and_sel.iter() {
            if let Some(node) = self.elems.get_mut(id) {
                node.elem.move_all_vertices(delta);
                moved = true;
            }
        }
        moved
    }
    // pub fn move_binded_elements(&mut self, position: Value<Vec2>, elem_moved: Vec<ElemUId>) {
    //     let delta = position.curr - position.saved;
    //     let elem_to_move: Vec<ElemUId> = self
    //         .elems_binded
    //         .iter()
    //         .filter_map(|couple| {
    //             let contains_first = elem_moved.contains(&couple.0);
    //             let contains_second = elem_moved.contains(&couple.1);
    //             // If both elements are in vids, discard the couple, otherwise keep it.
    //             match (contains_first, contains_second) {
    //                 (true, true) => None,
    //                 (true, false) => Some(couple.1),
    //                 (false, true) => Some(couple.0),
    //                 (false, false) => None,
    //             }
    //         })
    //         .collect();
    //     for id in elem_to_move {
    //         if let Some(node) = self.elems.get_mut(&id) {
    //             node.elem.move_all_vertices(delta);
    //         }
    //     }
    // }
}

#[derive(Debug)]
pub struct Elem<E: Clone> {
    pub id: ElemUId,
    pub name: String,
    pub elem: E,
}
impl<E: Clone> Elem<E> {
    pub fn new_element(name: &str, elem: E) -> Self {
        Elem {
            id: ElemUId::new(),
            name: name.to_string(),
            elem,
        }
    }
}
impl<E: Clone> Clone for Elem<E> {
    fn clone(&self) -> Self {
        Elem {
            id: ElemUId::new(),
            name: self.name.clone(),
            elem: self.elem.clone(),
        }
    }
}
impl<E: Clone> Hash for Elem<E> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
impl<E: Clone> PartialEq for Elem<E> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<E: Clone> Eq for Elem<E> {}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct ElemSelector {
    selectable_elems: Vec<ElemUId>, // IDs of selectable nodes
    current_index: usize,           // Current index in the list
}
#[allow(dead_code)]
impl ElemSelector {
    pub fn new() -> Self {
        Self {
            selectable_elems: Vec::new(),
            current_index: 0,
        }
    }
    pub fn refresh_selectable_elems(&mut self, new_elems: HashSet<ElemUId>) {
        let current_set: HashSet<_> = self.selectable_elems.iter().cloned().collect();
        // Compare the sets, ignoring order
        if current_set != new_elems {
            // Reset if the set of nodes changes
            self.selectable_elems = new_elems.into_iter().collect();
            self.current_index = 0;
        }
    }
    pub fn next_selection(&mut self) -> Option<ElemUId> {
        if self.selectable_elems.is_empty() {
            return None;
        }
        // Select the current node and move to the next
        let selected = self.selectable_elems[self.current_index];
        self.current_index = (self.current_index + 1) % self.selectable_elems.len();
        Some(selected)
    }
}

static COUNTER_NODE: AtomicUsize = AtomicUsize::new(0);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ElemUId {
    id: usize,
}
impl Display for ElemUId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
impl ElemUId {
    pub fn new() -> Self {
        let id = COUNTER_NODE.fetch_add(1, Ordering::SeqCst);
        ElemUId { id }
    }
}
