use crate::app::RefAV;
use crate::canvas::{Canvas, CanvasKind, Color};
use crate::dom::{get_element_height, get_element_width, Tabs};
use crate::inputs::{ButtonLevel, MouseButton, SystemMouse, UserAction};
use crate::math::MyError;
use crate::render::{
    render_draw_view, render_gcode_view, render_machine_view, render_toolpath_view,
};
use crate::shape::{GeneralShape, ShapeType};
use crate::shapes::{toolpath_to_plasma_gcode, Toolpath};

use wasm_bindgen::JsCast;
use web_sys::{
    Event, HtmlElement, HtmlInputElement, HtmlTextAreaElement, KeyboardEvent, MouseEvent,
    WheelEvent,
};

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
                    if avb.set_move_vertices_selected().is_some() || avb.set_move_elements() {
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

                    let canvas = avb.get_active_canvas_mut();
                    canvas.dataset.save_elements_positions();
                    if avb.set_element_select_vertex() {
                        do_render = true;
                        drop(avb);
                        if do_render {
                            render_active_view(av.clone());
                        }
                        return Ok(());
                    }
                    avb.set_select_elements();
                    do_render = true;
                } else {
                    avb.get_active_canvas_mut().save_offset();
                }
            }
            UserAction::ClickUp(button, _) => {
                if button == MouseButton::Left && is_draw_view {
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
                        let toolpath = avb.toolpath.clone().unwrap_or(Toolpath::new(Vec::new()));
                        let gcode = toolpath_to_plasma_gcode(&toolpath, &avb.toolpath_params);
                        avb.last_gcode = Some(gcode);
                        avb.refresh_gcode_cache();
                    }
                    do_render = true;
                }
            }
        },
        Disc
        | Square
        | Oblong
        | Voronoi { .. }
        | ConstrLine
        | ConstrCircle { .. }
        | Poly
        | Text { .. } => match user_action {
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
                            if let Some(e) = GeneralShape::new_shape_rectangle(vs[0], vs[1], 0) {
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
                        ShapeType::Voronoi { .. } => {
                            if let Some(e) = GeneralShape::new_shape_voronoi(vs[0], vs[1], 0) {
                                let eid = canvas.dataset.push_element(e);
                                canvas.dataset.select_only(eid);
                                finished = true;
                            }
                        }
                        ShapeType::ConstrLine { .. } => {
                            if let Some(e) = GeneralShape::new_shape_constr_line(vs[0], vs[1], 0) {
                                let eid = canvas.dataset.push_element(e);
                                canvas.dataset.select_only(eid);
                                finished = true;
                            }
                        }
                        ShapeType::ConstrCircle { .. } => {
                            if let Some(e) = GeneralShape::new_shape_constr_circle(vs[0], vs[1], 0)
                            {
                                let eid = canvas.dataset.push_element(e);
                                canvas.dataset.select_only(eid);
                                finished = true;
                            }
                        }
                        ShapeType::Text { .. } => {
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
        },
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
        let action = av.borrow_mut().update_canvas_inputs(mouse_event, sys_mouse);
        let _ = update(av.clone(), action);
    }
    if av.borrow().active_canvas == CanvasKind::Draw {
        render_draw_view(av);
    }
}

pub(crate) fn on_draw_mouse_down(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.dyn_into::<MouseEvent>() {
        let sys_mouse = SystemMouse::Down(mouse_event.detail());
        let action = av.borrow_mut().update_canvas_inputs(mouse_event, sys_mouse);
        let _ = update(av.clone(), action);
    }
    if av.borrow().active_canvas == CanvasKind::Draw {
        render_draw_view(av);
    }
}

pub(crate) fn on_draw_mouse_up(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.dyn_into::<MouseEvent>() {
        let sys_mouse = SystemMouse::Up(mouse_event.detail());
        let action = av.borrow_mut().update_canvas_inputs(mouse_event, sys_mouse);
        let _ = update(av.clone(), action);
    }
    if av.borrow().active_canvas == CanvasKind::Draw {
        render_draw_view(av);
    }
}

pub(crate) fn on_draw_mouse_wheel(av: RefAV, event: Event) {
    if let Ok(wheel_event) = event.dyn_into::<WheelEvent>() {
        wheel_event.prevent_default();
        wheel_event.stop_propagation();
        let delta_y = wheel_event.delta_y();
        if let Ok(mouse_event) = wheel_event.clone().dyn_into::<MouseEvent>() {
            let mut avb = av.borrow_mut();
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
    let mut avb = av.borrow_mut();
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
    let mut avb = av.borrow_mut();
    avb.canvases[CanvasKind::Draw.idx()].set_pointer_on_canvas(false);
    avb.element_on_creation = None;
    avb.go_to_arrow_tool();
    // avb.update_draw_cursor();
    drop(avb);
    render_draw_view(av);
}

pub(crate) fn on_draw_context_menu(_av: RefAV, event: Event) {
    event.prevent_default();
}

pub(crate) fn on_gcode_context_menu(_av: RefAV, event: Event) {
    event.prevent_default();
}

pub(crate) fn on_gcode_mouse_move(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.dyn_into::<MouseEvent>() {
        let sys_mouse = SystemMouse::Move;
        let action = av.borrow_mut().update_canvas_inputs(mouse_event, sys_mouse);
        let _ = update(av.clone(), action);
    }
}

pub(crate) fn on_gcode_mouse_down(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.dyn_into::<MouseEvent>() {
        let sys_mouse = SystemMouse::Down(mouse_event.detail());
        let action = av.borrow_mut().update_canvas_inputs(mouse_event, sys_mouse);
        let _ = update(av.clone(), action);
    }
}

pub(crate) fn on_gcode_mouse_up(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.dyn_into::<MouseEvent>() {
        let sys_mouse = SystemMouse::Up(mouse_event.detail());
        let action = av.borrow_mut().update_canvas_inputs(mouse_event, sys_mouse);
        let _ = update(av.clone(), action);
    }
}

pub(crate) fn on_gcode_mouse_wheel(av: RefAV, event: Event) {
    if let Ok(wheel_event) = event.dyn_into::<WheelEvent>() {
        wheel_event.prevent_default();
        wheel_event.stop_propagation();
        let delta_y = wheel_event.delta_y();
        if let Ok(mouse_event) = wheel_event.clone().dyn_into::<MouseEvent>() {
            let mut avb = av.borrow_mut();
            let sys_mouse = SystemMouse::Move;
            let _ = avb.update_canvas_inputs(mouse_event, sys_mouse);
            let canvas = &mut avb.canvases[CanvasKind::Gcode.idx()];
            let world_pos = canvas.get_user_ui().draw_pos;
            let factor = if delta_y > 0.0 { 0.95 } else { 1.05 };
            zoom_canvas_at(canvas, world_pos, factor);
            drop(avb);
            render_gcode_view(av.clone());
        }
    }
}

pub(crate) fn on_toolpath_context_menu(_av: RefAV, event: Event) {
    event.prevent_default();
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
            let mut avb = av.borrow_mut();
            let sys_mouse = SystemMouse::Move;
            let _ = avb.update_canvas_inputs(mouse_event, sys_mouse);
            let canvas = &mut avb.canvases[CanvasKind::Toolpath.idx()];
            let world_pos = canvas.get_user_ui().draw_pos;
            let factor = if delta_y > 0.0 { 0.95 } else { 1.05 };
            zoom_canvas_at(canvas, world_pos, factor);
            drop(avb);
            render_toolpath_view(av.clone());
        }
    }
}

fn zoom_canvas_at(canvas: &mut Canvas, world_pos: kurbo::Vec2, factor: f64) {
    let scale = canvas.get_scale();
    let view_size = canvas.get_canvas_size();
    let max_world_w = 3500.0;
    let max_world_h = 3500.0;
    let min_scale_x = view_size.width / max_world_w;
    let min_scale_y = view_size.height / max_world_h;
    let min_scale = min_scale_x.max(min_scale_y).max(0.01);
    let new_scale = (scale * factor).clamp(min_scale, 10.0);
    if (new_scale - scale).abs() < f64::EPSILON {
        return;
    }
    let offset = canvas.get_offset();
    let new_offset = offset + world_pos * (scale - new_scale);
    canvas.set_scale(new_scale);
    canvas.set_offset(new_offset);
}

pub(crate) fn _cnc_send(av: RefAV, cmd: String) {
    let avb = av.borrow();
    if let Some(cnc) = avb.cnc.as_ref() {
        let cnc = cnc.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let _ = cnc.send_http_cmd(&cmd).await;
        });
    }
}

pub(crate) fn resize_canvases(av: RefAV) {
    let mut avb = av.borrow_mut();

    let mut draw_width = get_element_width(&avb.left_panel) as u32;
    if matches!(avb.active_view, Tabs::Draw) {
        draw_width = draw_width.saturating_add(get_element_width(&avb.shapes_panel) as u32);
    }
    let draw_height = get_element_height(&avb.top_menu) as u32;
    let width = avb.window.inner_width().unwrap().as_f64().unwrap() as u32;
    let height = avb.window.inner_height().unwrap().as_f64().unwrap() as u32;

    {
        let canvas_draw = &mut avb.canvases[CanvasKind::Draw.idx()];
        canvas_draw.resize(width - draw_width, height - draw_height);
    }
    {
        let canvas_grid = &mut avb.canvases[CanvasKind::Grid.idx()];
        canvas_grid.resize(width - draw_width, height - draw_height);
    }
    {
        let canvas_back = &mut avb.canvases[CanvasKind::Background.idx()];
        canvas_back.resize(width - draw_width, height - draw_height);
    }

    if let Some(left) = avb.document.get_element_by_id("gcode-left") {
        let rect = left.get_bounding_client_rect();
        let gw = rect.width().max(1.0) as u32;
        let gh = rect.height().max(1.0) as u32;
        avb.canvases[CanvasKind::Gcode.idx()].resize(gw, gh);
    }

    if let Some(left) = avb.document.get_element_by_id("toolpath-left") {
        let rect = left.get_bounding_client_rect();
        let gw = rect.width().max(1.0) as u32;
        let gh = rect.height().max(1.0) as u32;
        avb.canvases[CanvasKind::Toolpath.idx()].resize(gw, gh);
    }
}

pub(crate) fn on_window_resize(av: RefAV, _event: Event) {
    resize_canvases(av.clone());
    render_draw_view(av.clone());
    render_gcode_view(av.clone());
    render_toolpath_view(av);
}

pub(crate) fn on_window_click(_pa: RefAV, _event: Event) {}

fn is_typing_in_input(event: &KeyboardEvent, document: &web_sys::Document) -> bool {
    if let Some(target) = event.target() {
        if target.dyn_ref::<HtmlInputElement>().is_some() {
            return true;
        }
        if target.dyn_ref::<HtmlTextAreaElement>().is_some() {
            return true;
        }
        if let Some(target) = target.dyn_ref::<HtmlElement>() {
            if target.is_content_editable() {
                return true;
            }
        }
    }
    if let Some(active) = document.active_element() {
        if active.dyn_ref::<HtmlInputElement>().is_some() {
            return true;
        }
        if active.dyn_ref::<HtmlTextAreaElement>().is_some() {
            return true;
        }
        if let Ok(active) = active.dyn_into::<HtmlElement>() {
            if active.is_content_editable() {
                return true;
            }
        }
    }
    false
}

pub(crate) fn on_window_keydown(av: RefAV, event: Event) {
    if let Ok(kb_event) = event.dyn_into::<KeyboardEvent>() {
        let key = kb_event.key();
        let document = av.borrow().document.clone();
        let ctrl_cmd = kb_event.ctrl_key() || kb_event.meta_key();
        let shift = kb_event.shift_key();
        let alt = kb_event.alt_key();
        {
            let mut avb = av.borrow_mut();
            let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
            let keys = &mut canvas.get_user_ui_mut().keys_states;
            keys.ctrl_cmd_pressed = ctrl_cmd;
            keys.shift_pressed = shift;
            keys.alt_pressed = alt;
        }

        let mut do_render = false;
        match key.as_str() {
            "Alt" => {
                do_render = true;
            }
            "Escape" => {
                if let Ok(mut avb) = av.try_borrow_mut() {
                    avb.esc_pressed();
                }
                do_render = true;
            }
            "Delete" | "Backspace" => {
                if is_typing_in_input(&kb_event, &document) {
                    return;
                }
                if let Ok(mut avb) = av.try_borrow_mut() {
                    avb.del_back_pressed();
                }
                do_render = true;
            }
            "Enter" => {
                if let Ok(mut avb) = av.try_borrow_mut() {
                    avb.group_toggle_pressed();
                }
                do_render = true;
            }
            " " => {
                if let Ok(mut avb) = av.try_borrow_mut() {
                    avb.space_pressed();
                }
                do_render = true;
            }
            "ArrowUp" => {
                if let Ok(mut avb) = av.try_borrow_mut() {
                    avb.arrow_up_pressed();
                }
                do_render = true;
            }
            "ArrowDown" => {
                if let Ok(mut avb) = av.try_borrow_mut() {
                    avb.arrow_down_pressed();
                }
                do_render = true;
            }
            "c" | "C" => {
                if ctrl_cmd {
                    if let Ok(mut avb) = av.try_borrow_mut() {
                        avb.ctrl_c_pressed();
                    }
                }
                do_render = true;
            }
            "v" | "V" => {
                if ctrl_cmd {
                    if let Ok(mut avb) = av.try_borrow_mut() {
                        avb.ctrl_v_pressed();
                    }
                }
                do_render = true;
            }
            "s" | "S" => {
                if ctrl_cmd {
                    kb_event.prevent_default();
                    log!("Ctrl+S pressed");
                    if let Some(save) = av.borrow().document.get_element_by_id("save-option") {
                        let save = save.dyn_into::<HtmlElement>().unwrap();
                        save.click();
                    }
                }
            }
            _ => {}
        }
        if do_render {
            render_draw_view(av);
        }
    }
}

pub(crate) fn on_window_keyup(av: RefAV, event: Event) {
    if let Ok(kb_event) = event.dyn_into::<KeyboardEvent>() {
        let key = kb_event.key();
        let mut avb = av.borrow_mut();
        let keys = &mut avb.canvases[CanvasKind::Draw.idx()]
            .get_user_ui_mut()
            .keys_states;
        keys.ctrl_cmd_pressed = kb_event.ctrl_key() || kb_event.meta_key();
        keys.shift_pressed = kb_event.shift_key();
        keys.alt_pressed = kb_event.alt_key();
        // avb.update_draw_cursor();
        if key == "Alt" {
            drop(avb);
            render_draw_view(av);
        }
    }
}

pub(crate) fn on_tab_click(av: RefAV, selected: Tabs) {
    let mut avb = av.borrow_mut();
    avb.active_view = selected;

    let tab_draw = avb.document.get_element_by_id("tab-draw");
    let tab_toolpath = avb.document.get_element_by_id("tab-toolpath");
    let tab_gcode = avb.document.get_element_by_id("tab-gcode");
    let tab_machine = avb.document.get_element_by_id("tab-machine");
    let draw = avb.document.get_element_by_id("view-draw");
    let toolpath = avb.document.get_element_by_id("view-toolpath");
    let gcode = avb.document.get_element_by_id("view-gcode");
    let machine = avb.document.get_element_by_id("view-machine");
    let left_panel = avb.document.get_element_by_id("left-panel");
    let shapes_panel = avb.document.get_element_by_id("shapes-panel");
    let file_menu = avb.document.get_element_by_id("file-menu");

    let set_active = |el: &Option<web_sys::Element>, active: bool| {
        if let Some(el) = el {
            if active {
                let _ = el.class_list().add_1("active");
            } else {
                let _ = el.class_list().remove_1("active");
            }
        }
    };

    set_active(&tab_draw, selected == Tabs::Draw);
    set_active(&tab_toolpath, selected == Tabs::Toolpath);
    set_active(&tab_gcode, selected == Tabs::Gcode);
    set_active(&tab_machine, selected == Tabs::Machine);
    set_active(&draw, selected == Tabs::Draw);
    set_active(&toolpath, selected == Tabs::Toolpath);
    set_active(&gcode, selected == Tabs::Gcode);
    set_active(&machine, selected == Tabs::Machine);

    if let Some(panel) = left_panel.as_ref() {
        let classes = panel.class_list();
        let _ = classes.remove_1("draw-mode");
        let _ = classes.remove_1("toolpath-mode");
        let _ = classes.remove_1("gcode-mode");
        match selected {
            Tabs::Draw => {
                let _ = classes.add_1("draw-mode");
            }
            Tabs::Toolpath => {
                let _ = classes.add_1("toolpath-mode");
            }
            Tabs::Gcode => {
                let _ = classes.add_1("gcode-mode");
            }
            Tabs::Machine => {}
        }
        if let Ok(panel) = panel.clone().dyn_into::<web_sys::HtmlElement>() {
            if matches!(selected, Tabs::Draw) {
                let _ = panel.style().set_property("display", "flex");
            } else {
                let _ = panel.style().set_property("display", "none");
            }
        }
    }

    if let Some(panel) = shapes_panel.as_ref() {
        if let Ok(panel) = panel.clone().dyn_into::<web_sys::HtmlElement>() {
            if matches!(selected, Tabs::Draw) {
                let _ = panel.style().set_property("display", "flex");
            } else {
                let _ = panel.style().set_property("display", "none");
            }
        }
    }

    if let Some(menu) = file_menu.as_ref() {
        if let Ok(menu) = menu.clone().dyn_into::<web_sys::HtmlElement>() {
            if matches!(selected, Tabs::Draw) {
                let _ = menu.style().set_property("display", "inline-block");
            } else {
                let _ = menu.style().set_property("display", "none");
            }
        }
    }

    if !matches!(selected, Tabs::Machine) {
        avb.active_canvas = match selected {
            Tabs::Draw => CanvasKind::Draw,
            Tabs::Toolpath => CanvasKind::Toolpath,
            Tabs::Gcode => CanvasKind::Gcode,
            Tabs::Machine => avb.active_canvas,
        };
    }

    if matches!(selected, Tabs::Toolpath) {
        avb.refresh_toolpath_cache();
        avb.last_gcode = None;
        avb.toolpath_auto_center = true;
    }
    if matches!(selected, Tabs::Gcode) {
        avb.refresh_toolpath_cache();
        avb.last_gcode = None;
        avb.gcode_auto_center = true;
    }
    if matches!(selected, Tabs::Machine) {
        let _ = avb.ensure_machine_view(av.clone());
        avb.request_machine_settings(av.clone());
    }

    drop(avb);
    resize_canvases(av.clone());

    match selected {
        Tabs::Draw => render_draw_view(av),
        Tabs::Toolpath => render_toolpath_view(av),
        Tabs::Gcode => render_gcode_view(av),
        Tabs::Machine => render_machine_view(av),
    }
}

pub(crate) fn on_icon_click(av: RefAV, icon: ShapeType) {
    let mut avb = av.borrow_mut();
    avb.icon_selected = icon;
    avb.user_icons
        .iter()
        .for_each(|icon| avb.html_deselect_icons(*icon));
    avb.html_select_icon(icon);
    drop(avb);
}

pub(crate) fn on_icon_mouseover(av: RefAV, event: Event, icon: ShapeType) {
    if let Ok(event) = event.dyn_into::<MouseEvent>() {
        let avb = av.borrow_mut();
        if let Some(html_element) = icon.get_html_element() {
            let selected_color = Color::OnCreation.get();
            html_element
                .set_attribute("style", &format!("color:{selected_color}"))
                .unwrap();
        }
        show_tooltip(&avb, event.x(), event.y(), icon_tooltip(icon));
    }
}

pub(crate) fn on_icon_mouseover_label(av: RefAV, event: Event, label: &'static str) {
    if let Ok(event) = event.dyn_into::<MouseEvent>() {
        let avb = av.borrow_mut();
        show_tooltip(&avb, event.x(), event.y(), label);
    }
}

pub(crate) fn on_icon_mouseout(av: RefAV, _event: Event) {
    let avb = av.borrow_mut();
    let _ = avb.tooltip.set_attribute("style", "display:none;");
    if let Some(html_element) = avb.icon_selected.get_html_element() {
        let selected_color = Color::OnCreation.get();
        html_element
            .set_attribute("style", &format!("color:{selected_color}"))
            .unwrap();
    }
    avb.user_icons.iter().for_each(|icon| {
        if *icon != avb.icon_selected {
            if let Some(html_element) = icon.get_html_element() {
                let text_color = Color::Text.get();
                html_element
                    .set_attribute("style", &format!("color:{text_color}"))
                    .unwrap();
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
    }
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
        ShapeType::ConstrCircle { .. } => "Construction Circle",
    }
}

fn show_tooltip(avb: &crate::app::AppVars, x: i32, y: i32, label: &str) {
    avb.tooltip
        .set_attribute("style", &format!("display:block;left:{x}px;top:{y}px"))
        .unwrap();
    avb.tooltip.set_inner_text(label);
}
