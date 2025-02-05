use super::helpers::HelperKind;
use super::helpers::HelperKindvars;
use crate::canvas::CanvasText;
use crate::canvas::Pattern;
use crate::dimensions::DimKind;
use crate::dimensions::Dimension;
use crate::math::*;
use crate::pools::HS;
use crate::positions::Status;
use crate::prefab::*;
use crate::GetEntityState;
use crate::KeysStates;
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
    radius_status: Status,

    state: Status,
}
impl HelperCircle {
    const MIN_RADIUS: f64 = 10.;

    pub fn new(center: Vec2, _pos2: Vec2) -> HelperKind {
        let center = Position::new(center);
        let radius = Value::new(0.);
        let mut radius_status = Status::default();
        radius_status.set_hs(HS::Select, true);

        HelperKind::Circle(HelperCircle {
            center,
            radius,
            radius_status,
            state: Status::default(),
        })
    }
    pub fn get_radius(&self) -> f64 {
        self.radius.value
    }

    fn get_circle(&self) -> Circle {
        let center = self.center.pos;
        let radius = self.radius.value;
        Circle::new(center.to_point(), radius)
    }

    fn hs_controls_from_pos(&mut self, pointer: &mut Pointer, _keys_states: KeysStates, hs: HS) {
        // circonference
        let within_grab_radius = |pos: Vec2, center: Vec2, radius: f64| -> bool {
            ((pos - center).hypot() - radius).abs() < Self::GRAB_RADIUS
        };
        self.radius_status.set_hs(
            hs,
            within_grab_radius(pointer.pos(), self.center.pos, self.radius.value),
        );

        // Pointer update
        if self.radius_status.is_hs(hs) {
            pointer.set_pos(
                self.center.pos + (pointer.pos() - self.center.pos).normalize() * self.radius.value,
            );
        }
        if self.radius_status.is_hs(HS::Select) {
            log!("hs_controls_from_pos");
            pointer.save_pos();
        }
    }
}
impl Display for HelperCircle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Helper line")
    }
}

impl ObjectsFuncs for HelperCircle {
    const TOLERANCE: f64 = 0.01;
    const GRAB_RADIUS: f64 = 5.;
    type Kindvars = HelperKindvars;

    fn save_vars(&mut self) {
        self.center.saved_pos = self.center.pos;
        self.radius.saved_val = self.radius.value;
    }
    fn restore_vars(&mut self) {
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
    fn finish_draw(&mut self) -> bool {
        self.radius_status.set_hs(HS::Select, false);
        self.radius_status.set_hs(HS::Highlight, false);
        true
    }
    fn get_state(&self, get: GetEntityState) -> Option<Vec2> {
        use GetEntityState::*;
        match get {
            IsHS(hs) => self.state.is_hs(hs).then(|| self.get_position()),
            GetFirstControlHS(hs) => self.radius_status.is_hs(hs).then(|| self.get_position()),
        }
    }
    fn set_state(&mut self, set: SetEntityState) {
        use SetEntityState::*;
        match set {
            SetHS(hs, value) => self.state.set_hs(hs, value),
            SetAllControlsHS(hs, value) => {
                self.radius_status.set_hs(hs, value);
            }
        }
    }
    fn set_state_from_pos(
        &mut self,
        pointer: &mut Pointer,
        keys_states: KeysStates,
        set: SetEntityStateFromPos,
    ) {
        use SetEntityStateFromPos::*;
        match set {
            SetHSFromPos(hs) => self.state.set_hs(
                hs,
                (pointer.pos() - self.center.pos).hypot() < Self::GRAB_RADIUS,
            ),
            SetControlHSFromPos(hs) => {
                self.hs_controls_from_pos(pointer, keys_states, hs);
            }
        }
    }

    fn toggle_selected_prop(&mut self) {
        ()
    }

    fn move_position(&mut self, pointer: &mut Pointer, _keys_states: KeysStates) -> bool {
        self.center.pos = snap_pt(
            self.center.saved_pos + pointer.dpos(),
            pointer.get_snap().val(),
        );
        true
    }
    fn move_controls(&mut self, pointer: &Pointer, _keys_states: KeysStates) -> bool {
        use HS::*;
        if self.radius_status.is_hs(Select) {
            let radius = snap_val(
                (pointer.pos() - self.center.pos).hypot(),
                pointer.get_snap().val(),
            );
            if radius >= HelperCircle::MIN_RADIUS {
                self.radius.value = radius;
            }
            true
        } else {
            false
        }
    }
    fn get_position(&self) -> Vec2 {
        self.center.pos
    }

    fn get_controls_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        use HS::*;
        let pattern_circle = match (
            self.radius_status.is_hs(Select),
            self.radius_status.is_hs(Highlight),
        ) {
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
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        let mut res = vec![];
        let offset = self.radius.value / 2_f64.sqrt();
        let end = self.center.pos + Vec2::new(offset, -offset);
        let dim = Dimension::new(DimKind::Radius, self.center.pos, end, self.radius.value)
            .get_path_and_pattern();
        res.push(dim);
        res
    }

    fn get_paths_and_patterns(&self, _: &Size, _: (Rect, f64, Vec2)) -> Vec<(BezPath, Pattern)> {
        use HS::*;
        let pattern_center = match (self.state.is_hs(Select), self.state.is_hs(Highlight)) {
            (false, false) => Pattern::HelperNormal,
            (false, true) => Pattern::HelperHighlighted,
            (true, false) => Pattern::HelperSelected,
            (true, true) => Pattern::HelperSelected,
        };
        vec![(
            center_path(self.center.pos, 1., Self::GRAB_RADIUS),
            pattern_center,
        )]
    }
}
