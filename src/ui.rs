use crate::app::{with_av_mut, with_av_try_mut, set_callback, RefAV};
use crate::canvas::CanvasKind;
use crate::dom::{get_element_height, get_element_width, Tabs};
use crate::view_draw::notes::delete_note_if_selected;
use crate::view_draw::notes_dom::{
    is_typing_in_input, note_id_from_element, on_window_click, update_notes_view,
};
use crate::view_draw::app::render_draw_view;
use crate::view_gcode::app::render_gcode_view;
use crate::view_machine::app::render_machine_view;
use crate::view_toolpath::app::render_toolpath_view;
use std::collections::HashSet;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Event, HtmlElement, KeyboardEvent};

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

pub(crate) fn resize_canvases(av: RefAV) {
    let mut avb = av.borrow_mut();

    let mut draw_width = get_element_width(&avb.left_panel);
    if matches!(avb.active_view, Tabs::Draw) {
        draw_width = draw_width.saturating_add(get_element_width(&avb.shapes_panel));
    }
    let draw_height = get_element_height(&avb.top_menu);
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

pub(crate) fn on_window_keydown(av: RefAV, event: Event) {
    if let Ok(kb_event) = event.dyn_into::<KeyboardEvent>() {
        let key = kb_event.key();
        let document = av.borrow().document.clone();
        let ctrl_cmd = kb_event.ctrl_key() || kb_event.meta_key();
        let shift = kb_event.shift_key();
        let alt = kb_event.alt_key();
        with_av_mut(&av, |avb| {
            let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
            let keys = &mut canvas.get_user_ui_mut().keys_states;
            keys.ctrl_cmd_pressed = ctrl_cmd;
            keys.shift_pressed = shift;
            keys.alt_pressed = alt;
        });

        let mut do_render = false;
        match key.as_str() {
            "Alt" => {
                do_render = true;
            }
            "Escape" => {
                let _ = with_av_try_mut(&av, |avb| avb.esc_pressed());
                do_render = true;
            }
            "Delete" | "Backspace" => {
                if is_typing_in_input(&kb_event, &document) {
                    return;
                }
                let active_note_id = document
                    .active_element()
                    .and_then(|el| note_id_from_element(&el));
                let deleted_note = delete_note_if_selected(&av, active_note_id);
                if !deleted_note {
                    let _ = with_av_try_mut(&av, |avb| avb.del_back_pressed());
                }
                if deleted_note {
                    update_notes_view(av.clone());
                }
                do_render = true;
            }
            "Enter" => {
                let _ = with_av_try_mut(&av, |avb| avb.group_toggle_pressed());
                do_render = true;
            }
            " " => {
                if is_typing_in_input(&kb_event, &document) {
                    return;
                }
                kb_event.prevent_default();
                let _ = with_av_try_mut(&av, |avb| avb.space_pressed());
                do_render = true;
            }
            "ArrowUp" => {
                let _ = with_av_try_mut(&av, |avb| avb.arrow_up_pressed());
                do_render = true;
            }
            "ArrowDown" => {
                let _ = with_av_try_mut(&av, |avb| avb.arrow_down_pressed());
                do_render = true;
            }
            "c" | "C" => {
                if ctrl_cmd {
                    let _ = with_av_try_mut(&av, |avb| avb.ctrl_c_pressed());
                }
                do_render = true;
            }
            "v" | "V" => {
                if ctrl_cmd {
                    let _ = with_av_try_mut(&av, |avb| avb.ctrl_v_pressed());
                }
                do_render = true;
            }
            "z" | "Z" => {
                if ctrl_cmd {
                    let _ = with_av_try_mut(&av, |avb| {
                        if shift {
                            avb._redo();
                        } else {
                            avb._undo();
                        }
                    });
                    do_render = true;
                }
            }
            "y" | "Y" => {
                if ctrl_cmd {
                    let _ = with_av_try_mut(&av, |avb| avb._redo());
                    do_render = true;
                }
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
        let is_alt = key == "Alt";
        with_av_mut(&av, |avb| {
            let keys = &mut avb.canvases[CanvasKind::Draw.idx()]
                .get_user_ui_mut()
                .keys_states;
            keys.ctrl_cmd_pressed = kb_event.ctrl_key() || kb_event.meta_key();
            keys.shift_pressed = kb_event.shift_key();
            keys.alt_pressed = kb_event.alt_key();
            // avb.update_draw_cursor();
        });
        if is_alt {
            render_draw_view(av);
        }
    }
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
    let tutorial_menu = avb.document.get_element_by_id("tutorial-menu");
    let examples_menu = avb.document.get_element_by_id("examples-menu");
    let clear_option = avb.document.get_element_by_id("clear-option");

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

    if let Some(menu) = tutorial_menu.as_ref() {
        if let Ok(menu) = menu.clone().dyn_into::<web_sys::HtmlElement>() {
            if matches!(selected, Tabs::Draw) {
                let _ = menu.style().set_property("display", "inline-block");
            } else {
                let _ = menu.style().set_property("display", "none");
            }
        }
    }

    if let Some(menu) = examples_menu.as_ref() {
        if let Ok(menu) = menu.clone().dyn_into::<web_sys::HtmlElement>() {
            if matches!(selected, Tabs::Draw) {
                let _ = menu.style().set_property("display", "inline-block");
            } else {
                let _ = menu.style().set_property("display", "none");
            }
        }
    }

    if let Some(clear) = clear_option.as_ref() {
        if let Ok(clear) = clear.clone().dyn_into::<web_sys::HtmlElement>() {
            if matches!(selected, Tabs::Draw) {
                let _ = clear.style().set_property("display", "inline-block");
            } else {
                let _ = clear.style().set_property("display", "none");
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
