use super::{
    prim_arc::PrimArc,
    prim_cbez::PrimCBez,
    prim_cbez_smooth::PrimCBezSmooth,
    prim_line::PrimLine,
    prim_qbez::PrimQBez,
    prim_qbez_smooth::PrimQBezSmooth,
    primitives::{D1Kind, PrimitiveControls},
};
use crate::{canvas::Pattern, math::*, positions::*, GetEntityState, SetEntityState};
use kurbo::{
    ArcAppendIter, BezPath, CubicBez, CubicBezIter, Line, LinePathIter, PathEl, QuadBez,
    QuadBezIter, Size, Vec2,
};

#[derive(Debug, Clone)]
pub struct D1 {
    kind: D1Kind,
    start: Position,
    end: Position,

    // Primitives
    // Line
    p_line: PrimLine,
    //Arc
    p_arc: PrimArc,
    // //QBez
    // p_qbez: PrimQBez,
    // //QBezSmooth
    // p_qbez_smooth: PrimQBezSmooth,
    // //CBez
    // p_cbez: PrimCBez,
    // //CBezSmooth
    // p_cbez_smooth: PrimCBezSmooth,
}

impl D1 {
    pub fn new(kind: D1Kind, start: Vec2, end: Vec2) -> Self {
        let start = Position::new(start, true);
        let end = Position::new(end, true);

        Self {
            kind,
            start,
            end,

            p_line: PrimLine::new(start.pos, end.pos),
            p_arc: PrimArc::new(start.pos, end.pos),
            // p_qbez: PrimQBez::new(),
            // p_qbez_smooth: PrimQBezSmooth::new(),
            // p_cbez: PrimCBez::new(),
            // p_cbez_smooth: PrimCBezSmooth::new(),
        }
        // let arc_radius = RadiusValue::new(100., true);
        // let qbez_control = Position::new(Vec2::ZERO, true);
        // let cbez_control1 = Position::new((start.pos + end.pos) / 3. - start.pos, true);
        // let cbez_control2 = Position::new((start.pos + end.pos) * 2. / 3. - end.pos, true);
        // Self {
        //     kind,
        //     start,
        //     end,
        //     highlighted,
        //     selected,
        //     arc_radius,
        //     qbez_control,
        //     cbez_control1,
        //     cbez_control2,
        // }
    }
    pub fn get_d1_kind(&self) -> &D1Kind {
        &self.kind
    }
    pub fn set_d1_kind_next(&mut self) -> Vec2 {
        self.kind = self.kind.next_kind();
        match self.kind {
            D1Kind::D1KLine => self.p_line.update_vars(self.start.pos, self.end.pos),
            D1Kind::D1KArc => self.p_arc.update_vars(self.start.pos, self.end.pos),
        }
    }
    pub fn set_d1_kind_prev(&mut self) -> Vec2 {
        self.kind.prev_kind();
        match self.kind {
            D1Kind::D1KLine => self.p_line.update_vars(self.start.pos, self.end.pos),
            D1Kind::D1KArc => self.p_arc.update_vars(self.start.pos, self.end.pos),
        }
    }
    pub fn set_d1_kind(&mut self, kind: D1Kind) {
        self.kind = kind;
    }
    pub fn save_vars(&mut self) {
        self.start.saved_pos = self.start.pos;
        self.end.saved_pos = self.end.pos;
    }
    pub fn restore_saved(&mut self) {
        self.start.pos = self.start.saved_pos;
        self.end.pos = self.end.saved_pos;
    }
    pub fn get_vars(&self) -> D1 {
        D1 {
            kind: self.kind.clone(),
            start: self.start.clone(),
            end: self.end.clone(),
            p_line: self.p_line.clone(),
            p_arc: self.p_arc.clone(),
        }
    }
    pub fn set_vars(&mut self, vars: &D1) {
        self.kind = vars.kind.clone();
        self.start = vars.start.clone();
        self.end = vars.end.clone();
        self.p_line = vars.p_line.clone();
        self.p_arc = vars.p_arc.clone();
    }
}
impl D1 {
    const _TOLERANCE: f64 = 0.01;
    const GRAB_RADIUS: f64 = 4.;

    pub fn toogle(&mut self) -> Vec2 {
        use D1Kind::*;
        match self.kind {
            D1KLine => {
                self.p_line.toggle();
                self.p_line.update_vars(self.start.pos, self.end.pos)
            }
            D1KArc => {
                self.p_arc.toggle();
                self.p_arc.update_vars(self.start.pos, self.end.pos)
            }
        }
    }
    pub fn path_elements(&self) -> D1KindIter {
        use D1Kind::*;
        match self.kind {
            D1KLine => self.p_line.path_elements(self.start.pos, self.end.pos),
            D1KArc => self.p_arc.path_elements(self.start.pos, self.end.pos),
        }
    }
    pub fn to_path(&self) -> BezPath {
        self.path_elements().collect()
    }
    pub fn get_paths_and_patterns(&self, _drawing_area_size: &Size) -> (BezPath, Pattern) {
        use D1Kind::*;
        let pattern = match self.kind {
            D1KLine => self.p_line.get_pattern(),
            D1KArc => self.p_arc.get_pattern(),
        };
        (self.to_path(), pattern)
    }

    pub fn get_start_position(&self) -> Vec2 {
        self.start.pos
    }
    pub fn get_start_saved_position(&self) -> Vec2 {
        self.start.saved_pos
    }
    pub fn set_start_position(&mut self, start: Vec2) {
        self.start.pos = start;
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
    }
    pub fn is_end_selected(&self) -> bool {
        self.end.selected
    }
    pub fn is_end_highlighted(&self) -> bool {
        self.end.highlighted
    }

    pub fn get_position(&self) -> Vec2 {
        use D1Kind::*;
        match self.kind {
            D1KLine => (self.start.pos + self.end.pos) / 2.,
            D1KArc => (self.start.pos + self.end.pos) / 2.,
        }
    }
    pub fn get_state(&self, get: GetEntityState) -> Option<Vec2> {
        use D1Kind::*;
        use GetEntityState::*;
        let start = self.start.pos;
        let end = self.end.pos;
        match get {
            IsSelected => match self.kind {
                D1KLine => self.p_line.get_state(start, end, IsSelected),
                D1KArc => self.p_arc.get_state(start, end, IsSelected),
            },
            IsHighligh => match self.kind {
                D1KLine => self.p_line.get_state(start, end, IsHighligh),
                D1KArc => self.p_arc.get_state(start, end, IsHighligh),
            },
            IsAnyModifierSelected => {
                if self.start.selected {
                    return Some(self.start.pos);
                }
                if self.end.selected {
                    return Some(self.end.pos);
                }
                use D1Kind::*;
                match self.kind {
                    D1KLine => {
                        self.p_line
                            .get_state(self.start.pos, self.end.pos, IsAnyModifierSelected)
                    }
                    D1KArc => {
                        self.p_arc
                            .get_state(self.start.pos, self.end.pos, IsAnyModifierSelected)
                    }
                }
            }
            IsAnyModifierHighligh => {
                if self.start.highlighted {
                    return Some(self.start.pos);
                }
                if self.end.highlighted {
                    return Some(self.end.pos);
                }
                use D1Kind::*;
                match self.kind {
                    D1KLine => {
                        self.p_line
                            .get_state(self.start.pos, self.end.pos, IsAnyModifierHighligh)
                    }
                    D1KArc => {
                        self.p_arc
                            .get_state(self.start.pos, self.end.pos, IsAnyModifierHighligh)
                    }
                }
            }
        }
    }
    pub fn set_state(&mut self, set: SetEntityState) {
        use D1Kind::*;
        use SetEntityState::*;
        let start = self.start.pos;
        let end = self.end.pos;
        match set {
            SetSelect(value) => match self.kind {
                D1KLine => self.p_line.set_state(start, end, SetSelect(value)),
                D1KArc => self.p_arc.set_state(start, end, SetSelect(value)),
            },
            SetHighli(value) => match self.kind {
                D1KLine => self.p_line.set_state(start, end, SetHighli(value)),
                D1KArc => self.p_arc.set_state(start, end, SetHighli(value)),
            },

            SelectFromPos(pos, a, b) => match self.kind {
                D1KLine => self.p_line.set_state(start, end, SelectFromPos(pos, a, b)),
                D1KArc => self.p_arc.set_state(start, end, SelectFromPos(pos, a, b)),
            },
            HighliFromPos(pos, a, b) => match self.kind {
                D1KLine => self.p_line.set_state(start, end, HighliFromPos(pos, a, b)),
                D1KArc => self.p_arc.set_state(start, end, HighliFromPos(pos, a, b)),
            },

            SelectAllModifiers(value) => {
                self.start.selected = value;
                self.end.selected = value;
                use D1Kind::*;
                match self.kind {
                    D1KLine => self.p_line.set_state(start, end, SelectAllModifiers(value)),
                    D1KArc => self.p_arc.set_state(start, end, SelectAllModifiers(value)),
                }
            }
            HighliAllModifiers(value) => {
                self.start.highlighted = value;
                self.end.highlighted = value;
                use D1Kind::*;
                match self.kind {
                    D1KLine => self.p_line.set_state(start, end, HighliAllModifiers(value)),
                    D1KArc => self.p_arc.set_state(start, end, HighliAllModifiers(value)),
                }
            }
            SelectModifierFromPos(pos, a, b) => {
                self.start.selected = (self.start.pos - pos).hypot() < Self::GRAB_RADIUS;
                self.end.selected = (self.end.pos - pos).hypot() < Self::GRAB_RADIUS;
                use D1Kind::*;
                match self.kind {
                    D1KLine => self
                        .p_line
                        .set_state(start, end, SelectModifierFromPos(pos, a, b)),
                    D1KArc => self
                        .p_arc
                        .set_state(start, end, SelectModifierFromPos(pos, a, b)),
                }
            }
            HighliModifierFromPos(pos, a, b) => {
                self.start.highlighted = (self.start.pos - pos).hypot() < Self::GRAB_RADIUS;
                self.end.highlighted = (self.end.pos - pos).hypot() < Self::GRAB_RADIUS;

                use D1Kind::*;
                match self.kind {
                    D1KLine => self
                        .p_line
                        .set_state(start, end, HighliModifierFromPos(pos, a, b)),
                    D1KArc => self
                        .p_arc
                        .set_state(start, end, HighliModifierFromPos(pos, a, b)),
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
