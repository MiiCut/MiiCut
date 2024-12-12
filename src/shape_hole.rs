// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

use kurbo::{BezPath, Circle, CirclePathIter, ParamCurveNearest, Point, Rect, Shape, Vec2};
use std::fmt::Display;

use crate::{
    closed_shapes::{COperation, CShape, CShapes, ClosedShapeId, Handle, HandleKind},
    math::is_near_position,
};
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CShapeHole {
    id: ClosedShapeId,
    op: COperation,
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

    fn new(cshid: ClosedShapeId, pos1: Vec2, pos2: Vec2) -> CShape {
        use HandleKind::*;
        let handles = (
            Handle::new(Vec2::new(pos1.x, pos1.y), Grab, false),
            Handle::new(Vec2::new(pos2.x, pos2.y), Modify, true),
        );
        CShape::CHole(CShapeHole {
            id: cshid,
            op: COperation::Add,
            handles,
            saved_handles: handles,
            selected: false,
            highlighted: false,
        })
    }
    fn get_id(&self) -> ClosedShapeId {
        self.id
    }
    fn get_op(&self) -> COperation {
        self.op
    }
    fn save_pos(&mut self) {
        self.saved_handles.0.set_pos(self.handles.0.get_pos());
        self.saved_handles.1.set_pos(self.handles.1.get_pos());
    }
    fn toggle_prop(&mut self) {
        ()
    }
    fn is_near_cursor(&self, pos: Vec2, precision: f64) -> bool {
        for seg in self.to_path(CShapeHole::TOLERANCE).segments() {
            let nearest = seg.nearest(pos.to_point(), precision);
            if nearest.distance_sq < precision {
                return true;
            }
        }
        false
    }
    fn get_shape_path(&self) -> BezPath {
        self.get_circle().to_path(Self::TOLERANCE)
    }
    fn highlight_object(&mut self, pos: Vec2, precision: f64) {
        self.handles
            .0
            .set_highlighted(is_near_position(pos, self.handles.0.get_pos(), precision));
        self.handles
            .1
            .set_highlighted(is_near_position(pos, self.handles.1.get_pos(), precision));
        if self.get_handle_highlighted().is_none() {
            self.highlighted = self.is_near_cursor(pos, precision)
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
            self.selected = self.is_near_cursor(pos, precision)
        } else {
            self.selected = false;
        }
    }
    fn is_selected(&self) -> bool {
        self.selected == true
    }
    fn clear_selection(&mut self) {
        self.selected = false;
    }
    fn clear_selection_all(&mut self) {
        self.clear_selection();
        self.handles.0.set_selection(false);
        self.handles.1.set_selection(false);
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
