// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use super::shapes::{BSKind, BSKindvars};
use crate::{
    canvas::{CanvasText, Pattern},
    dimensions::{DimKind, Dimension},
    math::*,
    positions::{Position, Value},
    prefab::{center_path, modifiers_path},
    traits::*,
};
use geo::{LineString, Polygon};
use kurbo::{
    Arc, ArcAppendIter, BezPath, Line, LinePathIter, PathEl, Point, Rect, Shape, Size, Vec2,
};
use std::{
    f64::consts::{FRAC_PI_2, PI},
    fmt::Display,
};

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeOblong {
    start: Position,
    end: Position,
    width: Value,

    highlighted: bool,
    selected: bool,

    segs: BezPath,
    polygon: Polygon<f64>,
}
impl ShapeOblong {
    const MIN_WIDTH_SIZE: f64 = 4.;
    const MIN_LENGTH_SIZE: f64 = 10.;

    pub fn new(pos1: Vec2, pos2: Vec2) -> BSKind {
        // let pos2 = pos2 + Vec2::new(30., 0.);
        let start = Position::new(pos1, true);
        let mut end = Position::new(pos2, true);
        end.selected = true;
        let width = Value::new(10.);
        BSKind::Oblong(ShapeOblong {
            start,
            end,
            width,
            highlighted: false,
            selected: false,
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
    pub fn get_polygon(&self) -> Polygon<f64> {
        self.polygon.clone()
    }

    fn update_polygon(&mut self) {
        self.segs = calc_segs(self.get_paths(&Size::ZERO));
        self.polygon = calc_polygon(&self.segs);
    }

    fn get_width(&self) -> f64 {
        self.width.value
    }
    fn get_saved_width(&self) -> f64 {
        self.width.saved_val
    }
    fn get_length(&self) -> f64 {
        (self.start.pos - self.end.pos).hypot()
    }
    fn get_arc(&self, start_arc: bool) -> Arc {
        let width = self.get_width();
        let (start, end, width) = (self.start.pos, self.end.pos, width);
        let radius = width / 2.;
        let angle = (end - start).atan2();
        if start_arc {
            Arc::new(
                start.to_point(),
                Vec2::new(radius, radius),
                FRAC_PI_2,
                PI,
                angle,
            )
        } else {
            Arc::new(
                end.to_point(),
                Vec2::new(radius, radius),
                3. * FRAC_PI_2,
                PI,
                angle,
            )
        }
    }
    fn get_line(&self, upper_line: bool) -> Line {
        let start_arc_points = calculate_arc_points(self.get_arc(true));
        let end_arc_points = calculate_arc_points(self.get_arc(false));
        if upper_line {
            Line::new(start_arc_points.1.to_point(), end_arc_points.0.to_point())
        } else {
            Line::new(end_arc_points.1.to_point(), start_arc_points.0.to_point())
        }
    }
    fn get_middle_modifier(&self) -> Vec2 {
        get_middle_from_start_end_positions(self.start.pos, self.end.pos, self.get_width())
    }
    fn get_middle_modifier_saved(&self) -> Vec2 {
        get_middle_from_start_end_positions(
            self.start.saved_pos,
            self.end.saved_pos,
            self.get_saved_width(),
        )
    }
    fn highlight_all_modifiers(&mut self, value: bool) {
        self.start.highlighted = value;
        self.end.highlighted = value;
        self.width.highlighted = value;
    }
    fn select_all_modifiers(&mut self, value: bool) {
        self.start.selected = value;
        self.end.selected = value;
        self.width.selected = value;
    }

    fn highlight_modifiers_from_pos(&mut self, pos: Vec2, grab: f64) {
        self.start.highlighted = (pos - self.start.pos).hypot() < grab;
        self.end.highlighted = (pos - self.end.pos).hypot() < grab;
        self.width.highlighted = (pos - self.get_middle_modifier()).hypot() < grab;
    }
    fn select_modifiers_from_pos(&mut self, pos: Vec2, grab: f64) {
        self.start.selected = (pos - self.start.pos).hypot() < grab;
        self.end.selected = (pos - self.end.pos).hypot() < grab;
        self.width.selected = (pos - self.get_middle_modifier()).hypot() < grab;
    }
}
impl Display for ShapeOblong {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Oblong")
    }
}
impl Shape for ShapeOblong {
    type PathElementsIter<'iter> = ShapeOblongIter;

    fn path_elements(&self, tolerance: f64) -> ShapeOblongIter {
        let arcs_iter = [
            self.get_arc(true).append_iter(tolerance),
            self.get_arc(false).append_iter(tolerance),
        ];
        let lines_iter = [
            self.get_line(false).path_elements(tolerance),
            self.get_line(true).path_elements(tolerance),
        ];

        ShapeOblongIter {
            idx: 0,
            lines_iter,
            arcs_iter,
        }
    }
    #[inline]
    fn area(&self) -> f64 {
        //TODO
        0.
    }
    #[inline]
    fn perimeter(&self, _accuracy: f64) -> f64 {
        //TODO
        0.
    }
    #[inline]
    fn winding(&self, pt: Point) -> i32 {
        compute_winding_number(&self.segs, pt.to_vec2())
    }
    #[inline]
    fn bounding_box(&self) -> Rect {
        //TODO
        Rect::ZERO
    }
    #[inline]
    fn contains(&self, pt: Point) -> bool {
        self.winding(pt) != 0
    }
}
impl ObjectsFuncs for ShapeOblong {
    const TOLERANCE: f64 = 0.01;
    const GRAB_RADIUS: f64 = 5.;
    type Kindvars = BSKindvars;

    fn save_vars(&mut self) {
        self.start.saved_pos = self.start.pos;
        self.end.saved_pos = self.end.pos;
        self.width.saved_val = self.width.value;
    }
    fn restore_saved(&mut self) {
        self.start.pos = self.start.saved_pos;
        self.end.pos = self.end.saved_pos;
        self.width.value = self.width.saved_val;
        self.update_polygon();
    }
    fn get_vars(&self) -> BSKindvars {
        BSKindvars::Oblong(self.start, self.end, self.width)
    }
    fn set_vars(&mut self, vars: &BSKindvars) {
        if let BSKindvars::Oblong(start, end, width) = vars {
            self.start = start.clone();
            self.end = end.clone();
            self.width = width.clone();
            self.update_polygon();
        }
    }

    fn good_size(&self) -> bool {
        self.get_width() >= ShapeOblong::MIN_WIDTH_SIZE - 0.1
            && self.get_length() >= ShapeOblong::MIN_LENGTH_SIZE - 0.1
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
                let select = self.start.selected || self.end.selected || self.width.selected;
                if select {
                    Some(self.get_position())
                } else {
                    None
                }
            }
            IsAnyModifierHighlighted => {
                let highlight =
                    self.start.highlighted || self.end.highlighted || self.width.highlighted;
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
                self.selected = self.contains(pos.to_point());
            }
            SetHighlight(value) => self.highlighted = value,
            HighlightFromPos(pos, ..) => {
                self.highlighted = self.contains(pos.to_point());
            }

            SelectAllModifiers(value) => self.select_all_modifiers(value),
            SelectModifierFromPos(pos, ..) => {
                self.select_modifiers_from_pos(pos, Self::GRAB_RADIUS);
            }

            HighlightAllModifiers(value) => self.highlight_all_modifiers(value),
            HighlightModifierFromPos(pos, ..) => {
                self.highlight_modifiers_from_pos(pos, Self::GRAB_RADIUS);
            }
        }
    }

    fn toggle_prop(&mut self) {
        ()
    }

    fn move_position(&mut self, mut dpos: Vec2, snap: f64) -> Option<Vec2> {
        dpos = snap_pt(dpos, snap);
        self.start.pos = self.start.saved_pos + dpos;
        self.end.pos = self.end.saved_pos + dpos;
        self.update_polygon();
        Some(self.get_position())
    }
    fn move_modifier(
        &mut self,
        pos_init: Vec2,
        pos: Vec2,
        snap: f64,
        _shift_pressed: bool,
    ) -> Option<Vec2> {
        let start_saved = self.start.saved_pos;
        let end_saved = self.end.saved_pos;
        let middle_saved = self.get_middle_modifier_saved();
        let center_saved = (start_saved + end_saved) / 2.;

        let start_sel = self.start.selected;
        let end_sel = self.end.selected;
        let width_sel = self.width.selected;

        let dpos = pos - pos_init;
        let (dpos_proj, _) = project_to_perpendicular(start_saved, end_saved, dpos);

        match (start_sel, end_sel, width_sel) {
            (true, false, false) => {
                let start = snap_pt(start_saved + dpos, snap);
                let length = snap_val((start - end_saved).hypot(), snap);
                let mut angle = snap_angle_hv((end_saved - start).atan2());
                angle = snap_val(angle / PI * 180., snap) / 180. * PI;
                let start = end_saved - Vec2::from_angle(angle) * length;

                if (start - end_saved).hypot() >= ShapeOblong::MIN_LENGTH_SIZE {
                    self.start.pos = start;
                    self.update_polygon();
                    return Some(self.start.pos);
                }
            }
            (false, true, false) => {
                let end = snap_pt(end_saved + dpos, snap);
                let length = snap_val((start_saved - end).hypot(), snap);
                let mut angle = snap_angle_hv((end - start_saved).atan2());
                angle = snap_val(angle / PI * 180., snap) / 180. * PI;
                let end = start_saved + Vec2::from_angle(angle) * length;

                if (end - start_saved).hypot() >= ShapeOblong::MIN_LENGTH_SIZE {
                    self.end.pos = end;
                    self.update_polygon();
                    return Some(self.end.pos);
                }
            }
            (false, false, true) => {
                let (_, dir1) =
                    project_to_perpendicular(start_saved, end_saved, middle_saved - center_saved);
                let middle = middle_saved + dpos_proj;
                let (_, dir2) =
                    project_to_perpendicular(start_saved, end_saved, middle - center_saved);

                let width = snap_val((middle - center_saved).hypot() * 2., snap);
                if width >= ShapeOblong::MIN_WIDTH_SIZE && dir1 * dir2 > 0. {
                    self.width.value = width;
                    self.update_polygon();
                    return Some(self.get_middle_modifier());
                }
            }
            _ => (),
        }
        None
    }
    fn get_position(&self) -> Vec2 {
        (self.start.pos + self.end.pos) / 2.
    }

    fn get_mod_paths_and_patterns(
        &self,
        _: &Size,
        _: (Rect, f64, Vec2),
    ) -> Vec<(BezPath, Pattern)> {
        vec![
            (
                modifiers_path(self.start.pos, 1., ShapeOblong::GRAB_RADIUS),
                self.get_pattern_status(self.start.selected, self.start.highlighted),
            ),
            (
                modifiers_path(self.end.pos, 1., ShapeOblong::GRAB_RADIUS),
                self.get_pattern_status(self.end.selected, self.end.highlighted),
            ),
            (
                modifiers_path(self.get_middle_modifier(), 1., ShapeOblong::GRAB_RADIUS),
                self.get_pattern_status(self.width.selected, self.width.highlighted),
            ),
            (
                center_path(
                    (self.start.pos + self.end.pos) / 2.,
                    1.,
                    ShapeOblong::GRAB_RADIUS,
                ),
                self.get_pattern_status(self.selected, self.highlighted),
            ),
        ]
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        let mut paths = vec![];
        let mut texts = vec![];
        let start = self.start.pos;
        let end = self.end.pos;
        let length = self.get_length();
        let width = self.get_width();
        let middle1_pt = self.get_middle_modifier();
        let middle2_pt = symmetric_point_to_segment(start, end, self.get_middle_modifier());

        let mut dim = Dimension::new(DimKind::Linear, start, end, length);
        dim.set_dim_offset(width / 2. + 6.);
        let (path, text) = dim.get_path();
        paths.push(path);
        texts.push(text);

        let mut dim = Dimension::new(DimKind::Linear, middle1_pt, middle2_pt, width);
        dim.set_dim_offset(length / 2. + width / 2. + 6.);
        let (path, text) = dim.get_path();
        paths.push(path);
        texts.push(text);

        let dim = Dimension::new(DimKind::Angle, start, end, 0.);
        let (path, text) = dim.get_path();
        paths.push(path);
        texts.push(text);

        (paths, texts)
    }
    fn get_paths(&self, _: &Size) -> Vec<BezPath> {
        let mut paths: Vec<BezPath> = vec![];
        paths.push(self.get_line(true).to_path(ShapeOblong::TOLERANCE));
        paths.push(self.get_arc(false).to_path(ShapeOblong::TOLERANCE));
        paths.push(self.get_line(false).to_path(ShapeOblong::TOLERANCE));
        paths.push(self.get_arc(true).to_path(ShapeOblong::TOLERANCE));
        paths
    }
    fn get_paths_and_patterns(&self, das: &Size, _: (Rect, f64, Vec2)) -> Vec<(BezPath, Pattern)> {
        let pattern = match (self.selected, self.highlighted) {
            (false, false) => Pattern::BasicNormal,
            (false, true) => Pattern::BasicHighlighted,
            (true, false) => Pattern::BasicSelected,
            (true, true) => Pattern::BasicSelected,
        };

        let mut paths = self.get_paths(das);
        let result = paths
            .iter_mut()
            .map(|path| (path.clone(), pattern))
            .collect();
        result
    }
}

#[doc(hidden)]
pub struct ShapeOblongIter {
    idx: usize,
    arcs_iter: [ArcAppendIter; 2],
    lines_iter: [LinePathIter; 2],
    // i:
    // 0: lines_iter[0]
    // 1: arcs_iter[0]/lines_iter[1]
    // 2: arcs_iter[1]
}
impl Iterator for ShapeOblongIter {
    type Item = PathEl;

    fn next(&mut self) -> Option<PathEl> {
        match self.idx {
            0 => match self.lines_iter[0].next() {
                Some(elem) => Some(elem),
                None => {
                    self.idx += 1;
                    self.arcs_iter[0].next()
                }
            },
            1 => match self.arcs_iter[0].next() {
                Some(elem) => Some(elem),
                None => {
                    self.idx += 1;
                    self.lines_iter[1].next(); // Skip MoveTo
                    self.lines_iter[1].next()
                }
            },
            2 => self.arcs_iter[1].next(),
            _ => None,
        }
    }
}
