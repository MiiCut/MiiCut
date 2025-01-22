use super::helpers::HelperKind;
use super::helpers::HelperKindvars;
use crate::canvas::CanvasText;
use crate::canvas::Pattern;
use crate::dimensions::DimKind;
use crate::dimensions::Dimension;
use crate::get_line_segment;
use crate::is_near_line;
use crate::math::*;
use crate::prefab::*;
use crate::traits::*;
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
    pub fn get_angle(&self) -> f64 {
        self.angle.value
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
        if self.angle.selected {
            let angle = (pointer.pos() - self.center.pos).atan2();
            self.angle.value = snap_val(angle / PI * 180., pointer.get_snap().val()) / 180. * PI;
            return true;
        }
        false
    }
    fn get_position(&self) -> Vec2 {
        self.center.pos
    }

    fn get_mod_paths_and_patterns(
        &self,
        _das: &Size,
        cinfo: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        // Determine the appropriate pattern
        let pattern_line = if self.angle.selected {
            Pattern::HelperSelectedCircle
        } else if self.angle.highlighted {
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
    ) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        let mut paths = vec![];
        let mut texts = vec![];
        let end = get_point_at_dist_from_angle(self.center.pos, self.angle.value, 200.);
        let dim = Dimension::new(DimKind::Angle, self.center.pos, end, self.angle.value);
        let (path, text) = dim.get_path_and_pattern();
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
