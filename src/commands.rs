use crate::app::AppVars;
use crate::dom::Tabs;
use crate::shape::{GeneralShape, ShapeType};
use crate::types::others::{EUId, VUId};

impl AppVars {
    pub(crate) fn esc_pressed(&mut self) {
        self.element_on_creation = None;
        self.go_to_arrow_tool();
    }

    pub(crate) fn ctrl_c_pressed(&mut self) {
        if let ShapeType::Arrow = self.icon_selected {
            let canvas_user = self.get_active_canvas_mut();
            if canvas_user.dataset.shapes_selected.len() == 1 {
                let eid = *canvas_user.dataset.shapes_selected.iter().next().unwrap();
                if let Some(elem) = canvas_user.dataset.get_element(eid) {
                    canvas_user
                        .clipboard
                        .copy(elem.clone(), canvas_user.get_user_ui().pointer.clone());
                    log!("Copying selected element to clipboard");
                }
            }
        }
    }

    pub(crate) fn ctrl_v_pressed(&mut self) {
        if let ShapeType::Arrow = self.icon_selected {
            let canvas_user = self.get_active_canvas_mut();
            if let Some(pasted) = canvas_user
                .clipboard
                .make_paste(&canvas_user.get_user_ui().pointer)
            {
                let _ = canvas_user.dataset.push_element(pasted);
                canvas_user.dataset.mark_final_polygon_dirty();
                canvas_user.dataset.calc_final_polygon();
            }
        }
    }

    pub(crate) fn del_back_pressed(&mut self) {
        if let ShapeType::Arrow = self.icon_selected {
            let canvas_user = self.get_active_canvas_mut();
            if canvas_user.dataset.delete_selected_elements() {
                canvas_user.dataset.refresh_svg_cache();
                canvas_user.dataset.mark_final_polygon_dirty();
                canvas_user.dataset.calc_final_polygon();
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
        } else if let ShapeType::Poly = self.icon_selected {
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
                }
            }
        }
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
                    if let Some(v) = elem.get_vertex_mut(&vid) {
                        v.change_apex_type();
                        elem.set_bezpath();
                        canvas_user.dataset.mark_final_polygon_dirty();
                        canvas_user.dataset.calc_final_polygon();
                    }
                }
            }
        } else if let ShapeType::Poly = self.icon_selected {
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
                }
            }
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
