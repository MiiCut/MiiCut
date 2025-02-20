use super::helpers::HelperKind;
use super::helpers::HelperKindvars;
use crate::canvas::CanvasText;
use crate::canvas::Pattern;
use crate::dimensions::DimKind;
use crate::dimensions::Dimension;
use crate::get_line_segment;
use crate::is_near_line;
use crate::math::*;
use crate::pools::HS;
use crate::positions::Status;
use crate::prefab::*;
use crate::traits::*;
use crate::KeysStates;
use crate::Pointer;
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
    angle_status: Status,

    state: Status,
}
impl HelperLine {
    pub fn new(pos1: Vec2, pos2: Vec2) -> Option<HelperKind> {
        if (pos1 - pos2).hypot() < EPSILON {
            return None;
        }
        Some(HelperKind::Line(HelperLine {
            center: Position::new(pos1),
            angle: Value::new((pos2 - pos1).atan2()),
            angle_status: Status::default(),
            state: Status::default(),
        }))
    }
    pub fn get_angle(&self) -> f64 {
        self.angle.value
    }

    fn hs_modifiers_from_pos(&mut self, pointer: &mut Pointer, hs: HS) {
        let state = is_near_line(
            self.center.pos,
            self.angle.value,
            pointer.pos(),
            Self::GRAB_RADIUS,
        );
        self.angle_status.set_hs(hs, state);
    }
}
impl Display for HelperLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Helper line")
    }
}

impl ObjectsFuncs for HelperLine {
    const TOLERANCE: f64 = 0.01;
    const GRAB_RADIUS: f64 = 5.;
    type Kindvars = HelperKindvars;

    fn save_vars(&mut self) {
        self.center.saved_pos = self.center.pos;
        self.angle.saved_val = self.angle.value;
    }
    fn restore_vars(&mut self) {
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

    fn get_state(&self, get: GetEntityState) -> Option<Vec2> {
        use GetEntityState::*;
        match get {
            IsHS(hs) => self.state.is_hs(hs).then(|| self.get_position()),
            GetFirstControlHS(hs) => self.angle_status.is_hs(hs).then(|| self.center.pos),
        }
    }
    fn set_state(&mut self, set: SetEntityState) {
        use SetEntityState::*;
        match set {
            SetHS(hs, value) => self.state.set_hs(hs, value),
            SetAllControlsHS(hs, value) => self.angle_status.set_hs(hs, value),
        }
    }

    fn set_state_from_pos(
        &mut self,
        pointer: &mut Pointer,
        _keys_states: KeysStates,
        set: SetEntityStateFromPos,
    ) {
        use SetEntityStateFromPos::*;
        // A closure to check if pointer is within the grab radius
        let within_grab_radius =
            |pointer: &Pointer, center: Vec2| (pointer.pos() - center).hypot() < Self::GRAB_RADIUS;

        match set {
            SetHSFromPos(hs) => self
                .state
                .set_hs(hs, within_grab_radius(pointer, self.center.pos)),
            SetControlHSFromPos(hs) => {
                self.hs_modifiers_from_pos(pointer, hs);
            }
        }
    }

    fn move_position(&mut self, pointer: &mut Pointer, _keys_states: KeysStates) -> bool {
        self.center.pos = snap_pt(
            self.center.saved_pos + pointer.dpos(),
            pointer.get_snap().val(),
        );
        pointer.set_pos(self.center.pos);
        true
    }
    fn move_controls(&mut self, pointer: &Pointer, _keys_states: KeysStates) -> bool {
        if self.angle_status.is_hs(HS::Select) {
            let angle = (pointer.pos() - self.center.pos).atan2();
            self.angle.value = snap_val(angle / PI * 180., pointer.get_snap().val()) / 180. * PI;
            return true;
        }
        false
    }
    fn get_position(&self) -> Vec2 {
        self.center.pos
    }

    fn get_controls_paths_and_patterns(
        &self,
        _das: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        // Determine the appropriate pattern
        let pattern_line = if self.angle_status.is_hs(HS::Select) {
            Pattern::HelperSelectedCircle
        } else if self.angle_status.is_hs(HS::Highlight) {
            Pattern::HelperHighlightedCircle
        } else {
            Pattern::HelperNormalCircle
        };
        // Extract canvas and scaling information
        let canvas_width = cinfo.0.width();
        let canvas_height = cinfo.0.height();
        let scale_factor = cinfo.1;
        let canvas_offset = cinfo.2;

        // Transform the center position to canvas coordinates
        let canvas_center = to_canvas(self.center.pos, scale_factor, canvas_offset);

        // Compute the line segment points on the canvas
        let (canvas_tl, canvas_br) = get_line_segment(
            &Size::new(canvas_width, canvas_height),
            canvas_center,
            self.angle.value,
        );

        // Convert canvas points to drawing coordinates
        let draw_tl = to_draw(canvas_tl, scale_factor, canvas_offset);
        let draw_br = to_draw(canvas_br, scale_factor, canvas_offset);

        // Construct the path and pattern
        vec![(
            Line::new(draw_tl.to_point(), draw_br.to_point()).to_path(HelperLine::TOLERANCE),
            pattern_line,
        )]
    }

    fn get_dimensions_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern, CanvasText)> {
        let mut res = vec![];
        let end = get_point_at_dist_from_angle(self.center.pos, self.angle.value, 200.);
        let dim = Dimension::new(DimKind::Angle, self.center.pos, end, self.angle.value);
        let dim = dim.get_path_and_pattern();
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
