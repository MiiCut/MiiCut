use kurbo::Vec2;
use wasm_bindgen::prelude::*;
use web_sys::{window, Document, Element, Event, EventTarget, HtmlElement};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Icons {
    Arrow,
    Disc,
    Triangle,
    Square,
    Oblong,
    Polygon,
}
impl Icons {
    pub fn id(&self) -> &'static str {
        use Icons::*;
        match self {
            Arrow => "icon-arrow",
            Disc => "icon-disc",
            Triangle => "icon-triangle",
            Square => "icon-square",
            Oblong => "icon-oblong",
            Polygon => "icon-polygon",
        }
    }
    pub fn get_element(&self) -> Option<Element> {
        document().get_element_by_id(&self.id())
    }
    pub fn get_html_element(&self) -> Option<HtmlElement> {
        if let Some(element) = self.get_element() {
            return element.dyn_into().ok();
        };
        None
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub enum DOMElements {
    ContextMenuShape,
    ContextMenuShapeToogle,
    Icon(Icons),
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
            _ => document().get_element_by_id(&self.id()),
        }
    }

    pub fn get_html_element(&self) -> Option<HtmlElement> {
        if let Some(element) = self.get_element() {
            return element.dyn_into().ok();
        };
        None
    }
}

pub fn document() -> Document {
    window()
        .unwrap()
        .document()
        .expect("should have a document on window")
}

// Helper function to get element height
pub fn get_element_height(element: &Element) -> u32 {
    let style = window()
        .unwrap()
        .get_computed_style(element)
        .unwrap()
        .unwrap();
    style
        .get_property_value("height")
        .unwrap()
        .replace("px", "")
        .parse::<u32>()
        .unwrap_or(0)
}
// Helper function to get element width
pub fn get_element_width(element: &Element) -> u32 {
    let style = window()
        .unwrap()
        .get_computed_style(element)
        .unwrap()
        .unwrap();
    style
        .get_property_value("width")
        .unwrap()
        .replace("px", "")
        .parse::<u32>()
        .unwrap_or(0)
}
// Adds a click event listener to an HTML element
pub fn add_click_listener<F>(element: &web_sys::Element, callback: F)
where
    F: Fn() + 'static,
{
    let closure = Closure::wrap(Box::new(move |_event: Event| {
        callback();
    }) as Box<dyn FnMut(_)>);

    // Cast to EventTarget and add the event listener
    let target = element.dyn_ref::<EventTarget>().unwrap();
    target
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();

    // Keep the closure alive to avoid being dropped
    closure.forget();
}
pub fn display_html_element(dom_element: DOMElements, display: bool) {
    if let Some(cm_shape) = dom_element.get_html_element() {
        if display {
            cm_shape.style().set_property("display", "block").unwrap();
        } else {
            cm_shape.style().set_property("display", "none").unwrap();
        }
    }
}
pub fn set_pos_html_element(dom_element: DOMElements, pos: Vec2) {
    if let Some(html_element) = dom_element.get_html_element() {
        html_element
            .style()
            .set_property("top", &format!("{}px", pos.y))
            .unwrap();
        html_element
            .style()
            .set_property("left", &format!("{}px", pos.x))
            .unwrap();
    }
}
