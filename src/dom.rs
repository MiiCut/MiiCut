use crate::math::*;
use kurbo::Vec2;
use wasm_bindgen::prelude::*;
use web_sys::{window, Document, Element, Event, EventTarget, HtmlElement, MouseEvent};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum IconsShapes {
    Rectangle,
    RectangleRounded,
    Disc,
    Oblong,
}
impl IconsShapes {
    pub fn id(&self) -> &'static str {
        use IconsShapes::*;
        match self {
            Rectangle => "icon-rectangle",
            RectangleRounded => "icon-rectangle-rounded",
            Disc => "icon-circle",
            Oblong => "icon-oblong",
        }
    }
}
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Icons {
    Arrow,
    Selection,
    Scissors,
    IShapes(IconsShapes),
}
impl Icons {
    pub fn id(&self) -> &'static str {
        use Icons::*;
        match self {
            Arrow => "icon-arrow",
            Selection => "icon-selection",
            Scissors => "icon-scissors",
            IShapes(shape) => shape.id(),
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

#[derive(Debug, Copy, Clone)]
pub enum SystemMouse {
    Down,
    Move,
    Up,
}

#[derive(Debug, Copy, Clone)]
enum ButtonLevel {
    Down,
    Up,
}

#[derive(Debug, Copy, Clone)]
enum MouseButton {
    Left,
    Middle,
    Right,
}

#[derive(Debug, Copy, Clone)]
pub enum MouseState {
    LeftDown(Vec2),
    LeftDownMove(Vec2, Vec2),
    LeftUp(Vec2),
    LeftUpMove(Vec2, Vec2),
    MiddleDown(Vec2),
    MiddleDownMove(Vec2, Vec2),
    MiddleUp(Vec2),
    MiddleUpMove(Vec2, Vec2),
    RightDown(Vec2),
    RightDownMove(Vec2, Vec2),
    RightUp(Vec2),
    RightUpMove(Vec2, Vec2),
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
#[repr(u16)]
enum JSMouseState {
    JSNoButton = 0,
    JSLeft = 1,
    JSRight = 2,
    JSMiddle = 4,
}

pub struct Mouse {
    mouse_button: MouseButton,
    button_level: ButtonLevel,
    moving: bool,

    canvas_pos: Vec2,
    canvas_pos_ms_dwn: Vec2,
    last_draw_pos: Vec2,
    draw_pos: Vec2,
    draw_pos_down: Vec2,
    mouse_client: Vec2,
}
impl Mouse {
    pub fn new() -> Mouse {
        Self {
            mouse_button: MouseButton::Left,
            button_level: ButtonLevel::Up,
            moving: false,

            canvas_pos: Vec2::ZERO,
            canvas_pos_ms_dwn: Vec2::ZERO,
            last_draw_pos: Vec2::ZERO,
            draw_pos: Vec2::ZERO,
            draw_pos_down: Vec2::ZERO,
            mouse_client: Vec2::ZERO,
        }
    }
    pub fn get_canvas_pos(&mut self) -> Vec2 {
        self.canvas_pos
    }
    pub fn get_canvas_pos_ms_dwn(&mut self) -> Vec2 {
        self.canvas_pos_ms_dwn
    }
    pub fn get_draw_pos_down(&mut self) -> Vec2 {
        self.draw_pos_down
    }
    pub fn get_draw_pos(&self) -> Vec2 {
        self.draw_pos
    }
    pub fn get_mouse_client(&self) -> Vec2 {
        self.mouse_client
    }
    pub fn get_mouse_state(&self) -> MouseState {
        let pos = self.draw_pos;
        match self.mouse_button {
            MouseButton::Left => {
                if let ButtonLevel::Down = self.button_level {
                    if !self.moving {
                        return MouseState::LeftDown(pos);
                    } else {
                        return MouseState::LeftDownMove(self.draw_pos_down, pos);
                    }
                } else {
                    if !self.moving {
                        return MouseState::LeftUp(pos);
                    } else {
                        return MouseState::LeftUpMove(self.draw_pos_down, pos);
                    }
                }
            }
            MouseButton::Middle => {
                if let ButtonLevel::Down = self.button_level {
                    if !self.moving {
                        return MouseState::MiddleDown(pos);
                    } else {
                        return MouseState::MiddleDownMove(self.draw_pos_down, pos);
                    }
                } else {
                    if !self.moving {
                        return MouseState::MiddleUp(pos);
                    } else {
                        return MouseState::MiddleUpMove(self.draw_pos_down, pos);
                    }
                }
            }
            MouseButton::Right => {
                if let ButtonLevel::Down = self.button_level {
                    if !self.moving {
                        return MouseState::RightDown(pos);
                    } else {
                        return MouseState::RightDownMove(self.draw_pos_down, pos);
                    }
                } else {
                    if !self.moving {
                        return MouseState::RightUp(pos);
                    } else {
                        return MouseState::RightUpMove(self.draw_pos_down, pos);
                    }
                }
            }
        }
    }
    pub fn update_mouse(
        &mut self,
        canvas_offset_x: u32,
        canvas_offset_y: u32,
        drawing_offset: Vec2,
        drawing_scale: f64,
        event: &Event,
        sys_mouse: SystemMouse,
    ) {
        if let Ok(mouse_event) = event.clone().dyn_into::<MouseEvent>() {
            if mouse_event.buttons() == JSMouseState::JSLeft as u16 {
                self.mouse_button = MouseButton::Left;
            }
            if mouse_event.buttons() == JSMouseState::JSMiddle as u16 {
                self.mouse_button = MouseButton::Middle;
            }
            if mouse_event.buttons() == JSMouseState::JSRight as u16 {
                self.mouse_button = MouseButton::Right;
            }
            self.mouse_client.x = mouse_event.client_x() as f64;
            self.mouse_client.y = mouse_event.client_y() as f64;
            self.canvas_pos = Vec2 {
                x: self.mouse_client.x - canvas_offset_x as f64,
                y: self.mouse_client.y - canvas_offset_y as f64,
            };
            self.draw_pos = to_draw(self.canvas_pos, drawing_scale, drawing_offset);
            self.draw_pos = magnet_to_grid(&self.draw_pos);
            if self.draw_pos != self.last_draw_pos {
                self.last_draw_pos = self.draw_pos;
            }
        }

        match sys_mouse {
            SystemMouse::Down => {
                self.canvas_pos_ms_dwn = self.canvas_pos;
                self.draw_pos_down = self.draw_pos;
                self.button_level = ButtonLevel::Down;
                self.moving = false;
            }
            SystemMouse::Move => self.moving = true,
            SystemMouse::Up => {
                self.button_level = ButtonLevel::Up;
                self.moving = false;
            }
        }
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
