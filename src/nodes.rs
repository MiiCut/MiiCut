use crate::inputs::UserUI;
use crate::math::snap_vertex;
use crate::shapes::drawable::{ClosedShapeType, Drawable};
use crate::types::{EUId, Snap, VUId, Value};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct DataSet<E: Clone> {
    pub set: HashMap<EUId, E>,
    pub set_highlighted: HashSet<EUId>,
    pub set_selected: HashSet<EUId>,
    pub elem_selector: ElemSelector,
    pub elem_vertex_selected: Option<(EUId, VUId)>,
    pub elem_vertex_highlighted: Option<(EUId, VUId)>,
}
impl<E: Clone> DataSet<E> {
    pub fn new() -> Self {
        DataSet {
            set: HashMap::new(),
            set_highlighted: HashSet::new(),
            set_selected: HashSet::new(),
            elem_selector: ElemSelector::new(),
            elem_vertex_selected: None,
            elem_vertex_highlighted: None,
        }
    }
    pub fn push_element(&mut self, elem: E) {
        self.set.insert(EUId::new(), elem);
    }
    pub fn pop_element(&mut self, id: EUId) -> Option<E> {
        self.set_highlighted.remove(&id);
        self.set_selected.remove(&id);
        self.set.remove(&id)
    }
    pub fn get_element(&self, id: EUId) -> Option<&E> {
        self.set.get(&id)
    }
    pub fn get_element_mut(&mut self, id: EUId) -> Option<&mut E> {
        self.set.get_mut(&id)
    }
}
impl<E: Drawable> DataSet<E> {
    fn create_vertices_between(
        &mut self,
        eid_sel: EUId,
        vid_sel: VUId,
        eid_hig: EUId,
        vid_hig: VUId,
    ) -> Option<()> {
        if eid_sel != eid_hig {
            return None;
        }
        let elem = self.get_element_mut(eid_sel)?;
        elem.get_vertices().dist_ok(
            elem.get_vertices().get_idx(&vid_sel)?,
            elem.get_vertices().get_idx(&vid_hig)?,
            1,
        )?;
        let v_sel = elem.get_vertex(&vid_sel)?;
        let v_hig = elem.get_vertex(&vid_hig)?;

        match elem.get_shape_type() {
            ClosedShapeType::PolyRectangle => {
                // Create new vertex between the selected and highlighted vertices
                let new_v1 = (
                    VUId::new(),
                    Value::new(snap_vertex((v_sel.curr + v_hig.curr) / 2.0, Snap::new())),
                );
                let new_v2 = (
                    VUId::new(),
                    Value::new(snap_vertex((v_sel.curr + v_hig.curr) / 2.0, Snap::new())),
                );
                elem.get_vertices_mut()
                    .insert_two_between(&vid_sel, &vid_hig, new_v1, new_v2);
                elem.set_bezpath();
                return Some(());
            }

            ClosedShapeType::Polygon => {
                // Create new vertex between the selected and highlighted vertices
                let new_v = (
                    VUId::new(),
                    Value::new(snap_vertex((v_sel.curr + v_hig.curr) / 2.0, Snap::new())),
                );
                elem.get_vertices_mut()
                    .insert_one_between(&vid_sel, &vid_hig, new_v);
                elem.set_bezpath();
                return Some(());
            }
            _ => return None,
        }
    }
    fn delete_vertices_between(
        &mut self,
        eid_sel: EUId,
        vid_sel: VUId,
        eid_hig: EUId,
        vid_hig: VUId,
    ) -> Option<()> {
        if eid_sel != eid_hig {
            return None;
        }
        let elem = self.get_element_mut(eid_sel)?;
        let n = elem.get_vertices().len() as i64;

        let idx_sel = elem.get_vertices().get_idx(&vid_sel)?;
        let idx_hig = elem.get_vertices().get_idx(&vid_hig)?;

        match elem.get_shape_type() {
            ClosedShapeType::PolyRectangle => {
                (n >= 6).then_some(())?;
                elem.get_vertices().dist_ok(idx_sel, idx_hig, 3)?;
                let max_idx = idx_sel.max(idx_hig);
                let min_idx = idx_sel.min(idx_hig);
                if min_idx == 0 && max_idx == n - 3 {
                    elem.get_vertices_mut().remove(&(n - 1));
                    elem.get_vertices_mut().remove(&(n - 2));
                    log!("Removing vertices {} and {}", n - 1, n - 2);
                } else {
                    if min_idx == 1 && max_idx == n - 2 {
                        elem.get_vertices_mut().remove(&(n - 1));
                        elem.get_vertices_mut().remove(&0);
                        log!("Removing vertices {} and {}", n - 1, 0);
                    } else {
                        if min_idx == 2 && max_idx == n - 1 {
                            elem.get_vertices_mut().remove(&1);
                            elem.get_vertices_mut().remove(&0);
                            log!("Removing vertices {} and {}", 0, 1);
                        } else {
                            elem.get_vertices_mut().remove(&(min_idx + 2));
                            elem.get_vertices_mut().remove(&(min_idx + 1));
                            log!("Removing vertices {} and {}", min_idx + 1, min_idx + 2);
                        }
                    }
                }
                let posvhig = elem.get_vertices().val_from_key(vid_hig)?.curr;
                let vsel = elem.get_vertices_mut().val_mut_from_key(vid_sel)?;
                if idx_sel % 2 == 0 {
                    vsel.curr.y = posvhig.y;
                } else {
                    vsel.curr.x = posvhig.x;
                }
                elem.set_bezpath();
                Some(())
            }
            ClosedShapeType::Polygon => {
                (n >= 4).then_some(())?;
                elem.get_vertices().dist_ok(idx_sel, idx_hig, 2)?;
                let max_idx = idx_sel.max(idx_hig);
                let min_idx = idx_sel.min(idx_hig);
                if min_idx == 0 && max_idx == n - 2 {
                    elem.get_vertices_mut().remove(&(n - 1));
                } else {
                    if min_idx == 1 && max_idx == n - 1 {
                        elem.get_vertices_mut().remove(&0);
                    } else {
                        elem.get_vertices_mut().remove(&((min_idx + max_idx) / 2));
                    }
                }
                elem.set_bezpath();
                Some(())
            }
            _ => None,
        }
    }
    pub fn element_select_vertex(&mut self, userui: &UserUI) -> bool {
        let mut vsel_new = None;
        for (eid, element) in self.set.iter_mut() {
            if let Some(vid_sel) = element.select_vertex(userui) {
                self.set_selected.clear();
                vsel_new = Some((*eid, vid_sel));
            }
        }

        match (self.elem_vertex_selected, vsel_new) {
            (None, None) => return false,
            (None, Some(vselnew)) => {
                self.elem_vertex_selected = Some(vselnew);
                return true;
            }
            (Some(_), None) => {
                self.elem_vertex_selected = None;
                return false;
            }
            (Some(vsel), Some(vselnew)) => {
                match (
                    userui.keys_states.ctrl_cmd_pressed,
                    userui.keys_states.shift_pressed,
                ) {
                    (false, false) => {
                        self.elem_vertex_selected = Some(vselnew);
                        return true;
                    }
                    (false, true) => {
                        if self.bind_unbind_vertices(vsel.0, vsel.1, vselnew.0, vselnew.1) {
                            return true;
                        } else {
                            self.elem_vertex_selected = None;
                            return true;
                        }
                    }
                    (true, false) => {
                        self.create_vertices_between(vsel.0, vsel.1, vselnew.0, vselnew.1);
                        return true;
                    }
                    (true, true) => {
                        self.delete_vertices_between(vsel.0, vsel.1, vselnew.0, vselnew.1);
                        return true;
                    }
                }
            }
        }
    }
    pub fn element_highlight_vertex(&mut self, userui: &UserUI) -> bool {
        self.elem_vertex_highlighted = None;
        for (eid, element) in self.set.iter_mut() {
            if let Some(vid_sel) = element.highlight_vertex(userui) {
                self.elem_vertex_highlighted = Some((*eid, vid_sel));
            }
        }
        return self.elem_vertex_highlighted.is_some();
    }
    pub fn select_elements(&mut self, userui: &UserUI) {
        // Select the nodes whose element contains the position
        if !userui.keys_states.shift_pressed {
            let mut nodes_selected = HashSet::new();
            for (id, node) in &self.set {
                if node.contains(userui.draw_pos) {
                    nodes_selected.insert(*id);
                }
            }
            if nodes_selected.len() > 0 {
                self.elem_selector
                    .refresh_selectable_elems(nodes_selected.clone());
                if let Some(id) = self.elem_selector.next_selection() {
                    self.set_selected.clear();
                    self.set_selected.insert(id);
                }
            } else {
                self.set_selected.clear();
            }
        } else {
            // Shift pressed, add to selection
            for (id, node) in &self.set {
                if node.contains(userui.draw_pos) {
                    self.set_selected.insert(*id);
                }
            }
        }
    }
    pub fn delete_selected_elements(&mut self) {
        for eid in &self.set_selected {
            self.set.remove(eid);
        }
        self.set_highlighted.clear();
        self.set_selected.clear();
        self.elem_vertex_selected = None;
        self.elem_vertex_highlighted = None;
        self.elem_selector.refresh_selectable_elems(HashSet::new());
    }

    pub fn highlight_elements(&mut self, userui: &UserUI) {
        // Select the nodes whose element contains the position
        self.set_highlighted.clear();
        for (id, node) in &self.set {
            if node.contains(userui.draw_pos) {
                self.set_highlighted.insert(*id);
            }
        }
    }
    pub fn save_elements_positions(&mut self) {
        for (_, node) in self.set.iter_mut() {
            node.save_vertices_positions();
        }
    }

    pub fn move_elements(&mut self, userui: &UserUI) -> bool {
        let delta = userui.pointer.curr - userui.pointer.saved;
        let mut moved = false;
        let mut sel_and_bind = self.set_selected.clone();

        if !userui.keys_states.shift_pressed {
            for eid in &self.set_selected {
                if let Some(_) = self.set.get_mut(eid) {
                    sel_and_bind.extend(self.get_binded_elements(*eid));
                }
            }
            let mut other_binds: HashSet<EUId> = HashSet::new();
            for eid in self.set.keys() {
                if let Some(s) = self.set.get(eid) {
                    s.get_binded_elements().iter().for_each(|bind_eid| {
                        if sel_and_bind.contains(bind_eid) {
                            other_binds.extend(s.get_binded_elements().iter());
                        }
                    });
                }
            }
            sel_and_bind.extend(other_binds);
        }
        // Move all selected elements and their binded elements
        for eid in sel_and_bind {
            if let Some(e) = self.set.get_mut(&eid) {
                e.move_drawable(delta);
                moved = true;
            }
        }
        moved
    }
    pub fn get_binded_elements(&self, eid: EUId) -> HashSet<EUId> {
        if let Some(element) = self.get_element(eid) {
            return element.get_binded_elements();
        }
        HashSet::new()
    }
    pub fn bind_unbind_vertices(
        &mut self,
        eid_sel: EUId,
        vid_sel: VUId,
        eid_hig: EUId,
        vid_hig: VUId,
    ) -> bool {
        // Ensure vertices belong to different elements
        if eid_sel == eid_hig {
            log!("Cannot bind vertices from the same element");
            return false;
        }

        let old_bind_sel = self
            .get_element(eid_sel)
            .and_then(|elem| elem.get_vertex(&vid_sel))
            .map(|vertex| vertex.bind.clone())
            .unwrap_or_else(HashSet::new); // Default to empty HashSet if not found

        let old_bind_hig = self
            .get_element(eid_hig)
            .and_then(|elem| elem.get_vertex(&vid_hig))
            .map(|vertex| vertex.bind.clone())
            .unwrap_or_else(HashSet::new);

        let unbound = if old_bind_sel.contains(&(eid_hig, vid_hig))
            && old_bind_hig.contains(&(eid_sel, vid_sel))
        {
            true
        } else {
            false
        };
        if let Some(elem_sel) = self.get_element_mut(eid_sel) {
            if let Some(vertex_sel) = elem_sel.get_vertex_mut(&vid_sel) {
                if unbound {
                    log!("Unbind {} to {}", vid_hig, vid_sel);
                    vertex_sel.bind.remove(&(eid_hig, vid_hig));
                } else {
                    log!("Binding {} to {}", vid_hig, vid_sel);
                    vertex_sel.bind.insert((eid_hig, vid_hig));
                }
            }
        }
        if let Some(elem_hig) = self.get_element_mut(eid_hig) {
            if let Some(vertex_hig) = elem_hig.get_vertex_mut(&vid_hig) {
                if unbound {
                    log!("Unbind {} to {}", vid_sel, vid_hig);
                    vertex_hig.bind.remove(&(eid_sel, vid_sel));
                } else {
                    log!("Binding {} to {}", vid_sel, vid_hig);
                    vertex_hig.bind.insert((eid_sel, vid_sel));
                }
            }
        }
        true
    }
}

// #[derive(Debug)]
// pub struct Elem<E: Clone> {
//     pub id: EUId,
//     pub name: String,
//     pub elem: E,
// }
// impl<E: Clone> Elem<E> {
//     pub fn new_element(name: &str, elem: E) -> Self {
//         Elem {
//             id: EUId::new(),
//             name: name.to_string(),
//             elem,
//         }
//     }
// }
// impl<E: Drawable> Elem<E> {
//     pub fn move_rel(&mut self, delta: Vec2) {
//         self.elem.move_all_vertices(delta);
//     }
// }
// impl<E: Clone> Clone for Elem<E> {
//     fn clone(&self) -> Self {
//         Elem {
//             id: EUId::new(),
//             name: self.name.clone(),
//             elem: self.elem.clone(),
//         }
//     }
// }
// impl<E: Clone> Hash for Elem<E> {
//     fn hash<H: Hasher>(&self, state: &mut H) {
//         self.id.hash(state);
//     }
// }
// impl<E: Clone> PartialEq for Elem<E> {
//     fn eq(&self, other: &Self) -> bool {
//         self.id == other.id
//     }
// }
// impl<E: Clone> Eq for Elem<E> {}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct ElemSelector {
    selectable_elems: Vec<EUId>, // IDs of selectable nodes
    current_index: usize,        // Current index in the list
}
#[allow(dead_code)]
impl ElemSelector {
    pub fn new() -> Self {
        Self {
            selectable_elems: Vec::new(),
            current_index: 0,
        }
    }
    pub fn refresh_selectable_elems(&mut self, new_elems: HashSet<EUId>) {
        let current_set: HashSet<_> = self.selectable_elems.iter().cloned().collect();
        // Compare the sets, ignoring order
        if current_set != new_elems {
            // Reset if the set of nodes changes
            self.selectable_elems = new_elems.into_iter().collect();
            self.current_index = 0;
        }
    }
    pub fn next_selection(&mut self) -> Option<EUId> {
        if self.selectable_elems.is_empty() {
            return None;
        }
        // Select the current node and move to the next
        let selected = self.selectable_elems[self.current_index];
        self.current_index = (self.current_index + 1) % self.selectable_elems.len();
        Some(selected)
    }
}
