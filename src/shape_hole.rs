// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

use kurbo::{BezPath, Circle, CirclePathIter, Point, Rect, Shape, Vec2};
use std::fmt::Display;

use crate::{
    handles::{Handle, HandleKind},
    math::*,
    shapes::CShapes,
    shapes_pool::CShapeKind,
};
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CShapeHole {
    handles: (Handle, Handle),
    saved_handles: (Handle, Handle),
    highlighted: bool,
    selected: bool,
}
impl CShapeHole {
    fn get_circle(&self) -> Circle {
        let (pos1, pos2) = (self.handles.0.get_pos(), self.handles.1.get_pos());
        Circle::new(pos1.to_point(), (pos2 - pos1).hypot())
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

    fn new(pos1: Vec2, pos2: Vec2) -> CShapeKind {
        use HandleKind::*;
        let handles = (
            Handle::new(Vec2::new(pos1.x, pos1.y), Grab, false),
            Handle::new(Vec2::new(pos2.x, pos2.y), Modify, true),
        );
        CShapeKind::CHole(CShapeHole {
            handles,
            saved_handles: handles,
            highlighted: false,
            selected: false,
        })
    }
    fn save_pos(&mut self) {
        self.saved_handles.0.set_pos(self.handles.0.get_pos());
        self.saved_handles.1.set_pos(self.handles.1.get_pos());
    }
    fn toggle_prop(&mut self) {
        ()
    }
    fn get_shape_path(&self) -> BezPath {
        self.get_circle().to_path(Self::TOLERANCE)
    }

    fn highlight_handles(&mut self, pos: Vec2, precision: f64) -> bool {
        self.handles
            .0
            .set_highlighted(is_near_position(pos, self.handles.0.get_pos(), precision));
        self.handles
            .1
            .set_highlighted(is_near_position(pos, self.handles.1.get_pos(), precision));
        self.handles.0.is_highlighted() || self.handles.1.is_highlighted()
    }
    fn highlight_shape(&mut self, pos: Vec2) -> bool {
        self.highlighted = self.contains(pos.to_point());
        self.highlighted
    }
    fn set_highlight(&mut self, value: bool) {
        self.highlighted = value;
    }
    fn is_highlighted(&self) -> bool {
        self.highlighted
    }

    fn select_handles(&mut self, pos: Vec2, precision: f64) -> bool {
        self.handles
            .0
            .set_selection(is_near_position(pos, self.handles.0.get_pos(), precision));
        self.handles
            .1
            .set_selection(is_near_position(pos, self.handles.1.get_pos(), precision));
        self.handles.0.is_selected() || self.handles.1.is_selected()
    }
    fn select_shape(&mut self, pos: Vec2) -> bool {
        self.selected = self.contains(pos.to_point());
        self.selected
    }
    fn set_selection(&mut self, value: bool) {
        self.selected = value;
    }
    fn is_selected(&self) -> bool {
        self.selected
    }
    fn clear_selection(&mut self) {
        self.selected = false;
    }
    fn clear_selection_all(&mut self) {
        self.clear_selection();
        self.handles.0.set_selection(false);
        self.handles.1.set_selection(false);
    }

    fn get_position(&self) -> Vec2 {
        self.handles.0.get_pos()
    }
    fn move_position(&mut self, pos_init: Vec2, pos: Vec2) {
        let dpos = pos - pos_init;
        let h1 = self.saved_handles.0.get_pos();
        let h2 = self.saved_handles.1.get_pos();
        match self.get_handle_selected() {
            None => {
                if self.selected {
                    self.handles.0.set_pos(h1 + dpos);
                    self.handles.1.set_pos(h2 + dpos);
                }
            }
            Some((_handle, idx)) => match idx {
                0 => {
                    self.handles.0.set_pos(h1 + dpos);
                    self.handles.1.set_pos(h2 + dpos);
                }
                1 => {
                    if (h2 + dpos - h1).hypot() >= 1.0 {
                        self.handles.1.set_pos(h2 + dpos);
                    }
                }
                _ => unreachable!(),
            },
        }
    }
    fn get_handles(&self) -> Vec<Handle> {
        vec![self.handles.0, self.handles.1]
    }
    fn get_handle_selected(&self) -> Option<(Handle, usize)> {
        if self.handles.0.is_selected() {
            return Some((self.handles.0, 0));
        }
        if self.handles.1.is_selected() {
            return Some((self.handles.1, 1));
        }
        None
    }
    fn get_handle_highlighted(&self) -> Option<(Handle, usize)> {
        if self.handles.0.is_highlighted() {
            return Some((self.handles.0, 0));
        }
        if self.handles.1.is_highlighted() {
            return Some((self.handles.1, 1));
        }
        None
    }
}
