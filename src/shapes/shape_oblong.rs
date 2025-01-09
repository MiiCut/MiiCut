// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

use crate::{
    canvas::{CanvasText, Pattern},
    dimensions::{DimKind, Dimension},
    math::*,
    positions::{Position, Value, HS},
    prefab::{center_path, modifiers_path},
};
use geo::{LineString, Polygon};
use kurbo::{Arc, ArcAppendIter, BezPath, Line, LinePathIter, PathEl, Point, Rect, Shape, Vec2};
use std::{
    f64::consts::{FRAC_PI_2, PI},
    fmt::Display,
};

use super::shapes::{ShapeKind, ShapeKindFuncs, ShapeKindvars};

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeOblongVars {
    start: Position,
    end: Position,
    width: Value,
}
impl ShapeOblongVars {
    pub fn save(&mut self) {
        self.start.save_pos();
        self.end.save_pos();
        self.width.save_val();
    }
    pub fn restore_saved(&mut self) {
        self.start.restore_saved();
        self.end.restore_saved();
        self.width.restore_saved();
    }
    pub fn highlight(&mut self, value: bool) {
        self.start.highlight(value);
        self.end.highlight(value);
        self.width.highlight(value);
    }
    pub fn select(&mut self, value: bool) {
        self.start.select(value);
        self.end.select(value);
        self.width.select(value);
    }
    pub fn move_position(&mut self, dpos: Vec2) {
        self.start.set_pos(self.start.get_saved_pos() + dpos);
        self.end.set_pos(self.end.get_saved_pos() + dpos);
    }
}

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
    const MIN_WIDTH_SIZE: f64 = 2.;
    const MIN_LENGTH_SIZE: f64 = 10.;

    pub fn new(pos1: Vec2, pos2: Vec2) -> ShapeKind {
        // let pos2 = pos2 + Vec2::new(30., 0.);
        let start = Position::new(pos1, true);
        let mut end = Position::new(pos2, true);
        end.select(true);
        let width = Value::new(10.);
        ShapeKind::Oblong(ShapeOblong {
            start,
            end,
            width,
            highlighted: false,
            selected: false,
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
    fn update_polygon(&mut self) {
        self.segs = calc_segs(self.get_paths());
        self.polygon = calc_polygon(&self.segs);
    }
    fn get_width(&self) -> f64 {
        self.width.get_val()
    }
    fn get_saved_width(&self) -> f64 {
        self.width.get_saved_val()
    }
    fn get_length(&self) -> f64 {
        (self.start.get_pos() - self.end.get_pos()).hypot()
    }
    fn get_arc(&self, start_arc: bool) -> Arc {
        let width = self.get_width();
        let (start, end, width) = (self.start.get_pos(), self.end.get_pos(), width);
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
        get_middle_from_start_end_positions(
            self.start.get_pos(),
            self.end.get_pos(),
            self.get_width(),
        )
    }
    fn get_middle_modifier_saved(&self) -> Vec2 {
        get_middle_from_start_end_positions(
            self.start.get_saved_pos(),
            self.end.get_saved_pos(),
            self.get_saved_width(),
        )
    }
}
impl Display for ShapeOblong {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Oblong")
    }
}
impl Shape for ShapeOblong {
    type PathElementsIter<'iter> = CShapeOblongIter;

    fn path_elements(&self, tolerance: f64) -> CShapeOblongIter {
        let arcs_iter = [
            self.get_arc(true).append_iter(tolerance),
            self.get_arc(false).append_iter(tolerance),
        ];
        let lines_iter = [
            self.get_line(false).path_elements(tolerance),
            self.get_line(true).path_elements(tolerance),
        ];

        CShapeOblongIter {
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

impl ShapeKindFuncs for ShapeOblong {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 2.;

    fn save_vars(&mut self) {
        self.start.save_pos();
        self.end.save_pos();
        self.width.save_val();
    }
    fn restore_saved(&mut self) {
        self.start.restore_saved();
        self.end.restore_saved();
        self.width.restore_saved();
        self.update_polygon();
    }
    fn get_vars(&self) -> ShapeKindvars {
        ShapeKindvars::Oblong(self.start, self.end, self.width)
    }
    fn set_vars(&mut self, vars: &ShapeKindvars) {
        if let ShapeKindvars::Oblong(start, end, width) = vars {
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

    fn set_hs_from_pos(&mut self, pos: Vec2, hors: HS) -> bool {
        match hors {
            HS::Highlight => {
                self.highlighted = self.contains(pos.to_point());
                self.highlighted
            }
            HS::Select => {
                self.selected = self.contains(pos.to_point());
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
        let start_hors = (pos - self.start.get_pos()).hypot() < Self::GRAB;
        let end_hors = (pos - self.end.get_pos()).hypot() < Self::GRAB;
        let middle_hors = (pos - self.get_middle_modifier()).hypot() < Self::GRAB;
        match hors {
            HS::Highlight => {
                self.start.highlight(start_hors);
                self.end.highlight(end_hors);
                self.width.highlight(middle_hors);
                self.start.is_highlighted()
                    || self.end.is_highlighted()
                    || self.width.is_highlighted()
            }
            HS::Select => {
                self.start.select(start_hors);
                self.end.select(end_hors);
                self.width.select(middle_hors);
                self.start.is_selected() || self.end.is_selected() || self.width.is_selected()
            }
        }
    }
    fn set_hs_modifiers(&mut self, value: bool, hors: HS) {
        match hors {
            HS::Highlight => {
                self.start.highlight(value);
                self.end.highlight(value);
                self.width.highlight(value);
            }
            HS::Select => {
                self.start.select(value);
                self.end.select(value);
                self.width.select(value);
            }
        }
    }
    fn get_hs_modifiers(&self, hors: HS) -> bool {
        match hors {
            HS::Highlight => {
                self.start.is_highlighted()
                    || self.end.is_highlighted()
                    || self.width.is_highlighted()
            }
            HS::Select => {
                self.start.is_selected() || self.end.is_selected() || self.width.is_selected()
            }
        }
    }

    fn toggle_prop(&mut self) {
        ()
    }

    fn move_position(&mut self, dpos: Vec2) {
        self.start.set_pos(self.start.get_saved_pos() + dpos);
        self.end.set_pos(self.end.get_saved_pos() + dpos);
        self.update_polygon();
    }
    fn move_modifier(&mut self, pos_init: Vec2, pos: Vec2, _shift_pressed: bool) -> bool {
        let start_saved = self.start.get_saved_pos();
        let end_saved = self.end.get_saved_pos();
        let middle_saved = self.get_middle_modifier_saved();
        let center_saved = (start_saved + end_saved) / 2.;

        let start_sel = self.start.is_selected();
        let end_sel = self.end.is_selected();
        let width_sel = self.width.is_selected();

        let dpos = pos - pos_init;
        let (dpos_proj, _) = project_to_perpendicular(start_saved, end_saved, dpos);

        match (start_sel, end_sel, width_sel) {
            (true, false, false) => {
                let start = start_saved + dpos;
                if (start - end_saved).hypot() >= ShapeOblong::MIN_LENGTH_SIZE {
                    self.start.set_pos(start);
                    self.update_polygon();
                    return true;
                }
            }
            (false, true, false) => {
                let end = end_saved + dpos;
                if (end - start_saved).hypot() >= ShapeOblong::MIN_LENGTH_SIZE {
                    self.end.set_pos(end);
                    self.update_polygon();
                    return true;
                }
            }
            (false, false, true) => {
                let (_, dir1) =
                    project_to_perpendicular(start_saved, end_saved, middle_saved - center_saved);
                let middle = middle_saved + dpos_proj;
                let (_, dir2) =
                    project_to_perpendicular(start_saved, end_saved, middle - center_saved);

                if (middle - center_saved).hypot() >= ShapeOblong::MIN_WIDTH_SIZE / 2.
                    && dir1 * dir2 > 0.
                {
                    self.width.set_val((middle - center_saved).hypot() * 2.);
                    self.update_polygon();
                    return true;
                }
            }
            _ => (),
        }
        false
    }
    fn get_position(&self) -> Vec2 {
        (self.start.get_pos() + self.end.get_pos()) / 2.
    }

    fn get_modifiers_paths(&self) -> Vec<(BezPath, Pattern)> {
        vec![
            (
                modifiers_path(self.start.get_pos(), 1., ShapeOblong::GRAB),
                self.get_pattern_modifiers(self.start.is_selected(), self.start.is_highlighted()),
            ),
            (
                modifiers_path(self.end.get_pos(), 1., ShapeOblong::GRAB),
                self.get_pattern_modifiers(self.end.is_selected(), self.end.is_highlighted()),
            ),
            (
                modifiers_path(self.get_middle_modifier(), 1., ShapeOblong::GRAB),
                self.get_pattern_modifiers(self.width.is_selected(), self.width.is_highlighted()),
            ),
            (
                center_path(
                    (self.start.get_pos() + self.end.get_pos()) / 2.,
                    1.,
                    ShapeOblong::GRAB,
                ),
                self.get_pattern_modifiers(self.selected, self.highlighted),
            ),
        ]
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        let mut paths = vec![];
        let mut texts = vec![];

        let mut dim = Dimension::new(DimKind::Linear, self.start.get_pos(), self.end.get_pos());
        dim.set_dim_offset(self.get_width() / 2. + 10.);
        let (path, text) = dim.get_path();
        paths.push(path);
        texts.push(text);

        let mut dim = Dimension::new(
            DimKind::Linear,
            self.get_middle_modifier(),
            symmetric_point_to_segment(
                self.start.get_pos(),
                self.end.get_pos(),
                self.get_middle_modifier(),
            ),
        );
        dim.set_dim_offset(10.);
        let (path, text) = dim.get_path();
        paths.push(path);
        texts.push(text);

        (paths, texts)
    }
    fn get_paths(&self) -> Vec<BezPath> {
        let mut paths: Vec<BezPath> = vec![];
        paths.push(self.get_line(true).to_path(ShapeOblong::TOLERANCE));
        paths.push(self.get_arc(false).to_path(ShapeOblong::TOLERANCE));

        paths.push(self.get_line(false).to_path(ShapeOblong::TOLERANCE));
        paths.push(self.get_arc(true).to_path(ShapeOblong::TOLERANCE));
        paths
    }
    fn get_polygon(&self) -> Polygon<f64> {
        self.polygon.clone()
    }
}

#[doc(hidden)]
pub struct CShapeOblongIter {
    idx: usize,
    arcs_iter: [ArcAppendIter; 2],
    lines_iter: [LinePathIter; 2],
    // i:
    // 0: lines_iter[0]
    // 1: arcs_iter[0]/lines_iter[1]
    // 2: arcs_iter[1]
}
impl Iterator for CShapeOblongIter {
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
