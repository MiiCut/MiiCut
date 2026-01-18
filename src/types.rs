use crate::math::{fillet_at_apex, ApexType, EPSILON};
use crate::shape::TextFont;
use crate::type_scalar::Scalar;
use crate::type_vertex::{ScalarU32, Vertex};
use kurbo::Vec2;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;
use std::hash::Hash;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Property {
    Center,
    Radius,
    Angle,
    Scale,
    BottomLeft,
    TopLeft,
    TopRight,
    BottomRight,
    Pt1,
    Pt2,
    Thickness,
    Text,
    Font,
    Seeds,
    Magnets,
    Apex { idx: usize },
}
impl Property {
    pub fn order(&self) -> usize {
        match self {
            Property::Center => 10,
            Property::Radius => 20,
            Property::Angle => 30,
            Property::Scale => 40,
            Property::BottomLeft => 50,
            Property::TopLeft => 60,
            Property::TopRight => 70,
            Property::BottomRight => 80,
            Property::Pt1 => 90,
            Property::Pt2 => 100,
            Property::Thickness => 110,
            Property::Text => 120,
            Property::Font => 130,
            Property::Seeds => 140,
            Property::Magnets => 150,
            Property::Apex { idx } => 1000 + idx,
        }
    }
}

#[derive(Debug, Clone)]
pub enum PropertyValue {
    Center {
        idx: usize, // index of the vertex in the shape's vertex list
        value: Vec2,
    },
    Radius {
        idx: usize, // index of the vertex in the shape's vertex list
        value: Scalar<f64>,
    },
    Angle {
        value: Scalar<f64>,
    },
    Scale {
        value: Scalar<f64>,
    },
    TopLeft {
        idx: usize, // index of the vertex in the shape's vertex list
        value: Vec2,
        radius: Option<ScalarU32>,
    },
    TopRight {
        idx: usize, // index of the vertex in the shape's vertex list
        value: Vec2,
        radius: Option<ScalarU32>,
    },
    BottomRight {
        idx: usize, // index of the vertex in the shape's vertex list
        value: Vec2,
        radius: Option<ScalarU32>,
    },
    BottomLeft {
        idx: usize, // index of the vertex in the shape's vertex list
        value: Vec2,
        radius: Option<ScalarU32>,
    },
    Pt1 {
        idx: usize, // index of the vertex in the shape's vertex list
        value: Vec2,
    },
    Pt2 {
        idx: usize, // index of the vertex in the shape's vertex list
        value: Vec2,
    },
    Thickness {
        value: Scalar<f64>,
    },
    Text {
        value: String,
    },
    Font {
        value: TextFont,
    },
    Seeds {
        value: Scalar<u64>,
    },
    Magnets {
        value: Scalar<usize>,
    },
    Apex {
        idx: usize, // index of the vertex in the shape's vertex list
        value: Vec2,
        radius: Option<ScalarU32>,
    },
}
impl fmt::Display for PropertyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use PropertyValue::*;
        match self {
            Center { .. } => write!(f, "center"),
            Angle { .. } => write!(f, "angle"),
            Radius { .. } => write!(f, "radius"),
            Scale { .. } => write!(f, "scale"),
            BottomLeft { .. } => write!(f, "bot left"),
            TopLeft { .. } => write!(f, "top left"),
            TopRight { .. } => write!(f, "top right"),
            BottomRight { .. } => write!(f, "bot right"),
            Pt1 { .. } => write!(f, "pt1"),
            Pt2 { .. } => write!(f, "pt2"),
            Thickness { .. } => write!(f, "thickness"),
            Text { .. } => write!(f, "text"),
            Font { .. } => write!(f, "font"),
            Seeds { .. } => write!(f, "seeds"),
            Magnets { .. } => write!(f, "magnets"),
            Apex { idx, .. } => write!(f, "apex {}", idx),
        }
    }
}
impl PropertyValue {
    pub fn as_vec2(&self) -> Option<Vec2> {
        match self {
            PropertyValue::Center { value, .. } => Some(*value),
            PropertyValue::TopLeft { value, .. } => Some(*value),
            PropertyValue::TopRight { value, .. } => Some(*value),
            PropertyValue::BottomRight { value, .. } => Some(*value),
            PropertyValue::BottomLeft { value, .. } => Some(*value),
            PropertyValue::Pt1 { value, .. } => Some(*value),
            PropertyValue::Pt2 { value, .. } => Some(*value),
            PropertyValue::Apex { value, .. } => Some(*value),
            _ => None,
        }
    }
    pub fn as_font(&self) -> Option<&TextFont> {
        match self {
            PropertyValue::Font { value } => Some(value),
            _ => None,
        }
    }
    pub fn get_idx(&self) -> Option<usize> {
        match self {
            PropertyValue::Center { idx, .. }
            | PropertyValue::TopLeft { idx, .. }
            | PropertyValue::TopRight { idx, .. }
            | PropertyValue::BottomRight { idx, .. }
            | PropertyValue::BottomLeft { idx, .. }
            | PropertyValue::Pt1 { idx, .. }
            | PropertyValue::Pt2 { idx, .. }
            | PropertyValue::Apex { idx, .. } => Some(*idx),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Properties {
    prop: HashMap<Property, PropertyValue>,
}
impl Properties {
    pub fn new() -> Self {
        Self {
            prop: HashMap::new(),
        }
    }
    pub fn add(&mut self, key: Property, value: PropertyValue) {
        self.prop.insert(key, value);
    }
    pub fn get(&self, key: &Property) -> Option<&PropertyValue> {
        self.prop.get(key)
    }
    pub fn get_mut(&mut self, key: &Property) -> Option<&mut PropertyValue> {
        self.prop.get_mut(key)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&Property, &PropertyValue)> {
        self.prop.iter()
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Property, &mut PropertyValue)> {
        self.prop.iter_mut()
    }
    pub fn update_vertex_property(&mut self, idx: usize, new_value: Vec2) -> Option<()> {
        for (_, val) in self.prop.iter_mut() {
            match val {
                PropertyValue::Center { idx: v_idx, value }
                | PropertyValue::TopLeft {
                    idx: v_idx, value, ..
                }
                | PropertyValue::TopRight {
                    idx: v_idx, value, ..
                }
                | PropertyValue::BottomRight {
                    idx: v_idx, value, ..
                }
                | PropertyValue::BottomLeft {
                    idx: v_idx, value, ..
                }
                | PropertyValue::Pt1 { idx: v_idx, value }
                | PropertyValue::Pt2 { idx: v_idx, value }
                | PropertyValue::Apex {
                    idx: v_idx, value, ..
                } => {
                    if *v_idx == idx {
                        *value = new_value;
                    }
                }
                _ => {}
            }
        }
        Some(())
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
    pub fn set_linear_value(&mut self, value: f64) {
        self.linear = match value {
            v if (v - 1.0).abs() < f64::EPSILON => SnapValue::SnapMin,
            v if (v - 5.0).abs() < f64::EPSILON => SnapValue::SnapMed,
            _ => SnapValue::SnapMax,
        };
    }
    pub fn next_angle(&mut self) {
        match self.angle {
            SnapValue::SnapMin => self.angle = SnapValue::SnapMed,
            SnapValue::SnapMed => self.angle = SnapValue::SnapMax,
            SnapValue::SnapMax => self.angle = SnapValue::SnapMin,
        }
    }
    pub fn set_angle_value(&mut self, value: f64) {
        self.angle = match value {
            v if (v - 1.0).abs() < f64::EPSILON => SnapValue::SnapMin,
            v if (v - 5.0).abs() < f64::EPSILON => SnapValue::SnapMed,
            _ => SnapValue::SnapMax,
        };
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

static COUNTER_VALUE: AtomicUsize = AtomicUsize::new(0);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VUId {
    id: usize,
}
impl Display for VUId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
impl VUId {
    pub fn new() -> Self {
        let id = COUNTER_VALUE.fetch_add(1, Ordering::SeqCst);
        VUId { id }
    }
}

static COUNTER_NODE: AtomicUsize = AtomicUsize::new(0);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EUId {
    id: usize,
}
impl Display for EUId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
impl EUId {
    pub fn new() -> Self {
        let id = COUNTER_NODE.fetch_add(1, Ordering::SeqCst);
        EUId { id }
    }
}

#[derive(Debug, Clone)]
pub struct VecRing<K> {
    vec: Vec<(K, Vertex)>,
}
impl<K: PartialEq + Clone + Debug> VecRing<K> {
    pub fn from_slice(slice: &[(K, Vertex)]) -> Option<Self> {
        if slice.is_empty() {
            None
        } else {
            Some(Self { vec: slice.into() })
        }
    }
    pub fn dist_ok(&self, idx1: i64, idx2: i64, dist: i64) -> Option<()> {
        let n = self.vec.len() as i64;
        if n == 0 {
            return None;
        }
        let i = ((idx1 % n) + n) % n;
        let j = ((idx2 % n) + n) % n;
        let diff = (i - j).abs();
        let min_dist = diff.min(n - diff);
        (min_dist == dist).then_some(())
    }
    pub fn insert_one_between(&mut self, key1: &K, key2: &K, kv: (K, Vertex)) -> Option<K> {
        let n = self.vec.len();
        if n < 2 {
            return None;
        }

        // find positions of the two keys
        let i = self.vec.iter().position(|(k, _)| k == key1)?;
        let j = self.vec.iter().position(|(k, _)| k == key2)?;

        // decide where to insert:
        // if key1 → key2, insert at j
        // if key2 → key1, insert at i
        let insert_at = if (i + 1) % n == j {
            j
        } else if (j + 1) % n == i {
            i
        } else {
            return None;
        };
        let (key, val) = kv;
        self.vec.insert(insert_at, (key.clone(), val));
        Some(key)
    }
    pub fn insert_two_between(
        &mut self,
        key1: &K,
        key2: &K,
        kv1: (K, Vertex),
        kv2: (K, Vertex),
    ) -> Option<(K, K)> {
        let n = self.vec.len();
        if n < 2 {
            return None;
        }

        // find positions of the two keys
        let i = self.vec.iter().position(|(k, _)| k == key1)?;
        let j = self.vec.iter().position(|(k, _)| k == key2)?;

        // decide where to insert:
        // if key1 → key2, insert at j
        // if key2 → key1, insert at i
        let insert_at = if (i + 1) % n == j {
            j
        } else if (j + 1) % n == i {
            i
        } else {
            return None;
        };
        let (key1, val1) = kv1;
        let (key2, val2) = kv2;
        self.vec.insert(insert_at, (key2.clone(), val2));
        self.vec.insert(insert_at, (key1.clone(), val1));
        Some((key1, key2))
    }
    pub fn remove(&mut self, idx: &i64) {
        let len = self.vec.len() as i64;
        let idx = (idx).rem_euclid(len) as usize;
        self.vec.remove(idx);
    }
    pub fn get_idx(&self, key: &K) -> Option<i64> {
        self.vec
            .iter()
            .position(|(k, _)| k == key)
            .and_then(|i| Some(i as i64))
    }
    pub fn key(&self, idx: i64) -> &K {
        let len = self.vec.len() as i64;
        let i = idx.rem_euclid(len) as usize;
        &self.vec[i].0
    }
    pub fn key_mut(&mut self, idx: i64) -> &mut K {
        let len = self.vec.len() as i64;
        let i = idx.rem_euclid(len) as usize;
        &mut self.vec[i].0
    }
    pub fn val(&self, idx: i64) -> &Vertex {
        let len = self.vec.len() as i64;
        let i = idx.rem_euclid(len) as usize;
        &self.vec[i].1
    }
    pub fn val_mut(&mut self, idx: i64) -> &mut Vertex {
        let len = self.vec.len() as i64;
        let i = idx.rem_euclid(len) as usize;
        &mut self.vec[i].1
    }
    pub fn val_from_key(&self, key: K) -> Option<&Vertex> {
        self.vec.iter().find_map(|(k, v)| (k == &key).then(|| v))
    }
    pub fn val_mut_from_key(&mut self, key: K) -> Option<&mut Vertex> {
        self.vec
            .iter_mut()
            .find_map(|(k, v)| (k == &key).then(|| v))
    }
    pub fn push(&mut self, e: (K, Vertex)) {
        self.vec.push(e);
    }
    pub fn replace_first(&mut self, e: (K, Vertex)) {
        self.vec[0] = e;
    }
    pub fn last_mut(&mut self) -> &mut (K, Vertex) {
        let len1 = self.vec.len() - 1;
        &mut self.vec[len1]
    }
    pub fn len(&self) -> usize {
        self.vec.len()
    }
    pub fn iter(&self) -> std::slice::Iter<'_, (K, Vertex)> {
        self.vec.iter()
    }
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, (K, Vertex)> {
        self.vec.iter_mut()
    }
    pub fn get_apices(&self) -> Vec<ApexType> {
        let n = self.vec.len();
        assert!(n >= 3);
        let mut out = Vec::with_capacity(n);

        for i in 0..n {
            let im1 = if i == 0 { n - 1 } else { i - 1 };
            let ip1 = if i + 1 == n { 0 } else { i + 1 };
            let a = &self.vec[im1].1;
            let b = &self.vec[i].1;
            let c = &self.vec[ip1].1;

            let apex = b
                .get_radius()
                .and_then(|r| fillet_at_apex(a.curr(), b.curr(), c.curr(), r as f64))
                .map(|(s, center, e)| ApexType::Arc { s, c: center, e })
                .unwrap_or(ApexType::TypeVertex { a: b.curr() });

            out.push(apex);
        }
        out
    }
}
