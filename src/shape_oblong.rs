// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

use crate::{
    canvas_core::Pattern,
    math::*,
    shapes::{ShapeKind, Shapes},
    sub_shapes::Position,
};
use kurbo::{
    flatten, Arc, ArcAppendIter, BezPath, Line, LinePathIter, PathEl, Point, Rect, Shape, Vec2,
};
use std::{
    f64::consts::{FRAC_PI_2, PI},
    fmt::Display,
};
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ShapeOblong {
    start: Position,
    end: Position,
    width: f64,
    width_saved: f64,

    arc_start_highlighted: bool,
    arc_end_highlighted: bool,
    line_up_highlighted: bool,
    line_down_highlighted: bool,

    arc_start_selected: bool,
    arc_end_selected: bool,
    line_up_selected: bool,
    line_down_selected: bool,

    highlighted: bool,
    selected: bool,
}
impl ShapeOblong {
    const MIN_SIZE: f64 = 10.;
    const GRAB: f64 = 2.;

    fn get_arc(&self, start_arc: bool) -> Arc {
        let (start, end, width) = (self.start.get_pos(), self.end.get_pos(), self.width);
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
    fn get_segs(&self, path: BezPath) -> BezPath {
        let mut segs = BezPath::new();
        flatten(path, 0.25, |s| segs.push(s));
        segs
    }
    fn get_segs_all(&self) -> BezPath {
        let mut segs = BezPath::new();
        segs.extend(self.get_segs(self.get_line(true).to_path(ShapeOblong::TOLERANCE)));
        segs.extend(self.get_segs(self.get_arc(false).to_path(ShapeOblong::TOLERANCE)));
        segs.extend(self.get_segs(self.get_line(false).to_path(ShapeOblong::TOLERANCE)));
        segs.extend(self.get_segs(self.get_arc(true).to_path(ShapeOblong::TOLERANCE)));
        segs
    }
    fn get_modifier_pattern(&self, mut selected: bool, mut highlighted: bool) -> Pattern {
        selected |= self.selected;
        highlighted |= self.highlighted;
        match (selected, highlighted) {
            (false, false) => Pattern::BasicNormal,
            (false, true) => Pattern::BasicHighlighted,
            (true, false) => Pattern::BasicSelected,
            (true, true) => Pattern::BasicSelected,
        }
    }
}
impl Display for ShapeOblong {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rounded rectangle")
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
        compute_winding_number(&self.get_segs_all(), pt.to_vec2())
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
impl Shapes for ShapeOblong {
    const TOLERANCE: f64 = 0.01;

    fn new(pos1: Vec2, pos2: Vec2) -> ShapeKind {
        ShapeKind::Oblong(ShapeOblong {
            start: Position::new(pos1),
            end: Position::new(pos2),
            width: 10.,
            width_saved: 10.,
            arc_start_highlighted: false,
            arc_end_highlighted: false,
            line_up_highlighted: false,
            line_down_highlighted: false,
            arc_start_selected: false,
            arc_end_selected: true,
            line_up_selected: false,
            line_down_selected: false,
            highlighted: false,
            selected: false,
        })
    }
    fn good_size(&self) -> bool {
        self.width >= ShapeOblong::MIN_SIZE
    }

    fn save_pos(&mut self) {
        self.start.save_pos();
        self.end.save_pos();
        self.width_saved = self.width;
    }
    fn toggle_prop(&mut self) {
        ()
    }
    fn get_shape_paths(&self) -> Vec<(BezPath, Pattern)> {
        let mut paths = vec![];
        paths.push((
            self.get_line(true).to_path(ShapeOblong::TOLERANCE),
            self.get_modifier_pattern(self.line_up_selected, self.line_up_highlighted),
        ));
        paths.push((
            self.get_arc(false).to_path(ShapeOblong::TOLERANCE),
            self.get_modifier_pattern(self.arc_end_selected, self.arc_end_highlighted),
        ));

        paths.push((
            self.get_line(false).to_path(ShapeOblong::TOLERANCE),
            self.get_modifier_pattern(self.line_down_selected, self.line_down_highlighted),
        ));
        paths.push((
            self.get_arc(true).to_path(ShapeOblong::TOLERANCE),
            self.get_modifier_pattern(self.arc_start_selected, self.arc_start_highlighted),
        ));
        paths
    }

    fn highlight_from_pos(&mut self, pos: Vec2) -> bool {
        self.highlighted = self.contains(pos.to_point());
        self.highlighted
    }
    fn highlight_modifiers_from_pos(&mut self, pos: Vec2) -> bool {
        let line_up = self.get_line(true);
        let end_segs = self.get_segs(self.get_arc(false).to_path(ShapeOblong::TOLERANCE));
        let line_down = self.get_line(false);
        let start_segs = self.get_segs(self.get_arc(true).to_path(ShapeOblong::TOLERANCE));

        self.line_up_highlighted = distance_to_line(pos, line_up) < ShapeOblong::GRAB;
        self.arc_end_highlighted = is_point_near_path(&end_segs, pos, 4. * ShapeOblong::GRAB);
        self.line_down_highlighted = distance_to_line(pos, line_down) < ShapeOblong::GRAB;
        self.arc_start_highlighted = is_point_near_path(&start_segs, pos, 4. * ShapeOblong::GRAB);
        self.line_up_highlighted
            || self.arc_end_highlighted
            || self.line_down_highlighted
            || self.arc_start_highlighted
    }
    fn highlight(&mut self, value: bool) {
        self.highlighted = value;
    }
    fn highlight_modifiers(&mut self, value: bool) {
        self.line_up_highlighted = value;
        self.arc_end_highlighted = value;
        self.line_down_highlighted = value;
        self.arc_start_highlighted = value;
    }
    fn is_highlighted(&self) -> bool {
        self.highlighted
    }

    fn select_from_pos(&mut self, pos: Vec2) -> bool {
        self.selected = self.contains(pos.to_point());
        self.selected
    }
    fn select_modifiers_from_pos(&mut self, pos: Vec2) -> bool {
        let line_up = self.get_line(true);
        let end_segs = self.get_segs(self.get_arc(false).to_path(ShapeOblong::TOLERANCE));
        let line_down = self.get_line(false);
        let start_segs = self.get_segs(self.get_arc(true).to_path(ShapeOblong::TOLERANCE));

        self.line_up_selected = distance_to_line(pos, line_up) < ShapeOblong::GRAB;
        self.arc_end_selected = is_point_near_path(&end_segs, pos, 4. * ShapeOblong::GRAB);
        self.line_down_selected = distance_to_line(pos, line_down) < ShapeOblong::GRAB;
        self.arc_start_selected = is_point_near_path(&start_segs, pos, 4. * ShapeOblong::GRAB);
        self.line_up_selected
            || self.arc_end_selected
            || self.line_down_selected
            || self.arc_start_selected
    }
    fn select(&mut self, value: bool) {
        self.selected = value;
    }
    fn select_modifiers(&mut self, value: bool) {
        self.line_up_selected = value;
        self.arc_end_selected = value;
        self.line_down_selected = value;
        self.arc_start_selected = value;
    }
    fn is_selected(&self) -> bool {
        self.selected
    }

    fn get_position(&self) -> Vec2 {
        (self.start.get_pos() + self.end.get_pos()) / 2.
    }
    fn move_position(&mut self, pos_init: Vec2, pos: Vec2) {
        let start_saved = self.start.get_saved_pos();
        let end_saved = self.end.get_saved_pos();
        let width_saved = self.width_saved;
        let dpos = pos - pos_init;

        let (dpos_rel, dpos_dir) =
            project_to_perpendicular_with_direction(start_saved, end_saved, dpos);

        match (
            self.line_up_selected,
            self.arc_end_selected,
            self.line_down_selected,
            self.arc_start_selected,
        ) {
            (false, false, false, false) => {
                if self.selected {
                    self.start.set_pos(start_saved + dpos);
                    self.end.set_pos(end_saved + dpos);
                }
            }
            (true, false, false, false) => {
                self.width =
                    (width_saved - 2. * dpos_rel.hypot() * dpos_dir).max(ShapeOblong::MIN_SIZE);
            }
            (false, true, false, false) => {
                self.end.set_pos(end_saved + dpos);
            }
            (false, false, true, false) => {
                self.width =
                    (width_saved + 2. * dpos_rel.hypot() * dpos_dir).max(ShapeOblong::MIN_SIZE);
            }
            (false, false, false, true) => {
                self.start.set_pos(start_saved + dpos);
            }
            _ => (),
        }
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
