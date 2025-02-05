use crate::curves::curve_arc::CurveArc;
use crate::curves::curve_line::CurveLine;
use crate::pools::HS;
use crate::positions::{Pointer, Position};
use crate::{
    canvas::{CanvasText, Pattern},
    KeysStates,
};
use kurbo::{ArcAppendIter, BezPath, CubicBezIter, LinePathIter, PathEl, QuadBezIter, Size, Vec2};

#[derive(Copy, Debug, Clone, PartialEq)]
pub enum Curve {
    CLine(CurveLine, CurveArc),
    CArc(CurveLine, CurveArc),
}
impl Curve {
    pub fn new_line() -> Self {
        Curve::CLine(CurveLine::default(), CurveArc::default())
    }
    pub fn new_arc() -> Self {
        Curve::CArc(CurveLine::default(), CurveArc::default())
    }
    pub fn get_line(&self) -> &CurveLine {
        use Curve::*;
        match self {
            CLine(l, _) | CArc(l, _) => l,
        }
    }
    pub fn get_line_mut(&mut self) -> &mut CurveLine {
        use Curve::*;
        match self {
            CLine(l, _) | CArc(l, _) => l,
        }
    }
    pub fn get_arc(&self) -> &CurveArc {
        use Curve::*;
        match self {
            CLine(_, a) | CArc(_, a) => a,
        }
    }
    pub fn get_arc_mut(&mut self) -> &mut CurveArc {
        use Curve::*;
        match self {
            CLine(_, a) | CArc(_, a) => a,
        }
    }
    pub fn get(&self) -> &Curve {
        self
    }
    pub fn get_mut(&mut self) -> &mut Curve {
        self
    }
    pub fn set(&mut self, curve: Curve) {
        *self = curve;
    }
    pub fn next(self) -> Curve {
        use Curve::*;
        match self {
            CLine(l, a) => CArc(l, a),
            CArc(l, a) => CLine(l, a),
        }
    }
    pub fn prev(self) -> Curve {
        use Curve::*;
        match self {
            CLine(l, a) => CArc(l, a),
            CArc(l, a) => CLine(l, a),
        }
    }

    pub fn get_pattern(&self, selected: bool, highlighted: bool) -> Pattern {
        use Curve::*;
        match self {
            CLine(l, _) => l.get_pattern(selected, highlighted),
            CArc(_, a) => a.get_pattern(selected, highlighted),
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

impl CurveControls for Curve {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 5.;

    fn toggle_prop(&mut self) {
        use Curve::*;
        match self {
            CLine(l, _) => l.toggle_prop(),
            CArc(_, a) => a.toggle_prop(),
        }
    }
    fn save_vars(&mut self) {
        use Curve::*;
        match self {
            CLine(l, a) | CArc(l, a) => {
                l.save_vars();
                a.save_vars();
            }
        }
    }
    fn restore_vars(&mut self) {
        use Curve::*;
        match self {
            CLine(l, _) => l.restore_vars(),
            CArc(_, a) => a.restore_vars(),
        }
    }
    fn set_from_start_end(&mut self, start: Position, end: Position) -> Option<Vec2> {
        use Curve::*;
        match self {
            CLine(l, _) => l.set_from_start_end(start, end),
            CArc(_, a) => a.set_from_start_end(start, end),
        }
    }
    fn set_from_dihedron(
        &mut self,
        p_prev: Position,
        p: Position,
        p_next: Position,
    ) -> Option<Vec2> {
        use Curve::*;
        match self {
            CLine(l, _) => l.set_from_dihedron(p_prev, p, p_next),
            CArc(_, a) => a.set_from_dihedron(p_prev, p, p_next),
        }
    }

    fn get_state(&self, hs: HS) -> Option<Vec2> {
        use Curve::*;
        match self {
            CLine(l, _) => l.get_state(hs),
            CArc(_, a) => a.get_state(hs),
        }
    }
    fn set_state(&mut self, hs: HS, state: bool) {
        use Curve::*;
        match self {
            CLine(l, _) => l.set_state(hs, state),
            CArc(_, a) => a.set_state(hs, state),
        }
    }
    fn get_dist_from_pos(&self, pointer_pos: Vec2) -> Option<(f64, Vec2)> {
        use Curve::*;
        match self {
            CLine(l, _) => l.get_dist_from_pos(pointer_pos),
            CArc(_, a) => a.get_dist_from_pos(pointer_pos),
        }
    }

    fn move_control_selected(
        &mut self,
        start: Vec2,
        end: Vec2,
        pointer: &Pointer,
        keys_states: KeysStates,
    ) -> bool {
        use Curve::*;
        match self {
            CLine(l, _) => l.move_control_selected(start, end, pointer, keys_states),
            CArc(_, a) => a.move_control_selected(start, end, pointer, keys_states),
        }
    }
    fn path_elements(&self) -> PrimitiveKindIter {
        use Curve::*;
        match self {
            CLine(l, _) => l.path_elements(),
            CArc(_, a) => a.path_elements(),
        }
    }
    fn get_paths_and_patterns(
        &self,
        das: &Size,
        parent_selected: bool,
        parent_highlighted: bool,
    ) -> (BezPath, Pattern) {
        use Curve::*;
        match self {
            CLine(l, _) => l.get_paths_and_patterns(das, parent_selected, parent_highlighted),
            CArc(_, a) => a.get_paths_and_patterns(das, parent_selected, parent_highlighted),
        }
    }
    fn get_dimensions_paths_and_patterns(&self, das: &Size) -> Vec<(BezPath, Pattern, CanvasText)> {
        use Curve::*;
        match self {
            CLine(l, _) => l.get_dimensions_paths_and_patterns(das),
            CArc(_, a) => a.get_dimensions_paths_and_patterns(das),
        }
    }
}

pub trait CurveControls {
    const TOLERANCE: f64;
    const GRAB: f64;

    fn toggle_prop(&mut self);
    fn save_vars(&mut self);
    fn restore_vars(&mut self);
    fn set_from_start_end(&mut self, start: Position, end: Position) -> Option<Vec2>;
    fn set_from_dihedron(
        &mut self,
        p_prev: Position,
        p: Position,
        p_next: Position,
    ) -> Option<Vec2>;

    fn get_state(&self, hs: HS) -> Option<Vec2>;
    fn set_state(&mut self, hs: HS, state: bool);
    fn get_dist_from_pos(&self, pointer_pos: Vec2) -> Option<(f64, Vec2)>;

    fn move_control_selected(
        &mut self,
        start: Vec2,
        end: Vec2,
        pointer: &Pointer,
        keys_states: KeysStates,
    ) -> bool;

    fn path_elements(&self) -> PrimitiveKindIter;
    fn get_pattern(&self, selected: bool, highlighted: bool) -> Pattern {
        match (selected, highlighted) {
            (false, false) => Pattern::BasicNormal,
            (false, true) => Pattern::BasicHighlighted,
            (true, false) => Pattern::BasicSelected,
            (true, true) => Pattern::BasicSelected,
        }
    }
    fn get_paths_and_patterns(
        &self,
        das: &Size,
        parent_selected: bool,
        parent_highlighted: bool,
    ) -> (BezPath, Pattern);
    fn get_dimensions_paths_and_patterns(&self, das: &Size) -> Vec<(BezPath, Pattern, CanvasText)>;
}
