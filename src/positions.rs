use kurbo::Vec2;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum HS {
    Highlight,
    Select,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Modifier {
    highlighted: bool,
    selected: bool,
}
impl Modifier {
    pub fn new() -> Self {
        Self {
            highlighted: false,
            selected: false,
        }
    }
    pub fn highlight(&mut self, value: bool) {
        self.highlighted = value;
    }
    pub fn is_highlighted(&self) -> bool {
        self.highlighted
    }
    pub fn select(&mut self, value: bool) {
        self.selected = value;
    }
    pub fn is_selected(&self) -> bool {
        self.selected
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Value {
    saved_val: f64,
    last_val: f64,
    val: f64,
    modifier: Modifier,
}
impl Value {
    pub fn new(val: f64) -> Self {
        Self {
            saved_val: val,
            last_val: val,
            val,
            modifier: Modifier::new(),
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
    pub fn restore_saved(&mut self) {
        self.val = self.saved_val;
        self.last_val = self.val;
    }
    pub fn highlight(&mut self, value: bool) {
        self.modifier.highlighted = value;
    }
    pub fn is_highlighted(&self) -> bool {
        self.modifier.highlighted
    }
    pub fn select(&mut self, value: bool) {
        self.modifier.selected = value;
    }
    pub fn is_selected(&self) -> bool {
        self.modifier.selected
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Position {
    saved_pos: Vec2,
    last_pos: Vec2,
    pos: Vec2,
    modifier: Modifier,
    magnet: bool,
}
impl Position {
    pub fn new(pos: Vec2, magnet: bool) -> Self {
        Self {
            saved_pos: pos,
            last_pos: pos,
            pos,
            modifier: Modifier::new(),
            magnet,
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
    pub fn restore_saved(&mut self) {
        self.pos = self.saved_pos;
        self.last_pos = self.pos;
    }
    pub fn highlight(&mut self, value: bool) {
        self.modifier.highlighted = value;
    }
    pub fn is_highlighted(&self) -> bool {
        self.modifier.highlighted
    }
    pub fn select(&mut self, value: bool) {
        self.modifier.selected = value;
    }
    pub fn is_selected(&self) -> bool {
        self.modifier.selected
    }
    pub fn is_magnet(&self) -> bool {
        self.magnet
    }
    pub fn is_horizontal(&self) -> bool {
        self.pos.y == self.saved_pos.y
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Pointer {
    pos: Position,
    active: bool,
}
impl Pointer {
    pub fn new() -> Pointer {
        Pointer {
            pos: Position::new(Vec2::ZERO, false),
            active: false,
        }
    }
    pub fn get_pos(&self) -> Vec2 {
        self.pos.get_pos()
    }
    pub fn set_pos(&mut self, pos: Vec2) {
        self.pos.set_pos(pos);
    }
    pub fn save_pos(&mut self) {
        self.pos.save_pos();
    }
    pub fn restore_saved(&mut self) {
        self.pos.restore_saved();
    }
    pub fn get_last_pos(&self) -> Vec2 {
        self.pos.get_last_pos()
    }
    pub fn get_saved_pos(&self) -> Vec2 {
        self.pos.get_saved_pos()
    }
    pub fn highlight(&mut self, value: bool) {
        self.pos.highlight(value);
    }
    pub fn is_highlighted(&self) -> bool {
        self.pos.is_highlighted()
    }
    pub fn select(&mut self, value: bool) {
        self.pos.select(value);
    }
    pub fn is_selected(&self) -> bool {
        self.pos.is_selected()
    }
    pub fn set_active(&mut self, value: bool) {
        self.active = value;
    }
    pub fn is_active(&self) -> bool {
        self.active
    }
}
