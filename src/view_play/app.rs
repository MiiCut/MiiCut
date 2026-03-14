use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;
use web_sys::{Document, Event, HtmlElement, HtmlInputElement, KeyboardEvent};

use crate::{
    app::{AppVars, RefAV},
    status::{begin_render, end_render, update_status_bar},
};

// ── grblHAL status parser ─────────────────────────────────────────────────────

/// Parse `<Idle|MPos:0.000,0.000,0.000|...>` → (mpos [x,y,z], state string)
pub(crate) fn parse_grbl_status(msg: &str) -> Option<([f64; 3], String)> {
    let msg = msg.trim();
    if !msg.starts_with('<') || !msg.ends_with('>') {
        return None;
    }
    let inner = &msg[1..msg.len() - 1];
    let parts: Vec<&str> = inner.split('|').collect();
    let state = parts.first()?.to_string();
    let mpos_part = parts.iter().find(|p| p.starts_with("MPos:"))?;
    let coords: Vec<f64> = mpos_part[5..]
        .split(',')
        .filter_map(|s| s.parse::<f64>().ok())
        .collect();
    if coords.len() < 3 {
        return None;
    }
    Some(([coords[0], coords[1], coords[2]], state))
}

// ── Console ───────────────────────────────────────────────────────────────────

/// Append a line to the play console.
/// `kind`: "send" | "recv" | "status" | "err"
pub(crate) fn play_console_append(document: &Document, kind: &str, text: &str) {
    let Some(console) = document.get_element_by_id("play-console") else {
        return;
    };
    let Ok(console) = console.dyn_into::<HtmlElement>() else {
        return;
    };
    let Ok(entry) = document.create_element("div") else {
        return;
    };
    let _ = entry.set_attribute("class", &format!("console-line console-{kind}"));
    entry.set_text_content(Some(text));
    let _ = console.append_child(&entry);
    // Trim to 300 lines
    while console.child_element_count() > 300 {
        if let Some(first) = console.first_child() {
            let _ = console.remove_child(&first);
        } else {
            break;
        }
    }
    console.set_scroll_top(console.scroll_height());
}

// ── DOM helpers ───────────────────────────────────────────────────────────────

fn set_text_el(document: &Document, id: &str, text: &str) {
    if let Some(el) = document.get_element_by_id(id) {
        if let Ok(el) = el.dyn_into::<HtmlElement>() {
            el.set_inner_text(text);
        }
    }
}

fn get_jog_step(document: &Document) -> f64 {
    document
        .query_selector("input[name='jog-step']:checked")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .and_then(|input| input.value().parse::<f64>().ok())
        .unwrap_or(1.0)
}

fn get_jog_feed(document: &Document) -> f64 {
    document
        .get_element_by_id("play-feed")
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .and_then(|input| input.value().parse::<f64>().ok())
        .unwrap_or(1000.0)
}

// ── Button wiring ─────────────────────────────────────────────────────────────

fn wire_jog_button(
    document: &Document,
    av: RefAV,
    button_id: &str,
    cmd: String,
) -> Result<(), JsValue> {
    let Some(el) = document.get_element_by_id(button_id) else {
        return Ok(());
    };
    let btn: HtmlElement = el.dyn_into()?;
    let doc = document.clone();
    let on_click = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
        let cnc = av.borrow().machine.cnc.clone();
        let Some(cnc) = cnc else { return };
        let cmd = cmd.clone();
        let av = av.clone();
        let doc = doc.clone();
        play_console_append(&doc, "send", &format!("> {cmd}"));
        spawn_local(async move {
            let result = cnc.send_http_cmd_ts(&cmd).await;
            if let Ok(mut avb) = av.try_borrow_mut() {
                match result {
                    Ok(true) => avb.machine.last_http_error = None,
                    Ok(false) | Err(_) => {
                        avb.machine.last_http_error = Some(format!("Failed: {cmd}"));
                        play_console_append(&doc, "err", "! http error");
                    }
                }
            }
        });
    });
    btn.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())?;
    on_click.forget();
    Ok(())
}

/// Wire a jog button whose command depends on step/feed at click time.
fn wire_jog_axis_button(
    document: &Document,
    av: RefAV,
    button_id: &'static str,
    axis_delta: fn(f64) -> String,
) -> Result<(), JsValue> {
    let Some(el) = document.get_element_by_id(button_id) else {
        return Ok(());
    };
    let btn: HtmlElement = el.dyn_into()?;
    let doc_click = document.clone();
    let on_click = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
        let step = get_jog_step(&doc_click);
        let feed = get_jog_feed(&doc_click);
        let delta = axis_delta(step);
        let cmd = format!("$J=G91 {} F{:.0}", delta, feed);
        let cnc = av.borrow().machine.cnc.clone();
        let Some(cnc) = cnc else { return };
        let av2 = av.clone();
        let doc2 = doc_click.clone();
        play_console_append(&doc_click, "send", &format!("> {cmd}"));
        spawn_local(async move {
            let result = cnc.send_http_cmd_ts(&cmd).await;
            if let Ok(mut avb) = av2.try_borrow_mut() {
                match result {
                    Ok(true) => avb.machine.last_http_error = None,
                    Ok(false) | Err(_) => {
                        avb.machine.last_http_error = Some(format!("Jog failed: {cmd}"));
                        play_console_append(&doc2, "err", "! http error");
                    }
                }
            }
        });
    });
    btn.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())?;
    on_click.forget();
    Ok(())
}

fn wire_simple_cmd_button(
    document: &Document,
    av: RefAV,
    button_id: &str,
    cmd: &'static str,
) -> Result<(), JsValue> {
    let Some(el) = document.get_element_by_id(button_id) else {
        return Ok(());
    };
    let btn: HtmlElement = el.dyn_into()?;
    let doc = document.clone();
    let on_click = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
        let cnc = av.borrow().machine.cnc.clone();
        let Some(cnc) = cnc else { return };
        let av2 = av.clone();
        let doc2 = doc.clone();
        // Show display label for non-printable bytes
        let display = match cmd {
            "\u{85}" => "> [jog cancel 0x85]",
            "\x18" => "> [soft reset 0x18]",
            _ => cmd,
        };
        play_console_append(&doc, "send", &format!("> {display}"));
        spawn_local(async move {
            let result = cnc.send_http_cmd_ts(cmd).await;
            if let Ok(mut avb) = av2.try_borrow_mut() {
                match result {
                    Ok(true) => avb.machine.last_http_error = None,
                    Ok(false) | Err(_) => {
                        avb.machine.last_http_error = Some(format!("Failed: {cmd}"));
                        play_console_append(&doc2, "err", "! http error");
                    }
                }
            }
        });
    });
    btn.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())?;
    on_click.forget();
    Ok(())
}

fn wire_play_buttons(document: &Document, av: RefAV) -> Result<(), JsValue> {
    // Axis jog buttons
    wire_jog_axis_button(document, av.clone(), "jog-x-plus", |s| format!("X{:.3}", s))?;
    wire_jog_axis_button(document, av.clone(), "jog-x-minus", |s| {
        format!("X{:.3}", -s)
    })?;
    wire_jog_axis_button(document, av.clone(), "jog-y-plus", |s| format!("Y{:.3}", s))?;
    wire_jog_axis_button(document, av.clone(), "jog-y-minus", |s| {
        format!("Y{:.3}", -s)
    })?;
    wire_jog_axis_button(document, av.clone(), "jog-z-plus", |s| format!("Z{:.3}", s))?;
    wire_jog_axis_button(document, av.clone(), "jog-z-minus", |s| {
        format!("Z{:.3}", -s)
    })?;

    // Home XY at current WCS zero
    wire_simple_cmd_button(document, av.clone(), "jog-home-xy", "G28.1")?;

    // Action buttons
    wire_simple_cmd_button(document, av.clone(), "play-home", "$H")?;
    wire_jog_button(
        document,
        av.clone(),
        "play-zero-xyz",
        "G10 L20 P1 X0 Y0 Z0".into(),
    )?;
    wire_jog_button(
        document,
        av.clone(),
        "play-zero-xy",
        "G10 L20 P1 X0 Y0".into(),
    )?;
    wire_jog_button(document, av.clone(), "play-zero-z", "G10 L20 P1 Z0".into())?;
    // Cancel jog: 0x85 real-time byte (encoded as Unicode NEL U+0085)
    wire_simple_cmd_button(document, av.clone(), "play-cancel-jog", "\u{85}")?;
    // Soft reset / E-stop: 0x18 (Ctrl-X)
    wire_simple_cmd_button(document, av.clone(), "play-estop", "\x18")?;
    // Refresh position
    wire_simple_cmd_button(document, av.clone(), "play-refresh-pos", "?")?;

    // Manual command — send button
    {
        let Some(el) = document.get_element_by_id("play-cmd-send") else {
            return Ok(());
        };
        let btn: HtmlElement = el.dyn_into()?;
        let av_send = av.clone();
        let doc_send = document.clone();
        let on_click = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
            send_manual_command(&doc_send, av_send.clone());
        });
        btn.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())?;
        on_click.forget();
    }

    // Manual command — Enter key
    {
        let Some(el) = document.get_element_by_id("play-cmd-input") else {
            return Ok(());
        };
        let input: HtmlElement = el.dyn_into()?;
        let av_enter = av.clone();
        let doc_enter = document.clone();
        let on_kd = Closure::<dyn FnMut(Event)>::new(move |evt: Event| {
            let Ok(evt) = evt.dyn_into::<KeyboardEvent>() else {
                return;
            };
            if evt.key() == "Enter" {
                evt.prevent_default();
                send_manual_command(&doc_enter, av_enter.clone());
            }
        });
        input.add_event_listener_with_callback("keydown", on_kd.as_ref().unchecked_ref())?;
        on_kd.forget();
    }

    // Console clear button
    {
        let Some(el) = document.get_element_by_id("play-console-clear") else {
            return Ok(());
        };
        let btn: HtmlElement = el.dyn_into()?;
        let doc_clear = document.clone();
        let on_click = Closure::<dyn FnMut(Event)>::new(move |_: Event| {
            if let Some(console) = doc_clear.get_element_by_id("play-console") {
                if let Ok(console) = console.dyn_into::<HtmlElement>() {
                    console.set_inner_html("");
                }
            }
        });
        btn.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref())?;
        on_click.forget();
    }

    Ok(())
}

fn send_manual_command(document: &Document, av: RefAV) {
    let Some(input_el) = document.get_element_by_id("play-cmd-input") else {
        return;
    };
    let Ok(input) = input_el.dyn_into::<HtmlInputElement>() else {
        return;
    };
    let cmd = input.value().trim().to_string();
    if cmd.is_empty() {
        return;
    }
    play_console_append(document, "send", &format!("> {cmd}"));
    input.set_value("");

    let cnc = av.borrow().machine.cnc.clone();
    let Some(cnc) = cnc else {
        play_console_append(document, "err", "! No machine connected");
        return;
    };
    let av2 = av.clone();
    let doc2 = document.clone();
    spawn_local(async move {
        let result = cnc.send_http_cmd_ts(&cmd).await;
        if let Ok(mut avb) = av2.try_borrow_mut() {
            match result {
                Ok(true) => {
                    avb.machine.last_http_error = None;
                }
                Ok(false) | Err(_) => {
                    avb.machine.last_http_error = Some(format!("Failed: {cmd}"));
                    play_console_append(&doc2, "err", "! http error");
                }
            }
        }
    });
}

// ── AppVars impl ──────────────────────────────────────────────────────────────

impl AppVars {
    pub(crate) fn ensure_play_view(&mut self, av: RefAV) -> Result<(), JsValue> {
        if !self.play.view_built {
            wire_play_buttons(&self.document, av)?;
            self.play.view_built = true;
        }
        Ok(())
    }

    pub(crate) fn request_play_position(&self, av: RefAV) {
        if let Some(cnc) = &self.machine.cnc {
            let cnc = cnc.clone();
            let doc = self.document.clone();
            play_console_append(&doc, "send", "> ?");
            spawn_local(async move {
                let _ = cnc.send_http_cmd_ts("?").await;
                if let Ok(avb) = av.try_borrow() {
                    avb.update_play_position();
                }
            });
        }
    }

    pub(crate) fn update_play_position(&self) {
        let document = &self.document;
        if let Some(pos) = self.play.mpos {
            set_text_el(document, "play-pos-x", &format!("{:.3}", pos[0]));
            set_text_el(document, "play-pos-y", &format!("{:.3}", pos[1]));
            set_text_el(document, "play-pos-z", &format!("{:.3}", pos[2]));
        }
        if let Some(state) = &self.play.grbl_state {
            let cls = match state.as_str() {
                "Idle" => "play-state-idle",
                "Run" | "Jog" => "play-state-run",
                "Alarm" => "play-state-alarm",
                _ => "",
            };
            if let Some(el) = document.get_element_by_id("play-state") {
                if let Ok(el) = el.dyn_into::<HtmlElement>() {
                    el.set_inner_text(state);
                    let classes = el.class_list();
                    let _ = classes
                        .remove_3("play-state-idle", "play-state-run", "play-state-alarm");
                    if !cls.is_empty() {
                        let _ = classes.add_1(cls);
                    }
                }
            }
        }
    }

    /// Called from handle_ws_text for grblHAL `<...>` status lines.
    pub(crate) fn handle_ws_grbl_status(&mut self, msg: &str) {
        if let Some((mpos, state)) = parse_grbl_status(msg) {
            self.play.mpos = Some(mpos);
            self.play.grbl_state = Some(state);
            self.update_play_position();
        }
        // Log status reports in a dimmed style, but only when on play tab
        play_console_append(&self.document, "status", msg.trim());
    }

    /// Called from handle_ws_text for non-status grblHAL response lines.
    pub(crate) fn console_recv(&self, line: &str) {
        play_console_append(&self.document, "recv", &format!("< {line}"));
    }
}

// ── Render ────────────────────────────────────────────────────────────────────

pub(crate) fn render_play_view(av: RefAV) {
    begin_render(av.clone(), "Play");
    update_status_bar(av.clone());
    let avb = av.borrow();
    avb.update_play_position();
    drop(avb);
    end_render(av.clone());
    update_status_bar(av);
}
