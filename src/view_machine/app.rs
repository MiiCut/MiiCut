use js_sys::Date;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;
use web_sys::{Event, HtmlElement, HtmlInputElement, KeyboardEvent};

use crate::{
    app::{AppVars, RefAV},
    machine::{
        build_machine_view, load_machine_schema, machine_input_id, parse_grbl_setting_line,
        update_machine_value,
    },
    status::{begin_render, end_render, update_status_bar},
};

impl AppVars {
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
    fn wire_machine_inputs(&self, av: RefAV) -> Result<(), JsValue> {
        let Some(cnc) = self.machine.cnc.clone() else {
            return Ok(());
        };
        for group in self.machine.groups.iter() {
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
                                Ok(true) => avb.machine.last_http_error = None,
                                Ok(false) => {
                                    avb.machine.last_http_error =
                                        Some(format!("Machine setting failed: {cmd}"))
                                }
                                Err(_) => {
                                    avb.machine.last_http_error =
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

    pub(crate) fn update_machine_input(&self, id: &str, value: &str) {
        let input_id = machine_input_id(id);
        let Some(el) = self.document.get_element_by_id(&input_id) else {
            return;
        };
        if let Ok(input) = el.dyn_into::<HtmlInputElement>() {
            input.set_value(value);
        }
    }
    pub(crate) fn set_machine_status_error(&self) {
        let Some(el) = self.document.get_element_by_id("machine-status-error") else {
            return;
        };
        let Ok(el) = el.dyn_into::<HtmlElement>() else {
            return;
        };
        let mut parts = Vec::new();
        if let Some(msg) = self.machine.last_ws_error.as_ref() {
            parts.push(format!("WS: {msg}"));
        }
        if let Some(msg) = self.machine.last_http_error.as_ref() {
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
    pub(crate) fn ensure_machine_view(&mut self, av: RefAV) -> Result<(), JsValue> {
        if self.machine.groups.is_empty() {
            self.machine.groups = load_machine_schema();
        }
        if !self.machine.view_built {
            build_machine_view(&self.document, &self.machine.groups)?;
            self.wire_machine_inputs(av.clone())?;
            self.wire_machine_controls(av)?;
            self.set_machine_status_time(None);
            self.set_machine_status_error();
            self.machine.view_built = true;
        }
        Ok(())
    }
    pub(crate) fn request_machine_settings(&mut self, av: RefAV) {
        if let Some(cnc) = &self.machine.cnc {
            self.set_machine_status("Status: updating...", Some("pending"));
            self.machine.last_http_error = None;
            self.set_machine_status_error();
            let cnc = cnc.clone();
            let av = av.clone();
            spawn_local(async move {
                let result = cnc.send_http_cmd_ts("$$").await;
                if let Ok(mut avb) = av.try_borrow_mut() {
                    match result {
                        Ok(true) => avb.machine.last_http_error = None,
                        Ok(false) => {
                            avb.machine.last_http_error =
                                Some("Machine settings request failed: $$".into())
                        }
                        Err(_) => {
                            avb.machine.last_http_error =
                                Some("Machine settings request failed: $$".into())
                        }
                    }
                    if avb.machine.last_http_error.is_some() {
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
                if update_machine_value(&mut self.machine.groups, &id, &value) {
                    self.update_machine_input(&id, &value);
                    updated += 1;
                }
            }
        }
        if updated > 0 {
            self.set_machine_status("Status: updated", Some("ok"));
            let now = Date::new_0().to_string().as_string().unwrap_or_default();
            self.machine.last_update = Some(now.clone());
            self.set_machine_status_time(Some(&now));
            self.machine.last_http_error = None;
            self.set_machine_status_error();
        }
    }
}

pub(crate) fn render_machine_view(av: RefAV) {
    begin_render(av.clone(), "Machine");
    update_status_bar(av.clone());
    let _ = av.borrow_mut().ensure_machine_view(av.clone());
    end_render(av.clone());
    update_status_bar(av);
}
