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
    Arc, ArcAppendIter, BezPath, Line, LinePathIter, PathEl, Point, Rect, RoundedRect,
    RoundedRectRadii, Shape, Vec2,
};
use std::{f64::consts::PI, fmt::Display};
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ShapeRectRounded {
    tl: Position,
    br: Position,
    top_highlighed: bool,
    right_highlighed: bool,
    bottom_highlighed: bool,
    left_highlighed: bool,
    top_selected: bool,
    right_selected: bool,
    bottom_selected: bool,
    left_selected: bool,

    radii: RoundedRectRadii,
    saved_radii: RoundedRectRadii,
    rad_tl_highlighed: bool,
    rad_tr_highlighed: bool,
    rad_br_highlighed: bool,
    rad_bl_highlighed: bool,
    rad_tl_selected: bool,
    rad_tr_selected: bool,
    rad_br_selected: bool,
    rad_bl_selected: bool,

    highlighted: bool,
    selected: bool,
}
impl ShapeRectRounded {
    const MIN_SIZE: f64 = 10.;
    fn get_lines(&self) -> (Line, Line, Line, Line) {
        let rad_tl = self.radii.top_left;
        let rad_tr = self.radii.top_right;
        let rad_br = self.radii.bottom_right;
        let rad_bl = self.radii.bottom_left;

        let tl_pos = self.tl.get_pos();
        let tr_pos = Vec2::new(self.br.get_pos().x, self.tl.get_pos().y);
        let br_pos = self.br.get_pos();
        let bl_pos = Vec2::new(self.tl.get_pos().x, self.br.get_pos().y);
        (
            Line::new(
                (tl_pos + Vec2::new(rad_tl, 0.)).to_point(),
                (tr_pos - Vec2::new(rad_tr, 0.)).to_point(),
            ),
            Line::new(
                (tr_pos + Vec2::new(0., rad_tr)).to_point(),
                (br_pos - Vec2::new(0., rad_br)).to_point(),
            ),
            Line::new(
                (br_pos - Vec2::new(rad_br, 0.)).to_point(),
                (bl_pos + Vec2::new(rad_bl, 0.)).to_point(),
            ),
            Line::new(
                (bl_pos - Vec2::new(0., rad_bl)).to_point(),
                (tl_pos + Vec2::new(0., rad_tl)).to_point(),
            ),
        )
    }
    fn get_corners(&self) -> (Arc, Arc, Arc, Arc) {
        let center_tl_pos = Vec2::new(
            self.tl.get_pos().x + self.radii.top_left,
            self.tl.get_pos().y + self.radii.top_left,
        );
        let center_tr_pos = Vec2::new(
            self.br.get_pos().x - self.radii.top_right,
            self.tl.get_pos().y + self.radii.top_right,
        );
        let center_br_pos = Vec2::new(
            self.br.get_pos().x - self.radii.bottom_right,
            self.br.get_pos().y - self.radii.bottom_right,
        );
        let center_bl_pos = Vec2::new(
            self.tl.get_pos().x + self.radii.bottom_left,
            self.br.get_pos().y - self.radii.bottom_left,
        );
        (
            Arc::new(
                center_tl_pos.to_point(),
                Vec2::new(self.radii.top_left, self.radii.top_left),
                PI,
                PI / 2.,
                0.,
            ),
            Arc::new(
                center_tr_pos.to_point(),
                Vec2::new(self.radii.top_right, self.radii.top_right),
                3. * PI / 2.,
                PI / 2.,
                0.,
            ),
            Arc::new(
                center_br_pos.to_point(),
                Vec2::new(self.radii.bottom_right, self.radii.bottom_right),
                0.,
                PI / 2.,
                0.,
            ),
            Arc::new(
                center_bl_pos.to_point(),
                Vec2::new(self.radii.bottom_left, self.radii.bottom_left),
                PI / 2.,
                PI / 2.,
                0.,
            ),
        )
    }
    fn get_rectangle_rounded(&self) -> RoundedRect {
        let (tl_pos, br_pos) = (self.tl.get_pos(), self.br.get_pos());
        RoundedRect::new(tl_pos.x, tl_pos.y, br_pos.x, br_pos.y, self.radii)
    }
    fn force_consistency(&self, pos: Vec2, other: Vec2, last_pos: Vec2) -> Vec2 {
        let dx = (pos.x - other.x).abs();
        let dy = (pos.y - other.y).abs();

        match (
            dx < ShapeRectRounded::MIN_SIZE,
            dy < ShapeRectRounded::MIN_SIZE,
        ) {
            (false, false) => pos,
            (true, true) => last_pos,
            (true, false) => Vec2::new(
                other.x + ShapeRectRounded::MIN_SIZE * (pos.x - other.x).signum(),
                pos.y,
            ),
            (false, true) => Vec2::new(
                pos.x,
                other.y + ShapeRectRounded::MIN_SIZE * (pos.y - other.y).signum(),
            ),
        }
    }
    fn clamp_radii(&mut self) {
        let (pos1, pos2) = (self.tl.get_pos(), self.br.get_pos());
        let r = RoundedRect::new(pos1.x, pos1.y, pos2.x, pos2.y, self.radii);
        self.radii = r.radii();
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

impl Display for ShapeRectRounded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rounded rectangle")
    }
}
impl Shape for ShapeRectRounded {
    type PathElementsIter<'iter> = CShapeRectRoundedIter;

    fn path_elements(&self, tolerance: f64) -> CShapeRectRoundedIter {
        let lines = self.get_lines();
        let arcs = self.get_corners();
        let lines_iter: [LinePathIter; 4] = [
            lines.0.path_elements(tolerance),
            lines.1.path_elements(tolerance),
            lines.2.path_elements(tolerance),
            lines.3.path_elements(tolerance),
        ];
        let arcs_iter: [std::iter::Chain<std::iter::Once<PathEl>, ArcAppendIter>; 4] = [
            arcs.0.path_elements(tolerance),
            arcs.1.path_elements(tolerance),
            arcs.2.path_elements(tolerance),
            arcs.3.path_elements(tolerance),
        ];
        CShapeRectRoundedIter {
            idx: 0,
            lines_iter,
            arcs_iter,
        }
    }
    #[inline]
    fn area(&self) -> f64 {
        self.get_rectangle_rounded().area()
    }
    #[inline]
    fn perimeter(&self, accuracy: f64) -> f64 {
        self.get_rectangle_rounded().perimeter(accuracy)
    }
    #[inline]
    fn winding(&self, pt: Point) -> i32 {
        self.get_rectangle_rounded().winding(pt)
    }
    #[inline]
    fn bounding_box(&self) -> Rect {
        self.get_rectangle_rounded().bounding_box()
    }
    #[inline]
    fn as_rounded_rect(&self) -> Option<RoundedRect> {
        self.get_rectangle_rounded().as_rounded_rect()
    }
    #[inline]
    fn contains(&self, pt: Point) -> bool {
        self.get_rectangle_rounded().contains(pt)
    }
}
impl Shapes for ShapeRectRounded {
    const TOLERANCE: f64 = 0.01;

    fn new(pos1: Vec2, pos2: Vec2) -> ShapeKind {
        // let pos2 = pos2 + Vec2::new(20., 20.);
        let radii = RoundedRectRadii::new(
            ShapeRectRounded::MIN_SIZE,
            ShapeRectRounded::MIN_SIZE,
            ShapeRectRounded::MIN_SIZE,
            ShapeRectRounded::MIN_SIZE,
        );
        ShapeKind::RectangleRounded(ShapeRectRounded {
            tl: Position::new(pos1),
            br: Position::new(pos2),
            top_highlighed: false,
            right_highlighed: false,
            bottom_highlighed: false,
            left_highlighed: false,
            top_selected: false,
            right_selected: true,
            bottom_selected: true,
            left_selected: false,

            radii,
            saved_radii: radii,
            rad_tl_highlighed: false,
            rad_tr_highlighed: false,
            rad_br_highlighed: false,
            rad_bl_highlighed: false,
            rad_tl_selected: false,
            rad_tr_selected: false,
            rad_br_selected: false,
            rad_bl_selected: false,

            highlighted: false,
            selected: false,
        })
    }
    fn good_size(&self) -> bool {
        (self.tl.get_pos().x - self.br.get_pos().x).abs() >= ShapeRectRounded::MIN_SIZE
            && (self.tl.get_pos().y - self.br.get_pos().y).abs() >= ShapeRectRounded::MIN_SIZE
    }
    fn save_pos(&mut self) {
        self.tl.save_pos();
        self.br.save_pos();
        self.saved_radii = self.radii;
    }
    fn toggle_prop(&mut self) {
        ()
    }
    fn get_shape_paths(&self) -> Vec<(BezPath, Pattern)> {
        let (top, right, bottom, left) = self.get_lines();
        let (tl, tr, br, bl) = self.get_corners();
        vec![
            (
                top.path_elements(ShapeRectRounded::TOLERANCE)
                    .collect::<BezPath>(),
                self.get_modifier_pattern(self.top_selected, self.top_highlighed),
            ),
            (
                tr.path_elements(ShapeRectRounded::TOLERANCE)
                    .collect::<BezPath>(),
                self.get_modifier_pattern(self.rad_tr_selected, self.rad_tr_highlighed),
            ),
            (
                right
                    .path_elements(ShapeRectRounded::TOLERANCE)
                    .collect::<BezPath>(),
                self.get_modifier_pattern(self.right_selected, self.right_highlighed),
            ),
            (
                br.path_elements(ShapeRectRounded::TOLERANCE)
                    .collect::<BezPath>(),
                self.get_modifier_pattern(self.rad_br_selected, self.rad_br_highlighed),
            ),
            (
                bottom
                    .path_elements(ShapeRectRounded::TOLERANCE)
                    .collect::<BezPath>(),
                self.get_modifier_pattern(self.bottom_selected, self.bottom_highlighed),
            ),
            (
                bl.path_elements(ShapeRectRounded::TOLERANCE)
                    .collect::<BezPath>(),
                self.get_modifier_pattern(self.rad_bl_selected, self.rad_bl_highlighed),
            ),
            (
                left.path_elements(ShapeRectRounded::TOLERANCE)
                    .collect::<BezPath>(),
                self.get_modifier_pattern(self.left_selected, self.left_highlighed),
            ),
            (
                tl.path_elements(ShapeRectRounded::TOLERANCE)
                    .collect::<BezPath>(),
                self.get_modifier_pattern(self.rad_tl_selected, self.rad_tl_highlighed),
            ),
        ]
    }

    fn highlight_from_pos(&mut self, pos: Vec2) -> bool {
        self.highlighted = self.contains(pos.to_point());
        self.highlighted
    }
    fn highlight_modifiers_from_pos(&mut self, pos: Vec2) -> bool {
        let rad_tl = self.radii.top_left;
        let rad_tr = self.radii.top_right;
        let rad_br = self.radii.bottom_right;
        let rad_bl = self.radii.bottom_left;
        let tl_pos = self.tl.get_pos();
        let tr_pos = Vec2::new(self.br.get_pos().x, self.tl.get_pos().y);
        let br_pos = self.br.get_pos();
        let bl_pos = Vec2::new(self.tl.get_pos().x, self.br.get_pos().y);
        self.top_highlighed = get_dist_to_segment(
            tl_pos + Vec2::new(self.radii.top_left, 0.),
            tr_pos - Vec2::new(self.radii.top_right, 0.),
            pos,
        ) < 4.
            || (tl_pos - pos).hypot() < 4.
            || (tr_pos - pos).hypot() < 4.;
        self.right_highlighed = get_dist_to_segment(
            tr_pos + Vec2::new(0., self.radii.top_right),
            br_pos - Vec2::new(0., self.radii.bottom_right),
            pos,
        ) < 4.
            || (tr_pos - pos).hypot() < 4.
            || (br_pos - pos).hypot() < 4.;
        self.bottom_highlighed = get_dist_to_segment(
            br_pos - Vec2::new(self.radii.bottom_right, 0.),
            bl_pos + Vec2::new(self.radii.bottom_left, 0.),
            pos,
        ) < 4.
            || (br_pos - pos).hypot() < 4.
            || (bl_pos - pos).hypot() < 4.;
        self.left_highlighed = get_dist_to_segment(
            bl_pos - Vec2::new(0., self.radii.bottom_left),
            tl_pos + Vec2::new(0., self.radii.top_left),
            pos,
        ) < 4.
            || (bl_pos - pos).hypot() < 4.
            || (tl_pos - pos).hypot() < 4.;
        self.rad_tl_highlighed =
            (tl_pos + (2. - 2_f64.sqrt()) / 2. * Vec2::new(rad_tl, rad_tl) - pos).hypot() < 4.;
        self.rad_tr_highlighed =
            (tr_pos + (2. - 2_f64.sqrt()) / 2. * Vec2::new(-rad_tr, rad_tr) - pos).hypot() < 4.;
        self.rad_br_highlighed =
            (br_pos + (2. - 2_f64.sqrt()) / 2. * Vec2::new(-rad_br, -rad_br) - pos).hypot() < 4.;
        self.rad_bl_highlighed =
            (bl_pos + (2. - 2_f64.sqrt()) / 2. * Vec2::new(rad_bl, -rad_bl) - pos).hypot() < 4.;
        self.top_highlighed
            || self.right_highlighed
            || self.bottom_highlighed
            || self.left_highlighed
            || self.rad_tl_highlighed
            || self.rad_tr_highlighed
            || self.rad_br_highlighed
            || self.rad_bl_highlighed
    }
    fn highlight(&mut self, value: bool) {
        self.highlighted = value;
    }
    fn highlight_modifiers(&mut self, value: bool) {
        self.top_highlighed = value;
        self.right_highlighed = value;
        self.bottom_highlighed = value;
        self.left_highlighed = value;
        self.rad_tl_highlighed = value;
        self.rad_tr_highlighed = value;
        self.rad_br_highlighed = value;
        self.rad_bl_highlighed = value;
    }
    fn is_highlighted(&self) -> bool {
        self.highlighted
    }

    fn select_from_pos(&mut self, pos: Vec2) -> bool {
        self.selected = self.contains(pos.to_point());
        self.selected
    }
    fn select_modifiers_from_pos(&mut self, pos: Vec2) -> bool {
        let rad_tl = self.radii.top_left;
        let rad_tr = self.radii.top_right;
        let rad_br = self.radii.bottom_right;
        let rad_bl = self.radii.bottom_left;
        let tl_pos = self.tl.get_pos();
        let tr_pos = Vec2::new(self.br.get_pos().x, self.tl.get_pos().y);
        let br_pos = self.br.get_pos();
        let bl_pos = Vec2::new(self.tl.get_pos().x, self.br.get_pos().y);
        self.top_selected = get_dist_to_segment(
            tl_pos + Vec2::new(self.radii.top_left, 0.),
            tr_pos - Vec2::new(self.radii.top_right, 0.),
            pos,
        ) < 4.
            || (tl_pos - pos).hypot() < 4.
            || (tr_pos - pos).hypot() < 4.;
        self.right_selected = get_dist_to_segment(
            tr_pos + Vec2::new(0., self.radii.top_right),
            br_pos - Vec2::new(0., self.radii.bottom_right),
            pos,
        ) < 4.
            || (tr_pos - pos).hypot() < 4.
            || (br_pos - pos).hypot() < 4.;
        self.bottom_selected = get_dist_to_segment(
            br_pos - Vec2::new(self.radii.bottom_right, 0.),
            bl_pos + Vec2::new(self.radii.bottom_left, 0.),
            pos,
        ) < 4.
            || (br_pos - pos).hypot() < 4.
            || (bl_pos - pos).hypot() < 4.;
        self.left_selected = get_dist_to_segment(
            bl_pos - Vec2::new(0., self.radii.bottom_left),
            tl_pos + Vec2::new(0., self.radii.top_left),
            pos,
        ) < 4.
            || (bl_pos - pos).hypot() < 4.
            || (tl_pos - pos).hypot() < 4.;
        self.rad_tl_selected =
            (tl_pos + (2. - 2_f64.sqrt()) / 2. * Vec2::new(rad_tl, rad_tl) - pos).hypot() < 4.;
        self.rad_tr_selected =
            (tr_pos + (2. - 2_f64.sqrt()) / 2. * Vec2::new(-rad_tr, rad_tr) - pos).hypot() < 4.;
        self.rad_br_selected =
            (br_pos + (2. - 2_f64.sqrt()) / 2. * Vec2::new(-rad_br, -rad_br) - pos).hypot() < 4.;
        self.rad_bl_selected =
            (bl_pos + (2. - 2_f64.sqrt()) / 2. * Vec2::new(rad_bl, -rad_bl) - pos).hypot() < 4.;
        self.top_selected
            || self.right_selected
            || self.bottom_selected
            || self.left_selected
            || self.rad_tl_selected
            || self.rad_tr_selected
            || self.rad_br_selected
            || self.rad_bl_selected
    }
    fn select(&mut self, value: bool) {
        self.selected = value;
    }
    fn select_modifiers(&mut self, value: bool) {
        self.top_selected = value;
        self.right_selected = value;
        self.bottom_selected = value;
        self.left_selected = value;
        self.rad_tl_selected = value;
        self.rad_tr_selected = value;
        self.rad_br_selected = value;
        self.rad_bl_selected = value;
    }
    fn is_selected(&self) -> bool {
        self.selected
    }

    fn get_position(&self) -> Vec2 {
        (self.tl.get_pos() + self.br.get_pos()) / 2.
    }
    fn move_position(&mut self, pos_init: Vec2, pos: Vec2) {
        let tl_saved = self.tl.get_saved_pos();
        let br_saved = self.br.get_saved_pos();
        let tl_last = self.tl.get_last_pos();
        let br_last = self.br.get_last_pos();
        let rad_saved = self.saved_radii;

        let top_sel = self.top_selected;
        let right_sel = self.right_selected;
        let bottom_sel = self.bottom_selected;
        let left_sel = self.left_selected;
        let rad_tl_sel = self.rad_tl_selected;
        let rad_tr_sel = self.rad_tr_selected;
        let rad_br_sel = self.rad_br_selected;
        let rad_bl_sel = self.rad_bl_selected;

        let dpos = pos - pos_init;
        let corner_moved = match (rad_tl_sel, rad_tr_sel, rad_br_sel, rad_bl_sel) {
            (false, false, false, false) => false,
            (true, false, false, false) => {
                let rad_tl = rad_saved.top_left + dpos.x.min(dpos.y);
                self.radii.top_left = rad_tl.max(ShapeRectRounded::MIN_SIZE);
                true
            }
            (false, true, false, false) => {
                let rad_tr = rad_saved.top_right - dpos.x.min(dpos.y);
                self.radii.top_right = rad_tr.max(ShapeRectRounded::MIN_SIZE);
                true
            }
            (false, false, true, false) => {
                let rad_br = rad_saved.bottom_right - dpos.x.min(-dpos.y);
                self.radii.bottom_right = rad_br.max(ShapeRectRounded::MIN_SIZE);
                true
            }
            (false, false, false, true) => {
                let rad_bl = rad_saved.bottom_left + dpos.x.min(-dpos.y);
                self.radii.bottom_left = rad_bl.max(ShapeRectRounded::MIN_SIZE);
                true
            }
            _ => false,
        };
        if !corner_moved {
            match (top_sel, right_sel, bottom_sel, left_sel) {
                (false, false, false, false) => {
                    if self.selected {
                        self.tl.set_pos(tl_saved + dpos);
                        self.br.set_pos(br_saved + dpos);
                    }
                }
                (true, false, false, false) => {
                    let tlpos = self.force_consistency(tl_saved + dpos, br_saved, tl_last);
                    self.tl.set_pos(Vec2::new(tl_saved.x, tlpos.y))
                }
                (false, true, false, false) => {
                    let brpos = self.force_consistency(br_saved + dpos, tl_saved, br_last);
                    self.br.set_pos(Vec2::new(brpos.x, br_saved.y))
                }
                (false, false, true, false) => {
                    let brpos = self.force_consistency(br_saved + dpos, tl_saved, br_last);
                    self.br.set_pos(Vec2::new(br_saved.x, brpos.y))
                }
                (false, false, false, true) => {
                    let tlpos = self.force_consistency(tl_saved + dpos, br_saved, tl_last);
                    self.tl.set_pos(Vec2::new(tlpos.x, tl_saved.y))
                }
                (true, true, false, false) => {
                    let tlpos = self.force_consistency(tl_saved + dpos, br_saved, tl_last);
                    let brpos = self.force_consistency(br_saved + dpos, tl_saved, br_last);
                    self.tl.set_pos(Vec2::new(tl_saved.x, tlpos.y));
                    self.br.set_pos(Vec2::new(brpos.x, br_saved.y))
                }
                (true, false, false, true) => {
                    let tlpos = self.force_consistency(tl_saved + dpos, br_saved, tl_last);
                    self.tl.set_pos(tlpos);
                }
                (false, true, true, false) => {
                    let brpos = self.force_consistency(br_saved + dpos, tl_saved, br_last);
                    self.br.set_pos(brpos);
                }
                (false, false, true, true) => {
                    let tlpos = self.force_consistency(tl_saved + dpos, br_saved, tl_last);
                    let brpos = self.force_consistency(br_saved + dpos, tl_saved, br_last);
                    self.tl.set_pos(Vec2::new(tlpos.x, tl_saved.y));
                    self.br.set_pos(Vec2::new(br_saved.x, brpos.y))
                }
                _ => (),
            }
        }
    }
}

pub struct CShapeRectRoundedIter {
    idx: usize,
    lines_iter: [LinePathIter; 4],
    arcs_iter: [std::iter::Chain<std::iter::Once<PathEl>, ArcAppendIter>; 4],
}
impl Iterator for CShapeRectRoundedIter {
    type Item = PathEl;

    fn next(&mut self) -> Option<PathEl> {
        // Iterate over lines and arcs in the desired order: top_line, tr_arc, right_line, br_arc, ...
        match self.idx {
            0 | 2 | 4 | 6 => {
                // Even indices correspond to lines (0, 2, 4, 6)
                let line_idx = self.idx / 2; // Map index to lines_iter
                let line = self.lines_iter[line_idx].next();
                if line.is_none() {
                    self.idx += 1; // Move to the next element (arc)
                    self.next()
                } else {
                    line
                }
            }
            1 | 3 | 5 | 7 => {
                // Odd indices correspond to arcs (1, 3, 5, 7)
                let arc_idx = (self.idx - 1) / 2; // Map index to arcs_iter
                let arc = self.arcs_iter[arc_idx].next();
                if arc.is_none() {
                    self.idx += 1; // Move to the next element (line)
                    self.next()
                } else {
                    arc
                }
            }
            _ => None, // End of iteration
        }
    }
}
