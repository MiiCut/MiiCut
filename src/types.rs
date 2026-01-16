use crate::math::{fillet_at_apex, ApexType, EPSILON};
use kurbo::Vec2;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Display;
use std::hash::Hash;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone)]
pub struct VecRing<K> {
    vec: Vec<(K, Value)>,
}
impl<K: PartialEq + Clone + Debug> VecRing<K> {
    pub fn from_slice(slice: &[(K, Value)]) -> Option<Self> {
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
    pub fn insert_one_between(&mut self, key1: &K, key2: &K, kv: (K, Value)) -> Option<K> {
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
        kv1: (K, Value),
        kv2: (K, Value),
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
    pub fn val(&self, idx: i64) -> &Value {
        let len = self.vec.len() as i64;
        let i = idx.rem_euclid(len) as usize;
        &self.vec[i].1
    }
    pub fn val_mut(&mut self, idx: i64) -> &mut Value {
        let len = self.vec.len() as i64;
        let i = idx.rem_euclid(len) as usize;
        &mut self.vec[i].1
    }
    pub fn val_from_key(&self, key: K) -> Option<&Value> {
        self.vec.iter().find_map(|(k, v)| (k == &key).then(|| v))
    }
    pub fn val_mut_from_key(&mut self, key: K) -> Option<&mut Value> {
        self.vec
            .iter_mut()
            .find_map(|(k, v)| (k == &key).then(|| v))
    }
    pub fn push(&mut self, e: (K, Value)) {
        self.vec.push(e);
    }
    pub fn replace_first(&mut self, e: (K, Value)) {
        self.vec[0] = e;
    }
    pub fn last_mut(&mut self) -> &mut (K, Value) {
        let len1 = self.vec.len() - 1;
        &mut self.vec[len1]
    }
    pub fn len(&self) -> usize {
        self.vec.len()
    }
    pub fn iter(&self) -> std::slice::Iter<'_, (K, Value)> {
        self.vec.iter()
    }
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, (K, Value)> {
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
                .and_then(|r| fillet_at_apex(a.curr, b.curr, c.curr, r as f64))
                .map(|(s, center, e)| ApexType::Arc { s, c: center, e })
                .unwrap_or(ApexType::Vertex { a: b.curr });

            out.push(apex);
        }
        out
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValuePropertyKind {
    Radius,
    Bind { eid: EUId, vid: VUId },
}

#[derive(Debug, Clone)]
pub enum ValueProperty {
    Radius {
        value: Option<u32>,
        last_value: Option<u32>,
    },
    Bind {
        eid: EUId,
        vid: VUId,
    },
}
impl ValueProperty {
    pub fn as_radius(&self) -> Option<(Option<u32>, Option<u32>)> {
        match self {
            ValueProperty::Radius { value, last_value } => Some((*value, *last_value)),
            _ => None,
        }
    }
    pub fn as_radius_mut(&mut self) -> Option<(&mut Option<u32>, &mut Option<u32>)> {
        match self {
            ValueProperty::Radius { value, last_value } => Some((value, last_value)),
            _ => None,
        }
    }

    pub fn as_bind(&self) -> Option<(EUId, VUId)> {
        match self {
            ValueProperty::Bind { eid, vid } => Some((*eid, *vid)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Value {
    properties: HashMap<ValuePropertyKind, ValueProperty>,
    saved: Vec2,
    curr: Vec2,
}
impl Value {
    pub fn new(value: Vec2) -> Self {
        let mut properties = HashMap::new();
        properties.insert(
            ValuePropertyKind::Radius,
            ValueProperty::Radius {
                value: None,
                last_value: None,
            },
        );
        Self {
            properties,
            saved: value,
            curr: value,
        }
    }
    pub fn new_from_coords(x: f64, y: f64) -> Self {
        let mut properties = HashMap::new();
        properties.insert(
            ValuePropertyKind::Radius,
            ValueProperty::Radius {
                value: None,
                last_value: None,
            },
        );
        let value = Vec2::new(x, y);
        Self {
            properties,
            saved: value,
            curr: value,
        }
    }
    pub fn save(&mut self) {
        self.saved = self.curr;
    }
    pub fn add(&mut self, value: Vec2) {
        self.curr = self.saved + value;
    }

    pub fn change_apex_type(&mut self) {
        if let Some(ValueProperty::Radius {
            value: radius,
            last_value: last_radius,
        }) = self.properties.get_mut(&ValuePropertyKind::Radius)
        {
            if radius.is_none() {
                if last_radius.is_none() {
                    *radius = Some(10);
                } else {
                    *radius = *last_radius;
                }
            } else {
                *last_radius = *radius;
                *radius = None;
            }
        }
    }

    pub fn curr(&self) -> Vec2 {
        self.curr
    }
    pub fn set_curr(&mut self, value: Vec2) {
        self.curr = value;
    }
    pub fn saved(&self) -> Vec2 {
        self.saved
    }
    pub fn set_saved(&mut self, value: Vec2) {
        self.saved = value;
    }
    pub fn set_saved_x(&mut self, x: f64) {
        self.saved.x = x;
    }
    pub fn set_saved_y(&mut self, y: f64) {
        self.saved.y = y;
    }
    pub fn get_radius(&self) -> Option<u32> {
        if let Some(ValueProperty::Radius { value, .. }) =
            self.properties.get(&ValuePropertyKind::Radius)
        {
            *value
        } else {
            None
        }
    }
    pub fn get_binds(&self) -> Vec<(EUId, VUId)> {
        let mut out = Vec::new();
        for (key, prop) in &self.properties {
            if let ValuePropertyKind::Bind { .. } = key {
                if let Some((eid, vid)) = prop.as_bind() {
                    out.push((eid, vid));
                }
            }
        }
        out
    }
    pub fn get_properties(&self) -> &HashMap<ValuePropertyKind, ValueProperty> {
        &self.properties
    }
    pub fn get_properties_mut(&mut self) -> &mut HashMap<ValuePropertyKind, ValueProperty> {
        &mut self.properties
    }
    pub fn property_remove(&mut self, key: &ValuePropertyKind) {
        self.properties.remove(key);
    }
    pub fn property_remove_all_binds(&mut self) {
        let bind_keys: Vec<ValuePropertyKind> = self
            .properties
            .keys()
            .filter_map(|k| match k {
                ValuePropertyKind::Bind { .. } => Some(k.clone()),
                _ => None,
            })
            .collect();
        for key in bind_keys {
            self.properties.remove(&key);
        }
    }
    pub fn property_insert_bind(&mut self, key: ValuePropertyKind) {
        let value = match key {
            ValuePropertyKind::Bind { eid, vid } => ValueProperty::Bind { eid, vid },
            _ => return,
        };
        self.properties.insert(key, value);
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

// #[derive(Clone, Debug)]
// pub struct Binding<T: Copy + Clone + PartialEq + Ord + Hash> {
//     bind: HashSet<Couple<T>>,
// }
// impl<T: Copy + Clone + PartialEq + Ord + Hash> Binding<T> {
//     pub fn new() -> Self {
//         Self {
//             bind: HashSet::new(),
//         }
//     }
//     pub fn contains(&self, elem: T) -> bool {
//         self.bind.iter().any(|couple| couple.contains(&elem))
//     }
//     pub fn get_other(&self, elem: T) -> Option<T> {
//         self.bind
//             .iter()
//             .find(|couple| couple.contains(&elem))
//             .map(|couple| if couple.0 == elem { couple.1 } else { couple.0 })
//     }
// }
// impl<T> Deref for Binding<T>
// where
//     T: Copy + Clone + PartialEq + Ord + Hash,
// {
//     type Target = HashSet<Couple<T>>;

//     fn deref(&self) -> &Self::Target {
//         &self.bind
//     }
// }
// impl<T> DerefMut for Binding<T>
// where
//     T: Copy + Clone + PartialEq + Ord + Hash,
// {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.bind
//     }
// }

// #[derive(Copy, Clone, Debug)]
// pub struct Couple<T: Copy + Clone + PartialEq + Hash + Ord>(pub T, pub T);
// impl<T: Copy + Clone + Hash + PartialEq + Ord> Couple<T> {
//     pub fn contains(&self, elem: &T) -> bool {
//         self.0 == *elem || self.1 == *elem
//     }
// }
// impl<T: Copy + Clone + PartialEq + Ord + Hash> PartialEq for Couple<T> {
//     fn eq(&self, other: &Self) -> bool {
//         (self.0 == other.0 && self.1 == other.1) || (self.0 == other.1 && self.1 == other.0)
//     }
// }
// impl<T: Copy + Clone + PartialEq + Ord + Hash> Eq for Couple<T> {}
// impl<T: Copy + Clone + PartialEq + Ord + Hash> Hash for Couple<T> {
//     fn hash<H: Hasher>(&self, state: &mut H) {
//         // sort the two IDs so (a,b) and (b,a) become the same pair
//         let a = min(self.0, self.1);
//         let b = max(self.0, self.1);
//         a.hash(state);
//         b.hash(state);
//     }
// }

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
