use crate::app::{NoteDrag, RefAV};
use crate::canvas::CanvasKind;
use crate::dom::Tabs;
use crate::helpers::math::{to_canvas, to_draw};
use js_sys::Array;
use kurbo::Vec2;
use pulldown_cmark::{html, Options, Parser};
use std::collections::HashSet;
use wasm_bindgen::{closure::Closure, prelude::JsValue, JsCast};
use web_sys::{
    Document, Element, Event, HtmlElement, HtmlInputElement, HtmlTextAreaElement, KeyboardEvent,
    MouseEvent, ResizeObserver, ResizeObserverEntry,
};

fn render_markdown(input: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(input, options);
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}

pub(crate) fn on_window_click(av: RefAV, event: Event) {
    let target = event
        .target()
        .and_then(|target| target.dyn_into::<Element>().ok());
    let note_id = target.as_ref().and_then(note_id_from_element);
    let mut updated = false;
    if let Ok(mut avb) = av.try_borrow_mut() {
        if avb.note_selected != note_id {
            avb.note_selected = note_id;
            updated = true;
        }
    }
    if updated {
        update_notes_view(av);
    }
}

pub(crate) fn note_id_from_element(element: &Element) -> Option<usize> {
    let mut current = Some(element.clone());
    while let Some(el) = current {
        if let Some(id) = el.get_attribute("data-note-id") {
            if let Ok(id) = id.parse::<usize>() {
                return Some(id);
            }
        }
        current = el.parent_element();
    }
    None
}

pub(crate) fn is_typing_in_input(event: &KeyboardEvent, document: &web_sys::Document) -> bool {
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

pub(crate) fn update_notes_view(av: RefAV) {
    let Ok(mut avb) = av.try_borrow_mut() else {
        return;
    };
    if avb.active_view != Tabs::Draw {
        return;
    }
    let document = avb.document.clone();
    let Some(container) = document.get_element_by_id("canvas-container") else {
        return;
    };
        let (scale, offset, notes) = {
            let canvas = &avb.canvases[CanvasKind::Draw.idx()];
            let scale = canvas.get_scale();
            let offset = canvas.get_offset();
            let notes = canvas.notes.notes.clone();
            (scale, offset, notes)
        };

    let note_ids: HashSet<usize> = notes.iter().map(|note| note.id).collect();
    if let Some(selected) = avb.note_selected {
        if !note_ids.contains(&selected) {
            avb.note_selected = None;
        }
    }
    let remove_ids: Vec<usize> = avb
        .notes_dom
        .keys()
        .filter(|id| !note_ids.contains(id))
        .copied()
        .collect();
    for id in remove_ids {
        if let Some(observer) = avb.notes_resize_observers.remove(&id) {
            observer.disconnect();
        }
        if let Some(el) = avb.notes_dom.remove(&id) {
            let _ = container.remove_child(&el);
        }
    }

    for note in &notes {
        let entry = if let Some(entry) = avb.notes_dom.get(&note.id) {
            entry.clone()
        } else {
            let Some(entry) = build_note_element(av.clone(), note.id, &container, &document) else {
                continue;
            };
            avb.notes_dom.insert(note.id, entry.clone());
            entry
        };
        if let std::collections::hash_map::Entry::Vacant(e) =
            avb.notes_resize_observers.entry(note.id)
        {
            if let Some(observer) = create_note_resize_observer(av.clone(), note.id, &entry) {
                e.insert(observer);
            }
        }
        let class_name = if avb.note_selected == Some(note.id) {
            "note-item note-selected"
        } else {
            "note-item"
        };
        let _ = entry.set_attribute("class", class_name);
        let pos_px = to_canvas(note.pos, scale, offset);
        let size_px = Vec2::new(note.size.x * scale, note.size.y * scale);
        let style = entry.style();
        let _ = style.set_property("left", &format!("{:.1}px", pos_px.x));
        let _ = style.set_property("top", &format!("{:.1}px", pos_px.y));
        let _ = style.set_property("width", &format!("{:.1}px", size_px.x));
        let _ = style.set_property("height", &format!("{:.1}px", size_px.y));
        if let Ok(Some(header)) = entry.query_selector("[data-role=\"note-header\"]") {
            if let Ok(header) = header.dyn_into::<HtmlElement>() {
                let _ = header.style().set_property("height", &format!("{:.1}px", 20.0 * scale));
                let _ = header
                    .style()
                    .set_property("padding", &format!("{:.1}px {:.1}px", 2.0 * scale, 6.0 * scale));
                let _ = header
                    .style()
                    .set_property("font-size", &format!("{:.1}px", 12.0 * scale));
            }
        }
        if let Ok(Some(preview)) = entry.query_selector("[data-role=\"note-preview\"]") {
            if let Ok(preview) = preview.dyn_into::<HtmlElement>() {
                let _ = preview
                    .style()
                    .set_property("font-size", &format!("{:.1}px", 14.0 * scale));
                let _ = preview
                    .style()
                    .set_property("padding", &format!("{:.1}px", 6.0 * scale));
            }
        }
        if let Ok(Some(textarea)) = entry.query_selector("textarea") {
            if let Ok(textarea) = textarea.dyn_into::<HtmlTextAreaElement>() {
                let _ = textarea
                    .style()
                    .set_property("font-size", &format!("{:.1}px", 14.0 * scale));
                let _ = textarea
                    .style()
                    .set_property("padding", &format!("{:.1}px", 6.0 * scale));
                if textarea.read_only() {
                    textarea.set_value(&note.text);
                    let _ = textarea.style().set_property("display", "none");
                    if let Ok(Some(preview)) = entry.query_selector("[data-role=\"note-preview\"]") {
                        if let Ok(preview) = preview.dyn_into::<HtmlElement>() {
                            preview.set_inner_html(&render_markdown(&note.text));
                            let _ = preview.style().set_property("display", "block");
                        }
                    }
                } else {
                    let _ = textarea.style().set_property("display", "block");
                    if let Ok(Some(preview)) = entry.query_selector("[data-role=\"note-preview\"]") {
                        if let Ok(preview) = preview.dyn_into::<HtmlElement>() {
                            let _ = preview.style().set_property("display", "none");
                        }
                    }
                }
            }
        }
    }
}

pub(crate) fn focus_note_editor(av: RefAV, note_id: usize) {
    let Ok(avb) = av.try_borrow_mut() else {
        return;
    };
    let Some(note_el) = avb.notes_dom.get(&note_id) else {
        return;
    };
    let Ok(Some(textarea)) = note_el.query_selector("textarea") else {
        return;
    };
    let Ok(textarea) = textarea.dyn_into::<HtmlTextAreaElement>() else {
        return;
    };
    textarea.set_read_only(false);
    let _ = textarea.focus();
}

fn draw_pos_from_event(canvas: &crate::canvas::Canvas, event: &MouseEvent) -> Vec2 {
    let rect = canvas.get_canvas().get_bounding_client_rect();
    let canvas_pos = Vec2::new(
        event.client_x() as f64 - rect.left(),
        event.client_y() as f64 - rect.top(),
    );
    to_draw(canvas_pos, canvas.get_scale(), canvas.get_offset())
}

fn build_note_element(
    av: RefAV,
    note_id: usize,
    container: &Element,
    document: &Document,
) -> Option<HtmlElement> {
    let note_el: HtmlElement = document.create_element("div").ok()?.dyn_into().ok()?;
    note_el.set_attribute("class", "note-item").ok()?;
    note_el
        .set_attribute("data-note-id", &note_id.to_string())
        .ok()?;

    let header: HtmlElement = document.create_element("div").ok()?.dyn_into().ok()?;
    header.set_attribute("class", "note-header").ok()?;
    header.set_attribute("data-role", "note-header").ok()?;
    header.set_inner_text("Note");

    let textarea: HtmlTextAreaElement =
        document.create_element("textarea").ok()?.dyn_into().ok()?;
    textarea.set_attribute("class", "note-text").ok()?;
    textarea.set_attribute("data-role", "note-text").ok()?;
    textarea.set_read_only(true);

    let preview: HtmlElement = document.create_element("div").ok()?.dyn_into().ok()?;
    preview.set_attribute("class", "note-preview").ok()?;
    preview.set_attribute("data-role", "note-preview").ok()?;
    preview.set_inner_html("");

    note_el.append_child(&header).ok()?;
    note_el.append_child(&textarea).ok()?;
    note_el.append_child(&preview).ok()?;
    container.append_child(&note_el).ok()?;

    {
        let av_clone = av.clone();
        let av_select = av.clone();
        let on_select = Closure::wrap(Box::new(move |_event: MouseEvent| {
            let Ok(mut avb) = av_select.try_borrow_mut() else {
                return;
            };
            if avb.note_selected == Some(note_id) {
                return;
            }
            avb.note_selected = Some(note_id);
            drop(avb);
            update_notes_view(av_select.clone());
        }) as Box<dyn FnMut(_)>);
        note_el
            .add_event_listener_with_callback("mousedown", on_select.as_ref().unchecked_ref())
            .ok()?;
        on_select.forget();

        let on_down = Closure::wrap(Box::new(move |event: MouseEvent| {
            event.prevent_default();
            event.stop_propagation();
            let Ok(mut avb) = av_clone.try_borrow_mut() else {
                return;
            };
            let mut changed = false;
            if avb.note_selected != Some(note_id) {
                avb.note_selected = Some(note_id);
                changed = true;
            }
            let canvas = &avb.canvases[CanvasKind::Draw.idx()];
            let draw_pos = draw_pos_from_event(canvas, &event);
            let Some(note) = canvas.notes.notes.iter().find(|note| note.id == note_id) else {
                return;
            };
            let offset = draw_pos - note.pos;
            avb.note_drag = Some(NoteDrag {
                id: note_id,
                offset,
            });
            if changed {
                drop(avb);
                update_notes_view(av_clone.clone());
            }
        }) as Box<dyn FnMut(_)>);
        header
            .add_event_listener_with_callback("mousedown", on_down.as_ref().unchecked_ref())
            .ok()?;
        on_down.forget();
    }

    {
        let av_clone = av.clone();
        let on_dbl = Closure::wrap(Box::new(move |event: Event| {
            event.prevent_default();
            if let Ok(avb) = av_clone.try_borrow_mut() {
                if let Some(note_el) = avb.notes_dom.get(&note_id) {
                    if let Ok(Some(textarea)) = note_el.query_selector("textarea") {
                        if let Ok(textarea) = textarea.dyn_into::<HtmlTextAreaElement>() {
                            textarea.set_read_only(false);
                            let _ = textarea.style().set_property("display", "block");
                            if let Ok(Some(preview)) =
                                note_el.query_selector("[data-role=\"note-preview\"]")
                            {
                                if let Ok(preview) = preview.dyn_into::<HtmlElement>() {
                                    let _ = preview.style().set_property("display", "none");
                                }
                            }
                            let _ = textarea.focus();
                        }
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);
        note_el
            .add_event_listener_with_callback("dblclick", on_dbl.as_ref().unchecked_ref())
            .ok()?;
        on_dbl.forget();
    }

    {
        let av_clone = av.clone();
        let on_blur = Closure::wrap(Box::new(move |_event: Event| {
            let Ok(mut avb) = av_clone.try_borrow_mut() else {
                return;
            };
            let Some(note_el) = avb.notes_dom.get(&note_id).cloned() else {
                return;
            };
            let Ok(Some(textarea)) = note_el.query_selector("textarea") else {
                return;
            };
            let Ok(textarea) = textarea.dyn_into::<HtmlTextAreaElement>() else {
                return;
            };
            textarea.set_read_only(true);
            let _ = textarea.style().set_property("display", "none");
            let text = textarea.value();
            if let Some(note) = avb.canvases[CanvasKind::Draw.idx()].notes.get_mut(note_id) {
                note.text = text.clone();
            }
            if let Ok(Some(preview)) = note_el.query_selector("[data-role=\"note-preview\"]") {
                if let Ok(preview) = preview.dyn_into::<HtmlElement>() {
                    preview.set_inner_html(&render_markdown(&text));
                    let _ = preview.style().set_property("display", "block");
                }
            }
        }) as Box<dyn FnMut(_)>);
        textarea
            .add_event_listener_with_callback("blur", on_blur.as_ref().unchecked_ref())
            .ok()?;
        on_blur.forget();
    }

    Some(note_el)
}

fn create_note_resize_observer(
    av: RefAV,
    note_id: usize,
    note_el: &HtmlElement,
) -> Option<ResizeObserver> {
    let av_clone = av.clone();
    let on_resize = Closure::wrap(Box::new(move |entries: Array, _observer: ResizeObserver| {
        let Ok(mut avb) = av_clone.try_borrow_mut() else {
            return;
        };
        if avb.active_view != Tabs::Draw {
            return;
        }
        let scale = avb.canvases[CanvasKind::Draw.idx()].get_scale();
        for entry in entries.iter() {
            let Ok(entry) = entry.dyn_into::<ResizeObserverEntry>() else {
                continue;
            };
            let rect = entry.content_rect();
            if rect.width() <= 1.0 || rect.height() <= 1.0 {
                continue;
            }
            if let Some(note) = avb.canvases[CanvasKind::Draw.idx()].notes.get_mut(note_id) {
                let size = Vec2::new(rect.width() / scale, rect.height() / scale);
                note.size = Vec2::new(size.x.max(10.0), size.y.max(10.0));
            }
        }
    }) as Box<dyn FnMut(_, _)>);
    let observer = ResizeObserver::new(on_resize.as_ref().unchecked_ref()).ok()?;
    observer.observe(note_el);
    on_resize.forget();
    Some(observer)
}

pub(crate) fn init_note_handlers(av: RefAV) -> Result<(), JsValue> {
    let window = av.borrow().window.clone();
    let av_clone = av.clone();
    let on_move = Closure::<dyn FnMut(MouseEvent)>::new(move |event: MouseEvent| {
        let Ok(mut avb) = av_clone.try_borrow_mut() else {
            return;
        };
        let Some(drag) = avb.note_drag.clone() else {
            return;
        };
        let (scale, offset, note_el) = {
            let canvas = &avb.canvases[CanvasKind::Draw.idx()];
            let scale = canvas.get_scale();
            let offset = canvas.get_offset();
            let note_el = avb.notes_dom.get(&drag.id).cloned();
            (scale, offset, note_el)
        };
        let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
        let draw_pos = draw_pos_from_event(canvas, &event);
        if let Some(note) = canvas.notes.get_mut(drag.id) {
            note.pos = draw_pos - drag.offset;
            if let Some(note_el) = note_el {
                let pos_px = to_canvas(note.pos, scale, offset);
                let size_px = Vec2::new(note.size.x * scale, note.size.y * scale);
                let style = note_el.style();
                let _ = style.set_property("left", &format!("{:.1}px", pos_px.x));
                let _ = style.set_property("top", &format!("{:.1}px", pos_px.y));
                let _ = style.set_property("width", &format!("{:.1}px", size_px.x));
                let _ = style.set_property("height", &format!("{:.1}px", size_px.y));
            }
        }
    });
    window.add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref())?;
    on_move.forget();

    let av_clone = av.clone();
    let on_up = Closure::<dyn FnMut(MouseEvent)>::new(move |_event: MouseEvent| {
        let Ok(mut avb) = av_clone.try_borrow_mut() else {
            return;
        };
        avb.note_drag = None;
    });
    window.add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref())?;
    on_up.forget();

    Ok(())
}
