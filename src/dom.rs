use crate::math::*;
use kurbo::Vec2;
use wasm_bindgen::prelude::*;
use web_sys::{Event, MouseEvent};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum IconsShapes {
    Rectangle,
    RectangleRounded,
    Circle,
    Oblong,
}
impl IconsShapes {
    pub fn as_str(&self) -> &'static str {
        use IconsShapes::*;
        match self {
            Rectangle => "icon-rectangle",
            RectangleRounded => "icon-rectangle-rounded",
            Circle => "icon-circle",
            Oblong => "icon-oblong",
        }
    }
}
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Icons {
    Arrow,
    Selection,
    Scissors,
    IShapes(IconsShapes),
}
impl Icons {
    pub fn as_str(&self) -> &'static str {
        use Icons::*;
        match self {
            Arrow => "icon-arrow",
            Selection => "icon-selection",
            Scissors => "icon-scissors",
            IShapes(shape) => shape.as_str(),
        }
    }
    pub fn from_str(input: &str) -> Option<Icons> {
        use Icons::*;
        match input {
            "icon-arrow" => Some(Arrow),
            "icon-selection" => Some(Selection),
            "icon-scissors" => Some(Scissors),
            "icon-rectangle" => Some(IShapes(IconsShapes::Rectangle)),
            "icon-rectangle-rounded" => Some(IShapes(IconsShapes::RectangleRounded)),
            "icon-circle" => Some(IShapes(IconsShapes::Circle)),
            "icon-oblong" => Some(IShapes(IconsShapes::Oblong)),
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
            self.draw_pos = to_draw(&self.canvas_pos, drawing_scale, &drawing_offset);
            self.draw_pos = magnet_to_grid(&self.draw_pos);
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
