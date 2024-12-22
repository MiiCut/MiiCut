// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

use crate::math::*;
use crate::shapes::{CShapes, Handle, HandleKind};
use crate::shapes_pool::CShapeKind;
use kurbo::{BezPath, Point, Rect, RectPathIter, Shape, Vec2};
use std::fmt::Display;
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CShapeRectangle {
    handles: (Handle, Handle),
    highlighted: bool,
    selected: bool,
}
impl CShapeRectangle {
    const MIN_SIZE: f64 = 1.;
    fn get_rectangle(&self) -> Rect {
        let (pos1, pos2) = (self.handles.0.get_pos(), self.handles.1.get_pos());
        Rect::new(pos1.x, pos1.y, pos2.x, pos2.y).abs()
    }
    fn enforce_constraints(&self, pos: Vec2, other: Vec2, last_pos: Vec2) -> Vec2 {
        let dx = (pos.x - other.x).abs();
        let dy = (pos.y - other.y).abs();

        match (
            dx < CShapeRectangle::MIN_SIZE,
            dy < CShapeRectangle::MIN_SIZE,
        ) {
            (false, false) => pos,
            (true, true) => last_pos,
            (true, false) => Vec2::new(
                other.x + CShapeRectangle::MIN_SIZE * (pos.x - other.x).signum(),
                pos.y,
            ),
            (false, true) => Vec2::new(
                pos.x,
                other.y + CShapeRectangle::MIN_SIZE * (pos.y - other.y).signum(),
            ),
        }
    }
}
impl Display for CShapeRectangle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rectangle")
    }
}
impl Shape for CShapeRectangle {
    type PathElementsIter<'iter> = RectPathIter;

    fn path_elements(&self, tolerance: f64) -> RectPathIter {
        self.get_rectangle().path_elements(tolerance)
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
        self.get_rectangle().contains(pt)
    }
}
impl CShapes for CShapeRectangle {
    const TOLERANCE: f64 = 0.01;

    fn new(pos1: Vec2, pos2: Vec2) -> CShapeKind {
        use HandleKind::*;
        let handles = (
            Handle::new(Vec2::new(pos1.x, pos1.y), Grab, false),
            Handle::new(Vec2::new(pos2.x, pos2.y), Grab, true),
        );
        CShapeKind::CRectangle(CShapeRectangle {
            handles,
            highlighted: false,
            selected: false,
        })
    }
    fn save_pos(&mut self) {
        self.handles.0.save_pos();
        self.handles.1.save_pos();
    }
    fn toggle_prop(&mut self) {
        ()
    }
    fn get_shape_path(&self) -> BezPath {
        self.get_rectangle().to_path(Self::TOLERANCE)
    }
    fn highlight_object(&mut self, pos: Vec2, precision: f64) {
        self.handles
            .0
            .set_highlighted(is_near_position(pos, self.handles.0.get_pos(), precision));
        self.handles
            .1
            .set_highlighted(is_near_position(pos, self.handles.1.get_pos(), precision));
        if self.get_handle_highlighted().is_none() {
            self.highlighted = self.contains(pos.to_point());
        } else {
            self.highlighted = false;
        }
    }
    fn select_object(&mut self, pos: Vec2, precision: f64) {
        self.handles
            .0
            .set_selection(is_near_position(pos, self.handles.0.get_pos(), precision));
        self.handles
            .1
            .set_selection(is_near_position(pos, self.handles.1.get_pos(), precision));

        if self.get_handle_selected().is_none() {
            self.selected = self.contains(pos.to_point());
        } else {
            self.selected = false;
        }
    }
    fn is_selected(&self) -> bool {
        self.selected
    }
    fn is_highlighted(&self) -> bool {
        self.highlighted
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
        (self.handles.0.get_pos() + self.handles.1.get_pos()) / 2.
    }
    fn move_position(&mut self, pos_init: Vec2, pos: Vec2) {
        let h1 = self.handles.0.get_saved_pos();
        let h2 = self.handles.1.get_saved_pos();
        let last_h1 = self.handles.0.get_last_pos();
        let last_h2 = self.handles.1.get_last_pos();
        let dpos = pos - pos_init;
        match self.get_handle_selected() {
            None => {
                if self.selected {
                    self.handles.0.set_pos(h1 + dpos);
                    self.handles.1.set_pos(h2 + dpos);
                }
            }
            Some((_handle, idx)) => match idx {
                0 => {
                    let new_h1 = self.enforce_constraints(h1 + dpos, h2, last_h1);
                    self.handles.0.set_pos(new_h1);
                }
                1 => {
                    let new_h2 = self.enforce_constraints(h2 + dpos, h1, last_h2);
                    self.handles.1.set_pos(new_h2);
                }
                _ => unreachable!(),
            },
        };
        self.update_handles_pos();
    }
    fn get_handles(&self) -> Vec<Handle> {
        vec![self.handles.0, self.handles.1]
    }
    fn update_handles_pos(&mut self) {
        ()
    }
    fn get_handle_selected(&self) -> Option<(Handle, usize)> {
        if self.handles.0.get_selection() {
            return Some((self.handles.0, 0));
        }
        if self.handles.1.get_selection() {
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
