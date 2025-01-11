use super::helpers::HelperKind;
use super::helpers::HelperKindvars;
use crate::canvas::CanvasText;
use crate::canvas::Pattern;
use crate::prefab::*;
use crate::snap_pt;
use crate::traits::*;
use crate::Position;
use crate::HS;
use kurbo::BezPath;
use kurbo::Size;
use kurbo::Vec2;
use std::fmt::Debug;
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq)]
pub struct HelperPoint {
    position: Position,

    highlighted: bool,
    selected: bool,
}
impl HelperPoint {
    pub fn new(position: Vec2, _pos2: Vec2) -> HelperKind {
        let mut position = Position::new(position, true);
        position.select(true);
        HelperKind::Point(HelperPoint {
            position,
            highlighted: false,
            selected: false,
        })
    }
    pub fn magnet_to(&self, pos: Vec2) -> Option<Vec2> {
        if self.selected {
            return None;
        }
        if (pos - self.position.get_pos()).hypot() < 2. * Self::GRAB_RADIUS {
            Some(self.position.get_pos())
        } else {
            None
        }
    }
}
impl Display for HelperPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Helper point")
    }
}

impl ObjectsFuncs for HelperPoint {
    const TOLERANCE: f64 = 0.01;
    const GRAB_RADIUS: f64 = 4.;
    type Kindvars = HelperKindvars;

    fn save_vars(&mut self) {
        self.position.save_pos();
    }
    fn restore_saved(&mut self) {
        self.position.restore_saved();
    }
    fn get_vars(&self) -> HelperKindvars {
        HelperKindvars::Point(self.position)
    }
    fn set_vars(&mut self, vars: &HelperKindvars) {
        if let HelperKindvars::Point(position) = vars {
            self.position = position.clone();
        }
    }
    fn good_size(&self) -> bool {
        true
    }

    fn set_hs_from_pos(&mut self, pos: Vec2, _snap: f64, hors: HS) -> bool {
        match hors {
            HS::Highlight => {
                self.highlighted = (pos - self.position.get_pos()).hypot() < Self::GRAB_RADIUS;
                self.highlighted
            }
            HS::Select => {
                self.selected = (pos - self.position.get_pos()).hypot() < Self::GRAB_RADIUS;
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

    fn set_hs_modifiers_from_pos(&mut self, _pos: Vec2, _snap: f64, _hors: HS) -> Option<Vec2> {
        None
    }
    fn set_hs_modifiers(&mut self, _value: bool, _hors: HS) {}
    fn get_hs_modifiers(&self, _hors: HS) -> bool {
        false
    }

    fn toggle_prop(&mut self) {
        ()
    }

    fn move_position(&mut self, dpos: Vec2, snap: f64) {
        self.position
            .set_pos(snap_pt(self.position.get_saved_pos() + dpos, snap));
    }
    fn move_modifier(
        &mut self,
        _pos_init: Vec2,
        _pos: Vec2,
        _snap: f64,
        _shift_pressed: bool,
    ) -> Option<Vec2> {
        None
    }
    fn get_position(&self) -> Vec2 {
        self.position.get_pos()
    }

    fn get_modifiers_paths(&self, _: &Size) -> Vec<(BezPath, Pattern)> {
        vec![]
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        (vec![], vec![])
    }
    fn get_paths(&self, _: &Size) -> Vec<BezPath> {
        vec![helper_point_path(
            self.position.get_pos(),
            Self::GRAB_RADIUS,
        )]
    }
}
