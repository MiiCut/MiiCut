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
use crate::Pointer;
use crate::Position;
use crate::SetEntityState;
use crate::SetEntityStateFromPos;
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
    const MIN_RADIUS: f64 = 10.;

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
    pub fn get_radius(&self) -> f64 {
        self.radius.value
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
        self.radius.value >= Self::MIN_RADIUS
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
                let select = self.radius.selected;
                if select {
                    Some(self.get_position())
                } else {
                    None
                }
            }
            IsAnyModifierHighligh => {
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
            SetHighli(value) => self.highlighted = value,
            SelectAllModifiers(value) => self.select_all_modifiers(value),
            HighliAllModifiers(value) => self.highlight_all_modifiers(value),
        }
    }
    fn set_state_from_pos(&mut self, pointer: &mut Pointer, set: SetEntityStateFromPos) {
        use SetEntityStateFromPos::*;
        match set {
            SelectFromPos => {
                self.selected = (pointer.pos() - self.center.pos).hypot() < Self::GRAB_RADIUS;
            }
            HighliFromPos => {
                self.highlighted = (pointer.pos() - self.center.pos).hypot() < Self::GRAB_RADIUS;
            }
            SelectModifierFromPos => {
                self.select_modifiers_from_pos(pointer.pos(), pointer.get_grab_dist());
            }
            HighliModifierFromPos => {
                self.highlight_modifiers_from_pos(pointer.pos(), pointer.get_grab_dist());
            }
        }
    }

    fn toggle_prop(&mut self) {
        ()
    }

    fn move_position(&mut self, pointer: &mut Pointer, _shift_pressed: bool) -> bool {
        self.center.pos = snap_pt(
            self.center.saved_pos + pointer.dpos(),
            pointer.get_snap().val(),
        );
        pointer.set_pos(self.center.pos);
        true
    }
    fn move_modifier(&mut self, pointer: &mut Pointer, _shift_pressed: bool) -> bool {
        let saved_radius = self.radius.saved_val;
        let radius = snap_val(saved_radius + pointer.dpos().x, pointer.get_snap().val());
        if radius >= HelperCircle::MIN_RADIUS {
            self.radius.value = radius;
        }
        true
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
    fn get_dimensions_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        let mut paths = vec![];
        let mut texts = vec![];
        let offset = self.radius.value / 2_f64.sqrt();
        let end = self.center.pos + Vec2::new(offset, -offset);
        let (path, text) = Dimension::new(DimKind::Radius, self.center.pos, end, self.radius.value)
            .get_path_and_pattern();
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
