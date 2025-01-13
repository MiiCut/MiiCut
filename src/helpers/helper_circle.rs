use super::helpers::HelperKind;
use super::helpers::HelperKindvars;
use crate::canvas::CanvasText;
use crate::canvas::Pattern;
use crate::dimensions::DimKind;
use crate::dimensions::Dimension;
use crate::math::*;
use crate::prefab::modifiers_path;
use crate::ObjectsFuncs;
use crate::Position;
use crate::Value;
use crate::HS;
use kurbo::BezPath;
use kurbo::Circle;
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
        radius.select(true);

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
        if (pos - self.center.get_pos()).hypot() < Self::GRAB_RADIUS {
            Some(self.center.get_pos())
        } else {
            None
        }
    }
    fn get_circle(&self) -> Circle {
        let center = self.center.get_pos();
        let radius = self.radius.get_val();
        Circle::new(center.to_point(), radius)
    }
    // fn get_radius_modifier(&self) -> Vec2 {
    //     self.center.get_pos() + Vec2::new(self.radius.get_val(), 0.)
    // }
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
        self.center.save_pos();
        self.radius.save_val();
    }
    fn restore_saved(&mut self) {
        self.center.restore_saved();
        self.radius.restore_saved();
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

    fn set_hs_from_pos(&mut self, pos: Vec2, _snap: f64, hors: HS) -> bool {
        let hs = (self.center.get_pos() - pos).hypot() < Self::GRAB_RADIUS;
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
        let hs: bool = ((pos - self.center.get_pos()).hypot() - self.radius.get_val()).abs()
            < Self::GRAB_RADIUS;
        if hs {
            match hors {
                HS::Highlight => self.radius.highlight(true),
                HS::Select => self.radius.select(true),
            }
            let angle = (pos - self.center.get_pos()).atan2();
            let pos = self.center.get_pos() + Vec2::from_angle(angle) * self.radius.get_val();
            return Some(pos);
        } else {
            match hors {
                HS::Highlight => self.radius.highlight(false),
                HS::Select => self.radius.select(false),
            }
        }
        None
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

    fn move_position(&mut self, dpos: Vec2, snap: f64) {
        self.center
            .set_pos(snap_pt(self.center.get_saved_pos() + dpos, snap));
    }
    fn move_modifier(
        &mut self,
        pos_init: Vec2,
        pos: Vec2,
        snap: f64,
        _shift_pressed: bool,
    ) -> Option<Vec2> {
        let dpos = pos - pos_init;
        let saved_radius = self.radius.get_saved_val();
        let radius = snap_val(saved_radius + dpos.x, snap);
        if radius >= HelperCircle::MIN_RADIUS {
            self.radius.set_val(radius);
        }
        Some(pos)
    }
    fn get_position(&self) -> Vec2 {
        self.center.get_pos()
    }

    fn get_modifiers_paths(&self, _: &Size) -> Vec<(BezPath, Pattern)> {
        let pattern_circle = match (self.radius.is_selected(), self.radius.is_highlighted()) {
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
        let offset = self.radius.get_val() / 2_f64.sqrt();
        let end = self.center.get_pos() + Vec2::new(offset, -offset);
        let (path, text) = Dimension::new(
            DimKind::Radius,
            self.center.get_pos(),
            end,
            self.radius.get_val(),
        )
        .get_path();
        paths.push(path);
        texts.push(text);
        (paths, texts)
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
