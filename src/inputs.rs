use crate::{
    math::*,
    types::{Snap, Value},
};
use kurbo::Vec2;
use strum_macros::{Display, EnumString};
use web_sys::MouseEvent;

pub enum UserAction {
    ClickDown(MouseButton),
    Move(bool),
    ClickUp(MouseButton),
}
#[derive(Copy, Clone, Debug)]
pub struct UserUI {
    pub pointer: Value<Vec2>,
    pub keys_states: KeysStates,

    pub snap: Snap,
    pub active: bool,
    pub magnetized: bool,

    pub canvas_pos: Vec2,
    canvas_pos_ms_dwn: Vec2,
    canvas_pos_ms_up: Vec2,
    draw_pos: Vec2,
    draw_pos_down: Vec2,
    draw_pos_up: Vec2,
    mouse_client: Vec2,

    mouse_button: MouseButton,
    button_level: ButtonLevel,
    moving: bool,
}
impl UserUI {
    pub fn new() -> Self {
        Self {
            pointer: Value::<Vec2>::new(Vec2::ZERO),
            keys_states: KeysStates::default(),
            snap: Snap::new(),
            active: false,
            magnetized: false,
            canvas_pos: Vec2::ZERO,
            canvas_pos_ms_dwn: Vec2::ZERO,
            canvas_pos_ms_up: Vec2::ZERO,
            draw_pos: Vec2::ZERO,
            draw_pos_down: Vec2::ZERO,
            draw_pos_up: Vec2::ZERO,
            mouse_client: Vec2::ZERO,

            mouse_button: MouseButton::Left,
            button_level: ButtonLevel::Up,
            moving: false,
        }
    }
    pub fn update(
        &mut self,
        canvas_offset_x: u32,
        canvas_offset_y: u32,
        drawing_offset: Vec2,
        drawing_scale: f64,
        mouse_event: &MouseEvent,
        sys_mouse: SystemMouse,
    ) -> UserAction {
        if mouse_event.buttons() == JSMouseState::JSLeft as u16 {
            self.mouse_button = MouseButton::Left;
        }
        if mouse_event.buttons() == JSMouseState::JSMiddle as u16 {
            self.mouse_button = MouseButton::Middle;
        }
        if mouse_event.buttons() == JSMouseState::JSRight as u16 {
            self.mouse_button = MouseButton::Right;
        }
        self.mouse_client = Vec2::new(mouse_event.client_x() as f64, mouse_event.client_y() as f64);
        self.canvas_pos = Vec2::new(
            self.mouse_client.x - canvas_offset_x as f64,
            self.mouse_client.y - canvas_offset_y as f64,
        );
        self.draw_pos = to_draw(self.canvas_pos, drawing_scale, drawing_offset);

        // Pointer
        self.pointer.set(self.draw_pos);

        match sys_mouse {
            SystemMouse::Down => {
                self.canvas_pos_ms_dwn = self.canvas_pos;
                self.draw_pos_down = self.draw_pos;
                self.button_level = ButtonLevel::Down;
                self.moving = false;
                UserAction::ClickDown(self.mouse_button)
            }
            SystemMouse::Move => {
                self.moving = true;
                UserAction::Move(self.moving)
            }
            SystemMouse::Up => {
                self.canvas_pos_ms_up = self.canvas_pos;
                self.draw_pos_up = self.draw_pos;
                self.button_level = ButtonLevel::Up;
                self.moving = false;
                UserAction::ClickUp(self.mouse_button)
            }
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

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SystemMouse {
    Down,
    Move,
    Up,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum ButtonLevel {
    Down,
    Up,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MouseButton {
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
