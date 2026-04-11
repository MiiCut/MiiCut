use crate::app::AppVars;
use crate::dom::Tabs;
use crate::shape::{GeneralShape, ShapeType};
use crate::types::others::{EUId, VUId};
use std::collections::{HashMap, HashSet};

impl AppVars {
    pub(crate) fn esc_pressed(&mut self) {
        self.element_on_creation = None;
        self.go_to_arrow_tool();
    }

    pub(crate) fn ctrl_c_pressed(&mut self) {
        if let ShapeType::Arrow = self.icon_selected {
            let canvas_user = self.get_active_canvas_mut();
            let selected_ids: HashSet<EUId> =
                canvas_user.dataset.shapes_selected.iter().copied().collect();
            let mut seen = HashSet::new();
            let mut items = Vec::new();

            fn collect_group_children(
                dataset: &crate::shapes::DataSet,
                selected_ids: &HashSet<EUId>,
                seen: &mut HashSet<EUId>,
                group_id: EUId,
                items: &mut Vec<crate::clipboard::ClipboardItem>,
            ) {
                let mut stack = vec![group_id];
                while let Some(current) = stack.pop() {
                    let children = dataset
                        .shapes
                        .get(&current)
                        .or_else(|| dataset.grouped_shapes.get(&current))
                        .and_then(|shape| shape.get_group_children())
                        .map(|children| children.to_vec())
                        .unwrap_or_default();
                    for child_id in children {
                        if !seen.insert(child_id) {
                            continue;
                        }
                        if let Some(child) = dataset.grouped_shapes.get(&child_id).cloned() {
                            let is_child = !selected_ids.contains(&child_id);
                            let is_group = child.is_group();
                            items.push(crate::clipboard::ClipboardItem {
                                id: child_id,
                                shape: child,
                                is_child,
                            });
                            if is_group {
                                stack.push(child_id);
                            }
                        }
                    }
                }
            }

            for eid in selected_ids.iter().copied() {
                if !seen.insert(eid) {
                    continue;
                }
                if let Some(shape) = canvas_user.dataset.shapes.get(&eid).cloned() {
                    let is_child = false;
                    items.push(crate::clipboard::ClipboardItem {
                        id: eid,
                        shape: shape.clone(),
                        is_child,
                    });
                    if shape.is_group() {
                        collect_group_children(
                            &canvas_user.dataset,
                            &selected_ids,
                            &mut seen,
                            eid,
                            &mut items,
                        );
                    }
                } else if let Some(shape) = canvas_user.dataset.grouped_shapes.get(&eid).cloned() {
                    let is_child = !selected_ids.contains(&eid);
                    items.push(crate::clipboard::ClipboardItem {
                        id: eid,
                        shape: shape.clone(),
                        is_child,
                    });
                    if shape.is_group() {
                        collect_group_children(
                            &canvas_user.dataset,
                            &selected_ids,
                            &mut seen,
                            eid,
                            &mut items,
                        );
                    }
                }
            }

            if !items.is_empty() {
                canvas_user
                    .clipboard
                    .copy(items, canvas_user.get_user_ui().pointer.clone());
                log!("Copying selected elements to clipboard");
            }
        }
    }

    pub(crate) fn ctrl_v_pressed(&mut self) {
        if let ShapeType::Arrow = self.icon_selected {
            let canvas_user = self.get_active_canvas_mut();
            let before = canvas_user.snapshot_draw_state();
            if let Some(pasted) = canvas_user
                .clipboard
                .make_paste(&canvas_user.get_user_ui().pointer)
            {
                let mut id_map: HashMap<EUId, EUId> = HashMap::new();
                let mut children = Vec::new();

                for item in pasted.iter() {
                    if item.is_child {
                        let new_id = EUId::new();
                        id_map.insert(item.id, new_id);
                        children.push((new_id, item.shape.clone()));
                    }
                }

                for (new_id, shape) in children {
                    canvas_user.dataset.grouped_shapes.insert(new_id, shape);
                }

                for item in pasted.iter() {
                    if !item.is_child {
                        let new_id = canvas_user.dataset.push_element(item.shape.clone());
                        id_map.insert(item.id, new_id);
                    }
                }

                for (_old_id, new_id) in id_map.iter() {
                    let shape = canvas_user
                        .dataset
                        .shapes
                        .get_mut(new_id)
                        .or_else(|| canvas_user.dataset.grouped_shapes.get_mut(new_id));
                    let Some(shape) = shape else {
                        continue;
                    };
                    let Some(children_ids) = shape.get_group_children() else {
                        continue;
                    };
                    let new_children: Vec<EUId> = children_ids
                        .iter()
                        .filter_map(|child| id_map.get(child).copied())
                        .collect();
                    shape.set_group_children(new_children);
                }

                canvas_user.dataset.mark_final_polygon_dirty();
                canvas_user.dataset.calc_final_polygon();
                canvas_user.push_history_action(before);
            }
        }
    }

    pub(crate) fn del_back_pressed(&mut self) {
        if let ShapeType::Arrow = self.icon_selected {
            // Delete selected user dimension first if any
            if let Some(dim_id) = self.user_dim_selected.take() {
                let canvas_user = self.get_active_canvas_mut();
                canvas_user.dataset.remove_user_dim(dim_id);
                return;
            }
            let canvas_user = self.get_active_canvas_mut();
            if canvas_user.dataset.delete_selected_elements() {
                canvas_user.dataset.refresh_svg_cache();
                canvas_user.dataset.mark_final_polygon_dirty();
                canvas_user.dataset.calc_final_polygon();
                canvas_user.dataset.cleanup_user_dims();
                if canvas_user.dataset.shapes.is_empty() {
                    self.clear_toolpath_gcode();
                }
            } else {
                let vs_sel: Vec<(EUId, VUId)> = canvas_user
                    .dataset
                    .vertex_selected
                    .iter()
                    .copied()
                    .collect();
                if vs_sel.len() == 1 {
                    canvas_user.dataset.delete_vertex(vs_sel[0].0, vs_sel[0].1);
                    canvas_user.dataset.mark_final_polygon_dirty();
                    canvas_user.dataset.calc_final_polygon();
                    canvas_user.dataset.cleanup_user_dims();
                }
            }
        }
    }

    pub(crate) fn group_toggle_pressed(&mut self) {
        if let ShapeType::Arrow = self.icon_selected {
            let canvas_user = self.get_active_canvas_mut();
            let elems_sel: Vec<EUId> = canvas_user
                .dataset
                .shapes_selected
                .iter()
                .copied()
                .collect();
            if elems_sel.len() > 1 {
                if canvas_user.dataset.group_selected().is_some() {
                    canvas_user.dataset.mark_final_polygon_dirty();
                    canvas_user.dataset.calc_final_polygon();
                }
                return;
            }
            if elems_sel.len() == 1 {
                if let Some(elem) = canvas_user.dataset.get_element_mut(elems_sel[0]) {
                    if elem.is_group() {
                        if canvas_user.dataset.ungroup_selected().is_some() {
                            canvas_user.dataset.mark_final_polygon_dirty();
                            canvas_user.dataset.calc_final_polygon();
                        }
                        return;
                    }
                }
            }

            let vs_sel: Vec<(EUId, VUId)> = canvas_user
                .dataset
                .vertex_selected
                .iter()
                .copied()
                .collect();
            let vs_high: Vec<(EUId, VUId)> = canvas_user
                .dataset
                .vertex_highlighted
                .iter()
                .copied()
                .collect();

            if vs_sel.len() == 1 && vs_high.len() == 1 && vs_sel != vs_high {
                let (eid1, vid1) = vs_sel[0];
                let (eid2, vid2) = vs_high[0];
                if eid1 == eid2 {
                    canvas_user
                        .dataset
                        .create_vertices_between(eid1, vid1, eid2, vid2);
                }
            }
        } else {
            self.finalize_poly();
        }
    }

    pub(crate) fn finalize_poly(&mut self) -> bool {
        if let ShapeType::Poly = self.icon_selected {
            if let Some((_, ref vs)) = self.element_on_creation {
                if vs.len() < 3 {
                    // Less than 3 points — cancel like Esc
                    self.element_on_creation = None;
                    self.go_to_arrow_tool();
                    return true;
                }
            }
            let el_on_creation = self.element_on_creation.clone();
            let canvas_user = self.get_active_canvas_mut();
            if let Some((_, vs)) = el_on_creation {
                if let Some(e) = GeneralShape::new_shape_poly(vs, 0) {
                    let eid = canvas_user.dataset.push_element(e);
                    canvas_user.dataset.select_only(eid);
                    canvas_user.dataset.mark_final_polygon_dirty();
                    canvas_user.dataset.calc_final_polygon();
                    self.element_on_creation = None;
                    self.go_to_arrow_tool();
                    return true;
                }
            }
        }
        false
    }

    pub(crate) fn space_pressed(&mut self) {
        if let ShapeType::Arrow = self.icon_selected {
            let canvas_user = self.get_active_canvas_mut();
            let elems_sel: Vec<EUId> = canvas_user
                .dataset
                .shapes_selected
                .iter()
                .copied()
                .collect();
            if elems_sel.len() == 1 {
                if let Some(elem) = canvas_user.dataset.get_element_mut(elems_sel[0]) {
                    elem.op_next();
                    canvas_user.dataset.mark_final_polygon_dirty();
                    canvas_user.dataset.calc_final_polygon();
                    return;
                }
            }

            let vs_sel: Vec<(EUId, VUId)> = canvas_user
                .dataset
                .vertex_selected
                .iter()
                .copied()
                .collect();
            let vs_high: Vec<(EUId, VUId)> = canvas_user
                .dataset
                .vertex_highlighted
                .iter()
                .copied()
                .collect();

            if vs_sel.len() == 1 && vs_high.len() == 1 && vs_sel != vs_high {
                let (eid1, vid1) = vs_sel[0];
                let (eid2, vid2) = vs_high[0];
                if eid1 == eid2 {
                    canvas_user
                        .dataset
                        .create_vertices_between(eid1, vid1, eid2, vid2);
                }
                return;
            }

            if vs_sel.len() == 1 {
                let (eid, vid) = vs_sel[0];
                if let Some(elem) = canvas_user.dataset.get_element_mut(eid) {
                    let corner_idx = elem.get_vertices().get_idx(&vid).map(|i| i as usize);
                    if let Some(v) = elem.get_vertex_mut(&vid) {
                        v.change_apex_type();
                    }
                    if let Some(idx) = corner_idx {
                        elem.reset_adjacent_sags(idx);
                    }
                    elem.set_bezpath();
                    canvas_user.dataset.mark_final_polygon_dirty();
                    canvas_user.dataset.calc_final_polygon();
                    canvas_user.dataset.cleanup_user_dims();
                }
            }
        } else {
            self.finalize_poly();
        }
    }

    pub(crate) fn arrow_up_pressed(&mut self) {
        if matches!(self.active_view, Tabs::Draw) {
            let canvas = self.get_active_canvas_mut();
            if canvas.dataset.shapes_selected.len() == 1 && canvas.dataset.vertex_selected.is_none()
            {
                let eid = *canvas.dataset.shapes_selected.iter().next().unwrap();
                if canvas.dataset.shift_order(eid, -1) {
                    canvas.dataset.mark_final_polygon_dirty();
                    canvas.dataset.calc_final_polygon();
                    self.refresh_toolpath_cache();
                    self.refresh_gcode_cache();
                }
                return;
            }
        }
        self.inc_vertex_radius();
    }

    pub(crate) fn arrow_down_pressed(&mut self) {
        if matches!(self.active_view, Tabs::Draw) {
            let canvas = self.get_active_canvas_mut();
            if canvas.dataset.shapes_selected.len() == 1 && canvas.dataset.vertex_selected.is_none()
            {
                let eid = *canvas.dataset.shapes_selected.iter().next().unwrap();
                if canvas.dataset.shift_order(eid, 1) {
                    canvas.dataset.mark_final_polygon_dirty();
                    canvas.dataset.calc_final_polygon();
                    self.refresh_toolpath_cache();
                    self.refresh_gcode_cache();
                }
                return;
            }
        }
        self.dec_vertex_radius();
    }
}
