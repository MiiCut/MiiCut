use crate::dom::Icons;
use crate::inputs::UserUI;
use crate::math::{geo_multipolygon_to_bez_paths, snap_vertex};
use crate::shape::{ClosedShape, Operation};
use crate::types::{EUId, Snap, VUId, Value};
use geo::algorithm::unary_union;
use geo::{BooleanOps, MultiPolygon};
use kurbo::BezPath;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct DataSet {
    pub shapes: HashMap<EUId, ClosedShape>,
    pub shapes_selected: HashSet<EUId>,
    pub shapes_highlighted: HashSet<EUId>,
    pub shapes_selector: ShapeSelector,
    pub vertices_selected: HashSet<(EUId, VUId)>,
    pub vertices_highlighted: HashSet<(EUId, VUId)>,
    pub last_vertex_selected: Option<(EUId, VUId)>,

    pub final_polygon: MultiPolygon<f64>,
    pub final_paths: Vec<BezPath>,
}
impl DataSet {
    pub fn new() -> Self {
        DataSet {
            shapes: HashMap::new(),
            shapes_selected: HashSet::new(),
            shapes_highlighted: HashSet::new(),
            shapes_selector: ShapeSelector::new(),
            vertices_selected: HashSet::new(),
            vertices_highlighted: HashSet::new(),
            last_vertex_selected: None,
            final_polygon: MultiPolygon::new(vec![]),
            final_paths: Vec::new(),
        }
    }
    pub fn push_element(&mut self, elem: ClosedShape) {
        self.shapes.insert(EUId::new(), elem);
    }
    pub fn pop_element(&mut self, eid: EUId) -> Option<ClosedShape> {
        // remove the shape
        let removed = self.shapes.remove(&eid);

        if removed.is_some() {
            // clear element-level state
            self.shapes_selected.remove(&eid);
            self.shapes_highlighted.remove(&eid);

            // drop all (EUId, VUId) matching this EUId
            self.vertices_selected
                .retain(|(sel_eid, _)| sel_eid != &eid);
            self.vertices_highlighted
                .retain(|(high_eid, _)| high_eid != &eid);
            // also for the last
            if let Some((last_eid, _)) = self.last_vertex_selected {
                if last_eid == eid {
                    self.last_vertex_selected = None;
                }
            }
        }

        removed
    }
    pub fn get_element(&self, eid: EUId) -> Option<&ClosedShape> {
        self.shapes.get(&eid)
    }
    pub fn get_element_mut(&mut self, eid: EUId) -> Option<&mut ClosedShape> {
        self.shapes.get_mut(&eid)
    }

    pub fn create_vertices_between(
        &mut self,
        eid1_sel: EUId,
        vid1_sel: VUId,
        eid2_sel: EUId,
        vid2_sel: VUId,
    ) -> Option<()> {
        if eid1_sel != eid2_sel {
            return None;
        }
        let elem = self.get_element_mut(eid1_sel)?;
        elem.get_vertices().dist_ok(
            elem.get_vertices().get_idx(&vid1_sel)?,
            elem.get_vertices().get_idx(&vid2_sel)?,
            1,
        )?;
        let v1_sel = elem.get_vertex(&vid1_sel)?;
        let v2_sel = elem.get_vertex(&vid2_sel)?;

        match elem.get_shape_type() {
            Icons::Poly => {
                // Create new vertex between the selected and highlighted vertices
                let new_v = (
                    VUId::new(),
                    Value::new(snap_vertex((v1_sel.curr + v2_sel.curr) / 2.0, Snap::new())),
                );
                elem.get_vertices_mut()
                    .insert_one_between(&vid1_sel, &vid2_sel, new_v);
                elem.set_bezpath();
                return Some(());
            }
            _ => return None,
        }
    }
    pub fn delete_vertex(&mut self, eid_sel: EUId, vid_sel: VUId) -> bool {
        if let Some(elem) = self.get_element_mut(eid_sel) {
            match elem.get_shape_type() {
                Icons::Poly => {
                    if elem.get_vertices().len() < 4 {
                        return false;
                    }
                    if let Some(idx_sel) = elem.get_vertices().get_idx(&vid_sel) {
                        elem.get_vertices_mut().remove(&idx_sel);
                        elem.set_bezpath();
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub fn select_vertices(&mut self, userui: &UserUI) -> bool {
        let mut selection_changed = false;
        for (eid, element) in self.shapes.iter_mut() {
            if let Some(vid_sel) = element.select_vertex(userui) {
                // Check if element was already in the vertices_selected set
                if self.vertices_selected.contains(&(*eid, vid_sel)) {
                    // Element is already selected, remove it
                    self.vertices_selected.remove(&(*eid, vid_sel));
                    self.last_vertex_selected = None;
                    selection_changed = true;
                } else {
                    self.vertices_selected.insert((*eid, vid_sel));
                    self.last_vertex_selected = Some((*eid, vid_sel));
                    selection_changed = true;
                }
            }
        }
        if !selection_changed {
            self.vertices_selected.clear();
            self.last_vertex_selected = None;
        } else {
            self.shapes_selected.clear();
        }
        selection_changed

        // match (self.vertices_selected, vsel_new) {
        //     (None, None) => return false,
        //     (None, Some(vselnew)) => {
        //         self.vertices_selected = Some(vselnew);
        //         return true;
        //     }
        //     (Some(_), None) => {
        //         self.vertices_selected = None;
        //         return false;
        //     }
        //     (Some(vsel), Some(vselnew)) => {
        //         match (
        //             userui.keys_states.ctrl_cmd_pressed,
        //             userui.keys_states.shift_pressed,
        //         ) {
        //             (false, false) => {
        //                 self.vertices_selected = Some(vselnew);
        //                 return true;
        //             }
        //             (false, true) => {
        //                 if self.bind_unbind_vertices(vsel.0, vsel.1, vselnew.0, vselnew.1) {
        //                     return true;
        //                 } else {
        //                     self.vertices_selected = None;
        //                     return true;
        //                 }
        //             }
        //             (true, false) => {
        //                 self.create_vertices_between(vsel.0, vsel.1, vselnew.0, vselnew.1);
        //                 return true;
        //             }
        //             (true, true) => {
        //                 self.delete_vertices_between(vsel.0, vsel.1, vselnew.0, vselnew.1);
        //                 return true;
        //             }
        //         }
        //     }
        // }
    }
    pub fn highlight_vertices(&mut self, userui: &UserUI) -> bool {
        self.vertices_highlighted.clear();
        let mut highlight_changed = false;
        for (eid, element) in self.shapes.iter_mut() {
            if let Some(vid_sel) = element.highlight_vertex(userui) {
                self.vertices_highlighted.insert((*eid, vid_sel));
                highlight_changed = true;
            }
        }
        highlight_changed
    }

    pub fn select_elements(&mut self, userui: &UserUI) {
        // Select the nodes whose element contains the position
        if !userui.keys_states.shift_pressed {
            let mut shapes_selected = HashSet::new();
            for (eid, elem) in &self.shapes {
                if elem.contains(userui.draw_pos) {
                    shapes_selected.insert(*eid);
                }
            }
            if shapes_selected.len() > 0 {
                self.shapes_selector
                    .refresh_selectable_elems(shapes_selected.clone());
                if let Some(eid) = self.shapes_selector.next_selection() {
                    self.shapes_selected.clear();
                    self.shapes_selected.insert(eid);
                }
            } else {
                self.shapes_selected.clear();
            }
        } else {
            // Shift pressed, add to selection
            for (id, node) in &self.shapes {
                if node.contains(userui.draw_pos) {
                    self.shapes_selected.insert(*id);
                }
            }
        }
    }
    pub fn highlight_elements(&mut self, userui: &UserUI) {
        self.shapes_highlighted.clear();
        for (eid, elem) in &self.shapes {
            if elem.contains(userui.draw_pos) {
                self.shapes_highlighted.insert(*eid);
            }
        }
    }
    pub fn delete_selected_elements(&mut self) -> bool {
        let mut deleted = false;
        for eid in &self.shapes_selected.clone() {
            if let Some(_) = self.shapes.remove(eid) {
                self.shapes_highlighted.remove(eid);
                self.shapes_selected.remove(eid);
                self.vertices_selected.clear();
                self.vertices_highlighted.clear();
                self.shapes_selector
                    .refresh_selectable_elems(HashSet::new());
                deleted = true;
            }
        }
        deleted
    }

    pub fn save_elements_positions(&mut self) {
        for (_, elem) in self.shapes.iter_mut() {
            elem.save_vertices_positions();
        }
    }

    pub fn move_elements(&mut self, userui: &UserUI) -> bool {
        let delta = userui.pointer.curr - userui.pointer.saved;
        let mut moved = false;
        let mut sel_and_bind = self.shapes_selected.clone();

        if !userui.keys_states.shift_pressed {
            for eid in &self.shapes_selected {
                if let Some(_) = self.shapes.get_mut(eid) {
                    sel_and_bind.extend(self.get_binded_elements(*eid));
                }
            }
            let mut other_binds: HashSet<EUId> = HashSet::new();
            for eid in self.shapes.keys() {
                if let Some(s) = self.shapes.get(eid) {
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
            if let Some(e) = self.shapes.get_mut(&eid) {
                e.move_shape(delta);
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

    pub fn calc_final_polygon(&mut self) {
        let mut unions: Vec<geo::Polygon<f64>> = Vec::new();
        let mut diffs: Vec<geo::Polygon<f64>> = Vec::new();
        for s in self.shapes.values() {
            match s.get_operation() {
                Operation::Union => unions.push(s.get_polygon().clone()),
                Operation::Difference => diffs.push(s.get_polygon().clone()),
            }
        }
        let poly_union = unary_union(&unions);
        let poly_diff = unary_union(&diffs);
        self.final_polygon = if diffs.len() > 0 {
            poly_union.boolean_op(&poly_diff, geo::OpType::Difference)
        } else {
            poly_union
        };
        self.calc_final_paths();
    }
    fn calc_final_paths(&mut self) {
        self.final_paths = geo_multipolygon_to_bez_paths(&self.final_polygon);
    }
    pub fn get_final_paths(&self) -> &Vec<BezPath> {
        &self.final_paths
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeSelector {
    selectable_elems: Vec<EUId>, // IDs of selectable elements
    current_index: usize,        // Current index in the list
}
#[allow(dead_code)]
impl ShapeSelector {
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
