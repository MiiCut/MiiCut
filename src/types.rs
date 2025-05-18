use kurbo::Vec2;

#[derive(Debug, Clone)]
pub struct VecRing<T> {
    pub vec: Vec<T>,
}
impl<T> VecRing<T> {
    pub fn from_element(e: T) -> Self {
        Self { vec: vec![e] }
    }
    pub fn get(&self, idx: i64) -> &T {
        let len = self.vec.len() as i64;
        let i = idx.rem_euclid(len) as usize;
        &self.vec[i]
    }
    pub fn get_mut(&mut self, idx: i64) -> &mut T {
        let len = self.vec.len() as i64;
        let i = idx.rem_euclid(len) as usize;
        &mut self.vec[i]
    }
    pub fn push(&mut self, e: T) {
        self.vec.push(e);
    }
    pub fn replace_first(&mut self, e: T) {
        self.vec[0] = e;
    }
    pub fn last_mut(&mut self) -> &mut T {
        let len1 = self.vec.len() - 1;
        &mut self.vec[len1]
    }
    pub fn len(&self) -> usize {
        self.vec.len()
    }
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.vec.iter()
    }
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.vec.iter_mut()
    }
}

#[derive(Copy, Debug, Clone)]
pub struct Minimum {
    min_bundle: Option<(f64, i64, Vec2)>,
}
impl Minimum {
    pub fn new() -> Self {
        Self { min_bundle: None }
    }
    pub fn update(&mut self, value: f64, index: i64, pos: Vec2) {
        if let Some((min, idx_min, pos_min)) = self.min_bundle.as_mut() {
            if value < *min {
                *min = value;
                *idx_min = index;
                *pos_min = pos;
            }
        } else {
            self.min_bundle = Some((value, index, pos));
        }
    }
    pub fn get_min(&self) -> Option<(f64, i64, Vec2)> {
        self.min_bundle
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum HS {
    Highlight,
    Select,
}

#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct Status {
    highlighted: bool,
    selected: bool,
}
impl Status {
    pub fn is_hs(&self, hs: HS) -> bool {
        match hs {
            HS::Highlight => self.highlighted,
            HS::Select => self.selected,
        }
    }
    pub fn set_hs(&mut self, hs: HS, value: bool) {
        match hs {
            HS::Highlight => self.highlighted = value,
            HS::Select => self.selected = value,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct Value {
    pub saved_val: f64,
    pub last_val: f64,
    pub value: f64,
}
impl Value {
    pub fn new(value: f64) -> Self {
        Self {
            saved_val: value,
            last_val: value,
            value,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct ValueBool {
    pub saved_val: bool,
    pub last_val: bool,
    pub value: bool,
}
impl ValueBool {
    pub fn new(value: bool) -> Self {
        Self {
            saved_val: value,
            last_val: value,
            value,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct Position {
    pub saved_pos: Vec2,
    pub last_pos: Vec2,
    pub pos: Vec2,
}
impl Position {
    pub fn new(pos: Vec2) -> Self {
        Self {
            saved_pos: pos,
            last_pos: pos,
            pos,
        }
    }
    pub fn move_pos(&mut self, dpos: Vec2) {
        self.pos = self.saved_pos + dpos;
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SnapValue {
    Snap1,
    Snap5,
    Snap10,
}
impl SnapValue {
    pub fn val(&self) -> f64 {
        match self {
            SnapValue::Snap1 => 1.,
            SnapValue::Snap5 => 5.,
            SnapValue::Snap10 => 10.,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SnapAngleValue {
    Snap1,
    Snap5,
    Snap10,
}
impl SnapAngleValue {
    pub fn val(&self) -> f64 {
        match self {
            SnapAngleValue::Snap1 => 1.,
            SnapAngleValue::Snap5 => 5.,
            SnapAngleValue::Snap10 => 10.,
        }
    }
}
