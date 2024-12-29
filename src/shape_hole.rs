// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

use kurbo::{BezPath, Circle, CirclePathIter, Point, Rect, Shape, Vec2};
use std::fmt::Display;

use crate::{
    canvas_core::Pattern,
    shapes::{CShapeKind, CShapes},
    sub_shapes::Position,
};
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CShapeHole {
    center: Position,
    radius: f64,
    saved_radius: f64,
    circonference_highlighted: bool,
    circonference_selected: bool,
    highlighted: bool,
    selected: bool,
}
impl CShapeHole {
    const MIN_SIZE: f64 = 2.;
    const GRAB: f64 = 2.;

    fn get_circle(&self) -> Circle {
        Circle::new(self.center.get_pos().to_point(), self.radius)
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

impl Display for CShapeHole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Circle")
    }
}
impl Shape for CShapeHole {
    type PathElementsIter<'iter> = CirclePathIter;

    fn path_elements(&self, tolerance: f64) -> CirclePathIter {
        self.get_circle().path_elements(tolerance)
    }
    #[inline]
    fn area(&self) -> f64 {
        self.get_circle().area()
    }
    #[inline]
    fn perimeter(&self, accuracy: f64) -> f64 {
        self.get_circle().perimeter(accuracy)
    }
    #[inline]
    fn winding(&self, pt: Point) -> i32 {
        self.get_circle().winding(pt)
    }
    #[inline]
    fn bounding_box(&self) -> Rect {
        self.get_circle().bounding_box()
    }
    #[inline]
    fn as_circle(&self) -> Option<Circle> {
        self.get_circle().as_circle()
    }
    #[inline]
    fn contains(&self, pt: Point) -> bool {
        self.get_circle().contains(pt)
    }
}

impl CShapes for CShapeHole {
    const TOLERANCE: f64 = 0.01;

    fn new(pos1: Vec2, _pos2: Vec2) -> CShapeKind {
        CShapeKind::CHole(CShapeHole {
            center: Position::new(pos1),
            radius: CShapeHole::MIN_SIZE,
            saved_radius: CShapeHole::MIN_SIZE,
            circonference_highlighted: false,
            circonference_selected: true,
            highlighted: false,
            selected: false,
        })
    }
    fn save_pos(&mut self) {
        self.center.save_pos();
        self.saved_radius = self.radius;
    }
    fn toggle_prop(&mut self) {
        ()
    }
    fn get_shape_paths(&self) -> Vec<(BezPath, Pattern)> {
        vec![(
            self.get_circle().to_path(Self::TOLERANCE),
            self.get_modifier_pattern(self.circonference_selected, self.circonference_highlighted),
        )]
    }

    fn highlight_from_pos(&mut self, pos: Vec2) -> bool {
        self.highlighted = self.contains(pos.to_point());
        self.highlighted
    }
    fn highlight_modifiers_from_pos(&mut self, pos: Vec2) -> bool {
        self.circonference_highlighted =
            ((pos - self.center.get_pos()).hypot() - self.radius).abs() < CShapeHole::GRAB;
        self.circonference_highlighted
    }
    fn highlight(&mut self, value: bool) {
        self.highlighted = value;
    }
    fn highlight_modifiers(&mut self, value: bool) {
        self.circonference_highlighted = value;
    }
    fn is_highlighted(&self) -> bool {
        self.highlighted
    }

    fn select_from_pos(&mut self, pos: Vec2) -> bool {
        self.selected = self.contains(pos.to_point());
        self.selected
    }
    fn select_modifiers_from_pos(&mut self, pos: Vec2) -> bool {
        self.circonference_selected =
            ((pos - self.center.get_pos()).hypot() - self.radius).abs() < CShapeHole::GRAB;
        self.circonference_selected
    }
    fn select(&mut self, value: bool) {
        self.selected = value;
    }
    fn select_modifiers(&mut self, value: bool) {
        self.circonference_selected = value;
    }
    fn is_selected(&self) -> bool {
        self.selected
    }

    fn get_position(&self) -> Vec2 {
        self.center.get_pos()
    }
    fn move_position(&mut self, pos_init: Vec2, pos: Vec2) {
        let c_saved = self.center.get_saved_pos();

        let dpos = pos - pos_init;

        if self.circonference_selected {
            let radius = (self.center.get_pos() - pos).hypot();
            self.radius = radius.max(CShapeHole::MIN_SIZE);
        } else {
            if self.selected {
                self.center.set_pos(c_saved + dpos);
            }
        }
    }
}
