use js_sys::{Array, Object, Reflect};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Document, HtmlElement, HtmlInputElement};

#[derive(Clone, Debug)]
pub struct MachineSetting {
    pub id: String,
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug)]
pub struct MachineGroup {
    pub name: String,
    pub settings: Vec<MachineSetting>,
}

fn build_machine_panel(document: &Document, group: &MachineGroup) -> Result<HtmlElement, JsValue> {
    let panel = document.create_element("div")?;
    panel.set_class_name("machine-panel");

    let title = document.create_element("div")?;
    title.set_class_name("machine-panel-title");
    title.set_text_content(Some(&group.name));
    panel.append_child(&title)?;

    let body = document.create_element("div")?;
    body.set_class_name("machine-panel-body");

    for setting in group.settings.iter() {
        let row = document.create_element("label")?;
        row.set_class_name("machine-setting");

        let label = document.create_element("span")?;
        label.set_class_name("machine-setting-label");
        label.set_text_content(Some(&format!("${} {}", setting.id, setting.label)));
        row.append_child(&label)?;

        let input: HtmlInputElement = document.create_element("input")?.dyn_into()?;
        let input_id = machine_input_id(&setting.id);
        input.set_id(&input_id);
        input.set_class_name("machine-setting-input");
        input.set_value(&setting.value);
        input.set_attribute("data-setting-id", &setting.id)?;
        row.append_child(&input)?;

        body.append_child(&row)?;
    }

    panel.append_child(&body)?;

    Ok(panel.dyn_into()?)
}

pub fn load_machine_schema() -> Vec<MachineGroup> {
    let schema = include_str!("../machine_settings.json");
    let parsed = match js_sys::JSON::parse(schema) {
        Ok(val) => val,
        Err(_) => return Vec::new(),
    };
    let root = Object::from(parsed);
    let keys = Object::keys(&root);
    let mut groups = Vec::new();

    for key in keys.iter() {
        let group_name = match key.as_string() {
            Some(name) => name,
            None => continue,
        };
        let group_val = match Reflect::get(&root, &key) {
            Ok(val) => val,
            Err(_) => continue,
        };
        let group_obj = Object::from(group_val);
        let inner_keys = Object::keys(&group_obj);
        if inner_keys.length() == 0 {
            continue;
        }
        let inner_key = inner_keys.get(0);
        let items_val = match Reflect::get(&group_obj, &inner_key) {
            Ok(val) => val,
            Err(_) => continue,
        };
        let items = Array::from(&items_val);
        let mut settings = Vec::new();
        for item in items.iter() {
            let id = Reflect::get(&item, &JsValue::from_str("id"))
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();
            let label = Reflect::get(&item, &JsValue::from_str("label"))
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();
            let value = Reflect::get(&item, &JsValue::from_str("value"))
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();
            if id.is_empty() {
                continue;
            }
            settings.push(MachineSetting { id, label, value });
        }
        groups.push(MachineGroup {
            name: group_name,
            settings,
        });
    }

    groups
}

pub fn machine_input_id(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    for ch in id.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    format!("machine-setting-{out}")
}

pub fn build_machine_view(document: &Document, groups: &[MachineGroup]) -> Result<(), JsValue> {
    let container = document
        .get_element_by_id("machine-container")
        .ok_or_else(|| JsValue::from_str("Missing machine-container"))?;
    let container: HtmlElement = container.dyn_into()?;
    container.set_inner_html("");

    let mut panels = Vec::new();
    for group in groups.iter() {
        let panel = build_machine_panel(document, group)?;
        container.append_child(&panel)?;
        panels.push(panel);
    }
    if let Some(window) = web_sys::window() {
        let panels = panels.clone();
        let callback = Closure::once(move || {
            for panel in panels {
                let classes = panel.class_list();
                let _ = classes.remove_1("machine-panel-overflow");
                let body = panel.query_selector(".machine-panel-body").ok().flatten();
                let Some(body) = body.and_then(|el| el.dyn_into::<HtmlElement>().ok()) else {
                    continue;
                };
                if body.scroll_height() > body.client_height() + 1 {
                    let _ = classes.add_1("machine-panel-overflow");
                }
            }
        });
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            callback.as_ref().unchecked_ref(),
            0,
        );
        callback.forget();
    }

    Ok(())
}

pub fn update_machine_value(groups: &mut [MachineGroup], id: &str, value: &str) -> bool {
    for group in groups.iter_mut() {
        for setting in group.settings.iter_mut() {
            if setting.id == id {
                setting.value = value.to_string();
                return true;
            }
        }
    }
    false
}

pub fn parse_grbl_setting_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    if !line.starts_with('$') {
        return None;
    }
    let mut parts = line[1..].splitn(2, '=');
    let id = parts.next()?.trim();
    let rest = parts.next()?.trim();
    let value = rest
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches(|c| c == '(' || c == ')');
    if id.is_empty() || value.is_empty() {
        return None;
    }
    Some((id.to_string(), value.to_string()))
}
