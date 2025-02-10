use crate::{
    curves::{
        curves::CurveControls, from_dihedron::CurveFromDihedron, from_segment::CurveFromSegment,
    },
    pools::HS,
};
use kurbo::Vec2;
use std::fmt::Display;

#[derive(Copy, Debug, Clone)]
pub enum HalfEdgeProperty {
    RectangleLike,
    General,
}
impl Display for HalfEdgeProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use HalfEdgeProperty::*;
        match self {
            RectangleLike => write!(f, "Rectangle"),
            General => write!(f, "Polygon"),
        }
    }
}

#[derive(Copy, Debug, Clone)]
pub enum HalfEdgeElement {
    Apex,
    Dihedron,
    Edge,
}
#[derive(Copy, Debug, Clone)]
pub struct HalfEdge {
    // vertex: Vertex,
    dihedron_curve: CurveFromDihedron,
    edge_curve: CurveFromSegment,
}
impl HalfEdge {
    pub fn new(
        // vertex: Vertex,
        dihedron_curve: CurveFromDihedron,
        edge_curve: CurveFromSegment,
    ) -> Self {
        Self {
            // vertex,
            dihedron_curve,
            edge_curve,
        }
    }
    // pub fn get_apex(&self) -> &Position {
    //     &self.vertex
    // }
    // pub fn get_vertex_mut(&mut self) -> &mut Position {
    //     &mut self.vertex
    // }
    pub fn get_wedge(&self) -> &CurveFromDihedron {
        &self.dihedron_curve
    }
    pub fn get_wedge_mut(&mut self) -> &mut CurveFromDihedron {
        &mut self.dihedron_curve
    }
    pub fn get_edge(&self) -> &CurveFromSegment {
        &self.edge_curve
    }
    pub fn get_edge_mut(&mut self) -> &mut CurveFromSegment {
        &mut self.edge_curve
    }

    pub fn toogle_wedge_curve(&mut self) {
        self.dihedron_curve.toogle();
    }
    pub fn save_vars(&mut self) {
        // self.vertex.save_vars();
        self.edge_curve.save_vars();
        self.dihedron_curve.save_vars();
    }
    pub fn restore_vars(&mut self) {
        // self.vertex.restore_vars();
        self.edge_curve.restore_vars();
        self.dihedron_curve.restore_vars();
    }
}

#[derive(Copy, Debug, Clone, PartialEq, Default)]
pub enum VertexCurveKind {
    Chamfer,
    Fillet,
    #[default]
    Dummy,
}

// #[derive(Copy, Debug, Clone, PartialEq)]
// pub struct Vertex {
//     pos: Position,
//     state: Status,
// }
// impl Vertex {
//     const GRAB_RADIUS: f64 = 5.;

//     pub fn new(pos: Vec2) -> Self {
//         Self {
//             pos: Position::new(pos),
//             state: Status::default(),
//         }
//     }
//     pub fn get_pos(&self) -> &Position {
//         &self.pos
//     }
//     pub fn get_pos_mut(&mut self) -> &mut Position {
//         &mut self.pos
//     }
//     pub fn set_pos(&mut self, pos: Vec2) {
//         self.pos.pos = pos;
//     }
//     pub fn get_state(&self, hs: HS) -> Option<Vec2> {
//         self.state.is_hs(hs).then(|| self.pos.pos)
//     }
//     pub fn set_state(&mut self, hs: HS, value: bool) {
//         match hs {
//             HS::Highlight => self.state.highlighted = value,
//             HS::Select => self.state.selected = value,
//         }
//     }
//     pub fn set_state_from_pos(&mut self, hs: HS, pointer: &mut Pointer) {
//         let state = (pointer.pos() - self.pos.pos).hypot() < Self::GRAB_RADIUS;
//         match hs {
//             HS::Highlight => {
//                 self.state.highlighted = state;
//                 if self.state.highlighted {
//                     pointer.set_pos(self.pos.pos);
//                 }
//             }
//             HS::Select => {
//                 self.state.selected = state;
//                 if self.state.selected {
//                     pointer.set_pos(self.pos.pos);
//                     pointer.save_pos();
//                 }
//             }
//         }
//     }
//     pub fn get_dist_from_pos(&self, pos: Vec2) -> f64 {
//         (pos - self.pos.pos).hypot()
//     }
//     pub fn save_pos(&mut self) {
//         self.pos.saved_pos = self.pos.pos;
//     }
//     pub fn restore_pos(&mut self) {
//         self.pos.pos = self.pos.saved_pos;
//     }
//     pub fn save_vars(&mut self) {
//         self.save_pos();
//     }
//     pub fn restore_vars(&mut self) {
//         self.restore_pos();
//     }

//     pub fn move_pos(&mut self, dpos: Vec2) {
//         self.pos.pos = self.pos.saved_pos + dpos;
//     }

//     pub fn get_pattern(&self) -> Pattern {
//         use HS::*;
//         match (
//             self.get_state(Select).is_some(),
//             self.get_state(Highlight).is_some(),
//         ) {
//             (false, false) => Pattern::BasicNormal,
//             (false, true) => Pattern::BasicHighlighted,
//             (true, false) => Pattern::BasicSelected,
//             (true, true) => Pattern::BasicSelected,
//         }
//     }
// }

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
pub struct RadiusValue {
    pub radius: Value,
    pub up: bool,
    pub saved_up: bool,
}
impl RadiusValue {
    const _GRAB_RADIUS: f64 = 5.;

    pub fn new(radius: f64, up: bool) -> Self {
        Self {
            radius: Value::new(radius),
            up,
            saved_up: up,
        }
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
