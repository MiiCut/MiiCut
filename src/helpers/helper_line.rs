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
use kurbo::BezPath;
use kurbo::Line;
use kurbo::Rect;
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
        let mut angle = Value::new((pos2 - position.pos).atan2());
        angle.selected = true;
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
        if (pos - self.center.pos).hypot() < Self::GRAB_RADIUS {
            Some(self.center.pos)
        } else {
            None
        }
    }
    fn highlight_all_modifiers(&mut self, value: bool) {
        self.angle.highlighted = value;
    }
    fn select_all_modifiers(&mut self, value: bool) {
        self.angle.selected = value;
    }

    fn highlight_modifiers_from_pos(&mut self, pos: Vec2, _grab: f64) {
        self.angle.highlighted =
            is_near_line(self.center.pos, self.angle.value, pos, Self::GRAB_RADIUS);
    }
    fn select_modifiers_from_pos(&mut self, pos: Vec2, _grab: f64) {
        self.angle.selected =
            is_near_line(self.center.pos, self.angle.value, pos, Self::GRAB_RADIUS);
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
        self.center.saved_pos = self.center.pos;
        self.angle.saved_val = self.angle.value;
    }
    fn restore_saved(&mut self) {
        self.center.pos = self.center.saved_pos;
        self.angle.value = self.angle.saved_val;
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

    fn get_state(&self, get: GetEntityState) -> Option<Vec2> {
        use GetEntityState::*;
        match get {
            IsSelected => {
                if self.selected {
                    Some(self.get_position())
                } else {
                    None
                }
            }
            IsHighligh => {
                if self.highlighted {
                    Some(self.get_position())
                } else {
                    None
                }
            }
            IsAnyModifierSelected => {
                let select = self.angle.selected;
                if select {
                    Some(self.get_position())
                } else {
                    None
                }
            }
            IsAnyModifierHighligh => {
                let highlight = self.angle.highlighted;
                if highlight {
                    Some(self.get_position())
                } else {
                    None
                }
            }
        }
    }
    fn set_state(&mut self, set: SetEntityState) {
        use SetEntityState::*;
        match set {
            SetSelect(value) => self.selected = value,
            SelectFromPos(pos, ..) => {
                self.selected = (pos - self.center.pos).hypot() < Self::GRAB_RADIUS;
            }
            SetHighli(value) => self.highlighted = value,
            HighliFromPos(pos, ..) => {
                self.highlighted = (pos - self.center.pos).hypot() < Self::GRAB_RADIUS;
            }

            SelectAllModifiers(value) => self.select_all_modifiers(value),
            SelectModifierFromPos(pos, precision, _) => {
                self.select_modifiers_from_pos(pos, precision);
            }

            HighliAllModifiers(value) => self.highlight_all_modifiers(value),
            HighliModifierFromPos(pos, precision, _) => {
                self.highlight_modifiers_from_pos(pos, precision);
            }
        }
    }

    fn toggle_prop(&mut self) {
        ()
    }

    fn move_position(&mut self, dpos: Vec2, snap: f64) -> Option<Vec2> {
        self.center.pos = snap_pt(self.center.saved_pos + dpos, snap);
        Some(self.get_position())
    }
    fn move_modifier(
        &mut self,
        _pos_init: Vec2,
        pos: Vec2,
        snap: f64,
        _shift_pressed: bool,
    ) -> Option<Vec2> {
        if self.angle.selected {
            let angle = (pos - self.center.pos).atan2();
            self.angle.value = snap_val(angle / PI * 180., snap) / 180. * PI;
            return Some(pos);
        }
        None
    }
    fn get_position(&self) -> Vec2 {
        self.center.pos
    }

    fn get_mod_paths_and_patterns(
        &self,
        _das: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        let pattern_line = match (self.angle.selected, self.angle.highlighted) {
            (false, false) => Pattern::HelperNormalCircle,
            (false, true) => Pattern::HelperHighlightedCircle,
            (true, false) => Pattern::HelperSelectedCircle,
            (true, true) => Pattern::HelperSelectedCircle,
        };
        let c_w = cinfo.0.width();
        let c_h = cinfo.0.height();
        let c_size = Size::new(c_w, c_h);
        let scale = cinfo.1;
        let offset = cinfo.2;
        let c_center = to_canvas(self.center.pos, scale, offset);
        let (pt_canvas_tl, pt_canvas_br) = get_line_segment(&c_size, c_center, self.angle.value);
        let pt_tl = to_draw(pt_canvas_tl, scale, offset);
        let pt_br = to_draw(pt_canvas_br, scale, offset);
        vec![(
            Line::new(pt_tl.to_point(), pt_br.to_point()).to_path(HelperLine::TOLERANCE),
            pattern_line,
        )]
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        (vec![], vec![])
    }
    fn get_paths(&self, _: &Size) -> Vec<BezPath> {
        vec![]
    }
    fn get_paths_and_patterns(&self, _: &Size, _: (Rect, f64, Vec2)) -> Vec<(BezPath, Pattern)> {
        let pattern_center = match (self.selected, self.highlighted) {
            (false, false) => Pattern::HelperNormal,
            (false, true) => Pattern::HelperHighlighted,
            (true, false) => Pattern::HelperSelected,
            (true, true) => Pattern::HelperSelected,
        };
        let paths = vec![(
            modifiers_path(self.center.pos, 1., Self::GRAB_RADIUS),
            pattern_center,
        )];
        paths
    }
}
