use crate::math::EPSILON;
use crate::nodes::ElemUId;
use crate::shapes::drawable::ValueUId;
use kurbo::Vec2;
use std::cmp::{max, min};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::{
    fmt::Debug,
    ops::{Add, AddAssign},
};

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

#[derive(Debug, Clone)]
pub struct Value<T: Copy + Clone + Debug> {
    pub saved: T,
    pub last: T,
    pub curr: T,
    pub bind: HashSet<(ElemUId, ValueUId)>,
}
impl<T: Copy + Clone + Debug + AddAssign + Add<Output = T>> Value<T> {
    pub fn new(value: T) -> Self {
        Self {
            saved: value,
            last: value,
            curr: value,
            bind: HashSet::new(),
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
            let u = (e - s).normalize();
            let a = (e - s).atan2();
            SegBundle {
                s,
                e,
                m: (e + s) / 2.,
                u,
                n: Vec2::new(-u.y, u.x),
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

#[derive(Clone, Debug)]
pub struct Binding<T: Copy + Clone + PartialEq + Ord + Hash> {
    bind: HashSet<Couple<T>>,
}
impl<T: Copy + Clone + PartialEq + Ord + Hash> Binding<T> {
    pub fn new() -> Self {
        Self {
            bind: HashSet::new(),
        }
    }
    pub fn contains(&self, elem: T) -> bool {
        self.bind.iter().any(|couple| couple.contains(&elem))
    }
    pub fn get_other(&self, elem: T) -> Option<T> {
        self.bind
            .iter()
            .find(|couple| couple.contains(&elem))
            .map(|couple| if couple.0 == elem { couple.1 } else { couple.0 })
    }
    // pub fn group_by_first_element(&self) -> Vec<HashSet<Couple<T>>> {
    //     let mut grouped: HashMap<T, HashSet<Couple<T>>> = HashMap::new();

    //     // Group the couples by their first element
    //     for couple in &self.bind {
    //         let group_key = couple.0; // Group by the first element (t1)
    //         grouped
    //             .entry(group_key)
    //             .or_insert_with(HashSet::new)
    //             .insert(*couple);
    //     }

    //     // Extract all grouped sets into a vector
    //     grouped.into_iter().map(|(_, group)| group).collect()
    // }
}
impl<T> Deref for Binding<T>
where
    T: Copy + Clone + PartialEq + Ord + Hash,
{
    type Target = HashSet<Couple<T>>;

    fn deref(&self) -> &Self::Target {
        &self.bind
    }
}
impl<T> DerefMut for Binding<T>
where
    T: Copy + Clone + PartialEq + Ord + Hash,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bind
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Couple<T: Copy + Clone + PartialEq + Hash + Ord>(pub T, pub T);
impl<T: Copy + Clone + Hash + PartialEq + Ord> Couple<T> {
    pub fn contains(&self, elem: &T) -> bool {
        self.0 == *elem || self.1 == *elem
    }
}
impl<T: Copy + Clone + PartialEq + Ord + Hash> PartialEq for Couple<T> {
    fn eq(&self, other: &Self) -> bool {
        (self.0 == other.0 && self.1 == other.1) || (self.0 == other.1 && self.1 == other.0)
    }
}
impl<T: Copy + Clone + PartialEq + Ord + Hash> Eq for Couple<T> {}
impl<T: Copy + Clone + PartialEq + Ord + Hash> Hash for Couple<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // sort the two IDs so (a,b) and (b,a) become the same pair
        let a = min(self.0, self.1);
        let b = max(self.0, self.1);
        a.hash(state);
        b.hash(state);
    }
}
