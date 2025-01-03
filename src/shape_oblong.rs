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
    positions::Position,
    prefab::magnet_path,
    shape_disc::HighLightOrSelect,
    shapes::{ShapeKind, Shapes},
};
use geo::{LineString, Polygon};
use kurbo::{Arc, ArcAppendIter, BezPath, Line, LinePathIter, PathEl, Point, Rect, Shape, Vec2};
use std::{
    f64::consts::{FRAC_PI_2, PI},
    fmt::Display,
};
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeOblong {
    start: Position,
    end: Position,
    center: Position,
    middle_up: Position,
    middle_down: Position,

    highlighted: bool,
    selected: bool,

    segs: BezPath,
    polygon: Polygon<f64>,
}
impl ShapeOblong {
    const MIN_WIDTH_SIZE: f64 = 2.;
    const MIN_LENGTH_SIZE: f64 = 10.;

    fn update_polygon(&mut self) {
        log!("calc oblong polygon");
        self.segs = calc_segs(self.get_paths_patterns());
        self.polygon = calc_polygon(&self.segs);
    }
    fn get_width(&self) -> f64 {
        (self.middle_up.get_pos() - self.middle_down.get_pos()).hypot()
    }
    fn get_length(&self) -> f64 {
        (self.start.get_pos() - self.end.get_pos()).hypot()
    }
    fn get_arc(&self, start_arc: bool) -> Arc {
        let width = (self.middle_up.get_pos() - self.middle_down.get_pos()).hypot();
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
    // fn get_segs_all(&self) -> BezPath {
    //     let mut segs = BezPath::new();
    //     segs.extend(self.get_segs(self.get_line(true).to_path(ShapeOblong::TOLERANCE)));
    //     segs.extend(self.get_segs(self.get_arc(false).to_path(ShapeOblong::TOLERANCE)));
    //     segs.extend(self.get_segs(self.get_line(false).to_path(ShapeOblong::TOLERANCE)));
    //     segs.extend(self.get_segs(self.get_arc(true).to_path(ShapeOblong::TOLERANCE)));
    //     segs
    // }
    fn update_center(&mut self) {
        self.center
            .set_pos((self.start.get_pos() + self.end.get_pos()) / 2.);
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
impl Shapes for ShapeOblong {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 2.;

    fn new(pos1: Vec2, pos2: Vec2) -> ShapeKind {
        // let pos2 = pos2 + Vec2::new(30., 0.);
        let start = Position::new(pos1, true);
        let mut end = Position::new(pos2, true);
        end.select(true);
        let center = Position::new((pos1 + pos2) / 2., false);
        let width = 10.;
        let middle_up = Position::new(
            get_middle_from_start_end_positions(start.get_pos(), end.get_pos(), width, true),
            true,
        );
        let middle_down = Position::new(
            get_middle_from_start_end_positions(start.get_pos(), end.get_pos(), width, false),
            true,
        );

        ShapeKind::Oblong(ShapeOblong {
            start,
            end,
            center,
            middle_up,
            middle_down,
            highlighted: false,
            selected: false,
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
    fn good_size(&self) -> bool {
        self.get_width() >= ShapeOblong::MIN_WIDTH_SIZE - 0.1
            && self.get_length() >= ShapeOblong::MIN_LENGTH_SIZE - 0.1
    }
    fn save_pos(&mut self) {
        self.start.save_pos();
        self.end.save_pos();
        self.center.save_pos();
        self.middle_up.save_pos();
        self.middle_down.save_pos();
    }
    fn toggle_prop(&mut self) {
        ()
    }
    fn get_paths_patterns(&self) -> Vec<(BezPath, Pattern)> {
        let mut paths: Vec<(BezPath, Pattern)> = vec![];
        paths.push((
            self.get_line(true).to_path(ShapeOblong::TOLERANCE),
            self.get_pattern(self.selected, self.highlighted),
        ));
        paths.push((
            self.get_arc(false).to_path(ShapeOblong::TOLERANCE),
            self.get_pattern(self.selected, self.highlighted),
        ));

        paths.push((
            self.get_line(false).to_path(ShapeOblong::TOLERANCE),
            self.get_pattern(self.selected, self.highlighted),
        ));
        paths.push((
            self.get_arc(true).to_path(ShapeOblong::TOLERANCE),
            self.get_pattern(self.selected, self.highlighted),
        ));
        paths
    }
    fn get_polygon(&self) -> Polygon<f64> {
        self.polygon.clone()
    }
    fn hors_from_pos(&mut self, pos: Vec2, hors: HighLightOrSelect) -> bool {
        match hors {
            HighLightOrSelect::Highlight => {
                self.highlighted = self.contains(pos.to_point());
                self.highlighted
            }
            HighLightOrSelect::Select => {
                self.selected = self.contains(pos.to_point());
                self.selected
            }
        }
    }
    fn hors_center_from_pos(&mut self, pos: Vec2, hors: HighLightOrSelect) -> bool {
        let center_hors = (pos - self.center.get_pos()).hypot() < Self::GRAB;
        match hors {
            HighLightOrSelect::Highlight => {
                self.center.highlight(center_hors);
                center_hors
            }
            HighLightOrSelect::Select => {
                self.center.select(center_hors);
                center_hors
            }
        }
    }
    fn hors_modifiers_from_pos(&mut self, pos: Vec2, hors: HighLightOrSelect) -> bool {
        let start_hors = (pos - self.start.get_pos()).hypot() < Self::GRAB;
        let end_hors = (pos - self.end.get_pos()).hypot() < Self::GRAB;
        let center_hors = (pos - self.center.get_pos()).hypot() < Self::GRAB;
        let middle_up_hors = (pos - self.middle_up.get_pos()).hypot() < Self::GRAB;
        let middle_down_hors = (pos - self.middle_down.get_pos()).hypot() < Self::GRAB;

        match hors {
            HighLightOrSelect::Highlight => {
                self.start.highlight(start_hors);
                self.end.highlight(end_hors);
                self.center.highlight(center_hors);
                self.middle_up.highlight(middle_up_hors);
                self.middle_down.highlight(middle_down_hors);
                self.start.is_highlighted()
                    || self.end.is_highlighted()
                    || self.center.is_highlighted()
                    || self.middle_up.is_highlighted()
                    || self.middle_down.is_highlighted()
            }
            HighLightOrSelect::Select => {
                self.start.select(start_hors);
                self.end.select(end_hors);
                self.center.select(center_hors);
                self.middle_up.select(middle_up_hors);
                self.middle_down.select(middle_down_hors);
                self.start.is_selected()
                    || self.end.is_selected()
                    || self.center.is_selected()
                    || self.middle_up.is_selected()
                    || self.middle_down.is_selected()
            }
        }
    }
    fn set_hors(&mut self, value: bool, hors: HighLightOrSelect) {
        match hors {
            HighLightOrSelect::Highlight => self.highlighted = value,
            HighLightOrSelect::Select => self.selected = value,
        }
    }
    fn set_hors_center(&mut self, value: bool, hors: HighLightOrSelect) {
        match hors {
            HighLightOrSelect::Highlight => self.center.highlight(value),
            HighLightOrSelect::Select => self.center.select(value),
        }
    }
    fn set_hors_modifiers(&mut self, value: bool, hors: HighLightOrSelect) {
        match hors {
            HighLightOrSelect::Highlight => {
                self.start.highlight(value);
                self.end.highlight(value);
                self.middle_up.highlight(value);
                self.middle_down.highlight(value);
            }
            HighLightOrSelect::Select => {
                self.start.select(value);
                self.end.select(value);
                self.middle_up.select(value);
                self.middle_down.select(value);
            }
        }
    }
    fn is_hors(&self, hors: HighLightOrSelect) -> bool {
        match hors {
            HighLightOrSelect::Highlight => self.highlighted,
            HighLightOrSelect::Select => self.selected,
        }
    }
    fn is_center_hors(&self, hors: HighLightOrSelect) -> bool {
        match hors {
            HighLightOrSelect::Highlight => self.center.is_highlighted(),
            HighLightOrSelect::Select => self.center.is_selected(),
        }
    }
    fn get_position(&self) -> Vec2 {
        self.center.get_pos()
    }
    fn move_position(&mut self, pos_init: Vec2, pos: Vec2, _shift_pressed: bool) {
        let start_saved = self.start.get_saved_pos();
        let end_saved = self.end.get_saved_pos();
        let center_saved = self.center.get_saved_pos();
        let middle_up_saved = self.middle_up.get_saved_pos();
        let middle_down_saved = self.middle_down.get_saved_pos();

        let start_sel = self.start.is_selected();
        let end_sel = self.end.is_selected();
        let middle_up_sel = self.middle_up.is_selected();
        let middle_down_sel = self.middle_down.is_selected();

        let dpos = pos - pos_init;
        let (dpos_proj, _) = project_to_perpendicular(start_saved, end_saved, dpos);

        if self.selected {
            self.start.set_pos(start_saved + dpos);
            self.end.set_pos(end_saved + dpos);
            self.center.set_pos(center_saved + dpos);
            self.middle_up.set_pos(middle_up_saved + dpos);
            self.middle_down.set_pos(middle_down_saved + dpos);
            self.update_polygon();
        } else {
            match (start_sel, end_sel, middle_up_sel, middle_down_sel) {
                (true, false, false, false) => {
                    let start = start_saved + dpos;
                    if (start - end_saved).hypot() >= ShapeOblong::MIN_LENGTH_SIZE {
                        self.start.set_pos(start);
                        let width = self.get_width();
                        let middle_up = get_middle_from_start_end_positions(
                            self.start.get_pos(),
                            self.end.get_pos(),
                            width,
                            true,
                        );
                        let middle_down = get_middle_from_start_end_positions(
                            self.start.get_pos(),
                            self.end.get_pos(),
                            width,
                            false,
                        );
                        self.middle_up.set_pos(middle_up);
                        self.middle_down.set_pos(middle_down);
                        self.update_center();
                        self.update_polygon();
                    }
                }
                (false, true, false, false) => {
                    let end = end_saved + dpos;
                    if (end - start_saved).hypot() >= ShapeOblong::MIN_LENGTH_SIZE {
                        self.end.set_pos(end);
                        let width = self.get_width();
                        let middle_up = get_middle_from_start_end_positions(
                            self.start.get_pos(),
                            self.end.get_pos(),
                            width,
                            true,
                        );
                        let middle_down = get_middle_from_start_end_positions(
                            self.start.get_pos(),
                            self.end.get_pos(),
                            width,
                            false,
                        );
                        self.middle_up.set_pos(middle_up);
                        self.middle_down.set_pos(middle_down);
                        self.update_center();
                        self.update_polygon();
                    }
                }
                (false, false, true, false) => {
                    let (_, dir1) = project_to_perpendicular(
                        start_saved,
                        end_saved,
                        middle_up_saved - center_saved,
                    );
                    let middle_up = middle_up_saved + dpos_proj;
                    let (_, dir2) =
                        project_to_perpendicular(start_saved, end_saved, middle_up - center_saved);

                    if (middle_up - center_saved).hypot() >= ShapeOblong::MIN_WIDTH_SIZE / 2.
                        && dir1 * dir2 > 0.
                    {
                        self.middle_up.set_pos(middle_up);
                        self.middle_down.set_pos(symmetric_point_to_segment(
                            start_saved,
                            end_saved,
                            middle_up,
                        ));
                        self.update_center();
                        self.update_polygon();
                    }
                }
                (false, false, false, true) => {
                    let (_, dir1) = project_to_perpendicular(
                        start_saved,
                        end_saved,
                        middle_down_saved - center_saved,
                    );
                    let middle_down = middle_down_saved + dpos_proj;
                    let (_, dir2) = project_to_perpendicular(
                        start_saved,
                        end_saved,
                        middle_down - center_saved,
                    );

                    if (middle_down - center_saved).hypot() >= ShapeOblong::MIN_WIDTH_SIZE / 2.
                        && dir1 * dir2 > 0.
                    {
                        self.middle_down.set_pos(middle_down);
                        self.middle_up.set_pos(symmetric_point_to_segment(
                            start_saved,
                            end_saved,
                            middle_down,
                        ));
                        self.update_center();
                        self.update_polygon();
                    }
                }
                _ => (),
            }
        }
    }
    fn get_magnets_paths(&self) -> Vec<(BezPath, Pattern)> {
        vec![
            (
                magnet_path(self.start.get_pos(), 1., ShapeOblong::GRAB),
                self.get_pattern_modifiers(self.start.is_selected(), self.start.is_highlighted()),
            ),
            (
                magnet_path(self.end.get_pos(), 1., ShapeOblong::GRAB),
                self.get_pattern_modifiers(self.end.is_selected(), self.end.is_highlighted()),
            ),
            (
                magnet_path(self.center.get_pos(), 1., ShapeOblong::GRAB),
                self.get_pattern_modifiers(self.center.is_selected(), self.center.is_highlighted()),
            ),
            (
                magnet_path(self.middle_up.get_pos(), 1., ShapeOblong::GRAB),
                self.get_pattern_modifiers(
                    self.middle_up.is_selected(),
                    self.middle_up.is_highlighted(),
                ),
            ),
            (
                magnet_path(self.middle_down.get_pos(), 1., ShapeOblong::GRAB),
                self.get_pattern_modifiers(
                    self.middle_down.is_selected(),
                    self.middle_down.is_highlighted(),
                ),
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
        (paths, texts)
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
