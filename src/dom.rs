use crate::math::*;
use js_sys::Array;
use kurbo::Vec2;
use wasm_bindgen::prelude::*;
use web_sys::{CssStyleDeclaration, DomRect, Event, MouseEvent};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Icons {
    Arrow,
    Selection,
    Scissors,
    Rectangle,
    RectangleRounded,
    Circle,
    Oblong,
}
impl Icons {
    pub fn as_str(&self) -> &'static str {
        use Icons::*;
        match self {
            Arrow => "icon-arrow",
            Selection => "icon-selection",
            Scissors => "icon-scissors",
            Rectangle => "icon-rectangle",
            RectangleRounded => "icon-rectangle-rounded",
            Circle => "icon-circle",
            Oblong => "icon-oblong",
        }
    }
    pub fn from_str(input: &str) -> Option<Icons> {
        use Icons::*;
        match input {
            "icon-arrow" => Some(Arrow),
            "icon-selection" => Some(Selection),
            "icon-scissors" => Some(Scissors),
            "icon-rectangle" => Some(Rectangle),
            "icon-rectangle-rounded" => Some(RectangleRounded),
            "icon-circle" => Some(Circle),
            "icon-oblong" => Some(Oblong),
            _ => None,
        }
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

    canvas_mouse_pos: Vec2,
    canvas_mouse_pos_ms_dwn: Vec2,
    cur_pos: Vec2,
    cur_pos_down: Vec2,
    mouse_client: Vec2,
}
impl Mouse {
    pub fn new() -> Mouse {
        Self {
            mouse_button: MouseButton::Left,
            button_level: ButtonLevel::Up,
            moving: false,

            canvas_mouse_pos: Vec2::ZERO,
            canvas_mouse_pos_ms_dwn: Vec2::ZERO,
            cur_pos: Vec2::ZERO,
            cur_pos_down: Vec2::ZERO,
            mouse_client: Vec2::ZERO,
        }
    }
    pub fn get_canvas_mouse_pos(&mut self) -> Vec2 {
        self.canvas_mouse_pos
    }
    pub fn get_canvas_mouse_pos_ms_dwn(&mut self) -> Vec2 {
        self.canvas_mouse_pos_ms_dwn
    }
    pub fn get_mouse_client(&self) -> Vec2 {
        self.mouse_client
    }
    pub fn get_mouse_state(&self) -> MouseState {
        let pos = self.cur_pos;
        match self.mouse_button {
            MouseButton::Left => {
                if let ButtonLevel::Down = self.button_level {
                    if !self.moving {
                        return MouseState::LeftDown(pos);
                    } else {
                        return MouseState::LeftDownMove(self.cur_pos_down, pos);
                    }
                } else {
                    if !self.moving {
                        return MouseState::LeftUp(pos);
                    } else {
                        return MouseState::LeftUpMove(self.cur_pos_down, pos);
                    }
                }
            }
            MouseButton::Middle => {
                if let ButtonLevel::Down = self.button_level {
                    if !self.moving {
                        return MouseState::MiddleDown(pos);
                    } else {
                        return MouseState::MiddleDownMove(self.cur_pos_down, pos);
                    }
                } else {
                    if !self.moving {
                        return MouseState::MiddleUp(pos);
                    } else {
                        return MouseState::MiddleUpMove(self.cur_pos_down, pos);
                    }
                }
            }
            MouseButton::Right => {
                if let ButtonLevel::Down = self.button_level {
                    if !self.moving {
                        return MouseState::RightDown(pos);
                    } else {
                        return MouseState::RightDownMove(self.cur_pos_down, pos);
                    }
                } else {
                    if !self.moving {
                        return MouseState::RightUp(pos);
                    } else {
                        return MouseState::RightUpMove(self.cur_pos_down, pos);
                    }
                }
            }
        }
    }
    pub fn update_mouse(
        &mut self,
        bound_rect: &DomRect,
        scale: f64,
        canvas_offset: &Vec2,
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
            self.canvas_mouse_pos = Vec2 {
                x: self.mouse_client.x - bound_rect.left(),
                y: self.mouse_client.y - bound_rect.top(),
            };
            self.cur_pos = to_world(&self.canvas_mouse_pos, scale, &canvas_offset);
            self.cur_pos = magnet_to_grid(&self.cur_pos);
        }

        match sys_mouse {
            SystemMouse::Down => {
                self.canvas_mouse_pos_ms_dwn = self.canvas_mouse_pos;
                self.cur_pos_down = self.cur_pos;
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

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum Layer {
    Worksheet,
    Dimension,
    Constraints,
    GeometryHelpers,
    Origin,
    Grid,
}

#[allow(dead_code)]
pub struct DrawStyles {
    // Drawing colors
    worksheet_color: String,
    dimension_color: String,
    geohelper_color: String,
    origin_color: String,
    grid_color: String,
    selection_color: String,
    selected_color: String,
    background_color: String,
    on_creation_color: String,
    on_creation_selected_color: String,
    fill_color: String,
    bold_color: String,
    light_color: String,
    normal_color: String,
    transparent_color: String,
    // line patterns
    pattern_dashed: JsValue,
    pattern_solid: JsValue,
}
impl DrawStyles {
    pub fn build(style: CssStyleDeclaration) -> Result<DrawStyles, JsValue> {
        let worksheet_color = style.get_property_value("--canvas-worksheet-color")?;
        let dimension_color = style.get_property_value("--canvas-dimension-color")?;
        let geohelper_color = style.get_property_value("--canvas-geohelper-color")?;
        let origin_color = style.get_property_value("--canvas-origin-color")?;
        let grid_color = style.get_property_value("--canvas-grid-color")?;
        let selection_color = style.get_property_value("--canvas-selection-color")?;
        let selected_color = style.get_property_value("--canvas-selected-color")?;
        let background_color = style.get_property_value("--canvas-background-color")?;
        let on_creation_color = style.get_property_value("--canvas-on-creation-color")?;
        let on_creation_selected_color =
            style.get_property_value("--canvas-on-creation-selected-color")?;
        let fill_color = style.get_property_value("--canvas-fill-color")?;
        let bold_color = style.get_property_value("--canvas-bold-color")?;
        let light_color = style.get_property_value("--canvas-light-color")?;
        let normal_color = style.get_property_value("--canvas-normal-color")?;
        let transparent_color = style.get_property_value("--canvas-transparent-color")?;
        let dash_pattern = Array::new();
        dash_pattern.push(&JsValue::from_f64(3.0));
        dash_pattern.push(&JsValue::from_f64(3.0));
        let solid_pattern = Array::new();
        Ok(DrawStyles {
            worksheet_color,
            dimension_color,
            geohelper_color,
            origin_color,
            grid_color,
            selection_color,
            selected_color,
            background_color,
            on_creation_color,
            on_creation_selected_color,
            fill_color,
            bold_color,
            light_color,
            normal_color,
            transparent_color,
            pattern_dashed: JsValue::from(dash_pattern),
            pattern_solid: JsValue::from(solid_pattern),
        })
    }
    pub fn get_styles(&self, pattern: Pattern) -> (&JsValue, f64) {
        use Pattern::*;
        let (line_dash, line_width) = match pattern {
            Selected => (&self.pattern_solid, 2.),
            Highlighted => (&self.pattern_solid, 3.),
            Normal => (&self.pattern_solid, 1.),
            Light => (&self.pattern_solid, 1.),
            Bold => (&self.pattern_solid, 2.),
            Grid => (&self.pattern_solid, 1.),
        };

        (line_dash, line_width)
    }
    pub fn get_colors(&self, pattern: Pattern) -> (&str, &str) {
        use Pattern::*;
        let (fill_color, color) = match pattern {
            Selected => (&self.selected_color, &self.selected_color),
            Highlighted => (&self.bold_color, &self.bold_color),
            Normal => (&self.light_color, &self.normal_color),
            Light => (&self.light_color, &self.light_color),
            Bold => (&self.worksheet_color, &self.worksheet_color),
            Grid => (&self.grid_color, &self.grid_color),
        };
        (fill_color, color)
    }
    pub fn get_background_color(&self) -> &str {
        &self.background_color
    }
    pub fn get_transparent_color(&self) -> &str {
        &self.transparent_color
    }
    pub fn get_selected_color(&self) -> &str {
        &self.selected_color
    }
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum InnerPattern {
    Filled,
    None,
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub enum Pattern {
    Normal,
    Light,
    Bold,
    Grid,
    Highlighted,
    Selected,
}
