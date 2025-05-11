use super::helpers::HelperKind;
use super::helpers::HelperKindvars;
use crate::canvas::CanvasText;
use crate::canvas::Colors;
use crate::canvas::Pattern;
use crate::dimensions::dim_linear_angle;
use crate::math::*;
use crate::pools::HS;
use crate::positions::Status;
use crate::prefab::*;
use crate::traits::*;
use crate::KeysStates;
use crate::Pointer;
use kurbo::BezPath;
use kurbo::Rect;
use kurbo::Size;
use kurbo::Vec2;
use std::fmt::Debug;
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq)]
pub struct HelperSegment {
    bdl: SegBundle,
    bdl_saved: SegBundle,
    state: Status,
    state_start: Status,
    state_end: Status,
}
impl HelperSegment {
    pub fn new(start: Vec2, end: Vec2) -> Option<HelperKind> {
        SegBundle::new(start, end).and_then(|bdl| {
            Some(HelperKind::Segment(HelperSegment {
                bdl,
                bdl_saved: bdl,
                state: Status::default(),
                state_start: Status::default(),
                state_end: Status::default(),
            }))
        })
    }
    pub fn get_seg_bdl(&self) -> SegBundle {
        self.bdl
    }
}
impl Display for HelperSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Helper line")
    }
}

impl ObjectsFuncs for HelperSegment {
    const TOLERANCE: f64 = 0.01;
    const GRAB_RADIUS: f64 = 5.;
    type Kindvars = HelperKindvars;

    fn tab(&mut self) -> bool {
        false
    }
    fn save_vars(&mut self) {
        self.bdl_saved = self.bdl;
    }
    fn restore_vars(&mut self) {
        self.bdl = self.bdl_saved;
    }
    fn get_vars(&self) -> HelperKindvars {
        HelperKindvars::Segment(self.bdl)
    }
    fn set_vars(&mut self, vars: &HelperKindvars) {
        if let HelperKindvars::Segment(bdl) = vars {
            self.bdl = bdl.clone();
        }
    }

    fn get_state(&self, get: GetEntityState) -> bool {
        use GetEntityState::*;
        match get {
            IsHS(hs) => self.state.is_hs(hs),
            IsAControlHS(hs) => self.state_start.is_hs(hs) || self.state_end.is_hs(hs),
        }
    }
    fn set_state(&mut self, set: SetEntityState) {
        use SetEntityState::*;
        match set {
            SetHS(hs, value) => self.state.set_hs(hs, value),
            SetAllControlsHS(hs, value) => {
                self.state_start.set_hs(hs, value);
                self.state_end.set_hs(hs, value);
            }
        }
    }
    fn set_state_from_pos(
        &mut self,
        pointer: &mut Pointer,
        _keys_states: KeysStates,
        set: SetEntityStateFromPos,
    ) -> bool {
        use SetEntityStateFromPos::*;
        match set {
            SetHSFromPos(hs) => {
                if let Some((dist, _)) = distance_and_projection_to_segment(
                    self.bdl.s(),
                    self.bdl.e(),
                    pointer.pos(),
                    Self::GRAB_RADIUS,
                ) {
                    let in_radius = dist < Self::GRAB_RADIUS;
                    self.state.set_hs(hs, in_radius);
                    in_radius
                } else {
                    self.state.set_hs(hs, false);
                    false
                }
            }
            SetControlHSFromPos(hs) => {
                let pos = pointer.pos();
                self.state_start
                    .set_hs(hs, (self.bdl.s() - pos).hypot() < Self::GRAB_RADIUS);
                self.state_end
                    .set_hs(hs, (self.bdl.e() - pos).hypot() < Self::GRAB_RADIUS);
                self.state_end.is_hs(hs) || self.state_start.is_hs(hs)
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
        let snap = pointer.get_snap().val();
        let snap_angle = pointer.get_snap_angle().val();
        if self.state_start.is_hs(HS::Select) {
            if !pointer.is_magnetized() {
                let s = snap_length_and_angle(self.bdl.e(), pointer.pos(), snap, snap_angle);
                return self.bdl.try_set_s(s);
            } else {
                return self.bdl.try_set_s(pointer.pos());
            }
        }
        if self.state_end.is_hs(HS::Select) {
            if !pointer.is_magnetized() {
                let e = snap_length_and_angle(self.bdl.s(), pointer.pos(), snap, snap_angle);
                return self.bdl.try_set_e(e);
            } else {
                return self.bdl.try_set_e(pointer.pos());
            }
        }
        false
    }
    fn get_position(&self) -> Vec2 {
        self.bdl.m()
    }
    fn get_centroid(&self) -> Vec<Vec2> {
        vec![]
    }
    fn get_controls_paths_and_patterns(
        &self,
        _das: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, Colors)> {
        let mut res = vec![];
        res.push((
            point_path(self.bdl.s(), cinfo.1),
            Pattern::Point,
            get_helpers_colors(self.state),
        ));
        res.push((
            point_path(self.bdl.e(), cinfo.1),
            Pattern::Point,
            get_helpers_colors(self.state),
        ));
        res
    }
    fn get_dimensions_paths_and_patterns(
        &self,
        _: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, Colors, Vec<CanvasText>)> {
        vec![dim_linear_angle(self.bdl, cinfo, self.state)]
    }
    fn get_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, Colors)> {
        vec![(
            line_path(self.bdl.s(), self.bdl.e()),
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
