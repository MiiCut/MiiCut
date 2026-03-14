use crate::app::RefAV;
use crate::canvas::CanvasKind;
pub(crate) use crate::import_export::init_menu;
use crate::shape::ShapeType;
pub(crate) use crate::status::init_status;
pub(crate) use crate::ui::{init_tabs, init_window};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::Window;
use web_sys::{window, Document, Element, HtmlElement};
use web_sys::{HtmlInputElement, MouseEvent};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Tabs {
    Draw,
    Gcode,
    Toolpath,
    Machine,
    Play,
}
impl Tabs {
    pub fn id(&self) -> &'static str {
        match self {
            Tabs::Draw => "tab-draw",
            Tabs::Gcode => "tab-gcode",
            Tabs::Toolpath => "tab-toolpath",
            Tabs::Machine => "tab-machine",
            Tabs::Play => "tab-play",
        }
    }

    pub fn view_id(&self) -> &'static str {
        match self {
            Tabs::Draw => "view-draw",
            Tabs::Gcode => "view-gcode",
            Tabs::Toolpath => "view-toolpath",
            Tabs::Machine => "view-machine",
            Tabs::Play => "view-play",
        }
    }

    pub fn get_element(&self) -> Option<Element> {
        document().get_element_by_id(self.id())
    }

    pub fn get_html_element(&self) -> Option<HtmlElement> {
        self.get_element()?.dyn_into().ok()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub enum DOMElements {
    ContextMenuShape,
    ContextMenuShapeToogle,
    Icon(ShapeType),
}
impl DOMElements {
    pub fn id(&self) -> &'static str {
        use DOMElements::*;
        match self {
            Icon(icon) => icon.id(),
            ContextMenuShape => "cm-shape",
            ContextMenuShapeToogle => "cm-shape-toogle",
        }
    }
    pub fn get_element(&self) -> Option<Element> {
        use DOMElements::*;
        match self {
            Icon(icon) => icon.get_element(),
            _ => document().get_element_by_id(self.id()),
        }
    }

    pub fn get_html_element(&self) -> Option<HtmlElement> {
        if let Some(element) = self.get_element() {
            return element.dyn_into().ok();
        };
        None
    }
}

pub(crate) fn document() -> Document {
    window()
        .unwrap()
        .document()
        .expect("should have a document on window")
}

// Helpers
pub(crate) fn get_element_height(element: &Element) -> u32 {
    window()
        .and_then(|w| w.get_computed_style(element).ok().flatten())
        .and_then(|style| style.get_property_value("height").ok())
        .and_then(|s| s.replace("px", "").trim().parse::<u32>().ok())
        .unwrap_or(0)
}
pub(crate) fn get_element_width(element: &Element) -> u32 {
    window()
        .and_then(|w| w.get_computed_style(element).ok().flatten())
        .and_then(|style| style.get_property_value("width").ok())
        .and_then(|s| s.replace("px", "").trim().parse::<u32>().ok())
        .unwrap_or(0)
}

pub(crate) fn add_property_row(
    document: &Document,
    body: &HtmlElement,
    label: &str,
) -> Option<HtmlElement> {
    let row = document.create_element("label").ok()?;
    row.set_class_name("shape-prop-row");

    let name = document.create_element("span").ok()?;
    name.set_class_name("shape-prop-label");
    name.set_text_content(Some(label));
    let _ = row.append_child(&name);
    let _ = body.append_child(&row);
    row.dyn_into::<HtmlElement>().ok()
}
pub(crate) fn add_property_value(
    document: &Document,
    body: &HtmlElement,
    label: &str,
    value: &str,
) -> Option<()> {
    let row = add_property_row(document, body, label)?;
    let value_el = document.create_element("span").ok()?;
    value_el.set_text_content(Some(value));
    let _ = row.append_child(&value_el);
    Some(())
}
pub(crate) fn add_property_number_input(
    document: &Document,
    body: &HtmlElement,
    label: &str,
    value: Option<f64>,
    step: f64,
) -> Option<HtmlInputElement> {
    let row = add_property_row(document, body, label)?;
    let input: HtmlInputElement = document.create_element("input").ok()?.dyn_into().ok()?;
    input.set_class_name("shape-prop-input shape-prop-input-single");
    input.set_type("number");
    input.set_step(&format!("{step:.3}"));
    let _ = input.set_attribute("inputmode", "decimal");
    if let Some(value) = value {
        input.set_value(&format!("{value:.3}"));
    }
    let _ = row.append_child(&input);
    Some(input)
}
pub(crate) fn add_property_two_number_inputs(
    document: &Document,
    body: &HtmlElement,
    label: &str,
    value_a: Option<f64>,
    value_b: Option<f64>,
    step: f64,
    decimals: usize,
) -> Option<(HtmlInputElement, HtmlInputElement)> {
    let row = add_property_row(document, body, label)?;
    let input_a: HtmlInputElement = document.create_element("input").ok()?.dyn_into().ok()?;
    input_a.set_class_name("shape-prop-input");
    input_a.set_type("number");
    input_a.set_step(&format!("{step:.1}"));
    let _ = input_a.set_attribute("inputmode", "decimal");
    if let Some(value) = value_a {
        input_a.set_value(&format!("{value:.decimals$}", decimals = decimals));
    }
    let _ = row.append_child(&input_a);

    let input_b: HtmlInputElement = document.create_element("input").ok()?.dyn_into().ok()?;
    input_b.set_class_name("shape-prop-input");
    input_b.set_type("number");
    input_b.set_step(&format!("{step:.1}"));
    let _ = input_b.set_attribute("inputmode", "decimal");
    if let Some(value) = value_b {
        input_b.set_value(&format!("{value:.decimals$}", decimals = decimals));
    }
    let _ = row.append_child(&input_b);
    Some((input_a, input_b))
}
pub(crate) fn add_property_text_input(
    document: &Document,
    body: &HtmlElement,
    label: &str,
    value: &str,
) -> Option<HtmlInputElement> {
    let row = add_property_row(document, body, label)?;
    let input: HtmlInputElement = document.create_element("input").ok()?.dyn_into().ok()?;
    input.set_class_name("shape-prop-input shape-prop-input-single");
    input.set_type("text");
    input.set_value(value);
    let _ = row.append_child(&input);
    Some(input)
}
pub(crate) fn add_property_section(
    document: &Document,
    body: &HtmlElement,
    label: &str,
) -> Option<()> {
    let row = add_property_row(document, body, label)?;
    let spacer = document.create_element("span").ok()?;
    spacer.set_text_content(Some(""));
    let _ = row.append_child(&spacer);
    Some(())
}

pub(crate) fn init_gcode_splitter(av: RefAV) -> Result<(), JsValue> {
    init_splitter(
        av,
        "gcode-split",
        "gcode-left",
        "gcode-splitter",
        CanvasKind::Gcode,
    )
}

pub(crate) fn init_toolpath_splitter(av: RefAV) -> Result<(), JsValue> {
    init_splitter(
        av,
        "toolpath-split",
        "toolpath-left",
        "toolpath-splitter",
        CanvasKind::Toolpath,
    )
}

fn init_splitter(
    av: RefAV,
    split_id: &str,
    left_id: &str,
    splitter_id: &str,
    canvas_kind: CanvasKind,
) -> Result<(), JsValue> {
    let window: Window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;

    let split: HtmlElement = document
        .get_element_by_id(split_id)
        .ok_or_else(|| JsValue::from_str(split_id))?
        .dyn_into()?;
    let left: HtmlElement = document
        .get_element_by_id(left_id)
        .ok_or_else(|| JsValue::from_str(left_id))?
        .dyn_into()?;
    let splitter: HtmlElement = document
        .get_element_by_id(splitter_id)
        .ok_or_else(|| JsValue::from_str(splitter_id))?
        .dyn_into()?;

    let left_id = left_id.to_string();
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
        let left_id = left_id.clone();

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
            if let Some(left) = avb.document.get_element_by_id(&left_id) {
                let rect = left.get_bounding_client_rect();
                let gw = rect.width().max(1.0) as u32;
                let gh = rect.height().max(1.0) as u32;
                avb.canvases[canvas_kind.idx()].resize(gw, gh);
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
