use super::helpers::HelperKind;
use super::helpers::HelperKindvars;
use crate::canvas::CanvasText;
use crate::canvas::Pattern;
use crate::dimensions::DimKind;
use crate::dimensions::Dimension;
use crate::math::*;
use crate::prefab::modifiers_path;
use crate::GetEntityState;
use crate::ObjectsFuncs;
use crate::Position;
use crate::SetEntityState;
use crate::Value;
use kurbo::BezPath;
use kurbo::Circle;
use kurbo::Rect;
use kurbo::Shape;
use kurbo::Size;
use kurbo::Vec2;
use std::fmt::Debug;
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq)]
pub struct HelperCircle {
    center: Position,
    radius: Value,

    highlighted: bool,
    selected: bool,
}
impl HelperCircle {
    const MIN_RADIUS: f64 = 2.;

    pub fn new(center: Vec2, _pos2: Vec2) -> HelperKind {
        let center = Position::new(center, true);
        let mut radius = Value::new(0.);
        radius.selected = true;

        HelperKind::Circle(HelperCircle {
            center,
            radius,
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
    fn get_circle(&self) -> Circle {
        let center = self.center.pos;
        let radius = self.radius.value;
        Circle::new(center.to_point(), radius)
    }

    fn highlight_all_modifiers(&mut self, value: bool) {
        self.radius.highlighted = value;
    }
    fn select_all_modifiers(&mut self, value: bool) {
        self.radius.selected = value;
    }

    fn highlight_modifiers_from_pos(&mut self, pos: Vec2, grab: f64) {
        self.radius.highlighted =
            ((pos - self.center.pos).hypot() - self.radius.value).abs() < grab;
    }
    fn select_modifiers_from_pos(&mut self, pos: Vec2, grab: f64) {
        self.radius.selected = ((pos - self.center.pos).hypot() - self.radius.value).abs() < grab;
    }
}
impl Display for HelperCircle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Helper line")
    }
}

impl ObjectsFuncs for HelperCircle {
    const TOLERANCE: f64 = 0.01;
    const GRAB_RADIUS: f64 = 4.;
    type Kindvars = HelperKindvars;

    fn save_vars(&mut self) {
        self.center.saved_pos = self.center.pos;
        self.radius.saved_val = self.radius.value;
    }
    fn restore_saved(&mut self) {
        self.center.pos = self.center.saved_pos;
        self.radius.value = self.radius.saved_val;
    }
    fn get_vars(&self) -> HelperKindvars {
        HelperKindvars::Line(self.center, self.radius)
    }
    fn set_vars(&mut self, vars: &HelperKindvars) {
        if let HelperKindvars::Line(position, radius) = vars {
            self.center = position.clone();
            self.radius = radius.clone();
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
            IsHighlighted => {
                if self.highlighted {
                    Some(self.get_position())
                } else {
                    None
                }
            }
            IsAnyModifierSelected => {
                let select = self.radius.selected;
                if select {
                    Some(self.get_position())
                } else {
                    None
                }
            }
            IsAnyModifierHighlighted => {
                let highlight = self.radius.highlighted;
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
            SetHighlight(value) => self.highlighted = value,
            HighlightFromPos(pos, ..) => {
                self.highlighted = (pos - self.center.pos).hypot() < Self::GRAB_RADIUS;
            }

            SelectAllModifiers(value) => self.select_all_modifiers(value),
            SelectModifierFromPos(pos, precision, _) => {
                self.select_modifiers_from_pos(pos, precision);
            }

            HighlightAllModifiers(value) => self.highlight_all_modifiers(value),
            HighlightModifierFromPos(pos, precision, _) => {
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
        pos_init: Vec2,
        pos: Vec2,
        snap: f64,
        _shift_pressed: bool,
    ) -> Option<Vec2> {
        let dpos = pos - pos_init;
        let saved_radius = self.radius.saved_val;
        let radius = snap_val(saved_radius + dpos.x, snap);
        if radius >= HelperCircle::MIN_RADIUS {
            self.radius.value = radius;
        }
        Some(pos)
    }
    fn get_position(&self) -> Vec2 {
        self.center.pos
    }

    fn get_mod_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        let pattern_circle = match (self.radius.selected, self.radius.highlighted) {
            (false, false) => Pattern::HelperNormalCircle,
            (false, true) => Pattern::HelperHighlightedCircle,
            (true, false) => Pattern::HelperSelectedCircle,
            (true, true) => Pattern::HelperSelectedCircle,
        };
        vec![((self.get_circle().to_path(Self::TOLERANCE), pattern_circle))]
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        let mut paths = vec![];
        let mut texts = vec![];
        let offset = self.radius.value / 2_f64.sqrt();
        let end = self.center.pos + Vec2::new(offset, -offset);
        let (path, text) =
            Dimension::new(DimKind::Radius, self.center.pos, end, self.radius.value).get_path();
        paths.push(path);
        texts.push(text);
        (paths, texts)
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
