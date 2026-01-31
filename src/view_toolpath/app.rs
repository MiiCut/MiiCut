use crate::{
    app::{load_toolpath_params, save_toolpath_params, with_av_mut, AppVars, RefAV},
    canvas::{CanvasKind, Color, Pattern},
    dom::{read_input_f64, read_input_string},
    helpers::canvas_fit::{center_paths_canvas, fit_paths_canvas, zoom_canvas_at},
    helpers::prefab::*,
    inputs::SystemMouse,
    shapes::{multipolygon_to_toolpath, toolpath_to_plasma_gcode, Toolpath},
    status::{begin_render, end_render, update_status_bar},
    view_draw::app::update,
    view_gcode::app::render_gcode_view,
};
use kurbo::Vec2;
use wasm_bindgen::{prelude::*, JsCast};
use web_sys::{Event, HtmlElement, HtmlInputElement, MouseEvent, WheelEvent};

impl AppVars {
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
                .unwrap_or_default();
            self.last_gcode = Some(gcode);
            self.refresh_gcode_cache();
        }
    }
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

pub(crate) fn on_toolpath_context_menu(_av: RefAV, event: Event) {
    event.prevent_default();
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

pub(crate) fn on_toolpath_mouse_move(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.dyn_into::<MouseEvent>() {
        let sys_mouse = SystemMouse::Move;
        let action = av.borrow_mut().update_canvas_inputs(mouse_event, sys_mouse);
        let _ = update(av.clone(), action);
    }
}

pub(crate) fn on_toolpath_mouse_down(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.dyn_into::<MouseEvent>() {
        let sys_mouse = SystemMouse::Down(mouse_event.detail());
        let action = av.borrow_mut().update_canvas_inputs(mouse_event, sys_mouse);
        let _ = update(av.clone(), action);
    }
}

pub(crate) fn on_toolpath_mouse_up(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.dyn_into::<MouseEvent>() {
        let sys_mouse = SystemMouse::Up(mouse_event.detail());
        let action = av.borrow_mut().update_canvas_inputs(mouse_event, sys_mouse);
        let _ = update(av.clone(), action);
    }
}

pub(crate) fn on_toolpath_mouse_wheel(av: RefAV, event: Event) {
    if let Ok(wheel_event) = event.dyn_into::<WheelEvent>() {
        wheel_event.prevent_default();
        wheel_event.stop_propagation();
        let delta_y = wheel_event.delta_y();
        if let Ok(mouse_event) = wheel_event.clone().dyn_into::<MouseEvent>() {
            with_av_mut(&av, |avb| {
                let sys_mouse = SystemMouse::Move;
                let _ = avb.update_canvas_inputs(mouse_event, sys_mouse);
                let canvas = &mut avb.canvases[CanvasKind::Toolpath.idx()];
                let world_pos = canvas.get_user_ui().draw_pos;
                let factor = if delta_y > 0.0 { 0.95 } else { 1.05 };
                zoom_canvas_at(canvas, world_pos, factor);
            });
            render_toolpath_view(av.clone());
        }
    }
}

pub(crate) fn render_toolpath_view(av: RefAV) {
    begin_render(av.clone(), "Toolpath");
    update_status_bar(av.clone());
    let mut avb = av.borrow_mut();

    let action = if avb.toolpath_auto_fit {
        avb.toolpath_auto_center = false;
        avb.toolpath_auto_fit = false;
        Some(true)
    } else if avb.toolpath_auto_center {
        avb.toolpath_auto_center = false;
        Some(false)
    } else {
        None
    };

    if let Some(do_fit) = action {
        let mut paths = avb.toolpath_paths.clone();
        paths.extend(avb.toolpath_lead_ins.iter().cloned());
        paths.extend(avb.toolpath_lead_outs.iter().cloned());
        paths.extend(avb.toolpath_travels.iter().cloned());
        paths.extend(avb.toolpath_travel_arrows.iter().cloned());
        paths.extend(avb.toolpath_arrows.iter().cloned());
        let canvas_toolpath = &mut avb.canvases[CanvasKind::Toolpath.idx()];
        if do_fit {
            fit_paths_canvas(canvas_toolpath, &paths);
        } else {
            center_paths_canvas(canvas_toolpath, &paths);
        }
    }

    let original_paths = avb.toolpath_original_paths.clone();
    let paths = avb.toolpath_paths.clone();
    let lead_ins = avb.toolpath_lead_ins.clone();
    let lead_outs = avb.toolpath_lead_outs.clone();
    let travels = avb.toolpath_travels.clone();
    let travel_arrows = avb.toolpath_travel_arrows.clone();
    let arrows = avb.toolpath_arrows.clone();
    let starts = avb.toolpath_starts.clone();
    let ends = avb.toolpath_ends.clone();
    let canvas_toolpath = &mut avb.canvases[CanvasKind::Toolpath.idx()];
    canvas_toolpath.clear();

    for path in original_paths.iter() {
        canvas_toolpath.draw_path(
            path,
            Pattern::Dim,
            Color::Transparent,
            Color::Gray20,
            vec![],
        );
    }
    for path in travels.iter() {
        canvas_toolpath.draw_path(
            path,
            Pattern::Dim,
            Color::Transparent,
            Color::Purple55,
            vec![],
        );
    }
    for path in travel_arrows.iter() {
        canvas_toolpath.draw_path(
            path,
            Pattern::Point,
            Color::Purple55,
            Color::Purple55,
            vec![],
        );
    }
    for path in paths.iter() {
        canvas_toolpath.draw_path(path, Pattern::Dim, Color::Transparent, Color::Black, vec![]);
    }
    for path in lead_ins.iter() {
        canvas_toolpath.draw_path(
            path,
            Pattern::Dim,
            Color::Transparent,
            Color::Green40,
            vec![],
        );
    }
    for path in lead_outs.iter() {
        canvas_toolpath.draw_path(path, Pattern::Dim, Color::Transparent, Color::Red60, vec![]);
    }
    for path in arrows.iter() {
        canvas_toolpath.draw_path(path, Pattern::Point, Color::Green40, Color::Green40, vec![]);
    }
    let scale = canvas_toolpath.get_scale();
    for pos in starts.iter() {
        canvas_toolpath.draw_path(
            &point_path(*pos, scale),
            Pattern::Point,
            Color::Transparent,
            Color::Green40,
            vec![],
        );
    }
    for pos in ends.iter() {
        canvas_toolpath.draw_path(
            &point_path(*pos, scale),
            Pattern::Point,
            Color::Transparent,
            Color::Red60,
            vec![],
        );
    }
    drop(avb);
    end_render(av.clone());
    update_status_bar(av);
}
