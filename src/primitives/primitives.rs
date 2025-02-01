use crate::pools::HS;
use crate::positions::{Pointer, Position};
use crate::primitives::prim_arc::PrimArc;
use crate::primitives::prim_line::PrimLine;
use crate::{
    canvas::{CanvasText, Pattern},
    KeysStates,
};
use kurbo::{ArcAppendIter, BezPath, CubicBezIter, LinePathIter, PathEl, QuadBezIter, Size, Vec2};

#[derive(Debug, Copy, Clone)]
pub enum ControlPoint {
    LineCtrl,
    ArcCtrl,
}
#[derive(Copy, Debug, Clone)]
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

#[derive(Copy, Debug, Clone)]
pub struct Primitive {
    curve: PrimitiveCurve,

    // Primitive curves (not on an enum because of memory retention)
    // when user switches between curves, the previous values are kept
    // Line
    p_line: PrimLine,
    // Arc
    p_arc: PrimArc,
}

impl Primitive {
    const _TOLERANCE: f64 = 0.01;

    pub fn new(curve: PrimitiveCurve) -> Self {
        Self {
            curve,
            p_line: PrimLine::new(),
            p_arc: PrimArc::new(),
        }
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

    pub fn get_line(&self) -> &PrimLine {
        &self.p_line
    }
    pub fn get_line_mut(&mut self) -> &mut PrimLine {
        &mut self.p_line
    }
    pub fn get_arc(&self) -> &PrimArc {
        &self.p_arc
    }
    pub fn get_arc_mut(&mut self) -> &mut PrimArc {
        &mut self.p_arc
    }

    pub fn get_vars(&self) -> Primitive {
        Primitive {
            curve: self.curve.clone(),
            p_line: self.p_line.clone(),
            p_arc: self.p_arc.clone(),
        }
    }
    pub fn set_vars(&mut self, vars: &Primitive) {
        self.curve = vars.curve.clone();
        self.p_line = vars.p_line.clone();
        self.p_arc = vars.p_arc.clone();
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
    fn get_control_state(&self, start: Vec2, end: Vec2, hs: HS) -> Option<Vec2> {
        use PrimitiveCurve::*;
        match self.curve {
            CurveLine => self.p_line.get_control_state(start, end, hs),
            CurveArc => self.p_arc.get_control_state(start, end, hs),
        }
    }
    fn set_all_controls_state(&mut self, hs: HS, state: bool) {
        use PrimitiveCurve::*;
        match self.curve {
            CurveLine => self.p_line.set_all_controls_state(hs, state),
            CurveArc => self.p_arc.set_all_controls_state(hs, state),
        }
    }
    fn get_dist_from_control(
        &self,
        start: Vec2,
        end: Vec2,
        pointer: &Pointer,
    ) -> Option<(f64, Vec2)> {
        use PrimitiveCurve::*;
        match self.curve {
            CurveLine => self.p_line.get_dist_from_control(start, end, pointer),
            CurveArc => self.p_arc.get_dist_from_control(start, end, pointer),
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
    fn get_paths_and_patterns(
        &self,
        start: Vec2,
        end: Vec2,
        das: &Size,
        parent_selected: bool,
        parent_highlighted: bool,
    ) -> (BezPath, Pattern) {
        use PrimitiveCurve::*;
        match self.curve {
            CurveLine => self.p_line.get_paths_and_patterns(
                start,
                end,
                das,
                parent_selected,
                parent_highlighted,
            ),
            CurveArc => self.p_arc.get_paths_and_patterns(
                start,
                end,
                das,
                parent_selected,
                parent_highlighted,
            ),
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
    fn get_control_state(&self, start: Vec2, end: Vec2, hs: HS) -> Option<Vec2>;
    fn set_all_controls_state(&mut self, hs: HS, state: bool);
    fn get_dist_from_control(
        &self,
        start: Vec2,
        end: Vec2,
        pointer: &Pointer,
    ) -> Option<(f64, Vec2)>;
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
    fn get_paths_and_patterns(
        &self,
        start: Vec2,
        end: Vec2,
        das: &Size,
        parent_selected: bool,
        parent_highlighted: bool,
    ) -> (BezPath, Pattern);
    fn get_dimensions_paths_and_patterns(
        &self,
        start: Vec2,
        end: Vec2,
        das: &Size,
    ) -> Vec<(BezPath, Pattern, CanvasText)>;
}
