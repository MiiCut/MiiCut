use super::helpers::HelperKind;
use super::helpers::HelperKindvars;
use crate::canvas::CanvasText;
use crate::canvas::Pattern;
use crate::get_line_segment;
use crate::is_near_line;
use crate::math::*;
use crate::prefab::*;
use crate::traits::*;
use crate::Position;
use crate::Value;
use crate::HS;
use kurbo::BezPath;
use kurbo::Line;
use kurbo::Shape;
use kurbo::Size;
use kurbo::Vec2;
use std::f64::consts::PI;
use std::fmt::Debug;
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq)]
pub struct HelperLine {
    center: Position,
    angle: Value,

    highlighted: bool,
    selected: bool,
}
impl HelperLine {
    pub fn new(position: Vec2, pos2: Vec2) -> HelperKind {
        let position = Position::new(position, true);
        let mut angle = Value::new((pos2 - position.get_pos()).atan2());
        angle.select(true);
        HelperKind::Line(HelperLine {
            center: position,
            angle,
            highlighted: false,
            selected: false,
        })
    }
    pub fn magnet_to(&self, pos: Vec2) -> Option<Vec2> {
        if self.selected {
            return None;
        }
        if (pos - self.center.get_pos()).hypot() < Self::GRAB_RADIUS {
            Some(self.center.get_pos())
        } else {
            None
        }
    }
}
impl Display for HelperLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Helper line")
    }
}

impl ObjectsFuncs for HelperLine {
    const TOLERANCE: f64 = 0.01;
    const GRAB_RADIUS: f64 = 4.;
    type Kindvars = HelperKindvars;

    fn save_vars(&mut self) {
        self.center.save_pos();
        self.angle.save_val();
    }
    fn restore_saved(&mut self) {
        self.center.restore_saved();
        self.angle.restore_saved();
    }
    fn get_vars(&self) -> HelperKindvars {
        HelperKindvars::Line(self.center, self.angle)
    }
    fn set_vars(&mut self, vars: &HelperKindvars) {
        if let HelperKindvars::Line(position, angle) = vars {
            self.center = position.clone();
            self.angle = angle.clone();
        }
    }
    fn good_size(&self) -> bool {
        true
    }

    fn set_hs_from_pos(&mut self, pos: Vec2, _snap: f64, hors: HS) -> bool {
        let hs = (pos - self.center.get_pos()).hypot() < Self::GRAB_RADIUS;
        match hors {
            HS::Highlight => {
                self.highlighted = hs;
                self.highlighted
            }
            HS::Select => {
                self.selected = hs;
                self.selected
            }
        }
    }
    fn set_hs(&mut self, value: bool, hors: HS) {
        match hors {
            HS::Highlight => self.highlighted = value,
            HS::Select => self.selected = value,
        }
    }
    fn get_hs(&self, hors: HS) -> bool {
        match hors {
            HS::Highlight => self.highlighted,
            HS::Select => self.selected,
        }
    }
    fn get_hhss(&self) -> (bool, bool) {
        (self.selected, self.highlighted)
    }
    fn set_hs_modifiers_from_pos(&mut self, pos: Vec2, _snap: f64, hors: HS) -> Option<Vec2> {
        let hs = is_near_line(
            self.center.get_pos(),
            self.angle.get_val(),
            pos,
            Self::GRAB_RADIUS,
        );
        match hors {
            HS::Highlight => self.angle.highlight(hs),
            HS::Select => self.angle.select(hs),
        }
        if hs {
            return Some(pos);
        } else {
            None
        }
    }

    fn set_hs_modifiers(&mut self, value: bool, hors: HS) {
        match hors {
            HS::Highlight => {
                self.angle.highlight(value);
            }
            HS::Select => {
                self.angle.select(value);
            }
        }
    }
    fn get_hs_modifiers(&self, hors: HS) -> bool {
        match hors {
            HS::Highlight => self.angle.is_highlighted(),
            HS::Select => self.angle.is_selected(),
        }
    }

    fn toggle_prop(&mut self) {
        ()
    }

    fn move_position(&mut self, dpos: Vec2, snap: f64) {
        self.center
            .set_pos(snap_pt(self.center.get_saved_pos() + dpos, snap));
    }
    fn move_modifier(
        &mut self,
        _pos_init: Vec2,
        pos: Vec2,
        snap: f64,
        _shift_pressed: bool,
    ) -> Option<Vec2> {
        if self.angle.is_selected() {
            let angle = (pos - self.center.get_pos()).atan2();
            self.angle
                .set_val(snap_val(angle / PI * 180., snap) / 180. * PI);
            return Some(pos);
        }
        None
    }
    fn get_position(&self) -> Vec2 {
        self.center.get_pos()
    }

    fn get_modifiers_paths(&self, drawing_area_size: &Size) -> Vec<(BezPath, Pattern)> {
        let pattern_line = match (self.angle.is_selected(), self.angle.is_highlighted()) {
            (false, false) => Pattern::HelperNormalCircle,
            (false, true) => Pattern::HelperHighlightedCircle,
            (true, false) => Pattern::HelperSelectedCircle,
            (true, true) => Pattern::HelperSelectedCircle,
        };
        let points = get_line_segment(
            drawing_area_size,
            self.center.get_pos(),
            self.angle.get_val(),
        );
        vec![(
            Line::new(points.0.to_point(), points.1.to_point()).to_path(HelperLine::TOLERANCE),
            pattern_line,
        )]
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        (vec![], vec![])
    }
    fn get_paths(&self, _: &Size) -> Vec<BezPath> {
        vec![]
    }
    fn get_paths_and_patterns(&self, _: &Size) -> Vec<(BezPath, Pattern)> {
        let hs = self.get_hhss();
        let pattern_center = match (hs.0, hs.1) {
            (false, false) => Pattern::HelperNormal,
            (false, true) => Pattern::HelperHighlighted,
            (true, false) => Pattern::HelperSelected,
            (true, true) => Pattern::HelperSelected,
        };
        let paths = vec![(
            modifiers_path(self.center.get_pos(), 1., Self::GRAB_RADIUS),
            pattern_center,
        )];
        paths
    }
}
