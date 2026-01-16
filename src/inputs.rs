use crate::{
    math::*,
    types::{Snap, Value},
};
use kurbo::Vec2;
use strum_macros::{Display, EnumString};
use web_sys::MouseEvent;

pub enum UserAction {
    ClickDown(MouseButton, i32),
    Move(MouseButton, ButtonLevel),
    ClickUp(MouseButton, i32),
}
#[derive(Clone, Debug)]
pub struct UserUI {
    pub pointer: Value,
    pub keys_states: KeysStates,

    pub snap: Snap,
    pub active: bool,
    pub magnetized: bool,

    pub canvas_pos: Vec2,
    canvas_pos_ms_dwn: Vec2,
    canvas_pos_ms_up: Vec2,
    pub draw_pos: Vec2,
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
            pointer: Value::new(Vec2::ZERO),
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
    pub fn update_ui(
        &mut self,
        origin: Vec2,
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
            self.mouse_client.x - origin.x,
            self.mouse_client.y - origin.y,
        );
        self.draw_pos = to_draw(self.canvas_pos, drawing_scale, drawing_offset);

        // Pointer update
        self.pointer
            .set_curr((self.draw_pos / self.snap.linear()).round() * self.snap.linear());

        match sys_mouse {
            SystemMouse::Down(clicks) => {
                self.canvas_pos_ms_dwn = self.canvas_pos;
                self.draw_pos_down = self.draw_pos;
                self.button_level = ButtonLevel::Down;
                self.moving = false;
                self.pointer.save();
                UserAction::ClickDown(self.mouse_button, clicks)
            }
            SystemMouse::Move => {
                self.moving = true;
                UserAction::Move(self.mouse_button, self.button_level)
            }
            SystemMouse::Up(clicks) => {
                self.canvas_pos_ms_up = self.canvas_pos;
                self.draw_pos_up = self.draw_pos;
                self.button_level = ButtonLevel::Up;
                self.moving = false;
                UserAction::ClickUp(self.mouse_button, clicks)
            }
        }
    }
    pub fn is_left_down(&self) -> bool {
        self.button_level == ButtonLevel::Down && self.mouse_button == MouseButton::Left
    }
    pub fn cancel_drag(&mut self) {
        self.button_level = ButtonLevel::Up;
        self.moving = false;
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
    #[strum(serialize = "&")]
    Ampersand,
    #[strum(serialize = "1")]
    One,
    #[strum(serialize = "a")]
    ALower,
    #[strum(serialize = "A")]
    AUpper,
    #[strum(serialize = "i")]
    ILower,
    #[strum(serialize = "I")]
    IUpper,
    #[strum(serialize = "b")]
    BLower,
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
    // Arrows
    #[strum(serialize = "ArrowUp", serialize = "Up", serialize = "↑")]
    ArrowUp,
    #[strum(serialize = "ArrowDown", serialize = "Down", serialize = "↓")]
    ArrowDown,
    #[strum(serialize = "ArrowLeft", serialize = "Left", serialize = "←")]
    ArrowLeft,
    #[strum(serialize = "ArrowRight", serialize = "Right", serialize = "→")]
    ArrowRight,
}

#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct KeysStates {
    pub ctrl_cmd_pressed: bool,
    pub shift_pressed: bool,
    pub alt_pressed: bool,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SystemMouse {
    Down(i32),
    Move,
    Up(i32),
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ButtonLevel {
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
