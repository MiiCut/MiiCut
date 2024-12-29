use kurbo::Vec2;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Value {
    saved_val: f64,
    last_val: f64,
    val: f64,
}
impl Value {
    pub fn new(val: f64) -> Self {
        Self {
            saved_val: val,
            last_val: val,
            val,
        }
    }
    pub fn get_val(&self) -> f64 {
        self.val
    }
    pub fn get_last_val(&self) -> f64 {
        self.last_val
    }
    pub fn get_saved_val(&self) -> f64 {
        self.saved_val
    }
    pub fn set_val(&mut self, val: f64) {
        self.last_val = self.val;
        self.val = val;
    }
    pub fn save_val(&mut self) {
        self.saved_val = self.val;
        self.last_val = self.val;
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Position {
    saved_pos: Vec2,
    last_pos: Vec2,
    pos: Vec2,
}
impl Position {
    pub fn new(pos: Vec2) -> Self {
        Self {
            saved_pos: pos,
            last_pos: pos,
            pos,
        }
    }
    pub fn get_pos(&self) -> Vec2 {
        self.pos
    }
    pub fn get_last_pos(&self) -> Vec2 {
        self.last_pos
    }
    pub fn get_saved_pos(&self) -> Vec2 {
        self.saved_pos
    }
    pub fn set_pos(&mut self, pos: Vec2) {
        self.last_pos = self.pos;
        self.pos = pos;
    }
    pub fn save_pos(&mut self) {
        self.saved_pos = self.pos;
        self.last_pos = self.pos;
    }
}
