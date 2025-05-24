use kurbo::Vec2;
use std::{
    fmt::Debug,
    ops::{Add, AddAssign},
};

use crate::math::EPSILON;

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

#[derive(Copy, Debug, Clone)]
pub struct Value<T: Copy + Clone + Debug> {
    pub saved: T,
    pub last: T,
    pub curr: T,
}
impl<T: Copy + Clone + Debug + AddAssign + Add<Output = T>> Value<T> {
    pub fn new(value: T) -> Self {
        Self {
            saved: value,
            last: value,
            curr: value,
        }
    }
    pub fn save(&mut self) {
        self.saved = self.curr;
    }
    pub fn add(&mut self, value: T) {
        self.curr = self.saved + value;
    }
    pub fn set(&mut self, value: T) {
        self.curr = value;
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Snap {
    linear: SnapValue,
    angle: SnapValue,
}
impl Snap {
    pub fn new() -> Self {
        Self {
            linear: SnapValue::SnapMax,
            angle: SnapValue::SnapMax,
        }
    }
    pub fn linear(&self) -> f64 {
        match self.linear {
            SnapValue::SnapMin => 1.,
            SnapValue::SnapMed => 5.,
            SnapValue::SnapMax => 10.,
        }
    }
    pub fn angle(&self) -> f64 {
        match self.angle {
            SnapValue::SnapMin => 1.,
            SnapValue::SnapMed => 5.,
            SnapValue::SnapMax => 10.,
        }
    }
    pub fn next_linear(&mut self) {
        match self.linear {
            SnapValue::SnapMin => self.linear = SnapValue::SnapMed,
            SnapValue::SnapMed => self.linear = SnapValue::SnapMax,
            SnapValue::SnapMax => self.linear = SnapValue::SnapMin,
        }
    }
    pub fn next_angle(&mut self) {
        match self.angle {
            SnapValue::SnapMin => self.angle = SnapValue::SnapMed,
            SnapValue::SnapMed => self.angle = SnapValue::SnapMax,
            SnapValue::SnapMax => self.angle = SnapValue::SnapMin,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SnapValue {
    SnapMin,
    SnapMed,
    SnapMax,
}

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct SegBundle {
    pub s: Vec2,
    pub e: Vec2,
    pub m: Vec2,
    pub u: Vec2,
    pub n: Vec2,
    pub len: f64,
    pub a: f64,
}
impl SegBundle {
    pub fn new(s: Vec2, e: Vec2) -> Option<Self> {
        let seg_len = (e - s).hypot();
        (seg_len >= EPSILON).then(|| {
            let mid_pt = (e + s) / 2.;
            let u_dir = (e - s).normalize();
            let n_dir = Vec2::new(-u_dir.y, u_dir.x);
            let a = (e - s).atan2();
            SegBundle {
                s,
                e,
                m: mid_pt,
                u: u_dir,
                n: n_dir,
                len: seg_len,
                a,
            }
        })
    }
    pub fn try_set_s(&mut self, s: Vec2) -> bool {
        if (s - self.e).hypot() > EPSILON {
            self.s = s;
            self.update_seg_bdle();
            true
        } else {
            false
        }
    }
    pub fn try_set_e(&mut self, e: Vec2) -> bool {
        if (e - self.s).hypot() > EPSILON {
            self.e = e;
            self.update_seg_bdle();
            true
        } else {
            false
        }
    }
    pub fn update_seg_bdle(&mut self) {
        self.len = (self.e - self.s).hypot();
        self.m = (self.e + self.s) / 2.;
        self.u = (self.e - self.s).normalize();
        self.n = Vec2::new(-self.u.y, self.u.x);
        self.a = (self.e - self.s).atan2();
    }
}
