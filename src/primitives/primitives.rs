use super::{
    prim_arc::PrimArc,
    // prim_cbez::PrimCBez,
    // prim_cbez_smooth::PrimCBezSmooth,
    prim_line::PrimLine,
    // prim_qbez::PrimQBez,
    // prim_qbez_smooth::PrimQBezSmooth,
};
use crate::{
    canvas::{CanvasText, Pattern},
    find_circle_center,
    positions::*,
    prefab::*,
    KeysStates,
};
use kurbo::{ArcAppendIter, BezPath, CubicBezIter, LinePathIter, PathEl, QuadBezIter, Size, Vec2};

#[derive(Copy, Clone, Debug)]
pub enum VertexChange {
    StartChanged,
    EndChanged,
    Nope,
}

#[derive(Copy, Debug, Clone)]
pub enum StartProps {
    PrevForceXNextForceY,
    PrevForceYNextForceX,
    Nope,
}

#[derive(Copy, Debug, Clone)]
pub enum StartModifier {
    Chamfer(bool),
    Fillet(bool),
    Nope(bool),
}
impl StartModifier {
    pub fn toogle(&self) -> Self {
        use StartModifier::*;
        match self {
            Chamfer(value) => Fillet(*value),
            Fillet(value) => Nope(*value),
            Nope(value) => Chamfer(*value),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Primitive {
    kind: PrivitiveKind,
    start: Position,
    end: Position,
    // For start position only
    start_props: StartProps,
    // For start position only
    // This will be applied also to the end position of the previous Prim
    // in the custom shape
    start_modifier: StartModifier,
    start_modifier_offset: Value,

    // Primitives
    // Line
    p_line: PrimLine,
    //Arc
    p_arc: PrimArc,
}

impl Primitive {
    const MIN_START_MOD_OFFSET: f64 = 10.;
    pub fn new(kind: PrivitiveKind, start: Vec2, end: Vec2) -> Self {
        let start = Position::new(start, true);
        let end = Position::new(end, true);
        Self {
            kind,
            start,
            end,
            start_props: StartProps::Nope,
            start_modifier: StartModifier::Nope(false),
            start_modifier_offset: Value::new(Self::MIN_START_MOD_OFFSET),
            p_line: PrimLine::new(),
            p_arc: PrimArc::new(),
        }
    }

    pub fn get_start_position(&self) -> Vec2 {
        self.start.pos
    }
    pub fn get_start_saved_position(&self) -> Vec2 {
        self.start.saved_pos
    }
    pub fn set_start_position(&mut self, start: Vec2) {
        self.start.pos = start;
        self.update_primitives_vars(VertexChange::StartChanged);
    }

    pub fn toogle_start_modifier(&mut self) {
        self.start_modifier = self.start_modifier.toogle();
    }
    pub fn get_start_modifier_offset(&self) -> f64 {
        self.start_modifier_offset.value
    }
    pub fn set_start_modifier_offset(&mut self, offset: f64) {
        self.start_modifier_offset.value = offset;
    }
    pub fn get_start_modifier_offset_saved(&self) -> f64 {
        self.start_modifier_offset.saved_val
    }

    pub fn get_start_modifier(&self) -> StartModifier {
        self.start_modifier
    }
    pub fn set_start_modifier(&mut self, modifier: StartModifier) {
        self.start_modifier = modifier;
    }

    pub fn is_start_selected(&self) -> bool {
        self.start.selected
    }
    pub fn is_start_highlighted(&self) -> bool {
        self.start.highlighted
    }

    pub fn get_end_position(&self) -> Vec2 {
        self.end.pos
    }
    pub fn get_end_saved_position(&self) -> Vec2 {
        self.end.saved_pos
    }
    pub fn set_end_position(&mut self, end: Vec2) {
        self.end.pos = end;
        self.update_primitives_vars(VertexChange::EndChanged);
    }

    pub fn move_position(&mut self, pointer: &mut Pointer) -> bool {
        let dpos = pointer.dpos();
        self.start.pos = self.start.saved_pos + dpos;
        self.end.pos = self.end.saved_pos + dpos;
        true
    }

    pub fn get_prim_kind(&self) -> &PrivitiveKind {
        &self.kind
    }
    pub fn set_prim_kind_next(&mut self) -> Vec2 {
        self.kind = self.kind.next_kind();
        use PrivitiveKind::*;
        match self.kind {
            PrimLine => (self.start.pos + self.end.pos) / 2.,
            PrimArc => {
                self.p_arc.validate_radius(self.start.pos, self.end.pos);
                find_circle_center(
                    self.start.pos,
                    self.end.pos,
                    self.p_arc.get_radius(),
                    self.p_arc.get_concavity(),
                )
            }
        }
    }
    pub fn set_prim_kind_prev(&mut self) -> Vec2 {
        self.kind = self.kind.prev_kind();
        use PrivitiveKind::*;
        match self.kind {
            PrimLine => (self.start.pos + self.end.pos) / 2.,
            PrimArc => {
                self.p_arc.validate_radius(self.start.pos, self.end.pos);
                find_circle_center(
                    self.start.pos,
                    self.end.pos,
                    self.p_arc.get_radius(),
                    self.p_arc.get_concavity(),
                )
            }
        }
    }
    pub fn set_prim_kind(&mut self, kind: PrivitiveKind) {
        self.kind = kind;
    }

    pub fn get_line(&self) -> &PrimLine {
        &self.p_line
    }
    pub fn get_arc(&self) -> &PrimArc {
        &self.p_arc
    }

    pub fn get_vars(&self) -> Primitive {
        Primitive {
            kind: self.kind.clone(),
            start: self.start.clone(),
            end: self.end.clone(),
            start_props: self.start_props,
            start_modifier: self.start_modifier.clone(),
            start_modifier_offset: self.start_modifier_offset,
            p_line: self.p_line.clone(),
            p_arc: self.p_arc.clone(),
        }
    }
    pub fn set_vars(&mut self, vars: &Primitive) {
        self.kind = vars.kind.clone();
        self.start = vars.start.clone();
        self.end = vars.end.clone();
        self.start_props = vars.start_props;
        self.start_modifier = vars.start_modifier.clone();
        self.start_modifier_offset = vars.start_modifier_offset;
        self.p_line = vars.p_line.clone();
        self.p_arc = vars.p_arc.clone();
    }

    pub fn get_pattern(&self, selected: bool, highlighted: bool) -> Pattern {
        use PrivitiveKind::*;
        match self.kind {
            PrimLine => self.p_line.get_pattern(selected, highlighted),
            PrimArc => self.p_arc.get_pattern(selected, highlighted),
        }
    }
    pub fn get_paths_and_patterns(&self, das: &Size) -> (BezPath, Pattern) {
        use PrivitiveKind::*;
        match self.kind {
            PrimLine => self
                .p_line
                .get_paths_and_patterns(self.start.pos, self.end.pos, das),
            PrimArc => self
                .p_arc
                .get_paths_and_patterns(self.start.pos, self.end.pos, das),
        }
    }
    pub fn get_dimensions_paths_and_patterns(
        &self,
        das: &Size,
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        use PrivitiveKind::*;
        match self.kind {
            PrimLine => {
                self.p_line
                    .get_dimensions_paths_and_patterns(self.start.pos, self.end.pos, das)
            }
            PrimArc => {
                self.p_arc
                    .get_dimensions_paths_and_patterns(self.start.pos, self.end.pos, das)
            }
        }
    }
    pub fn get_mod_paths_and_patterns(&self, das: &Size) -> Vec<(BezPath, Pattern)> {
        let mut paths_patterns: Vec<(BezPath, Pattern)> = vec![];
        paths_patterns.push((
            modifiers_path(self.start.pos, 1., Self::GRAB),
            match (self.start.selected, self.start.highlighted) {
                (false, false) => Pattern::Modifiers,
                (false, true) => Pattern::ModifiersHighlighted,
                (true, false) => Pattern::ModifiersSelected,
                (true, true) => Pattern::ModifiersSelected,
            },
        ));
        use PrivitiveKind::*;
        paths_patterns.extend(match self.kind {
            PrimLine => self
                .p_line
                .get_mod_paths_and_patterns(self.start.pos, self.end.pos, das),
            PrimArc => self
                .p_arc
                .get_mod_paths_and_patterns(self.start.pos, self.end.pos, das),
        });
        paths_patterns
    }
}

impl Primitive {
    const _TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 5.;

    pub fn toogle(&mut self) -> Vec2 {
        use PrivitiveKind::*;
        match self.kind {
            PrimLine => {
                self.p_line.toggle();
                self.p_line
                    .update_primitives_vars(self.start.pos, self.end.pos, VertexChange::Nope)
            }
            PrimArc => {
                self.p_arc.toggle();
                self.p_arc
                    .update_primitives_vars(self.start.pos, self.end.pos, VertexChange::Nope)
            }
        }
    }
    pub fn save_vars(&mut self) {
        self.start.saved_pos = self.start.pos;
        self.end.saved_pos = self.end.pos;
        self.start_modifier_offset.saved_val = self.start_modifier_offset.value;
        use PrivitiveKind::*;
        match self.kind {
            PrimLine => self.p_line.save_vars(),
            PrimArc => self.p_arc.save_vars(),
        }
    }
    pub fn restore_saved(&mut self) {
        self.start.pos = self.start.saved_pos;
        self.end.pos = self.end.saved_pos;
        use PrivitiveKind::*;
        match self.kind {
            PrimLine => self.p_line.restore_saved(),
            PrimArc => self.p_arc.restore_saved(),
        }
    }
    pub fn update_primitives_vars(&mut self, changed: VertexChange) -> Vec2 {
        use PrivitiveKind::*;
        match self.kind {
            PrimLine => self
                .p_line
                .update_primitives_vars(self.start.pos, self.end.pos, changed),
            PrimArc => self
                .p_arc
                .update_primitives_vars(self.start.pos, self.end.pos, changed),
        }
    }
    pub fn get_state(&self, get: GetPrimitiveState) -> Option<Vec2> {
        use GetPrimitiveState::*;
        use PrivitiveKind::*;
        let start = self.start.pos;
        let end = self.end.pos;
        match get {
            IsSelected => match self.kind {
                PrimLine => self.p_line.get_state(start, end, IsSelected),
                PrimArc => self.p_arc.get_state(start, end, IsSelected),
            },
            IsHighligh => match self.kind {
                PrimLine => self.p_line.get_state(start, end, IsHighligh),
                PrimArc => self.p_arc.get_state(start, end, IsHighligh),
            },
            IsStartSelected => return self.start.selected.then_some(self.start.pos),
            IsStartHighligh => return self.start.highlighted.then_some(self.start.pos),

            IsOtherModifiersSelected => {
                use PrivitiveKind::*;
                match self.kind {
                    PrimLine => self.p_line.get_state(
                        self.start.pos,
                        self.end.pos,
                        IsOtherModifiersSelected,
                    ),
                    PrimArc => {
                        self.p_arc
                            .get_state(self.start.pos, self.end.pos, IsOtherModifiersSelected)
                    }
                }
            }
            IsOtherModifiersHighligh => {
                use PrivitiveKind::*;
                match self.kind {
                    PrimLine => self.p_line.get_state(
                        self.start.pos,
                        self.end.pos,
                        IsOtherModifiersHighligh,
                    ),
                    PrimArc => {
                        self.p_arc
                            .get_state(self.start.pos, self.end.pos, IsOtherModifiersHighligh)
                    }
                }
            }
        }
    }
    pub fn set_state(&mut self, set: SetPrimitiveState) {
        use PrivitiveKind::*;
        use SetPrimitiveState::*;
        let start = self.start.pos;
        let end = self.end.pos;
        match set {
            SetSelect(value) => match self.kind {
                PrimLine => self.p_line.set_state(start, end, SetSelect(value)),
                PrimArc => self.p_arc.set_state(start, end, SetSelect(value)),
            },
            SetHighli(value) => match self.kind {
                PrimLine => self.p_line.set_state(start, end, SetHighli(value)),
                PrimArc => self.p_arc.set_state(start, end, SetHighli(value)),
            },
            SetStartSelected(value) => {
                self.start.selected = value;
            }
            SetStartHighligh(value) => {
                self.start.highlighted = value;
            }
            SelectAllOtherModifiers(value) => {
                // self.end.selected = value;
                use PrivitiveKind::*;
                match self.kind {
                    PrimLine => self
                        .p_line
                        .set_state(start, end, SelectAllOtherModifiers(value)),
                    PrimArc => self
                        .p_arc
                        .set_state(start, end, SelectAllOtherModifiers(value)),
                }
            }
            HighliAllOtherModifiers(value) => {
                self.start.highlighted = value;
                // self.end.highlighted = value;
                use PrivitiveKind::*;
                match self.kind {
                    PrimLine => self
                        .p_line
                        .set_state(start, end, HighliAllOtherModifiers(value)),
                    PrimArc => self
                        .p_arc
                        .set_state(start, end, HighliAllOtherModifiers(value)),
                }
            }
        }
    }
    pub fn set_state_from_pos(&mut self, pointer: &mut Pointer, set: SetPrimitiveStateFromPos) {
        use PrivitiveKind::*;
        use SetPrimitiveStateFromPos::*;
        let start = self.start.pos;
        let end = self.end.pos;
        match set {
            SelectFromPos => match self.kind {
                PrimLine => self
                    .p_line
                    .set_state_from_pos(start, end, pointer, SelectFromPos),
                PrimArc => self
                    .p_arc
                    .set_state_from_pos(start, end, pointer, SelectFromPos),
            },
            HighliFromPos => match self.kind {
                PrimLine => self
                    .p_line
                    .set_state_from_pos(start, end, pointer, HighliFromPos),
                PrimArc => self
                    .p_arc
                    .set_state_from_pos(start, end, pointer, HighliFromPos),
            },
            SelectStartFromPos => {
                if (self.start.pos - pointer.pos()).hypot() < 2. * Self::GRAB {
                    self.start.selected = true;
                    pointer.set_pos(self.start.pos);
                    pointer.save_pos();
                } else {
                    self.start.selected = false;
                }
            }
            SelectOtherModifierFromPos => {
                use PrivitiveKind::*;
                match self.kind {
                    PrimLine => self.p_line.set_state_from_pos(
                        start,
                        end,
                        pointer,
                        SelectOtherModifierFromPos,
                    ),
                    PrimArc => self.p_arc.set_state_from_pos(
                        start,
                        end,
                        pointer,
                        SelectOtherModifierFromPos,
                    ),
                }
            }
            HighliStartFromPos => {
                if (self.start.pos - pointer.pos()).hypot() < 2. * Self::GRAB {
                    self.start.highlighted = true;
                    pointer.set_pos(self.start.pos);
                } else {
                    self.start.highlighted = false;
                }
            }
            HighliOtherModifierFromPos => {
                use PrivitiveKind::*;
                match self.kind {
                    PrimLine => self.p_line.set_state_from_pos(
                        start,
                        end,
                        pointer,
                        HighliOtherModifierFromPos,
                    ),
                    PrimArc => self.p_arc.set_state_from_pos(
                        start,
                        end,
                        pointer,
                        HighliOtherModifierFromPos,
                    ),
                }
            }
        }
    }
    pub fn move_control_selected(&mut self, pointer: &Pointer, keys_states: KeysStates) -> bool {
        use PrivitiveKind::*;
        match self.kind {
            PrimLine => self.p_line.move_control_selected(
                self.start.pos,
                self.end.pos,
                pointer,
                keys_states,
            ),
            PrimArc => {
                self.p_arc
                    .move_control_selected(self.start.pos, self.end.pos, pointer, keys_states)
            }
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

#[derive(Debug, Clone)]
pub enum PrivitiveKind {
    PrimLine,
    PrimArc,
}
impl PrivitiveKind {
    pub fn next_kind(&self) -> PrivitiveKind {
        use PrivitiveKind::*;
        match self {
            PrimLine => PrimArc,
            PrimArc => PrimLine,
        }
    }
    pub fn prev_kind(&self) -> PrivitiveKind {
        use PrivitiveKind::*;
        match self {
            PrimLine => PrimArc,
            PrimArc => PrimLine,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GetPrimitiveState {
    IsSelected,
    IsHighligh,
    IsStartSelected,
    IsStartHighligh,
    IsOtherModifiersSelected,
    IsOtherModifiersHighligh,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SetPrimitiveState {
    SetSelect(bool),
    SetHighli(bool),
    SetStartSelected(bool),
    SetStartHighligh(bool),
    SelectAllOtherModifiers(bool),
    HighliAllOtherModifiers(bool),
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SetPrimitiveStateFromPos {
    SelectFromPos,
    HighliFromPos,
    SelectStartFromPos,
    HighliStartFromPos,
    SelectOtherModifierFromPos,
    HighliOtherModifierFromPos,
}

pub trait PrimitiveControls {
    const TOLERANCE: f64;
    const GRAB: f64;

    fn toggle(&mut self);
    fn save_vars(&mut self);
    fn restore_saved(&mut self);
    fn update_primitives_vars(&mut self, start: Vec2, end: Vec2, changed: VertexChange) -> Vec2;
    fn is_selected(&self) -> bool;
    fn is_highlighted(&self) -> bool;
    fn get_state(&self, start: Vec2, end: Vec2, state: GetPrimitiveState) -> Option<Vec2>;
    fn set_state(&mut self, start: Vec2, end: Vec2, state: SetPrimitiveState);
    fn set_state_from_pos(
        &mut self,
        start: Vec2,
        end: Vec2,
        pointer: &mut Pointer,
        state: SetPrimitiveStateFromPos,
    );
    fn move_control_selected(
        &mut self,
        start: Vec2,
        end: Vec2,
        pointer: &Pointer,
        keys_states: KeysStates,
    ) -> bool;
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
