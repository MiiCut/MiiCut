use crate::{
    curves::{curves::CurveControls, curves_edge::Edge, curves_wedge::CurveWedge},
    pools::HS,
};
use kurbo::Vec2;

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
pub enum HalfEdgeElement {
    Apex,
    Wedge,
    Edge,
}

#[derive(Copy, Debug, Clone)]
pub struct HalfEdge {
    wedge_curve: CurveWedge,
    edge_curve: Edge,
}
impl HalfEdge {
    pub fn new(dihedron_curve: CurveWedge, edge_curve: Edge) -> Self {
        Self {
            // vertex,
            wedge_curve: dihedron_curve,
            edge_curve,
        }
    }
    pub fn get_wedge(&self) -> &CurveWedge {
        &self.wedge_curve
    }
    pub fn get_wedge_mut(&mut self) -> &mut CurveWedge {
        &mut self.wedge_curve
    }
    pub fn get_edge(&self) -> &Edge {
        &self.edge_curve
    }
    pub fn get_edge_mut(&mut self) -> &mut Edge {
        &mut self.edge_curve
    }
    pub fn prev_wedge_curve(&mut self) {
        self.wedge_curve.next();
    }
    pub fn next_wedge_curve(&mut self) {
        self.wedge_curve.next();
    }
    pub fn save_vars(&mut self) {
        self.edge_curve.save_vars();
        self.wedge_curve.save_vars();
    }
    pub fn restore_vars(&mut self) {
        self.edge_curve.restore_vars();
        self.wedge_curve.restore_vars();
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Pointer {
    // Pointer position
    pos_saved: Vec2,
    pos: Vec2,

    snap: SnapValue,
    draw_scale: f64,
    active: bool,
    magnetized: bool,
}
impl Pointer {
    pub fn new() -> Self {
        Self {
            pos_saved: Vec2::new(0., 0.),
            pos: Vec2::new(0., 0.),

            snap: SnapValue::Snap10,
            draw_scale: 1.,
            active: false,
            magnetized: false,
        }
    }
    pub fn dpos(&self) -> Vec2 {
        self.pos - self.pos_saved
    }
    pub fn pos(&self) -> Vec2 {
        self.pos
    }
    pub fn pos_saved(&self) -> Vec2 {
        self.pos_saved
    }
    pub fn set_pos(&mut self, pos: Vec2) {
        self.pos = pos;
    }
    pub fn set_pos_rel(&mut self, dpos: Vec2) {
        self.pos = self.pos_saved + dpos;
    }
    pub fn save_pos(&mut self) {
        self.pos_saved = self.pos;
    }
    pub fn set_draw_scale(&mut self, scale: f64) {
        self.draw_scale = scale;
    }
    pub fn set_snap(&mut self, snap: SnapValue) {
        self.snap = snap;
    }
    pub fn get_snap(&self) -> SnapValue {
        self.snap
    }
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }
    pub fn is_active(&self) -> bool {
        self.active
    }
    pub fn set_magnetized(&mut self, magnetized: bool) {
        self.magnetized = magnetized;
    }
    pub fn is_magnetized(&self) -> bool {
        self.magnetized
    }
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
