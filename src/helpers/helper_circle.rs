use super::helpers::HelperKind;
use super::helpers::HelperKindvars;
use crate::canvas::CanvasText;
use crate::canvas::Color;
use crate::canvas::Colors;
use crate::canvas::Pattern;
use crate::dimensions::dim_radius;
use crate::math::*;
use crate::pools::HS;
use crate::positions::Status;
use crate::prefab::*;
use crate::GetEntityState;
use crate::KeysStates;
use crate::ObjectsFuncs;
use crate::Pointer;
use crate::SetEntityState;
use crate::SetEntityStateFromPos;
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
    bdl: SegBundle,
    bdl_saved: SegBundle,
    radius_state: Status,
    state: Status,
}
impl HelperCircle {
    const MIN_RADIUS: f64 = 10.;

    pub fn new(center: Vec2, radius_pt: Vec2) -> Option<HelperKind> {
        SegBundle::new(center, radius_pt).and_then(|bdl| {
            Some(HelperKind::Circle(HelperCircle {
                bdl,
                bdl_saved: bdl,
                radius_state: Status::default(),
                state: Status::default(),
            }))
        })
    }
    pub fn get_radius(&self) -> f64 {
        self.bdl.len()
    }

    fn get_circle(&self) -> Circle {
        let center = self.bdl.s();
        let radius = self.bdl.len();
        Circle::new(center.to_point(), radius)
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
        self.bdl_saved = self.bdl;
    }
    fn restore_vars(&mut self) {
        self.bdl = self.bdl_saved;
    }
    fn get_vars(&self) -> HelperKindvars {
        HelperKindvars::Circle(self.bdl)
    }
    fn set_vars(&mut self, vars: &HelperKindvars) {
        if let HelperKindvars::Circle(bdl) = vars {
            self.bdl = bdl.clone();
        }
    }
    fn get_state(&self, get: GetEntityState) -> bool {
        use GetEntityState::*;
        match get {
            IsHS(hs) => self.state.is_hs(hs),
            IsAControlHS(hs) => self.radius_state.is_hs(hs),
        }
    }
    fn set_state(&mut self, set: SetEntityState) {
        use SetEntityState::*;
        match set {
            SetHS(hs, value) => self.state.set_hs(hs, value),
            SetAllControlsHS(hs, value) => {
                self.radius_state.set_hs(hs, value);
            }
        }
    }
    fn set_state_from_pos(
        &mut self,
        pointer: &mut Pointer,
        keys_states: KeysStates,
        set: SetEntityStateFromPos,
    ) -> bool {
        use SetEntityStateFromPos::*;
        match set {
            SetHSFromPos(hs) => {
                let state = (pointer.pos() - self.bdl.s()).hypot() < Self::GRAB_RADIUS;
                self.state.set_hs(hs, state);
                state
            }
            SetControlHSFromPos(hs) => {
                use HS::*;
                // circonference
                if ((pointer.pos() - self.bdl.s()).hypot() - self.bdl.len()).abs()
                    < Self::GRAB_RADIUS
                {
                    self.radius_state.set_hs(hs, true);
                    if !keys_states.alt_pressed {
                        pointer.set_pos(
                            self.bdl.s()
                                + (pointer.pos() - self.bdl.s()).normalize() * self.bdl.len(),
                        );
                        if self.radius_state.is_hs(Select) {
                            pointer.save_pos();
                        }
                        pointer.set_magnetized(true);
                    }
                    true
                } else {
                    false
                }
            }
        }
    }
    fn contains_pointer(&self, _pointer: &Pointer) -> bool {
        false
    }

    fn move_position(&mut self, pointer: &mut Pointer, _keys_states: KeysStates) -> bool {
        let mut set = self.bdl.try_set_s(snap_pt(
            self.bdl_saved.s() + pointer.dpos(),
            pointer.get_snap().val(),
        ));
        set |= self.bdl.try_set_e(snap_pt(
            self.bdl_saved.e() + pointer.dpos(),
            pointer.get_snap().val(),
        ));
        set
    }
    fn move_controls(&mut self, pointer: &Pointer, _keys_states: KeysStates) -> bool {
        use HS::*;
        if self.radius_state.is_hs(Select) {
            let radius_pt = snap_length(self.bdl.s(), pointer.pos(), pointer.get_snap().val());
            if (radius_pt - self.bdl.s()).hypot() >= HelperCircle::MIN_RADIUS {
                self.bdl
                    .try_set_e(snap_pt(radius_pt, pointer.get_snap().val()));
            }
            true
        } else {
            false
        }
    }
    fn get_position(&self) -> Vec2 {
        self.bdl.s()
    }

    fn get_controls_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, Colors)> {
        vec![
            ((
                self.get_circle().to_path(Self::TOLERANCE),
                Pattern::Helper,
                get_helpers_colors(self.state),
            )),
        ]
    }
    fn get_dimensions_paths_and_patterns(
        &self,
        _: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, Colors, Vec<CanvasText>)> {
        vec![dim_radius(self.bdl, cinfo, self.radius_state)]
    }
    fn get_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, Colors)> {
        vec![(
            center_path(self.bdl.s(), 1., Self::GRAB_RADIUS),
            Pattern::Helper,
            get_helpers_colors(self.state),
        )]
    }
    fn get_prim_paths_and_patterns(
        &self,
        _das: &Size,
        _cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, Colors)> {
        vec![]
    }
}
