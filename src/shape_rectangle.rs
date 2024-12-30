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
use kurbo::{BezPath, Line, LinePathIter, PathEl, Point, Rect, Shape, Vec2};
use std::fmt::Display;
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeRectangle {
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

    highlighted: bool,
    selected: bool,
}
impl ShapeRectangle {
    const MIN_SIZE: f64 = 10.;
    fn get_lines(&self) -> (Line, Line, Line, Line) {
        let tl_pos = self.tl.get_pos();
        let br_pos = self.br.get_pos();
        (
            Line::new(tl_pos.to_point(), Vec2::new(br_pos.x, tl_pos.y).to_point()),
            Line::new(Vec2::new(br_pos.x, tl_pos.y).to_point(), br_pos.to_point()),
            Line::new(br_pos.to_point(), Vec2::new(tl_pos.x, br_pos.y).to_point()),
            Line::new(Vec2::new(tl_pos.x, br_pos.y).to_point(), tl_pos.to_point()),
        )
    }
    fn get_rectangle(&self) -> Rect {
        let tl_pos = self.tl.get_pos();
        let br_pos = self.br.get_pos();
        Rect::new(tl_pos.x, tl_pos.y, br_pos.x, br_pos.y)
    }
    fn force_consistency(&self, pos: Vec2, other: Vec2, last_pos: Vec2) -> Vec2 {
        let dx = (pos.x - other.x).abs();
        let dy = (pos.y - other.y).abs();

        match (dx < ShapeRectangle::MIN_SIZE, dy < ShapeRectangle::MIN_SIZE) {
            (false, false) => pos,
            (true, true) => last_pos,
            (true, false) => Vec2::new(
                other.x + ShapeRectangle::MIN_SIZE * (pos.x - other.x).signum(),
                pos.y,
            ),
            (false, true) => Vec2::new(
                pos.x,
                other.y + ShapeRectangle::MIN_SIZE * (pos.y - other.y).signum(),
            ),
        }
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
impl Display for ShapeRectangle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rectangle")
    }
}
impl Shape for ShapeRectangle {
    type PathElementsIter<'iter> = CShapeRectangleIter;

    fn path_elements(&self, tolerance: f64) -> CShapeRectangleIter {
        let lines = self.get_lines();
        let lines_iter = [
            lines.0.path_elements(tolerance),
            lines.1.path_elements(tolerance),
            lines.2.path_elements(tolerance),
            lines.3.path_elements(tolerance),
        ];

        CShapeRectangleIter { idx: 0, lines_iter }
    }
    #[inline]
    fn area(&self) -> f64 {
        self.get_rectangle().area()
    }
    #[inline]
    fn perimeter(&self, accuracy: f64) -> f64 {
        self.get_rectangle().perimeter(accuracy)
    }
    #[inline]
    fn winding(&self, pt: Point) -> i32 {
        self.get_rectangle().winding(pt)
    }
    #[inline]
    fn bounding_box(&self) -> Rect {
        self.get_rectangle().bounding_box()
    }
    #[inline]
    fn as_rect(&self) -> Option<Rect> {
        self.get_rectangle().as_rect()
    }
    #[inline]
    fn contains(&self, pt: Point) -> bool {
        self.get_rectangle().abs().contains(pt)
    }
}
impl Shapes for ShapeRectangle {
    const TOLERANCE: f64 = 0.01;

    fn new(pos1: Vec2, pos2: Vec2) -> ShapeKind {
        ShapeKind::Rectangle(ShapeRectangle {
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
            highlighted: false,
            selected: false,
        })
    }
    fn good_size(&self) -> bool {
        (self.tl.get_pos().x - self.br.get_pos().x).abs() >= ShapeRectangle::MIN_SIZE
            && (self.tl.get_pos().y - self.br.get_pos().y).abs() >= ShapeRectangle::MIN_SIZE
    }
    fn save_pos(&mut self) {
        self.tl.save_pos();
        self.br.save_pos();
    }
    fn toggle_prop(&mut self) {
        ()
    }
    fn get_shape_paths(&self) -> Vec<(BezPath, Pattern)> {
        let (top, right, bottom, left) = self.get_lines();
        vec![
            (
                top.path_elements(ShapeRectangle::TOLERANCE)
                    .collect::<BezPath>(),
                self.get_modifier_pattern(self.top_selected, self.top_highlighed),
            ),
            (
                right
                    .path_elements(ShapeRectangle::TOLERANCE)
                    .collect::<BezPath>(),
                self.get_modifier_pattern(self.right_selected, self.right_highlighed),
            ),
            (
                bottom
                    .path_elements(ShapeRectangle::TOLERANCE)
                    .collect::<BezPath>(),
                self.get_modifier_pattern(self.bottom_selected, self.bottom_highlighed),
            ),
            (
                left.path_elements(ShapeRectangle::TOLERANCE)
                    .collect::<BezPath>(),
                self.get_modifier_pattern(self.left_selected, self.left_highlighed),
            ),
        ]
    }

    fn highlight_from_pos(&mut self, pos: Vec2) -> bool {
        self.highlighted = self.contains(pos.to_point());
        self.highlighted
    }
    fn highlight_modifiers_from_pos(&mut self, pos: Vec2) -> bool {
        let tl_pos = self.tl.get_pos();
        let tr_pos = Vec2::new(self.br.get_pos().x, self.tl.get_pos().y);
        let br_pos = self.br.get_pos();
        let bl_pos = Vec2::new(self.tl.get_pos().x, self.br.get_pos().y);
        self.top_highlighed = get_dist_to_segment(tl_pos, tr_pos, pos) < 4.;
        self.right_highlighed = get_dist_to_segment(tr_pos, br_pos, pos) < 4.;
        self.bottom_highlighed = get_dist_to_segment(br_pos, bl_pos, pos) < 4.;
        self.left_highlighed = get_dist_to_segment(bl_pos, tl_pos, pos) < 4.;

        self.top_highlighed
            || self.right_highlighed
            || self.bottom_highlighed
            || self.left_highlighed
    }
    fn highlight(&mut self, value: bool) {
        self.highlighted = value;
    }
    fn highlight_modifiers(&mut self, value: bool) {
        self.top_highlighed = value;
        self.right_highlighed = value;
        self.bottom_highlighed = value;
        self.left_highlighed = value;
    }
    fn is_highlighted(&self) -> bool {
        self.highlighted
    }

    fn select_from_pos(&mut self, pos: Vec2) -> bool {
        self.selected = self.contains(pos.to_point());
        self.selected
    }
    fn select_modifiers_from_pos(&mut self, pos: Vec2) -> bool {
        let tl_pos = self.tl.get_pos();
        let tr_pos = Vec2::new(self.br.get_pos().x, self.tl.get_pos().y);
        let br_pos = self.br.get_pos();
        let bl_pos = Vec2::new(self.tl.get_pos().x, self.br.get_pos().y);
        self.top_selected = get_dist_to_segment(tl_pos, tr_pos, pos) < 4.;
        self.right_selected = get_dist_to_segment(tr_pos, br_pos, pos) < 4.;
        self.bottom_selected = get_dist_to_segment(br_pos, bl_pos, pos) < 4.;
        self.left_selected = get_dist_to_segment(bl_pos, tl_pos, pos) < 4.;

        self.top_selected || self.right_selected || self.bottom_selected || self.left_selected
    }
    fn select(&mut self, value: bool) {
        self.selected = value;
    }
    fn select_modifiers(&mut self, value: bool) {
        self.top_selected = value;
        self.right_selected = value;
        self.bottom_selected = value;
        self.left_selected = value;
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

        let top_sel = self.top_selected;
        let right_sel = self.right_selected;
        let bottom_sel = self.bottom_selected;
        let left_sel = self.left_selected;

        let dpos = pos - pos_init;
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

pub struct CShapeRectangleIter {
    idx: usize,
    lines_iter: [LinePathIter; 4],
}
impl Iterator for CShapeRectangleIter {
    type Item = PathEl;

    fn next(&mut self) -> Option<PathEl> {
        match self.idx {
            0..=3 => match self.lines_iter[self.idx].next() {
                Some(el) => Some(el),
                None => {
                    self.idx += 1;
                    self.next()
                }
            },
            _ => None,
        }
    }
}
