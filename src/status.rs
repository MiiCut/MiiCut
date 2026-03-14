use crate::app::RefAV;
use crate::dom::Tabs;
use crate::view_draw::app::render_draw_view;
use wasm_bindgen::{closure::Closure, prelude::*, JsCast};
use web_sys::{Document, Event, HtmlElement, HtmlInputElement, Window};

pub(crate) fn now_ms(window: &Window) -> f64 {
    window.performance().map(|perf| perf.now()).unwrap_or(0.0)
}

pub(crate) fn update_status_bar(av: RefAV) {
    let mut avb = av.borrow_mut();
    let Some(text_el) = avb.document.get_element_by_id("status-text") else {
        return;
    };
    if let Some(snap_el) = avb.document.get_element_by_id("status-snap") {
        if let Ok(snap_el) = snap_el.dyn_into::<HtmlElement>() {
            if matches!(avb.active_view, Tabs::Draw) {
                let _ = snap_el.style().set_property("display", "flex");
            } else {
                let _ = snap_el.style().set_property("display", "none");
            }
        }
    }
    let now = now_ms(&avb.window);
    let render_msg = match avb.render_notice.take() {
        Some((msg, until)) if now <= until => {
            avb.render_notice = Some((msg.clone(), until));
            Some(msg)
        }
        _ => None,
    };

    let text = match avb.active_view {
        Tabs::Draw => {
            let canvas = &avb.canvases[avb.active_canvas.idx()];
            let offset = canvas.get_offset();
            let scale = canvas.get_scale();
            let pos = canvas.get_user_ui().pointer.curr();
            let mut text = format!(
                "Draw | Scale: {:.2} | Offset: ({:.1}, {:.1}) | Pos: ({:.2}, {:.2})",
                scale, offset.x, offset.y, pos.x, pos.y
            );
            if canvas.dataset.shapes_selected.len() == 1 {
                let eid = *canvas.dataset.shapes_selected.iter().next().unwrap();
                if let Some((idx, total, order)) = canvas.dataset.order_info(eid) {
                    text.push_str(&format!(" | Order: {idx}/{total} ({order})"));
                }
            }
            text
        }
        Tabs::Machine => {
            let mut parts = vec![avb.machine.ws_status.clone()];
            if let Some(msg) = avb.machine.last_ws_error.as_ref() {
                parts.push(format!("WS error: {msg}"));
            }
            if let Some(msg) = avb.machine.last_http_error.as_ref() {
                parts.push(format!("HTTP error: {msg}"));
            }
            parts.join(" | ")
        }
        Tabs::Play => {
            let mut parts = vec![avb.machine.ws_status.clone()];
            if let Some(state) = avb.play.grbl_state.as_ref() {
                parts.push(state.clone());
            }
            if let Some(pos) = avb.play.mpos {
                parts.push(format!(
                    "MPos: ({:.3}, {:.3}, {:.3})",
                    pos[0], pos[1], pos[2]
                ));
            }
            parts.join(" | ")
        }
        _ => String::new(),
    };
    if let Ok(el) = text_el.dyn_into::<HtmlElement>() {
        el.set_inner_text(&text);
    }
    if let Some(render_el) = avb.document.get_element_by_id("render-indicator") {
        if let Ok(render_el) = render_el.dyn_into::<HtmlElement>() {
            if let Some(msg) = render_msg {
                render_el.set_inner_text(&msg);
                let _ = render_el.class_list().add_1("active");
            } else {
                render_el.set_inner_text("");
                let _ = render_el.class_list().remove_1("active");
            }
        }
    }

    if matches!(avb.active_view, Tabs::Draw) {
        let canvas = &avb.canvases[avb.active_canvas.idx()];
        let snap = &canvas.get_user_ui().snap;
        set_checkbox(&avb.document, "snap-linear-1", snap.linear() == 1.0);
        set_checkbox(&avb.document, "snap-linear-5", snap.linear() == 5.0);
        set_checkbox(&avb.document, "snap-linear-10", snap.linear() == 10.0);
        set_checkbox(&avb.document, "snap-angle-1", snap.angle() == 1.0);
        set_checkbox(&avb.document, "snap-angle-5", snap.angle() == 5.0);
        set_checkbox(&avb.document, "snap-angle-10", snap.angle() == 10.0);
    }
}

pub(crate) fn init_status(av: RefAV) -> Result<(), JsValue> {
    let document = av.borrow().document.clone();
    let active_canvas = av.borrow().active_canvas;
    let setup_checkbox =
        |id: &str, av: RefAV, is_linear: bool, value: f64| -> Result<(), JsValue> {
            let Some(el) = document.get_element_by_id(id) else {
                return Ok(());
            };
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

pub(crate) fn begin_render(av: RefAV, label: &'static str) {
    let mut avb = av.borrow_mut();
    let now = now_ms(&avb.window);
    avb.render_start = Some((label, now));
    avb.render_notice = Some((format!("Rendering: {label}"), now + 250.0));
}

pub(crate) fn end_render(av: RefAV) {
    let mut avb = av.borrow_mut();
    let now = now_ms(&avb.window);
    let Some((label, start)) = avb.render_start.take() else {
        return;
    };
    let elapsed = (now - start).max(0.0);
    avb.render_notice = Some((format!("Rendered {label}: {:.0} ms", elapsed), now + 600.0));
}

pub(crate) fn set_checkbox(document: &Document, id: &str, checked: bool) {
    let Some(el) = document.get_element_by_id(id) else {
        return;
    };
    if let Ok(input) = el.dyn_into::<HtmlInputElement>() {
        input.set_checked(checked);
    }
}
