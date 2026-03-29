use crate::{
    app::{set_callback, AppVars, DimDrag, RefAV, RotationDrag, SelectionWindow},
    canvas::{CanvasKind, Color, Pattern},
    dimensions::dim_hv,
    dom::{
        add_property_number_input, add_property_section, add_property_text_input,
        add_property_two_number_inputs, add_property_value, Tabs,
    },
    helpers::canvas_fit::zoom_canvas_at,
    helpers::prefab::*,
    inputs::{ButtonLevel, MouseButton, SystemMouse, UserAction},
    shape::{GeneralShape, Operation, ShapeType},
    shapes::{toolpath_to_plasma_gcode, Toolpath},
    status::{begin_render, end_render, update_status_bar},
    types::others::{EUId, MyError, Property, PropertyValue, SegBundle},
    view_draw::notes::{
        focus_note_after_create, handle_note_draft_finish, handle_note_draft_move,
        handle_note_draft_start, NoteDraftOutcome,
    },
    view_draw::notes_dom::update_notes_view,
    view_gcode::app::render_gcode_view,
    view_machine::app::render_machine_view,
    view_toolpath::app::render_toolpath_view,
};
use kurbo::{BezPath, Point, Shape, Vec2};
use std::collections::HashMap;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{Document, DragEvent, Element, Event, HtmlElement, MouseEvent, WheelEvent};

impl AppVars {
    pub(crate) fn dec_vertex_radius(&mut self) -> Option<()> {
        if let ShapeType::Arrow = self.icon_selected {
            let canvas_user = self.get_active_canvas_mut();
            let (eid, vid) = canvas_user.dataset.vertex_selected?;
            let elem = canvas_user.dataset.get_element_mut(eid)?;

            elem.get_vertex_mut(&vid)?.dec_radius();

            elem.set_bezpath();
            canvas_user.dataset.mark_final_polygon_dirty();
            canvas_user.dataset.calc_final_polygon();
            return Some(());
        }
        None
    }
    pub(crate) fn inc_vertex_radius(&mut self) -> Option<()> {
        if let ShapeType::Arrow = self.icon_selected {
            let canvas_user = self.get_active_canvas_mut();
            let (eid, vid) = canvas_user.dataset.vertex_selected?;
            let elem = canvas_user.dataset.get_element_mut(eid)?;

            elem.get_vertex_mut(&vid)?.inc_radius();

            elem.set_bezpath();
            canvas_user.dataset.mark_final_polygon_dirty();
            canvas_user.dataset.calc_final_polygon();
            return Some(());
        }
        None
    }

    pub(crate) fn go_to_arrow_tool(&mut self) {
        self.icon_selected = ShapeType::Arrow;
        self.notes.mode = false;
        self.notes.draft = None;
        self.notes.drag = None;
        self.notes.selected = None;
        self.user_icons
            .iter()
            .for_each(|icon| self.html_deselect_icons(*icon));
        self.html_select_icon(ShapeType::Arrow);
        self.html_deselect_note_icon();
    }
    pub(crate) fn select_note_tool(&mut self) {
        self.icon_selected = ShapeType::Arrow;
        self.notes.mode = true;
        self.element_on_creation = None;
        self.notes.drag = None;
        self.notes.selected = None;
        self.user_icons
            .iter()
            .for_each(|icon| self.html_deselect_icons(*icon));
        self.html_deselect_note_icon();
        self.html_select_note_icon();
    }
    pub(crate) fn html_select_note_icon(&self) {
        if let Some(html_element) = self.document.get_element_by_id("icon-note") {
            if let Ok(html_element) = html_element.dyn_into::<HtmlElement>() {
                let _ = html_element.set_attribute("class", "icon icon-selected");
            }
        }
    }
    pub(crate) fn html_deselect_note_icon(&self) {
        if let Some(html_element) = self.document.get_element_by_id("icon-note") {
            if let Ok(html_element) = html_element.dyn_into::<HtmlElement>() {
                let _ = html_element.set_attribute("class", "icon");
            }
        }
    }
    pub(crate) fn html_select_icon(&self, icon: ShapeType) {
        if let Some(html_element) = icon.get_html_element() {
            html_element
                .set_attribute("class", "icon icon-selected")
                .expect("Failed to set class attribute");
        }
    }
    pub(crate) fn html_deselect_icons(&self, icon: ShapeType) {
        if let Some(html_element) = icon.get_html_element() {
            html_element
                .set_attribute("class", "icon")
                .expect("Failed to set class attribute");
        }
    }

    pub(crate) fn set_element_select_vertex(&mut self) -> bool {
        let canvas_user = self.get_active_canvas_mut();
        canvas_user
            .dataset
            .select_vertices(canvas_user.get_user_ui().draw_pos)
    }
    pub(crate) fn set_select_elements(&mut self) {
        self.get_active_canvas_mut().select_elements();
    }
    pub(crate) fn set_highlight_elements(&mut self) {
        self.get_active_canvas_mut().highlight_elements();
    }
    pub(crate) fn set_element_highlight_vertex(&mut self) -> bool {
        self.get_active_canvas_mut().highlight_vertices()
    }
    pub(crate) fn set_move_elements(&mut self) -> bool {
        self.get_active_canvas_mut().move_elements()
    }
    pub(crate) fn set_move_vertices_selected(&mut self) -> Option<()> {
        self.get_active_canvas_mut().move_vertices_selected()
    }

    fn update_selection_window_current(&mut self) -> bool {
        let current = self.get_active_canvas().get_user_ui().draw_pos();
        if let Some(selection_window) = self.selection_window.as_mut() {
            selection_window.current = current;
            return true;
        }
        false
    }

    pub(crate) fn _undo(&mut self) {
        if self.active_canvas != CanvasKind::Draw {
            return;
        }
        let canvas = &mut self.canvases[CanvasKind::Draw.idx()];
        if canvas.undo_history() {
            canvas.dataset.refresh_svg_cache();
            canvas.dataset.mark_final_polygon_dirty();
            canvas.dataset.calc_final_polygon();
            self.refresh_toolpath_cache();
            self.refresh_gcode_cache();
        }
    }
    pub(crate) fn _redo(&mut self) {
        if self.active_canvas != CanvasKind::Draw {
            return;
        }
        let canvas = &mut self.canvases[CanvasKind::Draw.idx()];
        if canvas.redo_history() {
            canvas.dataset.refresh_svg_cache();
            canvas.dataset.mark_final_polygon_dirty();
            canvas.dataset.calc_final_polygon();
            self.refresh_toolpath_cache();
            self.refresh_gcode_cache();
        }
    }
    pub(crate) fn update_draw_cursor(&mut self) {
        let canvas = &mut self.canvases[CanvasKind::Draw.idx()];
        if self.active_canvas != CanvasKind::Draw || !canvas.is_pointer_on_canvas() {
            canvas.set_cursor("default");
            return;
        }
        let draw_pos = canvas.get_user_ui().draw_pos;
        let scale = canvas.get_scale();
        let hit_radius = 6.0 / scale.max(0.001);
        let on_rotation_handle = canvas.dataset.shapes.values().any(|shape| {
            shape
                .rotation_handle_world_pos(scale)
                .map(|p| (p - draw_pos).hypot() < hit_radius)
                .unwrap_or(false)
        });
        if on_rotation_handle {
            canvas.set_cursor("url(\"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAABAAAAAQCAYAAAAf8/9hAAAABGdBTUEAALGPC/xhBQAAACBjSFJNAAB6JgAAgIQAAPoAAACA6AAAdTAAAOpgAAA6mAAAF3CculE8AAAARGVYSWZNTQAqAAAACAABh2kABAAAAAEAAAAaAAAAAAADoAEAAwAAAAEAAQAAoAIABAAAAAEAAAAQoAMABAAAAAEAAAAQAAAAADRVcfIAAAHJaVRYdFhNTDpjb20uYWRvYmUueG1wAAAAAAA8eDp4bXBtZXRhIHhtbG5zOng9ImFkb2JlOm5zOm1ldGEvIiB4OnhtcHRrPSJYTVAgQ29yZSA2LjAuMCI+CiAgIDxyZGY6UkRGIHhtbG5zOnJkZj0iaHR0cDovL3d3dy53My5vcmcvMTk5OS8wMi8yMi1yZGYtc3ludGF4LW5zIyI+CiAgICAgIDxyZGY6RGVzY3JpcHRpb24gcmRmOmFib3V0PSIiCiAgICAgICAgICAgIHhtbG5zOmV4aWY9Imh0dHA6Ly9ucy5hZG9iZS5jb20vZXhpZi8xLjAvIj4KICAgICAgICAgPGV4aWY6Q29sb3JTcGFjZT4xPC9leGlmOkNvbG9yU3BhY2U+CiAgICAgICAgIDxleGlmOlBpeGVsWERpbWVuc2lvbj4yNDwvZXhpZjpQaXhlbFhEaW1lbnNpb24+CiAgICAgICAgIDxleGlmOlBpeGVsWURpbWVuc2lvbj4yNDwvZXhpZjpQaXhlbFlEaW1lbnNpb24+CiAgICAgIDwvcmRmOkRlc2NyaXB0aW9uPgogICA8L3JkZjpSREY+CjwveDp4bXBtZXRhPgoHMv/IAAABPklEQVQ4Ea2Tyy4EQRSGa0gIOyxJXBYWNhJLCysi8QZDrMVY8QTzBCy8AQ/BG2AjIqwtRCQkJBYucf2+7prJzKhOJ+JPvj5V55w+1VV1OoQQKvBndSfenMU3ByPwBY8wDlNwDaXaIOMdnmA5Zm9hn2EXBqIvaWp4H8ACZzAEahJ24BsOIVlkgcAHHMAiVKFTLvAJ9c5AP44TuIDhGEwdrr59uIPRmJeZeZ5vsNbqLBgv4X+FVeNdPtA0WODUSYluiL+At9Qs0MPYvVmkTOZ6/Vlu4wus2geN/TMs1AyRXjhvzRhjcg97UHR4hLIFLrHH4MG3qc7Mbay3efNJFePV2gP2iFf+SzaHCTbLNtg8ymayqXzRJkstgDuXRWxX23Yzd4UVrG1tgVr0NU1qvwb9oW7hCibArxmM8yPsv6nyA/j9OLxiC3kLAAAAAElFTkSuQmCC\") 8 8, auto");
        } else {
            canvas.set_cursor("default");
        }
    }

    pub(crate) fn snap_draw_to_constr_circle_vertices(&mut self) {
        if self.active_canvas != CanvasKind::Draw {
            return;
        }
        let canvas = &mut self.canvases[CanvasKind::Draw.idx()];
        let pos = canvas.get_user_ui().draw_pos;
        let threshold = canvas.get_user_ui().snap.linear().max(5.0);
        let mut best: Option<Vec2> = None;
        let mut best_dist = f64::INFINITY;
        for shape in canvas.dataset.shapes.values() {
            match shape.get_shape_type() {
                ShapeType::ConstrCircle => {
                    for (idx, (_, vertex)) in shape.get_vertices().iter().enumerate() {
                        if idx < 2 {
                            continue;
                        }
                        let dist = (pos - vertex.curr()).hypot();
                        if dist <= threshold && dist < best_dist {
                            best_dist = dist;
                            best = Some(vertex.curr());
                        }
                    }
                }
                ShapeType::ConstrLine => {
                    for (_, vertex) in shape.get_vertices().iter() {
                        let dist = (pos - vertex.curr()).hypot();
                        if dist <= threshold && dist < best_dist {
                            best_dist = dist;
                            best = Some(vertex.curr());
                        }
                    }
                }
                _ => {}
            }
        }
        let user_ui = canvas.get_user_ui_mut();
        if let Some(target) = best {
            user_ui.draw_pos = target;
            user_ui.pointer.set_curr(target);
            user_ui.magnetized = true;
        } else {
            user_ui.magnetized = false;
        }
    }
}

pub(crate) fn update_context_help(av: &RefAV) -> Option<()> {
    let document = av.borrow().document.clone();
    let bar_el: Element = document.get_element_by_id("context-bar")?;
    let bar: HtmlElement = bar_el.dyn_into::<HtmlElement>().ok()?;

    if !matches!(av.borrow().active_view, Tabs::Draw) {
        bar.set_inner_html("");
        return Some(());
    }

    let lines: Vec<String> = {
        let avb = av.borrow();
        let canvas = &avb.canvases[CanvasKind::Draw.idx()];
        let mut lines = Vec::new();

        if let Some((shape_type, _)) = avb.element_on_creation.as_ref() {
            if matches!(shape_type, ShapeType::Poly) {
                lines.push("Right-click to finish".to_string());
            } else {
                lines.push("Click to place the second point".to_string());
            }
        } else if matches!(avb.icon_selected, ShapeType::Poly) {
            lines.push("Click to add points".to_string());
            lines.push("Right-click to finish".to_string());
        } else if matches!(avb.icon_selected, ShapeType::Arrow) {
            if canvas.dataset.vertex_selected.is_some() {
                lines.push("Space: change apex type".to_string());
                lines.push("↑/↓ change radius".to_string());
            } else if canvas.dataset.shapes_selected.len() > 1 {
                lines.push("Enter: group".to_string());
            } else if canvas.dataset.shapes_selected.len() == 1 {
                if let Some(eid) = canvas.dataset.shapes_selected.iter().next() {
                    if let Some(shape) = canvas.dataset.shapes.get(eid) {
                        if shape.is_group() {
                            lines.push("Enter: ungroup".to_string());
                        } else {
                            lines.push("Space: toggle Union/Diff".to_string());
                            lines.push("↑/↓ change order".to_string());
                        }
                    }
                }
            }
        }

        if lines.is_empty() {
            lines.push("Alt: preview".to_string());
        }

        lines
    };

    bar.set_inner_html("");
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            let sep = document.create_element("span").ok()?;
            sep.set_class_name("context-bar-sep");
            sep.set_text_content(Some("|"));
            let _ = bar.append_child(&sep);
        }
        let item = document.create_element("span").ok()?;
        item.set_class_name("context-bar-line");
        item.set_text_content(Some(line));
        let _ = bar.append_child(&item);
    }

    Some(())
}

pub(crate) fn update_shape_properties_panel(
    av: &RefAV,
    ordered: &[EUId],
    allow_focus: bool,
) -> Option<()> {
    let document: Document = av.borrow().document.clone();
    let body_el: Element = document.get_element_by_id("shape-properties-body")?;
    let body: HtmlElement = body_el.dyn_into::<HtmlElement>().ok()?;

    if !allow_focus {
        let active = document.active_element()?;
        if body.contains(Some(&active)) {
            return None;
        }
    }

    if !matches!(av.borrow().active_view, Tabs::Draw) {
        return None;
    }
    body.set_inner_html("");

    let selected: Vec<EUId> = {
        let avb = av.borrow();
        let canvas = &avb.canvases[CanvasKind::Draw.idx()];
        let mut selected: Vec<EUId> = ordered
            .iter()
            .copied()
            .filter(|eid| canvas.dataset.shapes_selected.contains(eid))
            .collect();
        if selected.is_empty() {
            selected = canvas.dataset.shapes_selected.iter().copied().collect();
        }
        selected
    };
    if selected.is_empty() {
        let msg = document.create_element("div").ok()?;
        msg.set_class_name("shape-prop-empty");
        msg.set_text_content(Some("No shape selected"));
        let _ = body.append_child(&msg);
        return Some(());
    }
    if selected.len() > 1 {
        let msg = document.create_element("div").ok()?;
        msg.set_class_name("shape-prop-empty");
        msg.set_text_content(Some("Multiple shapes selected"));
        let _ = body.append_child(&msg);
        return Some(());
    }

    let (eid, shape_props, shape_section_label) = {
        let avb = av.borrow();
        let canvas = &avb.canvases[CanvasKind::Draw.idx()];
        let eid = selected[0];
        let shape = canvas.dataset.get_element(eid)?;
        let mut props: Vec<(String, Property, PropertyValue)> = shape
            .get_properties()
            .iter()
            .map(|(key, prop)| (prop.to_string(), *key, prop.clone()))
            .collect();
        props.sort_by(|a, b| a.1.order().cmp(&b.1.order()).then_with(|| a.0.cmp(&b.0)));

        let shape_type = shape.get_shape_type();
        let base = shape_type_label(shape_type);
        let mut count = 0usize;
        for ordered_eid in ordered {
            let Some(ordered_shape) = canvas.dataset.shapes.get(ordered_eid) else {
                continue;
            };
            let ordered_type = ordered_shape.get_shape_type();
            if matches!(
                ordered_type,
                ShapeType::ConstrLine | ShapeType::ConstrCircle
            ) {
                continue;
            }
            if ordered_type == shape_type {
                count += 1;
            }
            if *ordered_eid == eid {
                break;
            }
        }

        let default_name = if count == 0 {
            base.to_string()
        } else {
            format!("{base} {count}")
        };
        let display_name = shape
            .get_name()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .unwrap_or(default_name);
        let shape_section_label = format!("Shape: {display_name}");

        (eid, props, shape_section_label)
    };

    let _ = add_property_section(&document, &body, &shape_section_label);
    for (label, prop, prop_val) in shape_props {
        use PropertyValue::*;

        match prop_val {
            Center { value, .. }
            | BottomLeft { value, .. }
            | TopLeft { value, .. }
            | TopRight { value, .. }
            | BottomRight { value, .. }
            | Pt1 { value, .. }
            | Pt2 { value, .. }
            | Apex { value, .. } => {
                let (x_input, y_input) = add_property_two_number_inputs(
                    &document,
                    &body,
                    &label,
                    Some(value.x),
                    Some(value.y),
                    1.0,
                    0,
                )?;

                let av_x = av.clone();
                let x_input_clone = x_input.clone();
                let on_x = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
                    let Ok(x) = x_input_clone.value().parse::<f64>() else {
                        return;
                    };
                    let Ok(mut avb) = av_x.try_borrow_mut() else {
                        return;
                    };
                    let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
                    let user_ui = canvas.get_user_ui().clone();
                    let Some(shape) = canvas.dataset.get_element_mut(eid) else {
                        return;
                    };
                    let current = match shape.get_properties().get(&prop) {
                        Some(PropertyValue::Center { value, .. })
                        | Some(PropertyValue::BottomLeft { value, .. })
                        | Some(PropertyValue::TopLeft { value, .. })
                        | Some(PropertyValue::TopRight { value, .. })
                        | Some(PropertyValue::BottomRight { value, .. })
                        | Some(PropertyValue::Pt1 { value, .. })
                        | Some(PropertyValue::Pt2 { value, .. })
                        | Some(PropertyValue::Apex { value, .. }) => *value,
                        _ => Vec2::ZERO,
                    };
                    let new_value = Vec2::new(x, current.y);
                    match shape.get_properties_mut().get_mut(&prop) {
                        Some(PropertyValue::Center { value, .. })
                        | Some(PropertyValue::BottomLeft { value, .. })
                        | Some(PropertyValue::TopLeft { value, .. })
                        | Some(PropertyValue::TopRight { value, .. })
                        | Some(PropertyValue::BottomRight { value, .. })
                        | Some(PropertyValue::Pt1 { value, .. })
                        | Some(PropertyValue::Pt2 { value, .. })
                        | Some(PropertyValue::Apex { value, .. }) => *value = new_value,
                        _ => {}
                    }

                    let g = shape.move_vertex_by_props(&prop, user_ui);
                    log!("Moved vertex by props: {:?}", g);
                    canvas.dataset.mark_final_polygon_dirty();
                    let ordered = canvas.dataset.ordered_shapes();
                    avb.refresh_toolpath_cache();
                    avb.refresh_gcode_cache();
                    drop(avb);
                    render_draw_view(av_x.clone());
                    let _ = update_shape_properties_panel(&av_x, &ordered, true);
                });
                let _ = x_input
                    .add_event_listener_with_callback("change", on_x.as_ref().unchecked_ref());
                on_x.forget();

                let av_y = av.clone();
                let y_input_clone = y_input.clone();
                let on_y = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
                    let Ok(y) = y_input_clone.value().parse::<f64>() else {
                        return;
                    };
                    let Ok(mut avb) = av_y.try_borrow_mut() else {
                        return;
                    };
                    let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
                    let user_ui = canvas.get_user_ui().clone();
                    let Some(shape) = canvas.dataset.get_element_mut(eid) else {
                        return;
                    };
                    let current = match shape.get_properties().get(&prop) {
                        Some(PropertyValue::Center { value, .. })
                        | Some(PropertyValue::BottomLeft { value, .. })
                        | Some(PropertyValue::TopLeft { value, .. })
                        | Some(PropertyValue::TopRight { value, .. })
                        | Some(PropertyValue::BottomRight { value, .. })
                        | Some(PropertyValue::Pt1 { value, .. })
                        | Some(PropertyValue::Pt2 { value, .. })
                        | Some(PropertyValue::Apex { value, .. }) => *value,
                        _ => Vec2::ZERO,
                    };
                    let new_value = Vec2::new(current.x, y);
                    match shape.get_properties_mut().get_mut(&prop) {
                        Some(PropertyValue::Center { value, .. })
                        | Some(PropertyValue::BottomLeft { value, .. })
                        | Some(PropertyValue::TopLeft { value, .. })
                        | Some(PropertyValue::TopRight { value, .. })
                        | Some(PropertyValue::BottomRight { value, .. })
                        | Some(PropertyValue::Pt1 { value, .. })
                        | Some(PropertyValue::Pt2 { value, .. })
                        | Some(PropertyValue::Apex { value, .. }) => *value = new_value,
                        _ => {}
                    }
                    shape.move_vertex_by_props(&prop, user_ui);
                    canvas.dataset.mark_final_polygon_dirty();
                    let ordered = canvas.dataset.ordered_shapes();
                    avb.refresh_toolpath_cache();
                    avb.refresh_gcode_cache();
                    drop(avb);
                    render_draw_view(av_y.clone());
                    let _ = update_shape_properties_panel(&av_y, &ordered, true);
                });
                let _ = y_input
                    .add_event_listener_with_callback("change", on_y.as_ref().unchecked_ref());
                on_y.forget();
            }
            Radius { value, .. } => {
                let input = add_property_number_input(
                    &document,
                    &body,
                    &label,
                    Some(value.curr()),
                    value.step(),
                )?;

                let min = value.min();
                let max = value.max();
                input.set_min(&format!("{min:.3}"));
                input.set_max(&format!("{max:.3}"));

                let av_in = av.clone();
                let input_clone = input.clone();
                let on_change = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
                    let Ok(value) = input_clone.value().parse::<f64>() else {
                        return;
                    };
                    let Ok(mut avb) = av_in.try_borrow_mut() else {
                        return;
                    };
                    let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
                    let Some(shape) = canvas.dataset.get_element_mut(eid) else {
                        return;
                    };
                    let value = value.clamp(min, max);
                    if let Some(PropertyValue::Radius { value: v, .. }) =
                        shape.get_properties_mut().get_mut(&prop)
                    {
                        v.set_curr(value);
                    }
                    input_clone.set_value(&format!("{value:.3}"));
                    let _ = shape.update_from_property(&prop);
                    canvas.dataset.mark_final_polygon_dirty();
                    avb.refresh_toolpath_cache();
                    avb.refresh_gcode_cache();
                    drop(avb);
                    render_draw_view(av_in.clone());
                });
                let _ = input
                    .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
                on_change.forget();
            }
            Angle { value } => {
                let input = add_property_number_input(
                    &document,
                    &body,
                    &label,
                    Some(value.curr()),
                    value.step(),
                )?;

                let av_in = av.clone();
                let input_clone = input.clone();
                let on_change = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
                    let Ok(value) = input_clone.value().parse::<f64>() else {
                        return;
                    };
                    let Ok(mut avb) = av_in.try_borrow_mut() else {
                        return;
                    };
                    let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
                    let Some(shape) = canvas.dataset.get_element_mut(eid) else {
                        return;
                    };
                    if let Some(PropertyValue::Angle { value: v }) =
                        shape.get_properties_mut().get_mut(&prop)
                    {
                        v.set_curr(value);
                    }
                    let _ = shape.update_from_property(&prop);
                    canvas.dataset.mark_final_polygon_dirty();
                    avb.refresh_toolpath_cache();
                    avb.refresh_gcode_cache();
                    drop(avb);
                    render_draw_view(av_in.clone());
                });
                let _ = input
                    .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
                on_change.forget();
            }
            Scale { value } => {
                let input = add_property_number_input(
                    &document,
                    &body,
                    &label,
                    Some(value.curr()),
                    value.step(),
                )?;

                let min = value.min();
                let max = value.max();
                input.set_min(&format!("{min:.3}"));
                input.set_max(&format!("{max:.3}"));

                let av_in = av.clone();
                let input_clone = input.clone();
                let on_change = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
                    let Ok(value) = input_clone.value().parse::<f64>() else {
                        return;
                    };
                    let Ok(mut avb) = av_in.try_borrow_mut() else {
                        return;
                    };
                    let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
                    let Some(shape) = canvas.dataset.get_element_mut(eid) else {
                        return;
                    };
                    let value = value.clamp(min, max);
                    if let Some(Scale { value: v, .. }) = shape.get_properties_mut().get_mut(&prop)
                    {
                        v.set_curr(value);
                    }
                    input_clone.set_value(&format!("{value:.3}"));
                    let _ = shape.update_from_property(&prop);
                    canvas.dataset.mark_final_polygon_dirty();
                    avb.refresh_toolpath_cache();
                    avb.refresh_gcode_cache();
                    drop(avb);
                    render_draw_view(av_in.clone());
                });
                let _ = input
                    .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
                on_change.forget();
            }
            Thickness { value } => {
                let input = add_property_number_input(
                    &document,
                    &body,
                    &label,
                    Some(value.curr()),
                    value.step(),
                )?;

                let min = value.min();
                let max = value.max();
                input.set_min(&format!("{min:.3}"));
                input.set_max(&format!("{max:.3}"));

                let av_in = av.clone();
                let input_clone = input.clone();
                let on_change = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
                    let Ok(value) = input_clone.value().parse::<f64>() else {
                        return;
                    };
                    let Ok(mut avb) = av_in.try_borrow_mut() else {
                        return;
                    };
                    let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
                    let Some(shape) = canvas.dataset.get_element_mut(eid) else {
                        return;
                    };
                    let value = value.clamp(min, max);
                    if let Some(Thickness { value: v, .. }) =
                        shape.get_properties_mut().get_mut(&prop)
                    {
                        v.set_curr(value);
                    }
                    input_clone.set_value(&format!("{value:.3}"));
                    let _ = shape.update_from_property(&prop);
                    canvas.dataset.mark_final_polygon_dirty();
                    avb.refresh_toolpath_cache();
                    avb.refresh_gcode_cache();
                    drop(avb);
                    render_draw_view(av_in.clone());
                });
                let _ = input
                    .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
                on_change.forget();
            }
            PropertyValue::Seeds { value } => {
                let input = add_property_number_input(
                    &document,
                    &body,
                    &label,
                    Some(value.curr() as f64),
                    value.step() as f64,
                )?;
                input.set_step("1");
                input.set_value(&format!("{}", value.curr()));
                let _ = input.set_attribute("inputmode", "numeric");

                let min = value.min();
                let max = value.max();
                input.set_min(&format!("{min}"));
                input.set_max(&format!("{max}"));

                let av_in = av.clone();
                let input_clone = input.clone();
                let on_change = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
                    let Ok(value) = input_clone.value().parse::<f64>() else {
                        return;
                    };
                    let Ok(mut avb) = av_in.try_borrow_mut() else {
                        return;
                    };
                    let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
                    let Some(shape) = canvas.dataset.get_element_mut(eid) else {
                        return;
                    };
                    let value = value.round() as usize;
                    let value = value.clamp(min as usize, max as usize);
                    if let Some(PropertyValue::Seeds { value: v, .. }) =
                        shape.get_properties_mut().get_mut(&prop)
                    {
                        v.set_curr(value as u64);
                    }
                    input_clone.set_value(&format!("{value}"));
                    let _ = shape.update_from_property(&prop);
                    canvas.dataset.mark_final_polygon_dirty();
                    avb.refresh_toolpath_cache();
                    avb.refresh_gcode_cache();
                    drop(avb);
                    render_draw_view(av_in.clone());
                });
                let _ = input
                    .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
                on_change.forget();
            }
            PropertyValue::VoronoiGap { value } => {
                let input = add_property_number_input(
                    &document,
                    &body,
                    &label,
                    Some(value.curr()),
                    value.step(),
                )?;
                input.set_value(&format!("{:.3}", value.curr()));

                let min = value.min();
                let max = value.max();
                input.set_min(&format!("{min:.3}"));
                input.set_max(&format!("{max:.3}"));

                let av_in = av.clone();
                let input_clone = input.clone();
                let on_change = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
                    let Ok(value) = input_clone.value().parse::<f64>() else {
                        return;
                    };
                    let Ok(mut avb) = av_in.try_borrow_mut() else {
                        return;
                    };
                    let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
                    let Some(shape) = canvas.dataset.get_element_mut(eid) else {
                        return;
                    };
                    let value = value.clamp(min, max);
                    if let Some(PropertyValue::VoronoiGap { value: v }) =
                        shape.get_properties_mut().get_mut(&prop)
                    {
                        v.set_curr(value);
                    }
                    input_clone.set_value(&format!("{value:.3}"));
                    let _ = shape.update_from_property(&prop);
                    canvas.dataset.mark_final_polygon_dirty();
                    avb.refresh_toolpath_cache();
                    avb.refresh_gcode_cache();
                    drop(avb);
                    render_draw_view(av_in.clone());
                });
                let _ = input
                    .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
                on_change.forget();
            }
            PropertyValue::VoronoiRelaxation { value } => {
                let input = add_property_number_input(
                    &document,
                    &body,
                    &label,
                    Some(value.curr() as f64),
                    value.step() as f64,
                )?;
                input.set_step("1");
                input.set_value(&format!("{}", value.curr()));
                let _ = input.set_attribute("inputmode", "numeric");

                let min = value.min();
                let max = value.max();
                input.set_min(&format!("{min}"));
                input.set_max(&format!("{max}"));

                let av_in = av.clone();
                let input_clone = input.clone();
                let on_change = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
                    let Ok(value) = input_clone.value().parse::<f64>() else {
                        return;
                    };
                    let Ok(mut avb) = av_in.try_borrow_mut() else {
                        return;
                    };
                    let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
                    let Some(shape) = canvas.dataset.get_element_mut(eid) else {
                        return;
                    };
                    let value = value.round() as u64;
                    let value = value.clamp(min, max);
                    if let Some(PropertyValue::VoronoiRelaxation { value: v }) =
                        shape.get_properties_mut().get_mut(&prop)
                    {
                        v.set_curr(value);
                    }
                    input_clone.set_value(&format!("{value}"));
                    let _ = shape.update_from_property(&prop);
                    canvas.dataset.mark_final_polygon_dirty();
                    avb.refresh_toolpath_cache();
                    avb.refresh_gcode_cache();
                    drop(avb);
                    render_draw_view(av_in.clone());
                });
                let _ = input
                    .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
                on_change.forget();
            }
            Magnets { value } => {
                let input = add_property_number_input(
                    &document,
                    &body,
                    &label,
                    Some(value.curr() as f64),
                    value.step() as f64,
                )?;
                input.set_step("1");
                input.set_value(&format!("{}", value.curr()));
                let _ = input.set_attribute("inputmode", "numeric");

                let min = value.min();
                let max = value.max();
                input.set_min(&format!("{min}"));
                input.set_max(&format!("{max}"));

                let av_in = av.clone();
                let input_clone = input.clone();
                let on_change = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
                    let Ok(value) = input_clone.value().parse::<f64>() else {
                        return;
                    };
                    let Ok(mut avb) = av_in.try_borrow_mut() else {
                        return;
                    };
                    let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
                    let Some(shape) = canvas.dataset.get_element_mut(eid) else {
                        return;
                    };
                    let value = value.round() as usize;
                    let value = value.clamp(min, max);
                    if let Some(PropertyValue::Magnets { value: v, .. }) =
                        shape.get_properties_mut().get_mut(&prop)
                    {
                        v.set_curr(value);
                    }
                    input_clone.set_value(&format!("{value}"));
                    let _ = shape.update_from_property(&prop);
                    canvas.dataset.mark_final_polygon_dirty();
                    avb.refresh_toolpath_cache();
                    avb.refresh_gcode_cache();
                    drop(avb);
                    render_draw_view(av_in.clone());
                });
                let _ = input
                    .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
                on_change.forget();
            }
            Text { value } => {
                let input = add_property_text_input(&document, &body, &label, value.as_str())?;

                let av_in = av.clone();
                let input_clone = input.clone();
                let on_change = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
                    let value = input_clone.value();
                    let Ok(mut avb) = av_in.try_borrow_mut() else {
                        return;
                    };
                    let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
                    let Some(shape) = canvas.dataset.get_element_mut(eid) else {
                        return;
                    };
                    if let Some(PropertyValue::Text { value: v }) =
                        shape.get_properties_mut().get_mut(&prop)
                    {
                        *v = value.clone();
                    }
                    let _ = shape.update_from_property(&prop);
                    canvas.dataset.mark_final_polygon_dirty();
                    avb.refresh_toolpath_cache();
                    avb.refresh_gcode_cache();
                    drop(avb);
                    render_draw_view(av_in.clone());
                });
                let _ = input
                    .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
                on_change.forget();
            }
            PropertyValue::Font { value } => {
                let _ = add_property_value(&document, &body, &label, value.as_str());
            }
        }
    }
    Some(())
}

pub(crate) fn update_shapes_panel(av: RefAV) {
    let avb = av.borrow_mut();
    let Some(list) = avb.document.get_element_by_id("shapes-list") else {
        return;
    };
    let Ok(list) = list.dyn_into::<HtmlElement>() else {
        return;
    };
    if !matches!(avb.active_view, Tabs::Draw) {
        let _ = avb.shapes_panel.style().set_property("display", "none");
        list.set_inner_html("");
        drop(avb);
        update_shape_properties_panel(&av, &[], false);
        update_context_help(&av);
        return;
    }
    let _ = avb.shapes_panel.style().set_property("display", "flex");
    list.set_inner_html("");

    let canvas = &avb.canvases[CanvasKind::Draw.idx()];
    let ordered = canvas.dataset.ordered_shapes();
    let mut counts: HashMap<&'static str, usize> = HashMap::new();

    for (idx, eid) in ordered.iter().enumerate() {
        let Some(shape) = canvas.dataset.shapes.get(eid) else {
            continue;
        };
        let shape_type = shape.get_shape_type();
        if matches!(shape_type, ShapeType::ConstrLine | ShapeType::ConstrCircle) {
            continue;
        }
        let base = shape_type_label(shape_type);
        let entry = counts.entry(base).or_insert(0);
        *entry += 1;
        let count = *entry;
        let default_name = format!("{base} {count}");
        let display_name = shape
            .get_name()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or(default_name);

        let Ok(row) = avb.document.create_element("div") else {
            continue;
        };
        row.set_class_name("shape-row");
        let _ = row.set_attribute("draggable", "true");
        let _ = row.set_attribute("data-index", &idx.to_string());
        if canvas.dataset.shapes_selected.contains(eid) {
            let _ = row.class_list().add_1("selected");
        }

        let Ok(name_el) = avb.document.create_element("span") else {
            continue;
        };
        name_el.set_class_name("shape-name");
        name_el.set_text_content(Some(&display_name));

        let Ok(op_el) = avb.document.create_element("span") else {
            continue;
        };
        let (op_label, op_class) = match shape.get_operation() {
            Operation::Union => ("Union", "union"),
            Operation::Difference => ("Diff", "difference"),
        };
        op_el.set_class_name(&format!("shape-op {op_class}"));
        op_el.set_text_content(Some(op_label));

        let Ok(delete_el) = avb.document.create_element("button") else {
            continue;
        };
        delete_el.set_class_name("shape-delete");
        delete_el.set_text_content(Some("×"));
        let _ = delete_el.set_attribute("title", "Delete shape");

        let _ = row.append_child(&name_el);
        let _ = row.append_child(&op_el);
        let _ = row.append_child(&delete_el);
        let _ = list.append_child(&row);
    }

    drop(avb);
    update_shape_properties_panel(&av, &ordered, false);
    update_context_help(&av);
}

pub(crate) fn init_icons(av: RefAV) -> Result<(), JsValue> {
    let default_color = Color::Text.get();
    let selected_color = Color::OnCreation.get();
    av.borrow_mut().user_icons.iter().for_each(|icon| {
        let html_element = icon
            .get_html_element()
            .unwrap_or_else(|| panic!("Icon element not found: {}", icon.id()));
        html_element
            .set_attribute("style", &format!("color:{default_color}"))
            .unwrap();
        let icon_copy = *icon;
        set_callback(
            av.clone(),
            "click".into(),
            &html_element,
            Box::new(move |av, _event| on_icon_click(av, icon_copy)),
        )
        .unwrap();
        let icon_copy = *icon;
        set_callback(
            av.clone(),
            "mouseenter".into(),
            &html_element,
            Box::new(move |av, event| on_icon_mouseover(av, event, icon_copy)),
        )
        .unwrap();
        set_callback(
            av.clone(),
            "mouseleave".into(),
            &html_element,
            Box::new(on_icon_mouseout),
        )
        .unwrap();
    });

    if let Some(html_element) = ShapeType::Arrow.get_html_element() {
        html_element
            .set_attribute("style", &format!("color:{selected_color}"))
            .unwrap();
        av.borrow().html_select_icon(ShapeType::Arrow);
    }

    if let Some(line_icon) = av.borrow().document.get_element_by_id("icon-constr-line") {
        set_callback(
            av.clone(),
            "mouseenter".into(),
            &line_icon,
            Box::new(move |av, event| on_icon_mouseover_label(av, event, "Construction line")),
        )?;
        set_callback(
            av.clone(),
            "mouseleave".into(),
            &line_icon,
            Box::new(on_icon_mouseout),
        )?;
    }

    if let Some(circle_icon) = av.borrow().document.get_element_by_id("icon-constr-circle") {
        set_callback(
            av.clone(),
            "mouseenter".into(),
            &circle_icon,
            Box::new(move |av, event| on_icon_mouseover_label(av, event, "Construction circle")),
        )?;
        set_callback(
            av.clone(),
            "mouseleave".into(),
            &circle_icon,
            Box::new(on_icon_mouseout),
        )?;
    }

    if let Some(note_icon) = av.borrow().document.get_element_by_id("icon-note") {
        let av_clone = av.clone();
        set_callback(
            av.clone(),
            "click".into(),
            &note_icon,
            Box::new(move |av, _event| {
                let Ok(mut avb) = av.try_borrow_mut() else {
                    return;
                };
                avb.select_note_tool();
                drop(avb);
                update_notes_view(av_clone.clone());
            }),
        )?;
        set_callback(
            av.clone(),
            "mouseenter".into(),
            &note_icon,
            Box::new(move |av, event| on_icon_mouseover_label(av, event, "Note")),
        )?;
        set_callback(
            av.clone(),
            "mouseleave".into(),
            &note_icon,
            Box::new(on_icon_mouseout),
        )?;
    }

    Ok(())
}

pub(crate) fn init_draw_canvas(av: RefAV) -> Result<(), JsValue> {
    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();

    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_draw_mouse_move(pa_cloneprim.clone(), event);
    });

    let c_draw = pa_cloneme.borrow().canvases[CanvasKind::Draw.idx()]
        .get_canvas()
        .clone();
    c_draw.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())?;
    closure.forget();

    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_draw_mouse_down(pa_cloneprim.clone(), event);
    });
    let c_draw = pa_cloneme.borrow().canvases[CanvasKind::Draw.idx()]
        .get_canvas()
        .clone();
    c_draw.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())?;
    closure.forget();

    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_draw_mouse_up(pa_cloneprim.clone(), event);
    });
    let c_draw = pa_cloneme.borrow().canvases[CanvasKind::Draw.idx()]
        .get_canvas()
        .clone();
    c_draw.add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref())?;
    closure.forget();

    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_draw_mouse_wheel(pa_cloneprim.clone(), event);
    });
    let c_draw = pa_cloneme.borrow().canvases[CanvasKind::Draw.idx()]
        .get_canvas()
        .clone();
    c_draw.add_event_listener_with_callback("wheel", closure.as_ref().unchecked_ref())?;
    closure.forget();

    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_draw_mouse_enter(pa_cloneprim.clone(), event);
    });
    let c_draw = pa_cloneme.borrow().canvases[CanvasKind::Draw.idx()]
        .get_canvas()
        .clone();
    c_draw.add_event_listener_with_callback("mouseenter", closure.as_ref().unchecked_ref())?;
    closure.forget();

    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_draw_mouse_leave(pa_cloneprim.clone(), event);
    });
    let c_draw = pa_cloneme.borrow().canvases[CanvasKind::Draw.idx()]
        .get_canvas()
        .clone();
    c_draw.add_event_listener_with_callback("mouseleave", closure.as_ref().unchecked_ref())?;
    closure.forget();

    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_draw_context_menu(pa_cloneprim.clone(), event);
    });
    let c_draw = pa_cloneme.borrow().canvases[CanvasKind::Draw.idx()]
        .get_canvas()
        .clone();
    c_draw.add_event_listener_with_callback("contextmenu", closure.as_ref().unchecked_ref())?;
    closure.forget();

    Ok(())
}

pub(crate) fn init_shapes_panel(av: RefAV) -> Result<(), JsValue> {
    let document = av.borrow().document.clone();
    let list: HtmlElement = init_element!(document, "shapes-list", HtmlElement);

    let av_click = av.clone();
    let on_click = Closure::<dyn FnMut(Event)>::new(move |evt: Event| {
        let Ok(target) = evt.target().unwrap().dyn_into::<Element>() else {
            return;
        };
        let Ok(Some(row)) = target.closest(".shape-row") else {
            return;
        };
        let Some(idx) = row
            .get_attribute("data-index")
            .and_then(|value| value.parse::<usize>().ok())
        else {
            return;
        };
        let is_delete = target.class_list().contains("shape-delete");
        if let Ok(mut avb) = av_click.try_borrow_mut() {
            if !matches!(avb.active_view, Tabs::Draw) {
                return;
            }
            let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
            let ordered = canvas.dataset.ordered_shapes();
            let Some(eid) = ordered.get(idx).copied() else {
                return;
            };
            if is_delete {
                canvas.dataset.pop_element(eid);
                canvas.dataset.calc_final_polygon();
                avb.refresh_toolpath_cache();
                avb.refresh_gcode_cache();
            } else {
                canvas.dataset.select_only(eid);
            }
        }
        render_draw_view(av_click.clone());
    });
    list.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())?;
    on_click.forget();

    let av_dblclick = av.clone();
    let on_dblclick = Closure::<dyn FnMut(Event)>::new(move |evt: Event| {
        let Ok(target) = evt.target().unwrap().dyn_into::<Element>() else {
            return;
        };
        let Ok(Some(name_el)) = target.closest(".shape-name") else {
            return;
        };
        let Ok(Some(row)) = name_el.closest(".shape-row") else {
            return;
        };
        let Some(idx) = row
            .get_attribute("data-index")
            .and_then(|value| value.parse::<usize>().ok())
        else {
            return;
        };
        if let Ok(mut avb) = av_dblclick.try_borrow_mut() {
            if !matches!(avb.active_view, Tabs::Draw) {
                return;
            }
            let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
            let ordered = canvas.dataset.ordered_shapes();
            let Some(eid) = ordered.get(idx).copied() else {
                return;
            };
            let Some(shape) = canvas.dataset.get_element_mut(eid) else {
                return;
            };
            let base = shape_type_label(shape.get_shape_type());
            let current = shape
                .get_name()
                .map(|value| value.to_string())
                .unwrap_or_else(|| base.to_string());
            if let Some(window) = web_sys::window() {
                if let Ok(Some(name)) =
                    window.prompt_with_message_and_default("Shape name:", &current)
                {
                    shape.set_name(Some(name));
                }
            }
        }
        render_draw_view(av_dblclick.clone());
    });
    list.add_event_listener_with_callback("dblclick", on_dblclick.as_ref().unchecked_ref())?;
    on_dblclick.forget();

    let av_dragstart = av.clone();
    let on_dragstart = Closure::<dyn FnMut(Event)>::new(move |evt: Event| {
        let Some(target) = evt.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
            return;
        };
        let Ok(Some(row)) = target.closest(".shape-row") else {
            return;
        };
        let Some(idx) = row
            .get_attribute("data-index")
            .and_then(|value| value.parse::<usize>().ok())
        else {
            return;
        };
        let _ = row.class_list().add_1("dragging");
        if let Ok(mut avb) = av_dragstart.try_borrow_mut() {
            avb.shapes_drag_from = Some(idx);
        }
        let _ = evt.dyn_into::<DragEvent>();
    });
    list.add_event_listener_with_callback("dragstart", on_dragstart.as_ref().unchecked_ref())?;
    on_dragstart.forget();

    let av_dragend = av.clone();
    let on_dragend = Closure::<dyn FnMut(Event)>::new(move |evt: Event| {
        let Ok(target) = evt.target().unwrap().dyn_into::<Element>() else {
            return;
        };
        let Ok(Some(row)) = target.closest(".shape-row") else {
            return;
        };
        let _ = row.class_list().remove_1("dragging");
        if let Ok(mut avb) = av_dragend.try_borrow_mut() {
            avb.shapes_drag_from = None;
        }
    });
    list.add_event_listener_with_callback("dragend", on_dragend.as_ref().unchecked_ref())?;
    on_dragend.forget();

    let on_dragover = Closure::<dyn FnMut(Event)>::new(move |evt: Event| {
        evt.prevent_default();
        let _ = evt.dyn_into::<DragEvent>();
    });
    list.add_event_listener_with_callback("dragover", on_dragover.as_ref().unchecked_ref())?;
    on_dragover.forget();

    let av_drop = av.clone();
    let on_drop = Closure::<dyn FnMut(Event)>::new(move |evt: Event| {
        let Ok(evt) = evt.dyn_into::<DragEvent>() else {
            return;
        };
        evt.prevent_default();
        let from_idx = av_drop.borrow().shapes_drag_from.unwrap_or(usize::MAX);
        if from_idx == usize::MAX {
            return;
        }
        let Some(target) = evt.target().and_then(|t| t.dyn_into::<Element>().ok()) else {
            return;
        };
        let to_idx = target
            .closest(".shape-row")
            .ok()
            .flatten()
            .and_then(|row| row.get_attribute("data-index"))
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(usize::MAX);
        reorder_shapes_by_index(av_drop.clone(), from_idx, to_idx);
        if let Ok(mut avb) = av_drop.try_borrow_mut() {
            avb.shapes_drag_from = None;
        }
    });
    list.add_event_listener_with_callback("drop", on_drop.as_ref().unchecked_ref())?;
    on_drop.forget();

    Ok(())
}

/// Hit-test result: (shape_id, dim_idx, normal, is_angular, circle_center)
type DimHit = (EUId, usize, Vec2, bool, Vec2);

/// Returns the closest dim handle within hit_radius.
/// For rectangular shapes: dim_idx=0 → H, 1 → V (linear drag along normal).
/// For Disc: dim_idx=0 → radius angle handle (angular drag around circle center).
fn hit_test_dim_handle(
    shapes: &std::collections::HashMap<EUId, crate::shape::GeneralShape>,
    mouse_pos: Vec2,
    hit_radius: f64,
) -> Option<DimHit> {
    use crate::helpers::math::rotate_vector;
    use ShapeType::*;
    let mut best: Option<(DimHit, f64)> = None;

    for (eid, shape) in shapes.iter() {
        match shape.get_shape_type() {
            Disc => {
                let pts: Vec<Vec2> = shape.get_vertices().iter().map(|(_, v)| v.curr()).collect();
                if pts.len() < 2 {
                    continue;
                }
                let center = pts[0];
                let radius = (pts[1] - center).hypot();
                if radius < 0.01 {
                    continue;
                }
                let stored = shape.get_dim_offsets()[0];
                let natural = (pts[1] - center).y.atan2((pts[1] - center).x);
                let angle = if (stored - (-20.0)).abs() < 1e-9 {
                    natural
                } else {
                    stored
                };
                let u = Vec2::new(angle.cos(), angle.sin());
                let handle = center + u * radius;
                let dist = (handle - mouse_pos).hypot();
                if dist < hit_radius {
                    if best.as_ref().map_or(true, |(_, d)| dist < *d) {
                        best = Some(((*eid, 0, Vec2::ZERO, true, center), dist));
                    }
                }
            }
            Oblong => {
                let pts: Vec<Vec2> = shape.get_vertices().iter().map(|(_, v)| v.curr()).collect();
                if pts.len() < 4 {
                    continue;
                }
                let offsets = shape.get_dim_offsets();
                // Axis length handle (linear, dim_idx=0)
                {
                    let edge = pts[1] - pts[0];
                    let len = edge.hypot();
                    if len >= 0.1 {
                        let u = edge / len;
                        let n = Vec2::new(-u.y, u.x);
                        let d1 = pts[0] + n * offsets[0];
                        let d2 = pts[1] + n * offsets[0];
                        let handle = (d1 + d2) * 0.5;
                        let dist = (handle - mouse_pos).hypot();
                        if dist < hit_radius {
                            if best.as_ref().map_or(true, |(_, d)| dist < *d) {
                                best = Some(((*eid, 0, n, false, Vec2::ZERO), dist));
                            }
                        }
                    }
                }
                // Radius at pts[0] handle (angular, dim_idx=1)
                {
                    let r0 = (pts[2] - pts[0]).hypot();
                    if r0 > 0.01 {
                        let natural0 = (pts[2] - pts[0]).y.atan2((pts[2] - pts[0]).x);
                        let stored0 = offsets[1];
                        let a0 = if (stored0 - (-20.0)).abs() < 1e-9 {
                            natural0
                        } else {
                            stored0
                        };
                        let u0 = Vec2::new(a0.cos(), a0.sin());
                        let handle0 = pts[0] + u0 * r0;
                        let dist = (handle0 - mouse_pos).hypot();
                        if dist < hit_radius {
                            if best.as_ref().map_or(true, |(_, d)| dist < *d) {
                                best = Some(((*eid, 1, Vec2::ZERO, true, pts[0]), dist));
                            }
                        }
                    }
                }
                // Radius at pts[1] handle (angular, dim_idx=2)
                {
                    let r1 = (pts[3] - pts[1]).hypot();
                    if r1 > 0.01 {
                        let natural1 = (pts[3] - pts[1]).y.atan2((pts[3] - pts[1]).x);
                        let stored1 = offsets[2];
                        let a1 = if (stored1 - (-20.0)).abs() < 1e-9 {
                            natural1
                        } else {
                            stored1
                        };
                        let u1 = Vec2::new(a1.cos(), a1.sin());
                        let handle1 = pts[1] + u1 * r1;
                        let dist = (handle1 - mouse_pos).hypot();
                        if dist < hit_radius {
                            if best.as_ref().map_or(true, |(_, d)| dist < *d) {
                                best = Some(((*eid, 2, Vec2::ZERO, true, pts[1]), dist));
                            }
                        }
                    }
                }
            }
            Square | Text | Svg | Voronoi | Group => {
                let pts: Vec<Vec2> = shape.get_vertices().iter().map(|(_, v)| v.curr()).collect();
                if pts.len() < 4 {
                    continue;
                }
                let rotation = shape.get_rotation();
                let offsets = shape.get_dim_offsets();
                let bbox = shape.get_bezpath().bounding_box();
                let center = Vec2::new((bbox.x0 + bbox.x1) * 0.5, (bbox.y0 + bbox.y1) * 0.5);

                let edge_configs = [
                    (pts[0], pts[3], offsets[0], 0usize),
                    (pts[2], pts[3], offsets[1], 1usize),
                ];
                for (p1, p2, offset, idx) in edge_configs.iter() {
                    let edge = *p2 - *p1;
                    let len = edge.hypot();
                    if len < 0.1 {
                        continue;
                    }
                    let u = edge / len;
                    let n = Vec2::new(-u.y, u.x);
                    let d1 = *p1 + n * *offset;
                    let d2 = *p2 + n * *offset;
                    let handle_raw = (d1 + d2) * 0.5;
                    let handle = if rotation.abs() > 1e-9 {
                        rotate_vector(handle_raw - center, rotation) + center
                    } else {
                        handle_raw
                    };
                    let dist = (handle - mouse_pos).hypot();
                    if dist < hit_radius {
                        let n_world = if rotation.abs() > 1e-9 {
                            rotate_vector(n, rotation)
                        } else {
                            n
                        };
                        if best.as_ref().map_or(true, |(_, d)| dist < *d) {
                            best = Some(((*eid, *idx, n_world, false, Vec2::ZERO), dist));
                        }
                    }
                }
                // Corner radius handle (Square only, first active corner, dim_idx=2)
                if shape.get_shape_type() == Square {
                    if let Some((i, r_f)) =
                        shape
                            .get_vertices()
                            .iter()
                            .enumerate()
                            .find_map(|(i, (_, v))| {
                                v.get_radius().filter(|&r| r > 0).map(|r| (i, r as f64))
                            })
                    {
                        if i < pts.len() {
                            let corner = pts[i];
                            let sx = if center.x > corner.x { 1.0 } else { -1.0 };
                            let sy = if center.y > corner.y { 1.0 } else { -1.0 };
                            let arc_center_local = corner + Vec2::new(sx * r_f, sy * r_f);
                            let stored = offsets[2];
                            let natural = (corner - arc_center_local)
                                .y
                                .atan2((corner - arc_center_local).x);
                            let angle = if (stored - (-20.0)).abs() < 1e-9 {
                                natural
                            } else {
                                stored
                            };
                            let u = Vec2::new(angle.cos(), angle.sin());
                            let handle_local = arc_center_local + u * r_f;
                            let handle = if rotation.abs() > 1e-9 {
                                rotate_vector(handle_local - center, rotation) + center
                            } else {
                                handle_local
                            };
                            let dist = (handle - mouse_pos).hypot();
                            if dist < hit_radius {
                                // drag_center in world space
                                let arc_center_world = if rotation.abs() > 1e-9 {
                                    rotate_vector(arc_center_local - center, rotation) + center
                                } else {
                                    arc_center_local
                                };
                                if best.as_ref().map_or(true, |(_, d)| dist < *d) {
                                    best =
                                        Some(((*eid, 2, Vec2::ZERO, true, arc_center_world), dist));
                                }
                            }
                        }
                    }
                }
            }
            Poly => {
                let bbox = shape.get_bezpath().bounding_box();
                if bbox.width() < 0.1 && bbox.height() < 0.1 {
                    continue;
                }
                let offsets = shape.get_dim_offsets();
                let tl = Vec2::new(bbox.x0, bbox.y0);
                let tr = Vec2::new(bbox.x1, bbox.y0);
                let br = Vec2::new(bbox.x1, bbox.y1);
                // H : tl→tr, dim_idx=0
                {
                    let edge = tr - tl;
                    let len = edge.hypot();
                    if len >= 0.1 {
                        let u = edge / len;
                        let n = Vec2::new(-u.y, u.x);
                        let d1 = tl + n * offsets[0];
                        let d2 = tr + n * offsets[0];
                        let handle = (d1 + d2) * 0.5;
                        let dist = (handle - mouse_pos).hypot();
                        if dist < hit_radius {
                            if best.as_ref().map_or(true, |(_, d)| dist < *d) {
                                best = Some(((*eid, 0, n, false, Vec2::ZERO), dist));
                            }
                        }
                    }
                }
                // V : br→tr, dim_idx=1
                {
                    let edge = tr - br;
                    let len = edge.hypot();
                    if len >= 0.1 {
                        let u = edge / len;
                        let n = Vec2::new(-u.y, u.x);
                        let d1 = br + n * offsets[1];
                        let d2 = tr + n * offsets[1];
                        let handle = (d1 + d2) * 0.5;
                        let dist = (handle - mouse_pos).hypot();
                        if dist < hit_radius {
                            if best.as_ref().map_or(true, |(_, d)| dist < *d) {
                                best = Some(((*eid, 1, n, false, Vec2::ZERO), dist));
                            }
                        }
                    }
                }
                // Premier apex arrondi — handle angulaire, dim_idx=2
                if shape.get_vertices().len() >= 3 {
                    use crate::helpers::math::ApexType;
                    let apices = shape.get_vertices().get_apices();
                    if let Some(ApexType::Arc { s, c, .. }) =
                        apices.iter().find(|a| matches!(a, ApexType::Arc { .. }))
                    {
                        let radius = (*s - *c).hypot();
                        if radius > 0.01 {
                            let natural = (*s - *c).y.atan2((*s - *c).x);
                            let stored = offsets[2];
                            let angle = if (stored - (-20.0)).abs() < 1e-9 {
                                natural
                            } else {
                                stored
                            };
                            let u = Vec2::new(angle.cos(), angle.sin());
                            let handle = *c + u * radius;
                            let dist = (handle - mouse_pos).hypot();
                            if dist < hit_radius {
                                if best.as_ref().map_or(true, |(_, d)| dist < *d) {
                                    best = Some(((*eid, 2, Vec2::ZERO, true, *c), dist));
                                }
                            }
                        }
                    }
                }
            }
            _ => continue,
        }
    }
    best.map(|(hit, _)| hit)
}

pub(crate) fn update(av: RefAV, user_action: UserAction) -> Result<(), MyError> {
    use ShapeType::*;
    let mut avb = av.borrow_mut();
    let is_draw_view = avb.active_canvas == CanvasKind::Draw;
    let mut do_render = false;
    match avb.icon_selected {
        Arrow => match user_action {
            UserAction::Move(button, level) => {
                if level == ButtonLevel::Up {
                    if !is_draw_view {
                        return Ok(());
                    }
                    let highlight_vertex = avb.set_element_highlight_vertex();
                    avb.set_highlight_elements();
                    if highlight_vertex || is_draw_view {
                        do_render = true;
                    }
                } else if button == MouseButton::Left {
                    if !is_draw_view {
                        return Ok(());
                    }
                    if let Some(dd) = avb.dim_drag {
                        let mouse_pos = avb.canvases[CanvasKind::Draw.idx()].get_user_ui().draw_pos;
                        let new_offset = if dd.is_angular {
                            // Angle from circle center to current mouse position
                            let v = mouse_pos - dd.drag_center;
                            v.y.atan2(v.x)
                        } else {
                            let delta = mouse_pos - dd.start_mouse;
                            dd.start_offset + delta.x * dd.normal.x + delta.y * dd.normal.y
                        };
                        if let Some(shape) = avb.canvases[CanvasKind::Draw.idx()]
                            .dataset
                            .shapes
                            .get_mut(&dd.shape_id)
                        {
                            shape.set_dim_offset(dd.dim_idx, new_offset);
                        }
                        do_render = true;
                    } else if let Some(rd) = avb.rotation_drag {
                        use crate::helpers::math::snap_angle;
                        let mouse_pos = avb.canvases[CanvasKind::Draw.idx()].get_user_ui().draw_pos;
                        let snap = avb.canvases[CanvasKind::Draw.idx()].get_user_ui().snap;
                        let v = mouse_pos - rd.center;
                        let curr_angle = v.y.atan2(v.x);
                        let new_rotation =
                            snap_angle(rd.saved_rotation + (curr_angle - rd.start_angle), snap);
                        if let Some(shape) = avb.canvases[CanvasKind::Draw.idx()]
                            .dataset
                            .shapes
                            .get_mut(&rd.shape_id)
                        {
                            shape.set_rotation_curr(new_rotation);
                        }
                        do_render = true;
                    } else if avb.update_selection_window_current() {
                        do_render = true;
                    } else if avb.set_move_vertices_selected().is_some() || avb.set_move_elements()
                    {
                        do_render = true;
                    }
                } else {
                    let canvas = avb.get_active_canvas_mut();
                    canvas.move_offset();
                    do_render = true;
                }
            }
            UserAction::ClickDown(button, _) => {
                if button == MouseButton::Left {
                    if !is_draw_view {
                        return Ok(());
                    }

                    // Check if clicking on a dim handle
                    let draw_pos = avb.canvases[CanvasKind::Draw.idx()].get_user_ui().draw_pos;
                    let hit_radius =
                        8.0 / avb.canvases[CanvasKind::Draw.idx()].get_scale().max(0.001);
                    let shapes = &avb.canvases[CanvasKind::Draw.idx()].dataset.shapes;
                    if let Some((shape_id, dim_idx, normal, is_angular, drag_center)) =
                        hit_test_dim_handle(shapes, draw_pos, hit_radius)
                    {
                        let stored = shapes
                            .get(&shape_id)
                            .map(|s| s.get_dim_offsets()[dim_idx])
                            .unwrap_or(0.0);
                        // For angular drag, resolve the sentinel into the actual angle
                        let start_offset = if is_angular && (stored - (-20.0)).abs() < 1e-9 {
                            let pts: Vec<Vec2> = shapes
                                .get(&shape_id)
                                .map(|s| s.get_vertices().iter().map(|(_, v)| v.curr()).collect())
                                .unwrap_or_default();
                            let shape_type = shapes.get(&shape_id).map(|s| s.get_shape_type());
                            match (shape_type, dim_idx) {
                                (Some(ShapeType::Oblong), 1) if pts.len() >= 3 => {
                                    (pts[2] - pts[0]).y.atan2((pts[2] - pts[0]).x)
                                }
                                (Some(ShapeType::Oblong), 2) if pts.len() >= 4 => {
                                    (pts[3] - pts[1]).y.atan2((pts[3] - pts[1]).x)
                                }
                                // Square corner radius: natural angle = outward from arc_center
                                // drag_center IS the arc_center in world space;
                                // the outward direction is the same as arc_center → bbox_center
                                // mirrored (i.e., center → arc_center direction)
                                (Some(ShapeType::Square), 2) => {
                                    let bbox = shapes
                                        .get(&shape_id)
                                        .map(|s| s.get_bezpath().bounding_box())
                                        .unwrap_or_default();
                                    let shape_center = Vec2::new(
                                        (bbox.x0 + bbox.x1) * 0.5,
                                        (bbox.y0 + bbox.y1) * 0.5,
                                    );
                                    (drag_center - shape_center)
                                        .y
                                        .atan2((drag_center - shape_center).x)
                                }
                                (Some(ShapeType::Poly), 2) => {
                                    use crate::helpers::math::ApexType;
                                    // drag_center = arc center c; natural = c→s direction
                                    shapes
                                        .get(&shape_id)
                                        .and_then(|sh| {
                                            if sh.get_vertices().len() < 3 {
                                                return None;
                                            }
                                            let apices = sh.get_vertices().get_apices();
                                            apices.into_iter().find_map(|a| {
                                                if let ApexType::Arc { s, c, .. } = a {
                                                    Some((s - c).y.atan2((s - c).x))
                                                } else {
                                                    None
                                                }
                                            })
                                        })
                                        .unwrap_or(0.0)
                                }
                                _ if pts.len() >= 2 => {
                                    (pts[1] - pts[0]).y.atan2((pts[1] - pts[0]).x)
                                }
                                _ => 0.0,
                            }
                        } else {
                            stored
                        };
                        avb.dim_drag = Some(DimDrag {
                            shape_id,
                            dim_idx,
                            start_mouse: draw_pos,
                            start_offset,
                            normal,
                            is_angular,
                            drag_center,
                        });
                        do_render = true;
                        drop(avb);
                        if do_render {
                            render_active_view(av.clone());
                        }
                        return Ok(());
                    }

                    let canvas = avb.get_active_canvas_mut();
                    canvas.begin_history_action();
                    canvas.dataset.save_elements_positions();
                    // Check rotation handle before vertex selection
                    {
                        use crate::helpers::math::snap_angle;
                        let draw_pos = avb.canvases[CanvasKind::Draw.idx()].get_user_ui().draw_pos;
                        let scale = avb.canvases[CanvasKind::Draw.idx()].get_scale();
                        let hit_radius = 6.0 / scale.max(0.001);
                        let found = avb.canvases[CanvasKind::Draw.idx()]
                            .dataset
                            .shapes
                            .iter()
                            .find_map(|(&sid, shape)| {
                                shape
                                    .rotation_handle_world_pos(scale)
                                    .filter(|&p| (p - draw_pos).hypot() < hit_radius)
                                    .map(|_| {
                                        let center = shape.bbox_center_pub();
                                        let start_angle =
                                            (draw_pos - center).y.atan2((draw_pos - center).x);
                                        RotationDrag {
                                            shape_id: sid,
                                            center,
                                            start_angle,
                                            saved_rotation: shape.get_rotation(),
                                        }
                                    })
                            });
                        let _ = snap_angle; // used in Move handler
                        if let Some(rd) = found {
                            avb.rotation_drag = Some(rd);
                            avb.selection_window = None;
                            drop(avb);
                            render_active_view(av.clone());
                            return Ok(());
                        }
                    }
                    if avb.set_element_select_vertex() {
                        avb.selection_window = None;
                        do_render = true;
                        drop(avb);
                        if do_render {
                            render_active_view(av.clone());
                        }
                        return Ok(());
                    }
                    let draw_pos = avb.get_active_canvas().get_user_ui().draw_pos();
                    let selected_len = avb.canvases[CanvasKind::Draw.idx()]
                        .dataset
                        .shapes_selected
                        .len();
                    let has_selected_shape_at_pos = avb.canvases[CanvasKind::Draw.idx()]
                        .dataset
                        .has_selected_element_at(draw_pos);
                    let has_shape_at_pos = avb.canvases[CanvasKind::Draw.idx()]
                        .dataset
                        .has_element_at(draw_pos);
                    if has_selected_shape_at_pos && selected_len > 1 {
                        avb.selection_window = None;
                    } else if has_shape_at_pos {
                        avb.selection_window = None;
                        avb.set_select_elements();
                    } else {
                        avb.selection_window = Some(SelectionWindow {
                            start: draw_pos,
                            current: draw_pos,
                        });
                    }
                    do_render = true;
                } else {
                    avb.get_active_canvas_mut().save_offset();
                }
            }
            UserAction::ClickUp(button, _) => {
                if button == MouseButton::Left && is_draw_view {
                    if avb.dim_drag.take().is_some() {
                        do_render = true;
                        drop(avb);
                        if do_render {
                            render_active_view(av.clone());
                        }
                        return Ok(());
                    }
                    if avb.rotation_drag.take().is_some() {
                        {
                            let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
                            canvas.dataset.mark_final_polygon_dirty();
                            canvas.dataset.calc_final_polygon();
                            canvas.commit_history_action();
                        }
                        avb.refresh_toolpath_cache();
                        avb.refresh_gcode_cache();
                        drop(avb);
                        render_active_view(av.clone());
                        return Ok(());
                    }
                    if let Some(selection_window) = avb.selection_window.take() {
                        let end = avb.get_active_canvas().get_user_ui().draw_pos();
                        let delta = end - selection_window.start;
                        let append = avb
                            .get_active_canvas()
                            .get_user_ui()
                            .keys_states
                            .shift_pressed;
                        if delta.hypot() > 0.0 {
                            let select_touching = delta.x < 0.0;
                            avb.canvases[CanvasKind::Draw.idx()]
                                .dataset
                                .select_elements_in_window(
                                    selection_window.start,
                                    end,
                                    select_touching,
                                    append,
                                );
                        } else if !append {
                            let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
                            canvas.dataset.shapes_selected.clear();
                            canvas.dataset.vertex_selected = None;
                        }
                    }
                    let needs_final_update = avb.canvases[CanvasKind::Draw.idx()]
                        .dataset
                        .selection_affects_final_polygon();
                    if needs_final_update {
                        {
                            let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
                            canvas.dataset.refresh_svg_cache();
                            canvas.dataset.mark_final_polygon_dirty();
                        }
                        avb.refresh_toolpath_cache();
                        let toolpath = avb.tp.toolpath.clone().unwrap_or(Toolpath::new(Vec::new()));
                        let gcode = toolpath_to_plasma_gcode(&toolpath, &avb.tp.params);
                        avb.gc.last_gcode = Some(gcode);
                        avb.refresh_gcode_cache();
                    }
                    avb.canvases[CanvasKind::Draw.idx()].commit_history_action();
                    do_render = true;
                }
            }
        },
        Disc | Square | Oblong | Voronoi | ConstrLine | ConstrCircle | Poly | Text => {
            match user_action {
                UserAction::Move(_, _) => {
                    if is_draw_view && avb.element_on_creation.is_some() {
                        do_render = true;
                    }
                }
                UserAction::ClickDown(MouseButton::Left, _) => {
                    if !is_draw_view {
                        return Ok(());
                    }
                    if let Some((cs, mut vs)) = avb.element_on_creation.clone() {
                        vs.push(avb.get_active_canvas().get_user_ui().pointer.curr());
                        let canvas = avb.get_active_canvas_mut();
                        let mut finished = false;
                        match cs {
                            ShapeType::Disc => {
                                if let Some(e) = GeneralShape::new_shape_disc(vs[0], vs[1], 0) {
                                    let eid = canvas.dataset.push_element(e);
                                    canvas.dataset.select_only(eid);
                                    finished = true;
                                }
                            }
                            ShapeType::Square => {
                                if let Some(e) = GeneralShape::new_shape_rectangle(vs[0], vs[1], 0)
                                {
                                    let eid = canvas.dataset.push_element(e);
                                    canvas.dataset.select_only(eid);
                                    finished = true;
                                }
                            }
                            ShapeType::Oblong => {
                                if let Some(e) = GeneralShape::new_shape_oblong(vs[0], vs[1], 0) {
                                    let eid = canvas.dataset.push_element(e);
                                    canvas.dataset.select_only(eid);
                                    finished = true;
                                }
                            }
                            ShapeType::Voronoi => {
                                if let Some(e) = GeneralShape::new_shape_voronoi(vs[0], vs[1], 0) {
                                    let eid = canvas.dataset.push_element(e);
                                    canvas.dataset.select_only(eid);
                                    finished = true;
                                }
                            }
                            ShapeType::ConstrLine => {
                                if let Some(e) =
                                    GeneralShape::new_shape_constr_line(vs[0], vs[1], 0)
                                {
                                    let eid = canvas.dataset.push_element(e);
                                    canvas.dataset.select_only(eid);
                                    finished = true;
                                }
                            }
                            ShapeType::ConstrCircle => {
                                if let Some(e) =
                                    GeneralShape::new_shape_constr_circle(vs[0], vs[1], 0)
                                {
                                    let eid = canvas.dataset.push_element(e);
                                    canvas.dataset.select_only(eid);
                                    finished = true;
                                }
                            }
                            ShapeType::Text => {
                                if let Some(e) = GeneralShape::new_shape_text(vs[0], vs[1], 0) {
                                    let eid = canvas.dataset.push_element(e);
                                    canvas.dataset.select_only(eid);
                                    finished = true;
                                }
                            }
                            ShapeType::Poly => {
                                avb.element_on_creation = Some((cs, vs));
                            }
                            _ => {}
                        }
                        if finished {
                            avb.element_on_creation = None;
                            avb.go_to_arrow_tool();
                        }
                    } else {
                        let v1 = avb.get_active_canvas().get_user_ui().pointer.curr();
                        avb.element_on_creation = Some((avb.icon_selected, vec![v1]));
                    }
                    do_render = true;
                }
                _ => {}
            }
        }
        _ => {}
    }
    drop(avb);
    if do_render {
        render_active_view(av.clone());
    }
    Ok(())
}

pub(crate) fn on_draw_mouse_move(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.dyn_into::<MouseEvent>() {
        let sys_mouse = SystemMouse::Move;
        let action = {
            let Ok(mut avb) = av.try_borrow_mut() else {
                return;
            };
            let action = avb.update_canvas_inputs(mouse_event, sys_mouse);
            let note_outcome = handle_note_draft_move(&mut avb);
            (action, note_outcome)
        };
        if matches!(action.1, NoteDraftOutcome::Updated) {
            update_notes_view(av.clone());
        } else {
            let _ = update(av.clone(), action.0);
        }
    }
    let Ok(avb) = av.try_borrow() else {
        return;
    };
    let should_render = avb.active_canvas == CanvasKind::Draw;
    drop(avb);
    if should_render {
        render_draw_view(av);
    }
}

pub(crate) fn on_draw_mouse_down(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.dyn_into::<MouseEvent>() {
        let sys_mouse = SystemMouse::Down(mouse_event.detail());
        let action = {
            let Ok(mut avb) = av.try_borrow_mut() else {
                return;
            };
            let button = mouse_event.button();
            let action = avb.update_canvas_inputs(mouse_event.clone(), sys_mouse);
            let note_outcome = handle_note_draft_start(&mut avb, button);
            (action, note_outcome)
        };
        if matches!(action.1, NoteDraftOutcome::Started) {
            update_notes_view(av.clone());
        } else {
            let _ = update(av.clone(), action.0);
        }
    }
    let Ok(avb) = av.try_borrow() else {
        return;
    };
    let should_render = avb.active_canvas == CanvasKind::Draw;
    drop(avb);
    if should_render {
        render_draw_view(av);
    }
}

pub(crate) fn on_draw_mouse_up(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.dyn_into::<MouseEvent>() {
        let sys_mouse = SystemMouse::Up(mouse_event.detail());
        let action = {
            let Ok(mut avb) = av.try_borrow_mut() else {
                return;
            };
            let action = avb.update_canvas_inputs(mouse_event, sys_mouse);
            let note_outcome = handle_note_draft_finish(&mut avb);
            (action, note_outcome)
        };
        if let NoteDraftOutcome::Finished(id) = action.1 {
            focus_note_after_create(av.clone(), id);
        } else {
            let _ = update(av.clone(), action.0);
        }
    }
    let Ok(avb) = av.try_borrow() else {
        return;
    };
    let should_render = avb.active_canvas == CanvasKind::Draw;
    drop(avb);
    if should_render {
        render_draw_view(av);
    }
}

pub(crate) fn on_draw_mouse_wheel(av: RefAV, event: Event) {
    if let Ok(wheel_event) = event.dyn_into::<WheelEvent>() {
        wheel_event.prevent_default();
        wheel_event.stop_propagation();
        let delta_y = wheel_event.delta_y();
        if let Ok(mouse_event) = wheel_event.clone().dyn_into::<MouseEvent>() {
            let Ok(mut avb) = av.try_borrow_mut() else {
                return;
            };
            let sys_mouse = SystemMouse::Move;
            let _ = avb.update_canvas_inputs(mouse_event, sys_mouse);
            let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
            let world_pos = canvas.get_user_ui().draw_pos;
            let factor = if delta_y > 0.0 { 0.95 } else { 1.05 };
            zoom_canvas_at(canvas, world_pos, factor);
            drop(avb);
            render_draw_view(av.clone());
        }
    }
}

pub(crate) fn on_draw_mouse_enter(av: RefAV, _event: Event) {
    let Ok(mut avb) = av.try_borrow_mut() else {
        return;
    };
    avb.canvases[CanvasKind::Draw.idx()].set_pointer_on_canvas(true);
    // avb.update_draw_cursor();
    if avb.element_on_creation.is_some() {
        avb.element_on_creation = None;
        avb.go_to_arrow_tool();
        drop(avb);
        render_draw_view(av);
        return;
    }
    drop(avb);
}

pub(crate) fn on_draw_mouse_leave(av: RefAV, _event: Event) {
    let Ok(mut avb) = av.try_borrow_mut() else {
        return;
    };
    let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
    canvas.set_pointer_on_canvas(false);
    canvas.abort_history_action();
    avb.selection_window = None;
    avb.element_on_creation = None;
    avb.go_to_arrow_tool();
    // avb.update_draw_cursor();
    drop(avb);
    render_draw_view(av);
}

pub(crate) fn on_draw_context_menu(av: RefAV, event: Event) {
    event.prevent_default();
    let finalized = av
        .try_borrow_mut()
        .ok()
        .map_or(false, |mut avb| avb.finalize_poly());
    if finalized {
        render_draw_view(av);
    }
}

pub(crate) fn draw_reset_origin(av: RefAV) {
    av.borrow_mut().get_active_canvas_mut().reset_origin();
}

pub(crate) fn draw_grid_and_rules(av: RefAV) {
    let mut avb = av.borrow_mut();
    let draw_scale = avb.canvases[CanvasKind::Draw.idx()].get_scale();
    let draw_offset = avb.canvases[CanvasKind::Draw.idx()].get_offset();
    let grid_canvas = &mut avb.canvases[CanvasKind::Grid.idx()];
    grid_canvas.draw_rules_only(draw_scale, draw_offset);
}

pub(crate) fn render_draw_view(av: RefAV) {
    begin_render(av.clone(), "Draw");
    draw_grid_and_rules(av.clone());
    update_status_bar(av.clone());
    update_shapes_panel(av.clone());

    {
        let mut avb = av.borrow_mut();
        let svg_bbox_only = true;
        let element_on_creation = avb.element_on_creation.clone();
        let selection_window = avb.selection_window;
        let canvas_draw = &mut avb.canvases[CanvasKind::Draw.idx()];

        canvas_draw.clear();

        canvas_draw.draw_closed_path(Pattern::Composed(true), Color::Black, Color::Gray20, vec![]);

        if !canvas_draw.get_user_ui().keys_states.alt_pressed {
            canvas_draw.draw_paths_sets_with_svg_bbox(svg_bbox_only);

            canvas_draw.draw_vertices();
        }

        if let Some((cs, mut vs)) = element_on_creation {
            use crate::shape::ShapeType::*;
            match cs {
                Disc => {
                    if vs.len() == 1 {
                        vs.push(canvas_draw.get_user_ui().pointer.curr());
                        if let Some(e) = GeneralShape::new_shape_disc(vs[0], vs[1], 0) {
                            canvas_draw.draw_paths_creation(&e);
                            canvas_draw.draw_vs(&e);
                            canvas_draw.draw_dimensions(&e);
                        }
                    }
                }
                Square => {
                    if vs.len() == 1 {
                        vs.push(canvas_draw.get_user_ui().pointer.curr());
                        if let Some(e) = GeneralShape::new_shape_rectangle(vs[0], vs[1], 0) {
                            canvas_draw.draw_paths_creation(&e);
                            canvas_draw.draw_vs(&e);
                            canvas_draw.draw_dimensions(&e);
                        }
                    }
                }
                Oblong => {
                    if vs.len() == 1 {
                        vs.push(canvas_draw.get_user_ui().pointer.curr());
                        if let Some(e) = GeneralShape::new_shape_oblong(vs[0], vs[1], 0) {
                            canvas_draw.draw_paths_creation(&e);
                            canvas_draw.draw_vs(&e);
                            canvas_draw.draw_dimensions(&e);
                        }
                    }
                }
                Voronoi => {
                    if vs.len() == 1 {
                        vs.push(canvas_draw.get_user_ui().pointer.curr());
                        let min = Vec2::new(vs[0].x.min(vs[1].x), vs[0].y.min(vs[1].y));
                        let max = Vec2::new(vs[0].x.max(vs[1].x), vs[0].y.max(vs[1].y));
                        let mut path = BezPath::new();
                        path.move_to(Point::new(min.x, min.y));
                        path.line_to(Point::new(min.x, max.y));
                        path.line_to(Point::new(max.x, max.y));
                        path.line_to(Point::new(max.x, min.y));
                        path.close_path();
                        let colors = get_vertices_colors(false, false);
                        canvas_draw.draw_path(
                            &path,
                            Pattern::OnCreation,
                            colors.fill_color,
                            colors.stroke_color,
                            vec![],
                        );
                    }
                }
                ConstrLine => {
                    if vs.len() == 1 {
                        vs.push(canvas_draw.get_user_ui().pointer.curr());
                        if let Some(e) = GeneralShape::new_shape_constr_line(vs[0], vs[1], 0) {
                            canvas_draw.draw_paths_creation(&e);
                            canvas_draw.draw_vs(&e);
                            canvas_draw.draw_dimensions(&e);
                        }
                    }
                }
                ConstrCircle => {
                    if vs.len() == 1 {
                        vs.push(canvas_draw.get_user_ui().pointer.curr());
                        if let Some(e) = GeneralShape::new_shape_constr_circle(vs[0], vs[1], 0) {
                            canvas_draw.draw_paths_creation(&e);
                            canvas_draw.draw_vs(&e);
                            canvas_draw.draw_dimensions(&e);
                        }
                    }
                }
                Text => {
                    if vs.len() == 1 {
                        vs.push(canvas_draw.get_user_ui().pointer.curr());
                        if let Some(e) = GeneralShape::new_shape_text(vs[0], vs[1], 0) {
                            canvas_draw.draw_paths_creation(&e);
                            canvas_draw.draw_vs(&e);
                            canvas_draw.draw_dimensions(&e);
                        }
                    }
                }
                Poly => {
                    if vs.len() > 2 {
                        vs.push(canvas_draw.get_user_ui().pointer.curr());
                        if let Some(e) = GeneralShape::new_shape_poly(vs, 0) {
                            canvas_draw.draw_paths_creation(&e);
                            canvas_draw.draw_vs(&e);
                            canvas_draw.draw_dimensions(&e);
                        }
                    } else {
                        vs.push(canvas_draw.get_user_ui().pointer.curr());
                        if vs.len() >= 2 {
                            let colors = get_vertices_colors(false, false);
                            for (i, v) in vs.iter().enumerate() {
                                if i < vs.len() - 1 {
                                    canvas_draw.draw_path(
                                        &line_path(vs[i], vs[i + 1]),
                                        Pattern::OnCreation,
                                        colors.fill_color,
                                        colors.stroke_color,
                                        vec![],
                                    );
                                    if let Some(seg) = SegBundle::new(vs[i], vs[i + 1]) {
                                        let (path, pattern, colors, text) =
                                            dim_hv(seg, canvas_draw.get_canvas_infos(), false);
                                        canvas_draw.draw_path(
                                            &path,
                                            pattern,
                                            colors.fill_color,
                                            colors.stroke_color,
                                            text,
                                        );
                                    }
                                }
                                canvas_draw.draw_path(
                                    &point_path(*v, 1.),
                                    Pattern::Point,
                                    colors.fill_color,
                                    colors.stroke_color,
                                    vec![],
                                );
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if let Some(selection_window) = selection_window {
            let path = selection_window_path(selection_window.start, selection_window.current);
            canvas_draw.draw_path(
                &path,
                Pattern::OnCreation,
                Color::Transparent,
                Color::OnCreation,
                vec![],
            );
        }
        canvas_draw.draw_pointer(canvas_draw.get_user_ui().pointer.curr());
        avb.update_draw_cursor();
    }

    update_notes_view(av.clone());
    end_render(av.clone());
    update_status_bar(av);
}

pub(crate) fn on_icon_click(av: RefAV, icon: ShapeType) {
    let Ok(mut avb) = av.try_borrow_mut() else {
        return;
    };
    avb.icon_selected = icon;
    avb.notes.mode = false;
    avb.notes.draft = None;
    avb.notes.drag = None;
    avb.user_icons
        .iter()
        .for_each(|icon| avb.html_deselect_icons(*icon));
    avb.html_select_icon(icon);
    avb.html_deselect_note_icon();
    drop(avb);
}

pub(crate) fn on_icon_mouseover(av: RefAV, event: Event, icon: ShapeType) {
    if let Ok(event) = event.dyn_into::<MouseEvent>() {
        let Ok(avb) = av.try_borrow_mut() else {
            return;
        };
        if let Some(html_element) = icon.get_html_element() {
            let selected_color = Color::OnCreation.get();
            let _ = html_element.set_attribute("style", &format!("color:{selected_color}"));
        }
        show_tooltip(&avb, event.x(), event.y(), icon_tooltip(icon));
    }
}

pub(crate) fn on_icon_mouseover_label(av: RefAV, event: Event, label: &'static str) {
    if let Ok(event) = event.dyn_into::<MouseEvent>() {
        let Ok(avb) = av.try_borrow_mut() else {
            return;
        };
        show_tooltip(&avb, event.x(), event.y(), label);
    }
}

pub(crate) fn on_icon_mouseout(av: RefAV, _event: Event) {
    let Ok(avb) = av.try_borrow_mut() else {
        return;
    };
    let _ = avb.tooltip.set_attribute("style", "display:none;");
    if !avb.notes.mode {
        if let Some(html_element) = avb.icon_selected.get_html_element() {
            let selected_color = Color::OnCreation.get();
            let _ = html_element.set_attribute("style", &format!("color:{selected_color}"));
        }
    } else if let Some(html_element) = avb.document.get_element_by_id("icon-note") {
        if let Ok(html_element) = html_element.dyn_into::<HtmlElement>() {
            let selected_color = Color::OnCreation.get();
            let _ = html_element.set_attribute("style", &format!("color:{selected_color}"));
        }
    }
    avb.user_icons.iter().for_each(|icon| {
        if *icon != avb.icon_selected || avb.notes.mode {
            if let Some(html_element) = icon.get_html_element() {
                let text_color = Color::Text.get();
                let _ = html_element.set_attribute("style", &format!("color:{text_color}"));
            }
        }
    });
}

fn render_active_view(av: RefAV) {
    let active = av.borrow().active_view;
    match active {
        Tabs::Draw => render_draw_view(av),
        Tabs::Gcode => render_gcode_view(av),
        Tabs::Toolpath => render_toolpath_view(av),
        Tabs::Machine => render_machine_view(av),
        Tabs::Play => crate::view_play::app::render_play_view(av),
    }
}

fn selection_window_path(start: Vec2, end: Vec2) -> BezPath {
    let min = Vec2::new(start.x.min(end.x), start.y.min(end.y));
    let max = Vec2::new(start.x.max(end.x), start.y.max(end.y));
    let mut path = BezPath::new();
    path.move_to(Point::new(min.x, min.y));
    path.line_to(Point::new(min.x, max.y));
    path.line_to(Point::new(max.x, max.y));
    path.line_to(Point::new(max.x, min.y));
    path.close_path();
    path
}

fn icon_tooltip(icon: ShapeType) -> &'static str {
    match icon {
        ShapeType::Arrow => "Arrow",
        ShapeType::Disc => "Disc",
        ShapeType::Square => "Square",
        ShapeType::Oblong => "Oblong",
        ShapeType::Poly => "Polygon",
        ShapeType::Text => "Text",
        ShapeType::Svg => "SVG",
        ShapeType::Voronoi => "Voronoi",
        ShapeType::Group => "Group",
        ShapeType::ConstrLine => "Construction Line",
        ShapeType::ConstrCircle => "Construction Circle",
    }
}

fn show_tooltip(avb: &crate::app::AppVars, x: i32, y: i32, label: &str) {
    avb.tooltip
        .set_attribute("style", &format!("display:block;left:{x}px;top:{y}px"))
        .unwrap();
    avb.tooltip.set_inner_text(label);
}

pub(crate) fn shape_type_label(shape_type: ShapeType) -> &'static str {
    match shape_type {
        ShapeType::Disc => "Disc",
        ShapeType::Square => "Square",
        ShapeType::Oblong => "Oblong",
        ShapeType::Poly => "Polygon",
        ShapeType::Text => "Text",
        ShapeType::Svg => "Svg",
        ShapeType::Voronoi => "Voronoi",
        ShapeType::Group => "Group",
        ShapeType::ConstrLine => "Line",
        ShapeType::ConstrCircle => "Circle",
        ShapeType::Arrow => "Arrow",
    }
}
pub(crate) fn reorder_shapes_by_index(av: RefAV, from_idx: usize, to_idx: usize) {
    let mut avb = av.borrow_mut();
    if !matches!(avb.active_view, Tabs::Draw) {
        return;
    }
    let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
    let mut ordered = canvas.dataset.ordered_shapes();
    if from_idx >= ordered.len() {
        return;
    }
    let target_idx = to_idx.min(ordered.len());
    if from_idx == target_idx {
        return;
    }
    let eid = ordered.remove(from_idx);
    let insert_idx = if target_idx > from_idx {
        target_idx.saturating_sub(1)
    } else {
        target_idx
    };
    ordered.insert(insert_idx, eid);
    canvas.dataset.set_order_sequence(&ordered);
    canvas.dataset.mark_final_polygon_dirty();
    canvas.dataset.calc_final_polygon();
    avb.refresh_toolpath_cache();
    avb.refresh_gcode_cache();
    drop(avb);
    render_draw_view(av);
}
