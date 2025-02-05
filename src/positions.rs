use crate::{
    canvas::{CanvasText, Pattern},
    curves::{
        curve_arc::CurveArc,
        curve_line::CurveLine,
        curves::{Curve, CurveControls},
    },
    pools::HS,
};
use kurbo::{BezPath, Size, Vec2};
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
    Vertex,
    Modifier,
    Curve,
}
#[derive(Copy, Debug, Clone)]
pub struct HalfEdge {
    vertex: Vertex,
    vertex_curve: VertexCurve,
    edge: Curve,
}
impl HalfEdge {
    pub fn new(vertex: Vertex, curve: Curve) -> Self {
        Self {
            vertex,
            vertex_curve: VertexCurve::default(),
            edge: curve,
        }
    }
    pub fn get_vertex(&self) -> &Vertex {
        &self.vertex
    }
    pub fn get_vertex_mut(&mut self) -> &mut Vertex {
        &mut self.vertex
    }
    pub fn get_vertex_curve(&self) -> &VertexCurve {
        &self.vertex_curve
    }
    pub fn get_vertex_curve_mut(&mut self) -> &mut VertexCurve {
        &mut self.vertex_curve
    }
    pub fn get_edge(&self) -> &Curve {
        &self.edge
    }
    pub fn get_edge_mut(&mut self) -> &mut Curve {
        &mut self.edge
    }

    pub fn toogle_vertex_curve(&mut self) {
        self.vertex_curve.toogle();
    }
    pub fn save_vars(&mut self) {
        self.vertex.save_vars();
        self.edge.save_vars();
        self.vertex_curve.save_vars();
    }
    pub fn restore_vars(&mut self) {
        self.vertex.restore_vars();
        self.edge.restore_vars();
        self.vertex_curve.restore_vars();
    }
}

#[derive(Copy, Debug, Clone, PartialEq, Default)]
pub enum VertexCurveKind {
    Chamfer,
    Fillet,
    #[default]
    Dummy,
}
#[derive(Copy, Debug, Clone, PartialEq, Default)]
pub struct VertexCurve {
    kind: VertexCurveKind,
    kind_saved: VertexCurveKind,

    dummy: Position,
    chamfer: CurveLine,
    fillet: CurveArc,
    fillet_concavity: ValueBool,
    fillet_concavity_saved: ValueBool,

    chamfer_status: Status,
    fillet_status: Status,
}
impl VertexCurve {
    const GRAB_RADIUS: f64 = 5.;
    pub fn get_kind(&self) -> VertexCurveKind {
        self.kind
    }
    pub fn get_start(&self) -> Position {
        use VertexCurveKind::*;
        match self.kind {
            Dummy => self.dummy,
            Chamfer => self.chamfer.get_start(),
            Fillet => self.fillet.get_start(),
        }
    }
    pub fn get_end(&self) -> Position {
        use VertexCurveKind::*;
        match self.kind {
            Dummy => self.dummy,
            Chamfer => self.chamfer.get_end(),
            Fillet => self.fillet.get_end(),
        }
    }
    pub fn toogle(&mut self) {
        use VertexCurveKind::*;
        match self.kind {
            Dummy => self.kind = Chamfer,
            Chamfer => self.kind = Fillet,
            Fillet => self.kind = Dummy,
        }
    }
    pub fn save_vars(&mut self) {
        self.kind_saved = self.kind;
        self.chamfer.save_vars();
        self.fillet.save_vars();
        self.fillet_concavity_saved = self.fillet_concavity;
    }
    pub fn restore_vars(&mut self) {
        self.kind = self.kind_saved;
        self.chamfer.restore_vars();
        self.fillet.restore_vars();
        self.fillet_concavity = self.fillet_concavity_saved;
    }
    pub fn get_state(&self, hs: HS) -> Option<Vec2> {
        use VertexCurveKind::*;
        match self.kind {
            Fillet => self.fillet.get_state(hs),
            Chamfer => self.chamfer.get_state(hs),
            Dummy => None,
        }
    }
    pub fn set_state(&mut self, hs: HS, value: bool) {
        use VertexCurveKind::*;
        match self.kind {
            Fillet => self.fillet.set_state(hs, value),
            Chamfer => self.chamfer.set_state(hs, value),
            Dummy => (),
        }
    }
    pub fn set_hs_from_pos(&mut self, hs: HS, pointer: &mut Pointer) {
        use VertexCurveKind::*;
        let o_dist = self.get_dist_from_pos(pointer.pos());
        match self.kind {
            Dummy => (),
            Chamfer | Fillet => {
                if let Some(dist) = o_dist {
                    if dist < Self::GRAB_RADIUS {
                        self.set_state(hs, true);
                    } else {
                        self.set_state(hs, false);
                    }
                } else {
                    self.set_state(hs, false);
                }
            }
        }
    }
    pub fn get_dist_from_pos(&self, pos: Vec2) -> Option<f64> {
        use VertexCurveKind::*;
        match self.kind {
            Dummy => None,
            Chamfer => self.chamfer.get_dist_from_pos(pos).map(|(dist, _)| dist),
            Fillet => self.fillet.get_dist_from_pos(pos).map(|(dist, _)| dist),
        }
    }

    pub fn update_vars(&mut self, p_prev: Position, p: Position, p_next: Position) {
        self.chamfer.set_from_dihedron(p_prev, p, p_next);
        if let VertexCurveKind::Fillet = self.kind {
            self.fillet.set_from_dihedron(p_prev, p, p_next);
        }
        self.dummy = p;
    }
    pub fn get_paths_and_patterns(
        &self,
        das: &Size,
        parent_selected: bool,
        parent_highlighted: bool,
    ) -> (BezPath, Pattern) {
        use VertexCurveKind::*;
        match self.kind {
            Chamfer => {
                self.chamfer
                    .get_paths_and_patterns(das, parent_selected, parent_highlighted)
            }
            Fillet => self
                .fillet
                .get_paths_and_patterns(das, parent_selected, parent_highlighted),
            Dummy => (BezPath::new(), Pattern::BasicNormal),
        }
    }
    pub fn get_dimensions_paths_and_patterns(
        &self,
        das: &Size,
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        use VertexCurveKind::*;
        match self.kind {
            Chamfer => self.chamfer.get_dimensions_paths_and_patterns(das),
            Fillet => self.fillet.get_dimensions_paths_and_patterns(das),
            Dummy => vec![],
        }
    }
}

#[derive(Copy, Debug, Clone, PartialEq)]
pub struct Vertex {
    pos: Position,
    state: Status,
}
impl Vertex {
    const GRAB_RADIUS: f64 = 5.;

    pub fn new(pos: Vec2) -> Self {
        Self {
            pos: Position::new(pos),
            state: Status::default(),
        }
    }
    pub fn get_pos(&self) -> &Position {
        &self.pos
    }
    pub fn get_pos_mut(&mut self) -> &mut Position {
        &mut self.pos
    }
    pub fn set_pos(&mut self, pos: Vec2) {
        self.pos.pos = pos;
    }
    pub fn get_state(&self, hs: HS) -> Option<Vec2> {
        self.state.is_hs(hs).then(|| self.pos.pos)
    }
    pub fn set_state(&mut self, hs: HS, value: bool) {
        match hs {
            HS::Highlight => self.state.highlighted = value,
            HS::Select => self.state.selected = value,
        }
    }
    pub fn set_state_from_pos(&mut self, hs: HS, pointer: &mut Pointer) {
        let state = (pointer.pos() - self.pos.pos).hypot() < Self::GRAB_RADIUS;
        match hs {
            HS::Highlight => {
                self.state.highlighted = state;
                if self.state.highlighted {
                    pointer.set_pos(self.pos.pos);
                }
            }
            HS::Select => {
                self.state.selected = state;
                if self.state.selected {
                    pointer.set_pos(self.pos.pos);
                    pointer.save_pos();
                }
            }
        }
    }
    pub fn get_dist_from_pos(&self, pos: Vec2) -> f64 {
        (pos - self.pos.pos).hypot()
    }
    pub fn save_pos(&mut self) {
        self.pos.saved_pos = self.pos.pos;
    }
    pub fn restore_pos(&mut self) {
        self.pos.pos = self.pos.saved_pos;
    }
    pub fn save_vars(&mut self) {
        self.save_pos();
    }
    pub fn restore_vars(&mut self) {
        self.restore_pos();
    }

    pub fn move_pos(&mut self, dpos: Vec2) {
        self.pos.pos = self.pos.saved_pos + dpos;
    }

    pub fn get_pattern(&self) -> Pattern {
        use HS::*;
        match (
            self.get_state(Select).is_some(),
            self.get_state(Highlight).is_some(),
        ) {
            (false, false) => Pattern::BasicNormal,
            (false, true) => Pattern::BasicHighlighted,
            (true, false) => Pattern::BasicSelected,
            (true, true) => Pattern::BasicSelected,
        }
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
