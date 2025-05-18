use crate::{
    math::*,
    types::{SnapAngleValue, SnapValue},
};
use kurbo::Vec2;
use strum_macros::{Display, EnumString};
use wasm_bindgen::prelude::*;
use web_sys::{Event, MouseEvent};

pub struct Inputs {
    pub pointer: Pointer,
    pub mouse: Mouse,
    pub keys_states: KeysStates,
}
impl Inputs {
    pub fn new() -> Self {
        Self {
            pointer: Pointer::new(),
            mouse: Mouse::new(),
            keys_states: KeysStates::default(),
        }
    }
}

#[derive(Debug, EnumString, Display)]
pub enum Keys {
    Tab,
    Control,
    Meta,
    Alt,
    Shift,
    Delete,
    #[strum(serialize = " ")]
    Space,
    Backspace,
    Enter,
    Escape,
    #[strum(serialize = "a")]
    ALower,
    #[strum(serialize = "A")]
    AUpper,
    #[strum(serialize = "c")]
    CLower,
    #[strum(serialize = "v")]
    VLower,
    #[strum(serialize = "z")]
    ZLower,
    #[strum(serialize = "Z")]
    ZUpper,
    #[strum(serialize = "y")]
    YLower,
    #[strum(serialize = "s")]
    SLower,
    #[strum(serialize = "S")]
    SUpper,
    #[strum(serialize = "t")]
    TLower,
}

#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct KeysStates {
    pub crtl_cmd_pressed: bool,
    pub shift_pressed: bool,
    pub alt_pressed: bool,
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

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Pointer {
    // Pointer position
    pub pos_saved: Vec2,
    pub pos: Vec2,
    pub snap: SnapValue,
    pub snap_angle: SnapAngleValue,
    pub active: bool,
    pub magnetized: bool,
}

impl Pointer {
    pub fn new() -> Self {
        Self {
            pos_saved: Vec2::new(0., 0.),
            pos: Vec2::new(0., 0.),
            snap: SnapValue::Snap10,
            snap_angle: SnapAngleValue::Snap5,
            active: false,
            magnetized: false,
        }
    }
    pub fn dpos(&self) -> Vec2 {
        self.pos - self.pos_saved
    }
    pub fn set_pos_rel(&mut self, dpos: Vec2) {
        self.pos = self.pos_saved + dpos;
    }
}

pub struct Mouse {
    mouse_button: MouseButton,
    button_level: ButtonLevel,
    moving: bool,

    canvas_pos: Vec2,
    canvas_pos_ms_dwn: Vec2,
    canvas_pos_ms_up: Vec2,
    last_draw_pos: Vec2,
    draw_pos: Vec2,
    draw_pos_down: Vec2,
    draw_pos_up: Vec2,
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
            canvas_pos_ms_up: Vec2::ZERO,
            last_draw_pos: Vec2::ZERO,
            draw_pos: Vec2::ZERO,
            draw_pos_down: Vec2::ZERO,
            draw_pos_up: Vec2::ZERO,
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
                        return MouseState::LeftUpMove(self.draw_pos_up, pos);
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
                        return MouseState::MiddleUpMove(self.draw_pos_up, pos);
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
                        return MouseState::RightUpMove(self.draw_pos_up, pos);
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
                self.canvas_pos_ms_up = self.canvas_pos;
                self.draw_pos_up = self.draw_pos;
                self.button_level = ButtonLevel::Up;
                self.moving = false;
            }
        }
    }
}
