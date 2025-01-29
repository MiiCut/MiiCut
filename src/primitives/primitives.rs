use crate::positions::{Pointer, Position, Value, ValueBool};
use crate::primitives::prim_arc::PrimArc;
use crate::primitives::prim_line::PrimLine;
use crate::{
    canvas::{CanvasText, Pattern},
    pools::HS,
    GetEntityState, KeysStates, SetEntityState, SetEntityStateFromPos,
};
use kurbo::{ArcAppendIter, BezPath, CubicBezIter, LinePathIter, PathEl, QuadBezIter, Size, Vec2};

#[derive(Copy, Debug, Clone)]
pub enum VertexProperty {
    RectangleLike,
    Nope,
}
#[derive(Copy, Debug, Clone)]
pub enum VertexModifier {
    // Fillet radius or Chamfer distance, fillet concavity
    Chamfer(Value, ValueBool),
    Fillet(Value, ValueBool),
    Nope(Value, ValueBool),
}
impl VertexModifier {
    pub fn toogle(&self) -> Self {
        use VertexModifier::*;
        match self {
            Chamfer(radius, concavity) => Fillet(*radius, *concavity),
            Fillet(radius, concavity) => Nope(*radius, *concavity),
            Nope(radius, concavity) => Chamfer(*radius, *concavity),
        }
    }
}

#[derive(Copy, Debug, Clone)]
pub struct Vertex {
    pos: Position,
    property: VertexProperty,
    modifier: VertexModifier,
}
impl Vertex {
    const MIN_OFFSET: f64 = 10.;
    pub fn new(pos: Vec2, property: VertexProperty) -> Self {
        Self {
            pos: Position::new(pos, true),
            property,
            modifier: VertexModifier::Nope(Value::new(Self::MIN_OFFSET), ValueBool::new(false)),
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
    pub fn save_pos(&mut self) {
        self.pos.saved_pos = self.pos.pos;
    }
    pub fn restore_pos(&mut self) {
        self.pos.pos = self.pos.saved_pos;
    }
    pub fn save_vars(&mut self) {
        self.save_pos();
        self.save_modifier();
    }
    pub fn restore_vars(&mut self) {
        self.restore_pos();
        self.restore_modifier();
    }

    pub fn get_property(&self) -> VertexProperty {
        self.property
    }
    pub fn set_property(&mut self, property: VertexProperty) {
        self.property = property;
    }
    pub fn get_modifier(&self) -> VertexModifier {
        self.modifier
    }
    pub fn set_modifier(&mut self, modifier: VertexModifier) {
        self.modifier = modifier;
    }
    pub fn save_modifier(&mut self) {
        use VertexModifier::*;
        match &mut self.modifier {
            Nope(offset, concavity) => {
                offset.saved_val = offset.value;
                concavity.saved_val = concavity.value;
            }
            Chamfer(offset, concavity) => {
                offset.saved_val = offset.value;
                concavity.saved_val = concavity.value;
            }
            Fillet(offset, concavity) => {
                offset.saved_val = offset.value;
                concavity.saved_val = concavity.value;
            }
        }
    }
    pub fn restore_modifier(&mut self) {
        use VertexModifier::*;
        match &mut self.modifier {
            Nope(offset, concavity) => {
                offset.value = offset.saved_val;
                concavity.value = concavity.saved_val;
            }
            Chamfer(offset, concavity) => {
                offset.value = offset.saved_val;
                concavity.value = concavity.saved_val;
            }
            Fillet(offset, concavity) => {
                offset.value = offset.saved_val;
                concavity.value = concavity.saved_val;
            }
        }
    }

    pub fn toogle_modifier(&mut self) {
        self.modifier = self.modifier.toogle();
    }

    pub fn get_modifier_offset(&self) -> f64 {
        use VertexModifier::*;
        match self.modifier {
            Nope(offset, _) | Chamfer(offset, _) | Fillet(offset, _) => offset.value,
        }
    }
    pub fn set_modifier_offset(&mut self, offset: f64) {
        use VertexModifier::*;
        match self.modifier {
            Nope(_, concavity) => self.modifier = Nope(Value::new(offset), concavity),
            Chamfer(_, concavity) => self.modifier = Chamfer(Value::new(offset), concavity),
            Fillet(_, concavity) => self.modifier = Fillet(Value::new(offset), concavity),
        }
    }
    pub fn get_modifier_offset_saved(&self) -> f64 {
        use VertexModifier::*;
        match self.modifier {
            Nope(offset, _) | Chamfer(offset, _) | Fillet(offset, _) => offset.saved_val,
        }
    }
    pub fn move_pos(&mut self, pointer: &Pointer) -> bool {
        let dpos = pointer.dpos();
        self.pos.pos = self.pos.saved_pos + dpos;
        true
    }

    pub fn get_pattern(&self) -> Pattern {
        use HS::*;
        match (self.pos.is_hs(Select), self.pos.is_hs(Highlight)) {
            (false, false) => Pattern::Modifiers,
            (false, true) => Pattern::ModifiersHighlighted,
            (true, false) => Pattern::ModifiersSelected,
            (true, true) => Pattern::ModifiersSelected,
        }
    }
}

#[derive(Debug, Clone)]
pub enum PrimitiveCurve {
    CurveLine,
    CurveArc,
}
impl PrimitiveCurve {
    pub fn next_curve(&self) -> PrimitiveCurve {
        use PrimitiveCurve::*;
        match self {
            CurveLine => CurveArc,
            CurveArc => CurveLine,
        }
    }
    pub fn prev_curve(&self) -> PrimitiveCurve {
        use PrimitiveCurve::*;
        match self {
            CurveLine => CurveArc,
            CurveArc => CurveLine,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Primitive {
    curve: PrimitiveCurve,
    start: usize,
    end: usize,
    // Primitive curves (not on an enum because of memory retention)
    // when user switches between curves, the previous values are kept
    // Line
    p_line: PrimLine,
    // Arc
    p_arc: PrimArc,
}

impl Primitive {
    const _TOLERANCE: f64 = 0.01;

    pub fn new(curve: PrimitiveCurve, start: usize, end: usize) -> Self {
        Self {
            curve,
            start,
            end,
            p_line: PrimLine::new(),
            p_arc: PrimArc::new(),
        }
    }
    pub fn get_start(&self) -> i64 {
        self.start as i64
    }
    pub fn get_end(&self) -> i64 {
        self.end as i64
    }
    pub fn get_curve(&self) -> &PrimitiveCurve {
        &self.curve
    }
    pub fn get_curve_mut(&mut self) -> &mut PrimitiveCurve {
        &mut self.curve
    }
    pub fn set_curve(&mut self, curve: PrimitiveCurve) {
        self.curve = curve;
    }
    pub fn prev_curve(&mut self) {
        self.curve = self.curve.prev_curve();
    }
    pub fn next_curve(&mut self) {
        self.curve = self.curve.next_curve();
    }

    // pub fn get_line(&self) -> &PrimLine {
    //     &self.p_line
    // }
    // pub fn get_line_mut(&mut self) -> &mut PrimLine {
    //     &mut self.p_line
    // }
    // pub fn get_arc(&self) -> &PrimArc {
    //     &self.p_arc
    // }
    // pub fn get_arc_mut(&mut self) -> &mut PrimArc {
    //     &mut self.p_arc
    // }

    pub fn get_vars(&self) -> Primitive {
        Primitive {
            curve: self.curve.clone(),
            start: self.start,
            end: self.end,
            p_line: self.p_line.clone(),
            p_arc: self.p_arc.clone(),
        }
    }
    pub fn set_vars(&mut self, vars: &Primitive) {
        self.curve = vars.curve.clone();
        self.start = vars.start;
        self.end = vars.end;
        // self.p_line = vars.p_line.clone();
        // self.p_arc = vars.p_arc.clone();
    }
    pub fn save_vars(&mut self) {
        use PrimitiveCurve::*;
        match self.curve {
            CurveLine => self.p_line.save_vars(),
            CurveArc => self.p_arc.save_vars(),
        }
    }

    pub fn get_pattern(&self, selected: bool, highlighted: bool) -> Pattern {
        use PrimitiveCurve::*;
        match self.curve {
            CurveLine => self.p_line.get_pattern(selected, highlighted),
            CurveArc => self.p_arc.get_pattern(selected, highlighted),
        }
    }
}
pub enum PrimitiveKindIter {
    Line(LinePathIter),
    Arc(std::iter::Chain<std::iter::Once<PathEl>, ArcAppendIter>),
    QBez(QuadBezIter),
    QBezSmooth(QuadBezIter),
    CBez(CubicBezIter),
    CBezSmooth(CubicBezIter),
}
impl Iterator for PrimitiveKindIter {
    type Item = PathEl;
    fn next(&mut self) -> Option<Self::Item> {
        use PrimitiveKindIter::*;
        match self {
            Line(sh) => sh.next(),
            Arc(sh) => sh.next(),
            QBez(sh) => sh.next(),
            QBezSmooth(sh) => sh.next(),
            CBez(sh) => sh.next(),
            CBezSmooth(sh) => sh.next(),
        }
    }
}

impl PrimitiveControls for Primitive {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 5.;

    fn toggle_prop(&mut self) {
        use PrimitiveCurve::*;
        match self.curve {
            CurveLine => self.p_line.toggle_prop(),
            CurveArc => self.p_arc.toggle_prop(),
        }
    }
    fn save_vars(&mut self) {
        use PrimitiveCurve::*;
        match self.curve {
            CurveLine => self.p_line.save_vars(),
            CurveArc => self.p_arc.save_vars(),
        }
    }
    fn restore_vars(&mut self) {
        use PrimitiveCurve::*;
        match self.curve {
            CurveLine => self.p_line.restore_vars(),
            CurveArc => self.p_arc.restore_vars(),
        }
    }
    fn update_primitives_vars(&mut self, start: Position, end: Position) -> Vec2 {
        use PrimitiveCurve::*;
        match self.curve {
            CurveLine => self.p_line.update_primitives_vars(start, end),
            CurveArc => self.p_arc.update_primitives_vars(start, end),
        }
    }
    fn get_state(&self, start: Vec2, end: Vec2, state: GetEntityState) -> Option<Vec2> {
        use PrimitiveCurve::*;
        match self.curve {
            CurveLine => self.p_line.get_state(start, end, state),
            CurveArc => self.p_arc.get_state(start, end, state),
        }
    }
    fn set_state(&mut self, start: Vec2, end: Vec2, state: SetEntityState) {
        use PrimitiveCurve::*;
        match self.curve {
            CurveLine => self.p_line.set_state(start, end, state),
            CurveArc => self.p_arc.set_state(start, end, state),
        }
    }
    fn set_state_from_pos(
        &mut self,
        start: Vec2,
        end: Vec2,
        pointer: &mut Pointer,
        state: SetEntityStateFromPos,
    ) {
        use PrimitiveCurve::*;
        match self.curve {
            CurveLine => self.p_line.set_state_from_pos(start, end, pointer, state),
            CurveArc => self.p_arc.set_state_from_pos(start, end, pointer, state),
        }
    }
    fn move_control_selected(
        &mut self,
        start: Vec2,
        end: Vec2,
        pointer: &Pointer,
        keys_states: KeysStates,
    ) -> bool {
        use PrimitiveCurve::*;
        match self.curve {
            CurveLine => self
                .p_line
                .move_control_selected(start, end, pointer, keys_states),
            CurveArc => self
                .p_arc
                .move_control_selected(start, end, pointer, keys_states),
        }
    }
    fn get_all_controls_positions(&self, start: Vec2, end: Vec2) -> Vec<Vec2> {
        use PrimitiveCurve::*;
        match self.curve {
            CurveLine => self.p_line.get_all_controls_positions(start, end),
            CurveArc => self.p_arc.get_all_controls_positions(start, end),
        }
    }
    fn path_elements(&self, start: Vec2, end: Vec2) -> PrimitiveKindIter {
        use PrimitiveCurve::*;
        match self.curve {
            CurveLine => self.p_line.path_elements(start, end),
            CurveArc => self.p_arc.path_elements(start, end),
        }
    }
    fn get_paths_and_patterns(&self, start: Vec2, end: Vec2, das: &Size) -> (BezPath, Pattern) {
        use PrimitiveCurve::*;
        match self.curve {
            CurveLine => self.p_line.get_paths_and_patterns(start, end, das),
            CurveArc => self.p_arc.get_paths_and_patterns(start, end, das),
        }
    }
    fn get_mod_paths_and_patterns(
        &self,
        start: Vec2,
        end: Vec2,
        das: &Size,
    ) -> Vec<(BezPath, Pattern)> {
        use PrimitiveCurve::*;
        match self.curve {
            CurveLine => vec![self.p_line.get_paths_and_patterns(start, end, das)],
            CurveArc => self.p_arc.get_mod_paths_and_patterns(start, end, das),
        }
    }
    fn get_dimensions_paths_and_patterns(
        &self,
        start: Vec2,
        end: Vec2,
        das: &Size,
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        use PrimitiveCurve::*;
        match self.curve {
            CurveLine => self
                .p_line
                .get_dimensions_paths_and_patterns(start, end, das),
            CurveArc => self
                .p_arc
                .get_dimensions_paths_and_patterns(start, end, das),
        }
    }
}

pub trait PrimitiveControls {
    const TOLERANCE: f64;
    const GRAB: f64;

    fn toggle_prop(&mut self);
    fn save_vars(&mut self);
    fn restore_vars(&mut self);
    fn update_primitives_vars(&mut self, start: Position, end: Position) -> Vec2;
    fn get_state(&self, start: Vec2, end: Vec2, state: GetEntityState) -> Option<Vec2>;
    fn set_state(&mut self, start: Vec2, end: Vec2, state: SetEntityState);
    fn set_state_from_pos(
        &mut self,
        start: Vec2,
        end: Vec2,
        pointer: &mut Pointer,
        state: SetEntityStateFromPos,
    );
    fn move_control_selected(
        &mut self,
        start: Vec2,
        end: Vec2,
        pointer: &Pointer,
        keys_states: KeysStates,
    ) -> bool;
    fn get_all_controls_positions(&self, start: Vec2, end: Vec2) -> Vec<Vec2>;

    fn path_elements(&self, start: Vec2, end: Vec2) -> PrimitiveKindIter;
    fn get_pattern(&self, selected: bool, highlighted: bool) -> Pattern {
        match (selected, highlighted) {
            (false, false) => Pattern::BasicNormal,
            (false, true) => Pattern::BasicHighlighted,
            (true, false) => Pattern::BasicSelected,
            (true, true) => Pattern::BasicSelected,
        }
    }
    fn get_mod_pattern(&self, selected: bool, highlighted: bool) -> Pattern {
        match (selected, highlighted) {
            (false, false) => Pattern::Modifiers,
            (false, true) => Pattern::ModifiersHighlighted,
            (true, false) => Pattern::ModifiersSelected,
            (true, true) => Pattern::ModifiersSelected,
        }
    }
    fn get_paths_and_patterns(&self, start: Vec2, end: Vec2, das: &Size) -> (BezPath, Pattern);
    fn get_mod_paths_and_patterns(
        &self,
        start: Vec2,
        end: Vec2,
        das: &Size,
    ) -> Vec<(BezPath, Pattern)>;
    fn get_dimensions_paths_and_patterns(
        &self,
        start: Vec2,
        end: Vec2,
        das: &Size,
    ) -> Vec<(BezPath, Pattern, CanvasText)>;
}
