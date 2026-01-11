use crate::canvas::{Canvas, CanvasKind, Color};
use crate::cnc_link::CncLink;
use crate::dom::{get_element_height, get_element_width, ShapeType, Tabs};
use crate::gcode::gcode_to_segments;
use crate::handlers::{
    on_draw_context_menu, on_draw_mouse_down, on_draw_mouse_enter, on_draw_mouse_leave,
    on_draw_mouse_move, on_draw_mouse_up, on_draw_mouse_wheel, on_gcode_context_menu,
    on_gcode_mouse_down, on_gcode_mouse_move, on_gcode_mouse_up, on_gcode_mouse_wheel,
    on_icon_click, on_icon_mouseout, on_icon_mouseover, on_icon_mouseover_label, on_tab_click,
    on_toolpath_context_menu, on_toolpath_mouse_down, on_toolpath_mouse_move, on_toolpath_mouse_up,
    on_toolpath_mouse_wheel, on_window_click, on_window_keydown, on_window_keyup, on_window_resize,
    resize_canvases,
};
use crate::import_export::{
    build_json_from_dataset, build_svg_from_dataset, get_prop, load_json_to_dataset,
    load_svg_to_dataset, make_export_info, timestamp_string, trigger_download,
};
use crate::inputs::{SystemMouse, UserAction};
use crate::machine::{
    build_machine_view, load_machine_schema, machine_input_id, parse_grbl_setting_line,
    update_machine_value, MachineGroup,
};
use crate::prefab::line_path;
use crate::render::{
    draw_grid_and_rules, draw_reset_origin, render_draw_view, render_gcode_view,
    render_toolpath_view, toolpath_arc_path, toolpath_arrowhead_path, toolpath_points_to_path,
};
use crate::shapes::{multipolygon_to_toolpath, toolpath_to_plasma_gcode, Toolpath, ToolpathParams};
use crate::status::update_status_bar;
use crate::types::{EUId, VUId};
use js_sys::{Array, Date, Object, Reflect, JSON};
use kurbo::{BezPath, PathEl, Point, Size, Vec2};
use std::collections::HashSet;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    Blob, Document, Element, Event, FileReader, HtmlAnchorElement, HtmlCanvasElement, HtmlElement,
    HtmlInputElement, KeyboardEvent, MouseEvent, Url, Window,
};

pub(crate) type RefAV = Rc<RefCell<AppVars>>;

#[allow(dead_code)]
pub(crate) struct AppVars {
    pub(crate) element_on_creation: Option<(ShapeType, Vec<Vec2>)>,
    pub(crate) render_start: Option<(&'static str, f64)>,
    pub(crate) render_notice: Option<(String, f64)>,

    // DOM
    pub(crate) window: Window,
    pub(crate) document: Document,
    pub(crate) top_menu: HtmlElement,
    pub(crate) left_panel: HtmlElement,
    pub(crate) user_icons: HashSet<ShapeType>,
    pub(crate) tooltip: HtmlElement,
    pub(crate) icon_selected: ShapeType,
    pub(crate) canvases: [Canvas; CanvasKind::COUNT],
    pub(crate) active_canvas: CanvasKind,
    pub(crate) active_view: Tabs,

    pub(crate) last_gcode: Option<String>,
    pub(crate) gcode_auto_center: bool,
    pub(crate) gcode_auto_fit: bool,
    pub(crate) gcode_segments: Vec<crate::gcode::Seg>,
    pub(crate) gcode_cut_path: Option<BezPath>,
    pub(crate) toolpath: Option<Toolpath>,
    pub(crate) toolpath_params: ToolpathParams,
    pub(crate) toolpath_paths: Vec<BezPath>,
    pub(crate) toolpath_lead_ins: Vec<BezPath>,
    pub(crate) toolpath_lead_outs: Vec<BezPath>,
    pub(crate) toolpath_travels: Vec<BezPath>,
    pub(crate) toolpath_travel_arrows: Vec<BezPath>,
    pub(crate) toolpath_arrows: Vec<BezPath>,
    pub(crate) toolpath_starts: Vec<Vec2>,
    pub(crate) toolpath_ends: Vec<Vec2>,
    pub(crate) toolpath_original_paths: Vec<BezPath>,
    pub(crate) toolpath_auto_center: bool,
    pub(crate) toolpath_auto_fit: bool,
    pub(crate) machine_groups: Vec<MachineGroup>,
    pub(crate) machine_view_built: bool,
    pub(crate) machine_last_update: Option<String>,
    pub(crate) ws_connected: bool,
    pub(crate) ws_status: String,
    pub(crate) last_ws_error: Option<String>,
    pub(crate) last_http_error: Option<String>,
    pub(crate) cnc: Option<Rc<CncLink>>,
}

impl AppVars {
    pub(crate) fn ensure_machine_view(&mut self, av: RefAV) -> Result<(), JsValue> {
        if self.machine_groups.is_empty() {
            self.machine_groups = load_machine_schema();
        }
        if !self.machine_view_built {
            build_machine_view(&self.document, &self.machine_groups)?;
            self.wire_machine_inputs(av.clone())?;
            self.wire_machine_controls(av)?;
            self.set_machine_status_time(None);
            self.set_machine_status_error();
            self.machine_view_built = true;
        }
        Ok(())
    }

    pub(crate) fn request_machine_settings(&mut self, av: RefAV) {
        if let Some(cnc) = &self.cnc {
            self.set_machine_status("Status: updating...", Some("pending"));
            self.last_http_error = None;
            self.set_machine_status_error();
            let cnc = cnc.clone();
            let av = av.clone();
            spawn_local(async move {
                let result = cnc.send_http_cmd_ts("$$").await;
                if let Ok(mut avb) = av.try_borrow_mut() {
                    match result {
                        Ok(true) => avb.last_http_error = None,
                        Ok(false) => {
                            avb.last_http_error = Some("Machine settings request failed: $$".into())
                        }
                        Err(_) => {
                            avb.last_http_error = Some("Machine settings request failed: $$".into())
                        }
                    }
                    if avb.last_http_error.is_some() {
                        avb.set_machine_status("Status: unable to update", Some("error"));
                    }
                    avb.set_machine_status_error();
                }
            });
        }
    }

    pub(crate) fn handle_ws_text(&mut self, msg: &str) {
        let mut updated = 0;
        for line in msg.lines() {
            if let Some((id, value)) = parse_grbl_setting_line(line) {
                if update_machine_value(&mut self.machine_groups, &id, &value) {
                    self.update_machine_input(&id, &value);
                    updated += 1;
                }
            }
        }
        if updated > 0 {
            self.set_machine_status("Status: updated", Some("ok"));
            let now = Date::new_0().to_string().as_string().unwrap_or_default();
            self.machine_last_update = Some(now.clone());
            self.set_machine_status_time(Some(&now));
            self.last_http_error = None;
            self.set_machine_status_error();
        }
    }

    fn update_machine_input(&self, id: &str, value: &str) {
        let input_id = machine_input_id(id);
        let Some(el) = self.document.get_element_by_id(&input_id) else {
            return;
        };
        if let Ok(input) = el.dyn_into::<HtmlInputElement>() {
            input.set_value(value);
        }
    }

    fn wire_machine_inputs(&self, av: RefAV) -> Result<(), JsValue> {
        let Some(cnc) = self.cnc.clone() else {
            return Ok(());
        };
        for group in self.machine_groups.iter() {
            for setting in group.settings.iter() {
                let input_id = machine_input_id(&setting.id);
                let Some(el) = self.document.get_element_by_id(&input_id) else {
                    continue;
                };
                let input: HtmlInputElement = el.dyn_into()?;
                let input_cb = input.clone();
                let id = setting.id.clone();
                let cnc = cnc.clone();
                let av = av.clone();
                let on_keydown = Closure::<dyn FnMut(Event)>::new(move |evt: Event| {
                    let Ok(evt) = evt.dyn_into::<KeyboardEvent>() else {
                        return;
                    };
                    if evt.key() != "Enter" {
                        return;
                    }
                    evt.prevent_default();
                    let value = input_cb.value();
                    let cmd = format!("${}={}", id, value);
                    let cnc = cnc.clone();
                    let av = av.clone();
                    spawn_local(async move {
                        let result = cnc.send_http_cmd_ts(&cmd).await;
                        if let Ok(mut avb) = av.try_borrow_mut() {
                            match result {
                                Ok(true) => avb.last_http_error = None,
                                Ok(false) => {
                                    avb.last_http_error =
                                        Some(format!("Machine setting failed: {cmd}"))
                                }
                                Err(_) => {
                                    avb.last_http_error =
                                        Some(format!("Machine setting failed: {cmd}"))
                                }
                            }
                            avb.set_machine_status_error();
                        }
                    });
                });
                input.add_event_listener_with_callback(
                    "keydown",
                    on_keydown.as_ref().unchecked_ref(),
                )?;
                on_keydown.forget();
            }
        }
        Ok(())
    }

    fn wire_machine_controls(&self, av: RefAV) -> Result<(), JsValue> {
        let Some(el) = self.document.get_element_by_id("machine-refresh") else {
            return Ok(());
        };
        let button: HtmlElement = el.dyn_into()?;
        let av = av.clone();
        let on_click = Closure::<dyn FnMut(Event)>::new(move |_evt: Event| {
            if let Ok(mut avb) = av.try_borrow_mut() {
                avb.request_machine_settings(av.clone());
            }
        });
        button.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())?;
        on_click.forget();
        Ok(())
    }

    fn set_machine_status(&self, text: &str, state: Option<&str>) {
        let Some(el) = self.document.get_element_by_id("machine-status-text") else {
            return;
        };
        let Ok(el) = el.dyn_into::<HtmlElement>() else {
            return;
        };
        el.set_inner_text(text);
        let classes = el.class_list();
        let _ = classes.remove_1("ok");
        let _ = classes.remove_1("error");
        let _ = classes.remove_1("pending");
        if let Some(state) = state {
            let _ = classes.add_1(state);
        }
    }

    fn set_machine_status_time(&self, time: Option<&str>) {
        let Some(el) = self.document.get_element_by_id("machine-status-time") else {
            return;
        };
        let Ok(el) = el.dyn_into::<HtmlElement>() else {
            return;
        };
        let text = match time {
            Some(time) if !time.is_empty() => format!("Last update: {time}"),
            _ => "Last update: --".to_string(),
        };
        el.set_inner_text(&text);
    }

    fn set_machine_status_error(&self) {
        let Some(el) = self.document.get_element_by_id("machine-status-error") else {
            return;
        };
        let Ok(el) = el.dyn_into::<HtmlElement>() else {
            return;
        };
        let mut parts = Vec::new();
        if let Some(msg) = self.last_ws_error.as_ref() {
            parts.push(format!("WS: {msg}"));
        }
        if let Some(msg) = self.last_http_error.as_ref() {
            parts.push(format!("HTTP: {msg}"));
        }
        let text = parts.join(" | ");
        el.set_inner_text(&text);
        let classes = el.class_list();
        let _ = classes.remove_1("active");
        if !text.is_empty() {
            let _ = classes.add_1("active");
        }
    }

    pub(crate) fn esc_pressed(&mut self) {
        self.element_on_creation = None;
        self.go_to_arrow_tool();
    }
    pub(crate) fn ctrl_c_pressed(&mut self) {
        if let ShapeType::Arrow = self.icon_selected.clone() {
            let canvas_user = self.get_user_canvas_mut();
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
        if let ShapeType::Arrow = self.icon_selected.clone() {
            let canvas_user = self.get_user_canvas_mut();
            if let Some(pasted) = canvas_user
                .clipboard
                .make_paste(&canvas_user.get_user_ui().pointer)
            {
                canvas_user.dataset.push_element(pasted);
                canvas_user.dataset.mark_final_polygon_dirty();
                canvas_user.dataset.calc_final_polygon();
            }
        }
    }
    pub(crate) fn refresh_gcode_cache(&mut self) {
        let gcode = self.last_gcode.as_deref().unwrap_or("");
        let segs = gcode_to_segments(gcode);
        let mut path = BezPath::new();
        for s in segs.iter().filter(|s| s.cut) {
            path.push(PathEl::MoveTo(Point::new(s.x1, s.y1)));
            path.push(PathEl::LineTo(Point::new(s.x2, s.y2)));
        }
        self.gcode_segments = segs;
        self.gcode_cut_path = if path.is_empty() { None } else { Some(path) };
    }
    pub(crate) fn refresh_toolpath_cache(&mut self) {
        let canvas_draw = &mut self.canvases[CanvasKind::Draw.idx()];
        canvas_draw.dataset.calc_final_polygon();
        let original_paths = canvas_draw.dataset.final_paths.clone();
        let toolpath =
            multipolygon_to_toolpath(&canvas_draw.dataset.final_polygon, &self.toolpath_params);
        self.last_gcode = None;
        let mut paths = Vec::new();
        let mut lead_ins = Vec::new();
        let mut lead_outs = Vec::new();
        let mut travels = Vec::new();
        let mut travel_arrows = Vec::new();
        let mut arrows = Vec::new();
        let mut starts = Vec::new();
        let mut ends = Vec::new();
        let mut prev_end: Option<Vec2> = None;
        for contour in toolpath.contours.iter() {
            if let Some(path) = toolpath_points_to_path(&contour.points) {
                paths.push(path);
            }
            if let Some(lead) = contour.lead_in {
                if let Some(path) = toolpath_arc_path(lead.center, lead.start, lead.end, lead.ccw) {
                    lead_ins.push(path);
                }
            }
            if let Some(lead) = contour.lead_out {
                if let Some(path) = toolpath_arc_path(lead.center, lead.start, lead.end, lead.ccw) {
                    lead_outs.push(path);
                }
            }
            let start_point = contour
                .lead_in
                .map(|lead| lead.start)
                .unwrap_or(contour.points[0]);
            let end_point = contour
                .lead_out
                .map(|lead| lead.end)
                .unwrap_or(contour.points[0]);
            if let Some(prev) = prev_end {
                travels.push(line_path(prev, start_point));
                let dir = start_point - prev;
                let mid = (prev + start_point) * 0.5;
                if let Some(path) = toolpath_arrowhead_path(mid, dir, 2.5) {
                    travel_arrows.push(path);
                }
            }
            prev_end = Some(end_point);
            starts.push(start_point);
            ends.push(end_point);
            if contour.points.len() >= 2 {
                let dir = contour.points[1] - contour.points[0];
                if let Some(path) = toolpath_arrowhead_path(contour.points[0], dir, 2.5) {
                    arrows.push(path);
                }
            }
        }
        self.toolpath = Some(toolpath);
        self.toolpath_paths = paths;
        self.toolpath_lead_ins = lead_ins;
        self.toolpath_lead_outs = lead_outs;
        self.toolpath_travels = travels;
        self.toolpath_travel_arrows = travel_arrows;
        self.toolpath_arrows = arrows;
        self.toolpath_starts = starts;
        self.toolpath_ends = ends;
        self.toolpath_original_paths = original_paths;
        if self.active_canvas == CanvasKind::Gcode {
            let gcode = self
                .toolpath
                .as_ref()
                .map(|tp| toolpath_to_plasma_gcode(tp, &self.toolpath_params))
                .unwrap_or_else(String::new);
            self.last_gcode = Some(gcode);
            self.refresh_gcode_cache();
        }
    }
    pub(crate) fn edit_text_at(&mut self, pos: Vec2) -> bool {
        let canvas = &mut self.canvases[CanvasKind::Draw.idx()];
        let shift_pressed = canvas.get_user_ui().keys_states.shift_pressed;
        for (_eid, shape) in canvas.dataset.shapes.iter_mut() {
            if shape.get_shape_type() != ShapeType::Text {
                continue;
            }
            if !shape.contains(pos) {
                continue;
            }
            if shift_pressed {
                let auto_fit = shape.get_text().map(|text| !text.auto_fit).unwrap_or(false);
                shape.set_text_autofit(auto_fit);
                canvas.dataset.mark_final_polygon_dirty();
                canvas.dataset.calc_final_polygon();
                return true;
            }
            shape.ensure_text_scale();
            let current = shape
                .get_text()
                .map(|text| text.text.as_str())
                .unwrap_or("");
            let edited = self
                .window
                .prompt_with_message_and_default("Edit text", current)
                .ok()
                .flatten();
            if let Some(new_text) = edited {
                shape.set_text_value(new_text);
                shape.fit_text_bbox_width_to_content();
                canvas.dataset.mark_final_polygon_dirty();
                canvas.dataset.calc_final_polygon();
            }
            canvas.dataset.shapes_selected.clear();
            canvas.dataset.shapes_highlighted.clear();
            canvas.dataset.vertices_selected.clear();
            canvas.dataset.vertices_highlighted.clear();
            canvas
                .dataset
                .shapes_selector
                .refresh_selectable_elems(HashSet::new());
            return true;
        }
        false
    }
    pub(crate) fn del_back_pressed(&mut self) {
        if let ShapeType::Arrow = self.icon_selected {
            let canvas_user = self.get_user_canvas_mut();
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
                    .vertices_selected
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
    pub(crate) fn space_pressed(&mut self) {
        if let ShapeType::Arrow = self.icon_selected {
            let canvas_user = self.get_user_canvas_mut();
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
                .vertices_selected
                .iter()
                .copied()
                .collect();
            if vs_sel.len() == 2 {
                let (eid1, vid1) = vs_sel[0];
                let (eid2, vid2) = vs_sel[1];
                if eid1 == eid2 {
                    canvas_user
                        .dataset
                        .create_vertices_between(eid1, vid1, eid2, vid2);
                } else {
                    canvas_user
                        .dataset
                        .bind_unbind_vertices(eid1, vid1, eid2, vid2);
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
        }
    }

    pub(crate) fn change_vertex_radius(&mut self, inc: i32) {
        if let ShapeType::Arrow = self.icon_selected {
            let canvas_user = self.get_user_canvas_mut();
            let vs_sel: Vec<(EUId, VUId)> = canvas_user
                .dataset
                .vertices_selected
                .iter()
                .copied()
                .collect();
            if vs_sel.len() == 1 {
                let (eid, vid) = vs_sel[0];
                if let Some(elem) = canvas_user.dataset.get_element_mut(eid) {
                    if let Some(v) = elem.get_vertex_mut(&vid) {
                        if let Some(round) = v.rounded.as_mut() {
                            log!("round: {}", round);
                            let res = *round as i32 + inc;
                            if res > 0 {
                                *round = res as u32;
                                elem.set_bezpath();
                                canvas_user.dataset.mark_final_polygon_dirty();
                                canvas_user.dataset.calc_final_polygon();
                            }
                        }
                    }
                }
            }
        }
    }
    pub(crate) fn arrow_up_pressed(&mut self) {
        let inc = self.canvases[self.active_canvas.idx()]
            .get_user_ui()
            .snap
            .linear() as i32;
        self.change_vertex_radius(inc);
    }
    pub(crate) fn arrow_down_pressed(&mut self) {
        let inc = -(self.canvases[self.active_canvas.idx()]
            .get_user_ui()
            .snap
            .linear() as i32);
        self.change_vertex_radius(inc);
    }

    pub(crate) fn clear_toolpath_gcode(&mut self) {
        self.toolpath = None;
        self.toolpath_paths.clear();
        self.toolpath_lead_ins.clear();
        self.toolpath_lead_outs.clear();
        self.toolpath_travels.clear();
        self.toolpath_travel_arrows.clear();
        self.toolpath_arrows.clear();
        self.toolpath_starts.clear();
        self.toolpath_ends.clear();
        self.toolpath_original_paths.clear();
        self.last_gcode = Some(String::new());
        self.gcode_segments.clear();
        self.gcode_cut_path = None;
    }

    pub(crate) fn _undo(&mut self) {}
    pub(crate) fn _redo(&mut self) {}

    pub(crate) fn go_to_arrow_tool(&mut self) {
        self.icon_selected = ShapeType::Arrow;
        self.user_icons
            .iter()
            .for_each(|icon| self.html_deselect_icons(*icon));
        self.html_select_icon(ShapeType::Arrow);
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
        let canvas_user = self.get_user_canvas_mut();
        canvas_user
            .dataset
            .select_vertices(canvas_user.get_user_ui().draw_pos)
    }
    pub(crate) fn set_select_elements(&mut self) {
        self.get_user_canvas_mut().select_elements();
    }
    pub(crate) fn set_highlight_elements(&mut self) {
        self.get_user_canvas_mut().highlight_elements();
    }
    pub(crate) fn set_element_highlight_vertex(&mut self) -> bool {
        self.get_user_canvas_mut().highlight_vertices()
    }
    pub(crate) fn set_move_elements(&mut self) -> bool {
        self.get_user_canvas_mut().move_elements()
    }
    pub(crate) fn set_move_vertices_selected(&mut self) -> bool {
        self.get_user_canvas_mut().move_vertices_selected()
    }

    pub(crate) fn _get_user_canvas(&self) -> &Canvas {
        match self.active_canvas {
            CanvasKind::Gcode => &self.canvases[CanvasKind::Gcode.idx()],
            CanvasKind::Draw => &self.canvases[CanvasKind::Draw.idx()],
            CanvasKind::Background => &self.canvases[CanvasKind::Draw.idx()],
            CanvasKind::Grid => &self.canvases[CanvasKind::Draw.idx()],
            CanvasKind::Toolpath => &self.canvases[CanvasKind::Toolpath.idx()],
        }
    }
    pub(crate) fn get_user_canvas_mut(&mut self) -> &mut Canvas {
        match self.active_canvas {
            CanvasKind::Gcode => &mut self.canvases[CanvasKind::Gcode.idx()],
            CanvasKind::Draw => &mut self.canvases[CanvasKind::Draw.idx()],
            CanvasKind::Background => &mut self.canvases[CanvasKind::Draw.idx()],
            CanvasKind::Grid => &mut self.canvases[CanvasKind::Draw.idx()],
            CanvasKind::Toolpath => &mut self.canvases[CanvasKind::Toolpath.idx()],
        }
    }
    pub(crate) fn update_canvas_inputs(
        &mut self,
        mouse_event: MouseEvent,
        sys_mouse: SystemMouse,
    ) -> UserAction {
        let c_draw_origin = Vec2::new(
            get_element_width(&self.left_panel) as f64,
            get_element_height(&self.top_menu) as f64,
        );
        let action = self.canvases[self.active_canvas.idx()].update_ui(
            c_draw_origin,
            &mouse_event,
            sys_mouse,
        );
        self.snap_draw_to_constr_circle_vertices();
        action
    }

    pub(crate) fn update_draw_cursor(&mut self) {
        let canvas = &mut self.canvases[CanvasKind::Draw.idx()];
        if self.active_canvas != CanvasKind::Draw || !canvas.is_pointer_on_canvas() {
            canvas.set_cursor("default");
            return;
        }
        let has_vertex = canvas.dataset.vertices_highlighted.len() == 1;
        if has_vertex {
            if canvas.get_user_ui().keys_states.shift_pressed {
                canvas.set_cursor("url(\"assets/cursor_rotate_16.png\") 8 8, auto");
            }
        } else {
            canvas.set_cursor("default");
        }
    }

    fn snap_draw_to_constr_circle_vertices(&mut self) {
        if self.active_canvas != CanvasKind::Draw {
            return;
        }
        let canvas = &mut self.canvases[CanvasKind::Draw.idx()];
        let pos = canvas.get_user_ui().draw_pos;
        let threshold = canvas.get_user_ui().snap.linear().max(5.0);
        let mut best: Option<Vec2> = None;
        let mut best_dist = f64::INFINITY;
        for shape in canvas.dataset.shapes.values() {
            if shape.get_shape_type() != ShapeType::ConstrCircle {
                continue;
            }
            for (idx, (_, vertex)) in shape.get_vertices().iter().enumerate() {
                if idx < 2 {
                    continue;
                }
                let dist = (pos - vertex.curr).hypot();
                if dist <= threshold && dist < best_dist {
                    best_dist = dist;
                    best = Some(vertex.curr);
                }
            }
        }
        let user_ui = canvas.get_user_ui_mut();
        if let Some(target) = best {
            user_ui.draw_pos = target;
            user_ui.pointer.set(target);
            user_ui.magnetized = true;
        } else {
            user_ui.magnetized = false;
        }
    }
}

pub(crate) fn create_app_vars(window: Window) -> Result<(), JsValue> {
    log!("Creating application variables");
    log!("Initializing icons");
    let document = window.document().expect("should have a document on window");
    let c_draw: HtmlCanvasElement = init_element!(document, "mainCanvas", HtmlCanvasElement);
    let c_grid: HtmlCanvasElement = init_element!(document, "gridCanvas", HtmlCanvasElement);
    let c_back: HtmlCanvasElement = init_element!(document, "backgroundCanvas", HtmlCanvasElement);
    let c_gcode: HtmlCanvasElement = init_element!(document, "gcodeCanvas", HtmlCanvasElement);
    let c_toolpath: HtmlCanvasElement =
        init_element!(document, "toolpathCanvas", HtmlCanvasElement);
    let tooltip: HtmlElement = init_element!(document, "tooltip", HtmlElement);
    let left_panel: HtmlElement = init_element!(document, "left-panel", HtmlElement);
    let top_menu: HtmlElement = init_element!(document, "top-menu", HtmlElement);
    let canvases: [Canvas; CanvasKind::COUNT] = [
        Canvas::new(c_back, Size::new(3000., 1500.))?, // Background
        Canvas::new(c_grid, Size::new(3000., 1500.))?, // Grid
        Canvas::new(c_draw, Size::new(3000., 1500.))?, // Draw
        Canvas::new(c_gcode, Size::new(3000., 1500.))?, // Gcode
        Canvas::new(c_toolpath, Size::new(3000., 1500.))?, // Toolpath
    ];
    let active_canvas = CanvasKind::Draw;
    let mut user_icons: HashSet<ShapeType> = HashSet::new();
    use ShapeType::*;
    user_icons.insert(Arrow);
    user_icons.insert(Disc);
    user_icons.insert(Square);
    user_icons.insert(Oblong);
    user_icons.insert(Poly);
    user_icons.insert(Text);
    user_icons.insert(ConstrLine);
    user_icons.insert(ConstrCircle);

    let cnc: Option<Rc<CncLink>> = CncLink::connect("http://192.168.1.36", "ws://192.168.1.36:81/")
        .ok()
        .map(Rc::new);
    // let cnc: Option<Rc<CncLink>> =
    //     CncLink::connect("http://192.168.100.100", "ws://192.168.100.100:81/")
    //         .ok()
    //         .map(Rc::new);

    let app_vars = Rc::new(RefCell::new(AppVars {
        element_on_creation: None,
        render_start: None,
        render_notice: None,
        window,
        document,
        top_menu,
        left_panel,
        canvases,
        active_canvas,
        active_view: Tabs::Draw,
        user_icons,
        tooltip,
        icon_selected: ShapeType::Arrow,
        last_gcode: None,
        gcode_auto_center: false,
        gcode_auto_fit: false,
        gcode_segments: Vec::new(),
        gcode_cut_path: None,
        toolpath: None,
        toolpath_params: ToolpathParams {
            feed_xy: 1200.0,
            travel_feed_xy: 3000.0,
            pierce_delay_s: 0.3,
            lead_in_mm: 2.0,
            lead_out_mm: 2.0,
            kerf_mm: 1.0,
            torch_on_m3: "M3".to_string(),
            torch_off_m5: "M5".to_string(),
        },
        toolpath_paths: Vec::new(),
        toolpath_lead_ins: Vec::new(),
        toolpath_lead_outs: Vec::new(),
        toolpath_travels: Vec::new(),
        toolpath_travel_arrows: Vec::new(),
        toolpath_arrows: Vec::new(),
        toolpath_starts: Vec::new(),
        toolpath_ends: Vec::new(),
        toolpath_original_paths: Vec::new(),
        toolpath_auto_center: false,
        toolpath_auto_fit: false,
        machine_groups: Vec::new(),
        machine_view_built: false,
        machine_last_update: None,
        ws_connected: false,
        ws_status: "WS disconnected".to_string(),
        last_ws_error: None,
        last_http_error: None,
        cnc,
    }));

    if let Some(cnc) = app_vars.borrow().cnc.clone() {
        let av_clone = app_vars.clone();
        cnc.set_on_text_handler(Some(Box::new(move |msg| {
            if let Ok(mut avb) = av_clone.try_borrow_mut() {
                avb.handle_ws_text(&msg);
            }
        })));
        let av_clone = app_vars.clone();
        cnc.set_on_status_handler(Some(Box::new(move |msg| {
            if let Ok(mut avb) = av_clone.try_borrow_mut() {
                avb.ws_status = msg.clone();
                avb.ws_connected = msg.starts_with("WS connected");
                if msg.starts_with("WS error") || msg.starts_with("WS closed") {
                    avb.last_ws_error = Some(msg);
                } else {
                    avb.last_ws_error = None;
                }
                avb.set_machine_status_error();
            }
        })));
    }

    init_menu(app_vars.clone())?;
    init_tabs(app_vars.clone())?;
    init_gcode_splitter(app_vars.clone())?;
    init_toolpath_splitter(app_vars.clone())?;
    init_icons(app_vars.clone())?;
    init_status(app_vars.clone())?;
    init_draw_canvas(app_vars.clone())?;
    init_gcode_canvas(app_vars.clone())?;
    init_toolpath_canvas(app_vars.clone())?;
    init_toolpath_panel(app_vars.clone())?;
    init_window(app_vars.clone())?;
    resize_canvases(app_vars.clone());

    let av = app_vars;

    if let Some(panel) = av.borrow().document.get_element_by_id("left-panel") {
        let _ = panel.class_list().add_1("draw-mode");
    }

    draw_reset_origin(av.clone());
    draw_grid_and_rules(av.clone());
    update_status_bar(av.clone());
    render_draw_view(av.clone());

    Ok(())
}

pub(crate) fn init_window(av: RefAV) -> Result<(), JsValue> {
    let pa_cloneprim = av.clone();
    let pa_cloned2 = av.clone();

    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_window_resize(pa_cloneprim.clone(), event);
    });

    pa_cloned2
        .borrow_mut()
        .window
        .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())?;

    closure.forget();

    let pa_cloneprim = av.clone();
    let pa_cloned2 = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_window_click(pa_cloneprim.clone(), event);
    });
    pa_cloned2
        .borrow_mut()
        .window
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
    closure.forget();

    let pa_cloneprim = av.clone();
    let pa_cloned2 = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_window_keydown(pa_cloneprim.clone(), event);
    });
    pa_cloned2
        .borrow_mut()
        .window
        .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())?;
    closure.forget();

    let pa_cloneprim = av.clone();
    let pa_cloned2 = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_window_keyup(pa_cloneprim.clone(), event);
    });
    pa_cloned2
        .borrow_mut()
        .window
        .add_event_listener_with_callback("keyup", closure.as_ref().unchecked_ref())?;
    closure.forget();

    Ok(())
}

pub(crate) fn init_tabs(av: RefAV) -> Result<(), JsValue> {
    let tabs: HashSet<Tabs> = [Tabs::Draw, Tabs::Toolpath, Tabs::Gcode, Tabs::Machine]
        .into_iter()
        .collect();

    for tab in tabs.iter() {
        let el = tab
            .get_element()
            .unwrap_or_else(|| panic!("Tab element not found: {}", tab.id()));
        let tab_copy = *tab;
        set_callback(
            av.clone(),
            "click".into(),
            &el,
            Box::new(move |av, _evt| on_tab_click(av, tab_copy)),
        )?;
    }

    Ok(())
}

pub(crate) fn init_gcode_splitter(av: RefAV) -> Result<(), JsValue> {
    let window: Window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    let split: HtmlElement = document
        .get_element_by_id("gcode-split")
        .unwrap()
        .dyn_into()?;
    let left: HtmlElement = document
        .get_element_by_id("gcode-left")
        .unwrap()
        .dyn_into()?;
    let splitter: HtmlElement = document
        .get_element_by_id("gcode-splitter")
        .unwrap()
        .dyn_into()?;

    let dragging = std::rc::Rc::new(std::cell::Cell::new(false));

    {
        let dragging = dragging.clone();
        let on_down = Closure::<dyn FnMut(MouseEvent)>::new(move |e: MouseEvent| {
            e.prevent_default();
            dragging.set(true);
        });
        splitter.add_event_listener_with_callback("mousedown", on_down.as_ref().unchecked_ref())?;
        on_down.forget();
    }

    {
        let dragging = dragging.clone();
        let split = split.clone();
        let left = left.clone();
        let av2 = av.clone();

        let on_move = Closure::<dyn FnMut(MouseEvent)>::new(move |e: MouseEvent| {
            if !dragging.get() {
                return;
            }

            let rect = split.get_bounding_client_rect();
            let x = (e.client_x() as f64) - rect.left();
            let w = rect.width();

            let min_left = 200.0;
            let min_right = 260.0;
            let max_left = (w - min_right).max(min_left);

            let new_left = x.clamp(min_left, max_left);

            let _ = left
                .style()
                .set_property("width", &format!("{new_left:.0}px"));

            let mut avb = av2.borrow_mut();
            if let Some(left) = avb.document.get_element_by_id("gcode-left") {
                let rect = left.get_bounding_client_rect();
                let gw = rect.width().max(1.0) as u32;
                let gh = rect.height().max(1.0) as u32;
                avb.canvases[CanvasKind::Gcode.idx()].resize(gw, gh);
            }
        });

        window.add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref())?;
        on_move.forget();
    }

    {
        let dragging = dragging.clone();
        let on_up = Closure::<dyn FnMut(MouseEvent)>::new(move |_e: MouseEvent| {
            dragging.set(false);
        });
        window.add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref())?;
        on_up.forget();
    }

    Ok(())
}

pub(crate) fn init_toolpath_splitter(av: RefAV) -> Result<(), JsValue> {
    let window: Window = web_sys::window().unwrap();
    let document = window.document().unwrap();

    let split: HtmlElement = document
        .get_element_by_id("toolpath-split")
        .unwrap()
        .dyn_into()?;
    let left: HtmlElement = document
        .get_element_by_id("toolpath-left")
        .unwrap()
        .dyn_into()?;
    let splitter: HtmlElement = document
        .get_element_by_id("toolpath-splitter")
        .unwrap()
        .dyn_into()?;

    let dragging = std::rc::Rc::new(std::cell::Cell::new(false));

    {
        let dragging = dragging.clone();
        let on_down = Closure::<dyn FnMut(MouseEvent)>::new(move |e: MouseEvent| {
            e.prevent_default();
            dragging.set(true);
        });
        splitter.add_event_listener_with_callback("mousedown", on_down.as_ref().unchecked_ref())?;
        on_down.forget();
    }

    {
        let dragging = dragging.clone();
        let split = split.clone();
        let left = left.clone();
        let av2 = av.clone();

        let on_move = Closure::<dyn FnMut(MouseEvent)>::new(move |e: MouseEvent| {
            if !dragging.get() {
                return;
            }

            let rect = split.get_bounding_client_rect();
            let x = (e.client_x() as f64) - rect.left();
            let w = rect.width();

            let min_left = 200.0;
            let min_right = 260.0;
            let max_left = (w - min_right).max(min_left);

            let new_left = x.clamp(min_left, max_left);

            let _ = left
                .style()
                .set_property("width", &format!("{new_left:.0}px"));

            let mut avb = av2.borrow_mut();
            if let Some(left) = avb.document.get_element_by_id("toolpath-left") {
                let rect = left.get_bounding_client_rect();
                let gw = rect.width().max(1.0) as u32;
                let gh = rect.height().max(1.0) as u32;
                avb.canvases[CanvasKind::Toolpath.idx()].resize(gw, gh);
            }
        });

        window.add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref())?;
        on_move.forget();
    }

    {
        let dragging = dragging.clone();
        let on_up = Closure::<dyn FnMut(MouseEvent)>::new(move |_e: MouseEvent| {
            dragging.set(false);
        });
        window.add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref())?;
        on_up.forget();
    }

    Ok(())
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
            Box::new(move |av, event| on_icon_mouseout(av, event)),
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
            Box::new(move |av, event| on_icon_mouseout(av, event)),
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
            Box::new(move |av, event| on_icon_mouseout(av, event)),
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

pub(crate) fn init_gcode_canvas(av: RefAV) -> Result<(), JsValue> {
    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_gcode_mouse_move(pa_cloneprim.clone(), event);
    });
    let c_gcode = pa_cloneme.borrow().canvases[CanvasKind::Gcode.idx()]
        .get_canvas()
        .clone();
    c_gcode.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())?;
    closure.forget();

    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_gcode_mouse_down(pa_cloneprim.clone(), event);
    });
    let c_gcode = pa_cloneme.borrow().canvases[CanvasKind::Gcode.idx()]
        .get_canvas()
        .clone();
    c_gcode.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())?;
    closure.forget();

    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_gcode_mouse_up(pa_cloneprim.clone(), event);
    });
    let c_gcode = pa_cloneme.borrow().canvases[CanvasKind::Gcode.idx()]
        .get_canvas()
        .clone();
    c_gcode.add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref())?;
    closure.forget();

    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_gcode_mouse_wheel(pa_cloneprim.clone(), event);
    });
    let c_gcode = pa_cloneme.borrow().canvases[CanvasKind::Gcode.idx()]
        .get_canvas()
        .clone();
    c_gcode.add_event_listener_with_callback("wheel", closure.as_ref().unchecked_ref())?;
    closure.forget();

    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_gcode_context_menu(pa_cloneprim.clone(), event);
    });
    let c_gcode = pa_cloneme.borrow().canvases[CanvasKind::Gcode.idx()]
        .get_canvas()
        .clone();
    c_gcode.add_event_listener_with_callback("contextmenu", closure.as_ref().unchecked_ref())?;
    closure.forget();

    Ok(())
}

pub(crate) fn init_toolpath_canvas(av: RefAV) -> Result<(), JsValue> {
    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_toolpath_mouse_move(pa_cloneprim.clone(), event);
    });
    let c_toolpath = pa_cloneme.borrow().canvases[CanvasKind::Toolpath.idx()]
        .get_canvas()
        .clone();
    c_toolpath.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())?;
    closure.forget();

    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_toolpath_mouse_down(pa_cloneprim.clone(), event);
    });
    let c_toolpath = pa_cloneme.borrow().canvases[CanvasKind::Toolpath.idx()]
        .get_canvas()
        .clone();
    c_toolpath.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())?;
    closure.forget();

    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_toolpath_mouse_up(pa_cloneprim.clone(), event);
    });
    let c_toolpath = pa_cloneme.borrow().canvases[CanvasKind::Toolpath.idx()]
        .get_canvas()
        .clone();
    c_toolpath.add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref())?;
    closure.forget();

    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_toolpath_mouse_wheel(pa_cloneprim.clone(), event);
    });
    let c_toolpath = pa_cloneme.borrow().canvases[CanvasKind::Toolpath.idx()]
        .get_canvas()
        .clone();
    c_toolpath.add_event_listener_with_callback("wheel", closure.as_ref().unchecked_ref())?;
    closure.forget();

    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_toolpath_context_menu(pa_cloneprim.clone(), event);
    });
    let c_toolpath = pa_cloneme.borrow().canvases[CanvasKind::Toolpath.idx()]
        .get_canvas()
        .clone();
    c_toolpath.add_event_listener_with_callback("contextmenu", closure.as_ref().unchecked_ref())?;
    closure.forget();

    Ok(())
}

pub(crate) fn init_toolpath_panel(av: RefAV) -> Result<(), JsValue> {
    let document = av.borrow().document.clone();
    let bind_input = |id: &str, av: RefAV| -> Result<(), JsValue> {
        let el = document.get_element_by_id(id).unwrap();
        let input: HtmlInputElement = el.dyn_into::<HtmlInputElement>()?;
        let on_change = Closure::wrap(Box::new(move |_event: Event| {
            update_toolpath_params(av.clone());
        }) as Box<dyn FnMut(_)>);
        input.add_event_listener_with_callback("input", on_change.as_ref().unchecked_ref())?;
        on_change.forget();
        Ok(())
    };

    bind_input("tp-feed", av.clone())?;
    bind_input("tp-travel-feed", av.clone())?;
    bind_input("tp-pierce", av.clone())?;
    bind_input("tp-lead-in", av.clone())?;
    bind_input("tp-lead-out", av.clone())?;
    bind_input("tp-kerf", av.clone())?;
    bind_input("tp-torch-on", av.clone())?;
    bind_input("tp-torch-off", av.clone())?;

    if let Some(save) = document.get_element_by_id("tp-save") {
        let av_clone = av.clone();
        let save = save.dyn_into::<HtmlElement>()?;
        let on_click = Closure::wrap(Box::new(move |_event: Event| {
            save_toolpath_params(av_clone.clone());
        }) as Box<dyn FnMut(_)>);
        save.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())?;
        on_click.forget();
    }
    if let Some(load) = document.get_element_by_id("tp-load") {
        let av_clone = av.clone();
        let load = load.dyn_into::<HtmlElement>()?;
        let on_click = Closure::wrap(Box::new(move |_event: Event| {
            load_toolpath_params(av_clone.clone());
        }) as Box<dyn FnMut(_)>);
        load.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())?;
        on_click.forget();
    }

    let params = av.borrow().toolpath_params.clone();
    let set_value = |id: &str, value: &str| {
        if let Some(el) = document.get_element_by_id(id) {
            if let Ok(input) = el.dyn_into::<HtmlInputElement>() {
                input.set_value(value);
            }
        }
    };
    set_value("tp-feed", &format!("{:.0}", params.feed_xy));
    set_value("tp-travel-feed", &format!("{:.0}", params.travel_feed_xy));
    set_value("tp-pierce", &format!("{:.1}", params.pierce_delay_s));
    set_value("tp-lead-in", &format!("{:.0}", params.lead_in_mm));
    set_value("tp-lead-out", &format!("{:.0}", params.lead_out_mm));
    set_value("tp-kerf", &format!("{:.1}", params.kerf_mm));
    set_value("tp-torch-on", &params.torch_on_m3);
    set_value("tp-torch-off", &params.torch_off_m5);

    Ok(())
}

pub(crate) fn init_menu(av: RefAV) -> Result<(), JsValue> {
    let document = av.borrow().document.clone();
    if let Some(save) = document.get_element_by_id("save-option") {
        let av_clone = av.clone();
        let save = save.dyn_into::<HtmlElement>()?;
        let on_save = Closure::wrap(Box::new(move |event: Event| {
            event.prevent_default();
            let (document, json, filename) = {
                let avb = av_clone.borrow();
                let canvas = &avb.canvases[CanvasKind::Draw.idx()];
                let meta = make_export_info(&avb.document);
                let json = build_json_from_dataset(&canvas.dataset, &meta);
                let Some(json) = json else {
                    return;
                };
                let file_base = meta
                    .title
                    .as_ref()
                    .map(|title| title.as_str())
                    .filter(|title| !title.trim().is_empty())
                    .unwrap_or("miicut");
                let timestamp = timestamp_string();
                let filename = format!("{file_base}-{timestamp}.mii.json");
                (avb.document.clone(), json, filename)
            };
            trigger_download(&document, &filename, &json, "application/json");
        }) as Box<dyn FnMut(_)>);
        save.add_event_listener_with_callback("click", on_save.as_ref().unchecked_ref())?;
        on_save.forget();
    }

    if let Some(load) = document.get_element_by_id("load-option") {
        let av_clone = av.clone();
        let load = load.dyn_into::<HtmlElement>()?;
        let on_load = Closure::wrap(Box::new(move || {
            let document_clone = av_clone.borrow().document.clone();
            if let Ok(input) = document_clone.create_element("input") {
                if let Ok(input) = input.dyn_into::<HtmlInputElement>() {
                    input.set_type("file");
                    input.set_accept(".mii.json,application/json");
                    let av_inner = av_clone.clone();
                    let input_clone = input.clone();
                    let on_change = Closure::wrap(Box::new(move |_event: Event| {
                        let files = match input_clone.files() {
                            Some(files) => files,
                            None => return,
                        };
                        let file = match files.get(0) {
                            Some(file) => file,
                            None => return,
                        };
                        let reader = match FileReader::new() {
                            Ok(reader) => reader,
                            Err(_) => return,
                        };
                        let reader_clone = reader.clone();
                        let av_inner = av_inner.clone();
                        let on_load = Closure::wrap(Box::new(move |_event: Event| {
                            let result = reader_clone.result().ok().and_then(|val| val.as_string());
                            if let Some(result) = result {
                                load_json_to_dataset(av_inner.clone(), result);
                            }
                        })
                            as Box<dyn FnMut(_)>);
                        reader.set_onload(Some(on_load.as_ref().unchecked_ref()));
                        on_load.forget();
                        let _ = reader.read_as_text(&file);
                    }) as Box<dyn FnMut(_)>);
                    input
                        .add_event_listener_with_callback(
                            "change",
                            on_change.as_ref().unchecked_ref(),
                        )
                        .ok();
                    on_change.forget();
                    input.click();
                }
            }
        }) as Box<dyn FnMut()>);
        load.add_event_listener_with_callback("click", on_load.as_ref().unchecked_ref())?;
        on_load.forget();
    }

    if let Some(export) = document.get_element_by_id("export-svg") {
        let av_clone = av.clone();
        let export = export.dyn_into::<HtmlElement>()?;
        let on_export = Closure::wrap(Box::new(move |event: Event| {
            event.prevent_default();
            let (document, svg, filename) = {
                let avb = av_clone.borrow();
                let canvas = &avb.canvases[CanvasKind::Draw.idx()];
                let Some(svg) = build_svg_from_dataset(&canvas.dataset) else {
                    return;
                };
                let filename = "drawing.svg".to_string();
                (avb.document.clone(), svg, filename)
            };
            trigger_download(&document, &filename, &svg, "image/svg+xml");
        }) as Box<dyn FnMut(_)>);
        export.add_event_listener_with_callback("click", on_export.as_ref().unchecked_ref())?;
        on_export.forget();
    }

    if let Some(svg_input) = document.get_element_by_id("svg-input") {
        let svg_input: HtmlInputElement = svg_input.dyn_into().unwrap();
        let av_clone = av.clone();
        let document_clone = document.clone();
        let svg_input_clone = svg_input.clone();
        let on_svg_select = Closure::wrap(Box::new(move || {
            let files = match svg_input_clone.files() {
                Some(files) => files,
                None => return,
            };
            let file = match files.get(0) {
                Some(file) => file,
                None => return,
            };
            let combine_paths = document_clone
                .get_element_by_id("import-svg-single")
                .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
                .map(|el| el.checked())
                .unwrap_or(false);

            let file_reader = FileReader::new().unwrap();
            let av_for_load = av_clone.clone();
            let on_load = Closure::wrap(Box::new(move |event: web_sys::Event| {
                let target = event.target().unwrap();
                let file_reader: FileReader = target.dyn_into().unwrap();
                if let Some(result) = file_reader.result().unwrap().as_string() {
                    log!("SVG file content loaded!");
                    load_svg_to_dataset(av_for_load.clone(), result, combine_paths);
                }
            }) as Box<dyn FnMut(_)>);

            file_reader
                .add_event_listener_with_callback("load", on_load.as_ref().unchecked_ref())
                .unwrap();
            on_load.forget();
            file_reader.read_as_text(&file).unwrap();
            svg_input_clone.set_value("");
        }) as Box<dyn FnMut()>);
        svg_input
            .add_event_listener_with_callback("change", on_svg_select.as_ref().unchecked_ref())?;
        on_svg_select.forget();
    }

    if let Some(import) = document.get_element_by_id("import-svg") {
        let import = import.dyn_into::<HtmlElement>()?;
        let svg_input = document
            .get_element_by_id("svg-input")
            .and_then(|el| el.dyn_into::<HtmlInputElement>().ok());
        let on_click = Closure::wrap(Box::new(move |_event: Event| {
            if let Some(input) = svg_input.as_ref() {
                input.click();
            }
        }) as Box<dyn FnMut(_)>);
        import.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())?;
        on_click.forget();
    }

    if let Some(load) = document.get_element_by_id("machine-load-params") {
        let av_clone = av.clone();
        let load = load.dyn_into::<HtmlElement>()?;
        let on_click = Closure::wrap(Box::new(move |_event: Event| {
            load_toolpath_params(av_clone.clone());
        }) as Box<dyn FnMut(_)>);
        load.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())?;
        on_click.forget();
    }
    if let Some(save) = document.get_element_by_id("machine-save-params") {
        let av_clone = av.clone();
        let save = save.dyn_into::<HtmlElement>()?;
        let on_click = Closure::wrap(Box::new(move |_event: Event| {
            save_toolpath_params(av_clone.clone());
        }) as Box<dyn FnMut(_)>);
        save.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())?;
        on_click.forget();
    }

    Ok(())
}

pub(crate) fn init_status(av: RefAV) -> Result<(), JsValue> {
    let document = av.borrow().document.clone();
    let active_canvas = av.borrow().active_canvas;
    let setup_checkbox =
        |id: &str, av: RefAV, is_linear: bool, value: f64| -> Result<(), JsValue> {
            let el = document.get_element_by_id(id).unwrap();
            let input: HtmlInputElement = el.dyn_into::<HtmlInputElement>()?;
            let on_change = Closure::wrap(Box::new(move |_event: Event| {
                {
                    let mut avb = av.borrow_mut();
                    let snap = &mut avb.canvases[active_canvas.idx()].get_user_ui_mut().snap;
                    if is_linear {
                        snap.set_linear_value(value);
                    } else {
                        snap.set_angle_value(value);
                    }
                }
                update_status_bar(av.clone());
                render_draw_view(av.clone());
            }) as Box<dyn FnMut(_)>);
            input.add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref())?;
            on_change.forget();
            Ok(())
        };

    setup_checkbox("snap-linear-1", av.clone(), true, 1.0)?;
    setup_checkbox("snap-linear-5", av.clone(), true, 5.0)?;
    setup_checkbox("snap-linear-10", av.clone(), true, 10.0)?;
    setup_checkbox("snap-angle-1", av.clone(), false, 1.0)?;
    setup_checkbox("snap-angle-5", av.clone(), false, 5.0)?;
    setup_checkbox("snap-angle-10", av.clone(), false, 10.0)?;

    update_status_bar(av);
    Ok(())
}

pub(crate) fn save_toolpath_params(av: RefAV) {
    let document = av.borrow().document.clone();
    let (params, machine_values) = {
        let avb = av.borrow();
        let params = avb.toolpath_params.clone();
        let mut machine_values = Vec::new();
        for group in avb.machine_groups.iter() {
            for setting in group.settings.iter() {
                machine_values.push((setting.id.clone(), setting.value.clone()));
            }
        }
        (params, machine_values)
    };
    let payload = Object::new();
    let toolpath = Object::new();
    let _ = Reflect::set(
        &toolpath,
        &"feed_xy".into(),
        &JsValue::from_f64(params.feed_xy),
    );
    let _ = Reflect::set(
        &toolpath,
        &"travel_feed_xy".into(),
        &JsValue::from_f64(params.travel_feed_xy),
    );
    let _ = Reflect::set(
        &toolpath,
        &"pierce_delay_s".into(),
        &JsValue::from_f64(params.pierce_delay_s),
    );
    let _ = Reflect::set(
        &toolpath,
        &"lead_in_mm".into(),
        &JsValue::from_f64(params.lead_in_mm),
    );
    let _ = Reflect::set(
        &toolpath,
        &"lead_out_mm".into(),
        &JsValue::from_f64(params.lead_out_mm),
    );
    let _ = Reflect::set(
        &toolpath,
        &"kerf_mm".into(),
        &JsValue::from_f64(params.kerf_mm),
    );
    let _ = Reflect::set(
        &toolpath,
        &"torch_on_m3".into(),
        &JsValue::from_str(&params.torch_on_m3),
    );
    let _ = Reflect::set(
        &toolpath,
        &"torch_off_m5".into(),
        &JsValue::from_str(&params.torch_off_m5),
    );
    let _ = Reflect::set(&payload, &"toolpath".into(), &toolpath);

    let machine = Object::new();
    for (id, value) in machine_values {
        let _ = Reflect::set(
            &machine,
            &JsValue::from_str(&id),
            &JsValue::from_str(&value),
        );
    }
    let _ = Reflect::set(&payload, &"machine_settings".into(), &machine);
    let payload = JSON::stringify_with_replacer_and_space(&payload, &JsValue::NULL, &2.into())
        .ok()
        .and_then(|val| val.as_string())
        .unwrap_or_default();

    if let Some(body) = document.body() {
        if let Ok(el) = document.create_element("a") {
            if let Ok(link) = el.dyn_into::<HtmlAnchorElement>() {
                let parts = Array::new();
                parts.push(&JsValue::from_str(&payload));
                if let Ok(blob) = Blob::new_with_str_sequence(&parts) {
                    if let Ok(url) = Url::create_object_url_with_blob(&blob) {
                        link.set_href(&url);
                        link.set_download("toolpath-params.machparams.json");
                        let _ = body.append_child(&link);
                        link.click();
                        let _ = body.remove_child(&link);
                        let _ = Url::revoke_object_url(&url);
                    }
                }
            }
        }
    }
}

pub(crate) fn load_toolpath_params(av: RefAV) {
    let document = av.borrow().document.clone();
    let input = match document.create_element("input") {
        Ok(input) => input,
        Err(_) => return,
    };
    let input: HtmlInputElement = match input.dyn_into() {
        Ok(input) => input,
        Err(_) => return,
    };
    input.set_type("file");
    input.set_accept(".machparams.json,application/json");

    let av_clone = av.clone();
    let input_clone = input.clone();
    let on_change = Closure::wrap(Box::new(move |_event: Event| {
        let files = match input_clone.files() {
            Some(files) => files,
            None => return,
        };
        let file = match files.get(0) {
            Some(file) => file,
            None => return,
        };
        let reader = match FileReader::new() {
            Ok(reader) => reader,
            Err(_) => return,
        };
        let reader_clone = reader.clone();
        let av_inner = av_clone.clone();
        let on_load = Closure::wrap(Box::new(move |_event: Event| {
            let result = reader_clone.result().ok().and_then(|val| val.as_string());
            let Some(json) = result else { return };
            apply_toolpath_params_from_json(av_inner.clone(), json);
        }) as Box<dyn FnMut(_)>);
        reader.set_onload(Some(on_load.as_ref().unchecked_ref()));
        on_load.forget();
        let _ = reader.read_as_text(&file);
    }) as Box<dyn FnMut(_)>);
    input
        .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref())
        .ok();
    on_change.forget();
    input.click();
}

pub(crate) fn apply_toolpath_params_from_json(av: RefAV, json_data: String) {
    let Ok(value) = JSON::parse(&json_data) else {
        return;
    };
    let document = av.borrow().document.clone();
    let mut avb = av.borrow_mut();
    let Some(toolpath_value) = get_prop(&value, "toolpath") else {
        return;
    };
    let params = &mut avb.toolpath_params;

    if let Some(val) = get_prop(&toolpath_value, "feed_xy").and_then(|val| val.as_f64()) {
        params.feed_xy = val;
    }
    if let Some(val) = get_prop(&toolpath_value, "travel_feed_xy").and_then(|val| val.as_f64()) {
        params.travel_feed_xy = val;
    }
    if let Some(val) = get_prop(&toolpath_value, "pierce_delay_s").and_then(|val| val.as_f64()) {
        params.pierce_delay_s = val;
    }
    if let Some(val) = get_prop(&toolpath_value, "lead_in_mm").and_then(|val| val.as_f64()) {
        params.lead_in_mm = val;
    }
    if let Some(val) = get_prop(&toolpath_value, "lead_out_mm").and_then(|val| val.as_f64()) {
        params.lead_out_mm = val;
    }
    if let Some(val) = get_prop(&toolpath_value, "kerf_mm").and_then(|val| val.as_f64()) {
        params.kerf_mm = val;
    }
    if let Some(val) = get_prop(&toolpath_value, "torch_on_m3").and_then(|val| val.as_string()) {
        params.torch_on_m3 = val;
    }
    if let Some(val) = get_prop(&toolpath_value, "torch_off_m5").and_then(|val| val.as_string()) {
        params.torch_off_m5 = val;
    }

    if let Some(machine_val) = get_prop(&value, "machine_settings") {
        let machine_obj = Object::from(machine_val);
        let keys = Object::keys(&machine_obj);
        for key in keys.iter() {
            let Some(id) = key.as_string() else {
                continue;
            };
            let value_str = match Reflect::get(&machine_obj, &key) {
                Ok(val) => val
                    .as_string()
                    .or_else(|| val.as_f64().map(|num| num.to_string()))
                    .unwrap_or_default(),
                Err(_) => String::new(),
            };
            if value_str.is_empty() {
                continue;
            }
            if update_machine_value(&mut avb.machine_groups, &id, &value_str) {
                avb.update_machine_input(&id, &value_str);
            }
        }
    }

    drop(avb);
    let params = av.borrow().toolpath_params.clone();
    let set_value = |id: &str, value: &str| {
        if let Some(el) = document.get_element_by_id(id) {
            if let Ok(input) = el.dyn_into::<HtmlInputElement>() {
                input.set_value(value);
            }
        }
    };
    set_value("tp-feed", &format!("{:.0}", params.feed_xy));
    set_value("tp-travel-feed", &format!("{:.0}", params.travel_feed_xy));
    set_value("tp-pierce", &format!("{:.1}", params.pierce_delay_s));
    set_value("tp-lead-in", &format!("{:.0}", params.lead_in_mm));
    set_value("tp-lead-out", &format!("{:.0}", params.lead_out_mm));
    set_value("tp-kerf", &format!("{:.1}", params.kerf_mm));
    set_value("tp-torch-on", &params.torch_on_m3);
    set_value("tp-torch-off", &params.torch_off_m5);

    update_toolpath_params(av);
}

pub(crate) fn read_input_f64(document: &Document, id: &str, fallback: f64) -> f64 {
    document
        .get_element_by_id(id)
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .and_then(|input| input.value().trim().parse::<f64>().ok())
        .unwrap_or(fallback)
}

pub(crate) fn read_input_string(document: &Document, id: &str, fallback: &str) -> String {
    document
        .get_element_by_id(id)
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|input| input.value())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

pub(crate) fn update_toolpath_params(av: RefAV) {
    let document = av.borrow().document.clone();
    let active = av.borrow().active_canvas;
    let mut avb = av.borrow_mut();
    let params = &mut avb.toolpath_params;
    let torch_on = params.torch_on_m3.clone();
    let torch_off = params.torch_off_m5.clone();

    params.feed_xy = read_input_f64(&document, "tp-feed", params.feed_xy).max(0.0);
    params.travel_feed_xy =
        read_input_f64(&document, "tp-travel-feed", params.travel_feed_xy).max(0.0);
    params.pierce_delay_s = read_input_f64(&document, "tp-pierce", params.pierce_delay_s).max(0.0);
    params.lead_in_mm = read_input_f64(&document, "tp-lead-in", params.lead_in_mm).max(0.0);
    params.lead_out_mm = read_input_f64(&document, "tp-lead-out", params.lead_out_mm).max(0.0);
    params.kerf_mm = read_input_f64(&document, "tp-kerf", params.kerf_mm).max(0.0);
    params.torch_on_m3 = read_input_string(&document, "tp-torch-on", &torch_on);
    params.torch_off_m5 = read_input_string(&document, "tp-torch-off", &torch_off);

    avb.refresh_toolpath_cache();
    let toolpath = avb.toolpath.clone().unwrap_or(Toolpath::new(Vec::new()));
    let gcode = toolpath_to_plasma_gcode(&toolpath, &avb.toolpath_params);
    avb.last_gcode = Some(gcode);
    avb.refresh_gcode_cache();
    drop(avb);

    if active == CanvasKind::Toolpath {
        render_toolpath_view(av);
    } else if active == CanvasKind::Gcode {
        render_gcode_view(av);
    }
}

pub(crate) fn set_callback(
    av: RefAV,
    event: String,
    el: &Element,
    cb: Box<dyn FnMut(RefAV, Event)>,
) -> Result<(), JsValue> {
    let mut cb = cb;
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        cb(av.clone(), event);
    });
    el.add_event_listener_with_callback(&event, closure.as_ref().unchecked_ref())?;
    closure.forget();
    Ok(())
}
