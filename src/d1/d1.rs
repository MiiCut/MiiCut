use crate::{canvas::Pattern, math::*, positions::*, GetEntityState, SetEntityState};
use kurbo::{
    Arc, ArcAppendIter, BezPath, CubicBez, CubicBezIter, Line, LinePathIter, PathEl, QuadBez,
    QuadBezIter, Shape, Size, Vec2,
};

#[derive(Debug, Clone)]
pub enum D1Kind {
    D1KLine,
    D1KArc,
    D1KQBez,
    D1KQBezSmooth,
    D1KCBez,
    D1KCBezSmooth,
}

#[derive(Debug, Clone)]
pub struct D1 {
    kind: D1Kind,
    start: Position,
    end: Position,

    highlighted: bool,
    selected: bool,
    //Arc
    arc_radius: RadiusPosition,
    //QBez
    qbez_control: Position,
    //CBez
    cbez_control1: Position,
    cbez_control2: Position,
}

impl D1 {
    pub fn new(kind: D1Kind, start: Vec2, end: Vec2) -> Self {
        let start = Position::new(start, true);
        let end = Position::new(end, true);
        let highlighted = false;
        let selected = false;
        let arc_values = RadiusPosition::new((start.pos + end.pos) / 2., true);
        let qbez_control = Position::new((start.pos + end.pos) / 2., true);
        let cbez_control1 = Position::new((start.pos + end.pos) / 3., true);
        let cbez_control2 = Position::new((start.pos + end.pos) * 2. / 3., true);
        Self {
            kind,
            start,
            end,
            highlighted,
            selected,
            arc_radius: arc_values,
            qbez_control,
            cbez_control1,
            cbez_control2,
        }
    }
    pub fn get_kind(&self) -> &D1Kind {
        &self.kind
    }
    pub fn get_kind_mut(&mut self) -> &mut D1Kind {
        &mut self.kind
    }
    pub fn get_start(&self) -> &Position {
        &self.start
    }
    pub fn get_start_mut(&mut self) -> &mut Position {
        &mut self.start
    }
    pub fn get_end(&self) -> &Position {
        &self.end
    }
    pub fn get_end_mut(&mut self) -> &mut Position {
        &mut self.end
    }
    pub fn get_highlighted(&self) -> bool {
        self.highlighted
    }
    pub fn get_selected(&self) -> bool {
        self.selected
    }
    pub fn get_arc_vars(&self) -> (Vec2, bool) {
        (self.arc_radius.radius.pos, self.arc_radius.up)
    }
    pub fn get_qbez_control(&self) -> &Position {
        &self.qbez_control
    }
    pub fn get_qbez_control_mut(&mut self) -> &mut Position {
        &mut self.qbez_control
    }
    pub fn get_cbez_control1(&self) -> &Position {
        &self.cbez_control1
    }
    pub fn get_cbez_control1_mut(&mut self) -> &mut Position {
        &mut self.cbez_control1
    }
    pub fn get_cbez_control2(&self) -> &Position {
        &self.cbez_control2
    }
    pub fn get_cbez_control2_mut(&mut self) -> &mut Position {
        &mut self.cbez_control2
    }

    pub fn set_kind(&mut self, kind: D1Kind) {
        self.kind = kind;
    }
    pub fn set_start(&mut self, start: Vec2) {
        self.start.pos = start;
    }
    pub fn set_end(&mut self, end: Vec2) {
        self.end.pos = end;
    }
    pub fn set_arc_radius(&mut self, arc_radius: Vec2) {
        self.arc_radius.radius.pos = arc_radius;
    }
    pub fn set_arc_up(&mut self, arc_up: bool) {
        self.arc_radius.up = arc_up;
    }
    pub fn set_qbez_control(&mut self, qbez_control: Vec2) {
        self.qbez_control.pos = qbez_control;
    }
    pub fn set_cbez_control1(&mut self, cbez_control1: Vec2) {
        self.cbez_control1.pos = cbez_control1;
    }
    pub fn set_cbez_control2(&mut self, cbez_control2: Vec2) {
        self.cbez_control2.pos = cbez_control2;
    }
    pub fn save_vars(&mut self) {
        self.start.saved_pos = self.start.pos;
        self.end.saved_pos = self.end.pos;
        self.arc_radius.radius.saved_pos = self.arc_radius.radius.pos;
        self.qbez_control.saved_pos = self.qbez_control.pos;
        self.cbez_control1.saved_pos = self.cbez_control1.pos;
        self.cbez_control2.saved_pos = self.cbez_control2.pos;
    }
    pub fn restore_saved(&mut self) {
        self.start.pos = self.start.saved_pos;
        self.end.pos = self.end.saved_pos;
        self.arc_radius.radius.pos = self.arc_radius.radius.saved_pos;
        self.qbez_control.pos = self.qbez_control.saved_pos;
        self.cbez_control1.pos = self.cbez_control1.saved_pos;
        self.cbez_control2.pos = self.cbez_control2.saved_pos;
    }
    pub fn get_vars(&self) -> D1 {
        D1 {
            kind: self.kind.clone(),
            start: self.start.clone(),
            end: self.end.clone(),
            highlighted: self.highlighted,
            selected: self.selected,
            arc_radius: self.arc_radius.clone(),
            qbez_control: self.qbez_control.clone(),
            cbez_control1: self.cbez_control1.clone(),
            cbez_control2: self.cbez_control2.clone(),
        }
    }
    pub fn set_vars(&mut self, vars: &D1) {
        self.kind = vars.kind.clone();
        self.start = vars.start.clone();
        self.end = vars.end.clone();
        self.highlighted = vars.highlighted;
        self.selected = vars.selected;
        self.arc_radius = vars.arc_radius.clone();
        self.qbez_control = vars.qbez_control.clone();
        self.cbez_control1 = vars.cbez_control1.clone();
        self.cbez_control2 = vars.cbez_control2.clone();
    }
}
impl D1 {
    const TOLERANCE: f64 = 0.01;

    pub fn path_elements(&self) -> D1KindIter {
        use D1Kind::*;
        match self.kind {
            D1KLine => D1KindIter::Line(
                Line::new(self.start.pos.to_point(), self.end.pos.to_point())
                    .path_elements(Self::TOLERANCE),
            ),
            D1KArc => D1KindIter::Arc(
                Arc::new(
                    self.start.pos.to_point(),
                    (self.arc_radius.radius.pos.x, self.arc_radius.radius.pos.y),
                    0.,
                    360.,
                    0.,
                )
                .path_elements(Self::TOLERANCE),
            ),
            D1KQBez => D1KindIter::QBez(
                QuadBez::new(
                    self.start.pos.to_point(),
                    self.qbez_control.pos.to_point(),
                    self.end.pos.to_point(),
                )
                .path_elements(Self::TOLERANCE),
            ),
            D1KQBezSmooth => D1KindIter::QBezSmooth(
                QuadBez::new(
                    self.start.pos.to_point(),
                    self.qbez_control.pos.to_point(),
                    self.end.pos.to_point(),
                )
                .path_elements(Self::TOLERANCE),
            ),
            D1KCBez => D1KindIter::CBez(
                CubicBez::new(
                    self.start.pos.to_point(),
                    self.cbez_control1.pos.to_point(),
                    self.cbez_control2.pos.to_point(),
                    self.end.pos.to_point(),
                )
                .path_elements(Self::TOLERANCE),
            ),
            D1KCBezSmooth => D1KindIter::CBezSmooth(
                CubicBez::new(
                    self.start.pos.to_point(),
                    self.cbez_control1.pos.to_point(),
                    self.cbez_control2.pos.to_point(),
                    self.end.pos.to_point(),
                )
                .path_elements(Self::TOLERANCE),
            ),
        }
    }
    pub fn to_path(&self) -> BezPath {
        self.path_elements().collect()
    }
    pub fn get_paths_and_patterns(&self, _drawing_area_size: &Size) -> (BezPath, Pattern) {
        let pattern = match (self.selected, self.highlighted) {
            (false, false) => Pattern::BasicNormal,
            (false, true) => Pattern::BasicHighlighted,
            (true, false) => Pattern::BasicSelected,
            (true, true) => Pattern::BasicSelected,
        };
        (self.to_path(), pattern)
    }
    pub fn get_positions(&self) -> (&Position, &Position) {
        (&self.start, &self.end)
    }
    pub fn get_positions_mut(&mut self) -> (&mut Position, &mut Position) {
        (&mut self.start, &mut self.end)
    }
    pub fn get_positions_into(&self) -> (Position, Position) {
        (self.start, self.end)
    }
    pub fn set_positions(&mut self, start: &Position, end: &Position) {
        self.start = start.clone();
        self.end = end.clone();
    }
    pub fn get_position(&self) -> Vec2 {
        use D1Kind::*;
        match self.kind {
            D1KLine => (self.start.pos + self.end.pos) / 2.,
            D1KArc => (self.start.pos + self.end.pos) / 2.,
            D1KQBez => (self.start.pos + self.end.pos) / 2.,
            D1KQBezSmooth => (self.start.pos + self.end.pos) / 2.,
            D1KCBez => (self.start.pos + self.end.pos) / 2.,
            D1KCBezSmooth => (self.start.pos + self.end.pos) / 2.,
        }
    }
    pub fn get_state(&self, get: GetEntityState) -> Option<Vec2> {
        use GetEntityState::*;
        match get {
            IsSelected => {
                if self.selected {
                    Some(self.get_position())
                } else {
                    None
                }
            }
            IsHighlighted => {
                if self.highlighted {
                    Some(self.get_position())
                } else {
                    None
                }
            }
            IsAnyModifierSelected => {
                if self.start.selected {
                    return Some(self.start.pos);
                }
                if self.end.selected {
                    return Some(self.end.pos);
                }
                use D1Kind::*;
                match self.kind {
                    D1KLine => None,
                    D1KArc => self
                        .arc_radius
                        .radius
                        .selected
                        .then(|| self.arc_radius.radius.pos),
                    D1KQBez | D1KQBezSmooth => {
                        self.qbez_control.selected.then(|| self.qbez_control.pos)
                    }
                    D1KCBez | D1KCBezSmooth => {
                        if self.cbez_control1.selected {
                            Some(self.cbez_control1.pos)
                        } else {
                            self.cbez_control2.selected.then(|| self.cbez_control2.pos)
                        }
                    }
                }
            }
            IsAnyModifierHighlighted => {
                if self.start.highlighted {
                    return Some(self.start.pos);
                }
                if self.end.highlighted {
                    return Some(self.end.pos);
                }
                use D1Kind::*;
                match self.kind {
                    D1KLine => None,
                    D1KArc => self
                        .arc_radius
                        .radius
                        .highlighted
                        .then(|| self.arc_radius.radius.pos),
                    D1KQBez | D1KQBezSmooth => {
                        self.qbez_control.highlighted.then(|| self.qbez_control.pos)
                    }
                    D1KCBez | D1KCBezSmooth => {
                        if self.cbez_control1.highlighted {
                            Some(self.cbez_control1.pos)
                        } else {
                            self.cbez_control2
                                .highlighted
                                .then(|| self.cbez_control2.pos)
                        }
                    }
                }
            }
        }
    }
    pub fn set_state(&mut self, set: SetEntityState) {
        use SetEntityState::*;
        match set {
            SetSelect(value) => self.selected = value,
            SelectFromPos(pos, grab, _precision) => {
                use D1Kind::*;
                self.selected = match self.kind {
                    D1KLine | D1KArc | D1KCBez | D1KCBezSmooth | D1KQBez | D1KQBezSmooth => {
                        distance_to_segment(self.start.pos, self.end.pos, pos) < grab
                    }
                };
            }

            SetHighlight(value) => self.highlighted = value,
            HighlightFromPos(pos, grab, _precision) => {
                use D1Kind::*;
                self.highlighted = match self.kind {
                    D1KLine | D1KArc | D1KCBez | D1KCBezSmooth | D1KQBez | D1KQBezSmooth => {
                        distance_to_segment(self.start.pos, self.end.pos, pos) < grab
                    }
                };
            }

            SelectAllModifiers(value) => {
                self.start.selected = value;
                self.end.selected = value;
                use D1Kind::*;
                match self.kind {
                    D1KLine => (),
                    D1KArc => _ = self.arc_radius.radius.selected = value,
                    D1KQBez | D1KQBezSmooth => {
                        _ = self.qbez_control.selected = value;
                    }
                    D1KCBez | D1KCBezSmooth => {
                        self.cbez_control1.selected = value;
                        self.cbez_control2.selected = value;
                    }
                }
            }
            SelectModifierFromPos(pos, grab, _precision) => {
                self.start.selected = (self.start.pos - pos).hypot() < grab;
                self.end.selected = (self.end.pos - pos).hypot() < grab;

                use D1Kind::*;
                match self.kind {
                    D1KLine => (),
                    D1KArc => {
                        self.arc_radius.radius.selected =
                            (self.arc_radius.radius.pos - pos).hypot() < grab;
                    }
                    D1KQBez | D1KQBezSmooth => {
                        self.qbez_control.selected = (self.qbez_control.pos - pos).hypot() < grab;
                    }
                    D1KCBez | D1KCBezSmooth => {
                        self.cbez_control1.selected = (self.cbez_control1.pos - pos).hypot() < grab;
                        self.cbez_control2.selected = (self.cbez_control2.pos - pos).hypot() < grab;
                    }
                }
            }

            HighlightAllModifiers(value) => {
                self.start.highlighted = value;
                self.end.highlighted = value;
                use D1Kind::*;
                match self.kind {
                    D1KLine => (),
                    D1KArc => _ = self.arc_radius.radius.highlighted = value,
                    D1KQBez | D1KQBezSmooth => {
                        _ = self.qbez_control.highlighted = value;
                    }
                    D1KCBez | D1KCBezSmooth => {
                        self.cbez_control1.highlighted = value;
                        self.cbez_control2.highlighted = value;
                    }
                }
            }
            HighlightModifierFromPos(pos, grab, _precision) => {
                self.start.highlighted = (self.start.pos - pos).hypot() < grab;
                self.end.highlighted = (self.end.pos - pos).hypot() < grab;

                use D1Kind::*;
                match self.kind {
                    D1KLine => (),
                    D1KArc => {
                        self.arc_radius.radius.highlighted =
                            (self.arc_radius.radius.pos - pos).hypot() < grab
                    }

                    D1KQBez | D1KQBezSmooth => {
                        self.qbez_control.highlighted =
                            (self.qbez_control.pos - pos).hypot() < grab;
                    }
                    D1KCBez | D1KCBezSmooth => {
                        self.cbez_control1.highlighted =
                            (self.cbez_control1.pos - pos).hypot() < grab;
                        self.cbez_control2.highlighted =
                            (self.cbez_control2.pos - pos).hypot() < grab;
                    }
                }
            }
        }
    }
}

impl Iterator for D1 {
    type Item = PathEl;
    fn next(&mut self) -> Option<Self::Item> {
        self.path_elements().next()
    }
}
pub enum D1KindIter {
    Line(LinePathIter),
    Arc(std::iter::Chain<std::iter::Once<PathEl>, ArcAppendIter>),
    QBez(QuadBezIter),
    QBezSmooth(QuadBezIter),
    CBez(CubicBezIter),
    CBezSmooth(CubicBezIter),
}
impl Iterator for D1KindIter {
    type Item = PathEl;
    fn next(&mut self) -> Option<Self::Item> {
        use D1KindIter::*;
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
