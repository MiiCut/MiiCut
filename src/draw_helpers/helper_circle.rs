use super::helpers::HelperKind;
use super::helpers::HelperKindFuncs;
use super::helpers::HelperKindvars;
use crate::canvas::CanvasText;
use crate::canvas::Pattern;
use crate::prefab::modifiers_path;
use crate::Position;
use crate::Value;
use crate::HS;
use kurbo::BezPath;
use kurbo::Vec2;
use std::fmt::Debug;
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq)]
pub struct HelperCircle {
    position: Position,
    radius: Value,

    highlighted: bool,
    selected: bool,
}
impl HelperCircle {
    const MIN_RADIUS: f64 = 2.;

    pub fn new(position: Vec2, _pos2: Vec2) -> HelperKind {
        let position = Position::new(position, true);
        let mut radius = Value::new(HelperCircle::MIN_RADIUS);
        radius.select(true);
        HelperKind::Circle(HelperCircle {
            position,
            radius,
            highlighted: false,
            selected: false,
        })
    }
    fn get_modifier(&self) -> Vec2 {
        self.position.get_pos() + Vec2::new(self.radius.get_val(), 0.)
    }
}
impl Display for HelperCircle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Helper line")
    }
}

impl HelperKindFuncs for HelperCircle {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 2.;

    fn save_vars(&mut self) {
        self.position.save_pos();
        self.radius.save_val();
    }
    fn restore_saved(&mut self) {
        self.position.restore_saved();
        self.radius.restore_saved();
    }
    fn get_vars(&self) -> HelperKindvars {
        HelperKindvars::Line(self.position, self.radius)
    }
    fn set_vars(&mut self, vars: &HelperKindvars) {
        if let HelperKindvars::Line(position, radius) = vars {
            self.position = position.clone();
            self.radius = radius.clone();
        }
    }
    fn good_size(&self) -> bool {
        true
    }

    fn set_hs_from_pos(&mut self, pos: Vec2, hors: HS) -> bool {
        let hs = (self.position.get_pos() - pos).hypot() < Self::GRAB;
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

    fn set_hs_modifiers_from_pos(&mut self, pos: Vec2, hors: HS) -> bool {
        let mod_hs = (self.get_modifier() - pos).hypot() < Self::GRAB;
        match hors {
            HS::Highlight => {
                self.radius.highlight(mod_hs);
                self.radius.is_highlighted()
            }
            HS::Select => {
                self.radius.select(mod_hs);
                self.radius.is_selected()
            }
        }
    }
    fn set_hs_modifiers(&mut self, value: bool, hors: HS) {
        match hors {
            HS::Highlight => {
                self.radius.highlight(value);
            }
            HS::Select => {
                self.radius.select(value);
            }
        }
    }
    fn get_hs_modifiers(&self, hors: HS) -> bool {
        match hors {
            HS::Highlight => self.radius.is_highlighted(),
            HS::Select => self.radius.is_selected(),
        }
    }

    fn toggle_prop(&mut self) {
        ()
    }

    fn move_position(&mut self, dpos: Vec2) {
        self.position.set_pos(self.position.get_saved_pos() + dpos);
    }
    fn move_modifier(&mut self, pos_init: Vec2, pos: Vec2, _shift_pressed: bool) -> bool {
        self.radius
            .set_val(self.radius.get_saved_val() + (pos.x - pos_init.x));
        true
    }
    fn get_position(&self) -> Vec2 {
        self.position.get_pos()
    }

    fn get_modifiers_paths(&self) -> Vec<(BezPath, Pattern)> {
        vec![
            (
                modifiers_path(self.get_modifier(), 1., HelperCircle::GRAB),
                self.get_pattern_modifiers(self.radius.is_selected(), self.radius.is_highlighted()),
            ),
            (
                modifiers_path(self.position.get_pos(), 1., HelperCircle::GRAB),
                self.get_pattern_modifiers(
                    self.position.is_selected(),
                    self.position.is_highlighted(),
                ),
            ),
        ]
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        (vec![], vec![])
    }
    fn get_paths(&self) -> Vec<BezPath> {
        vec![modifiers_path(
            self.position.get_pos(),
            1.,
            HelperCircle::GRAB,
        )]
    }
}
