// A macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

use crate::{
    closed_shapes::{COperation, CShape, CShapes, ClosedShapeId, Handle, HandleKind},
    math::*,
};
use kurbo::{
    BezPath, ParamCurveNearest, Point, Rect, RoundedRect, RoundedRectPathIter, RoundedRectRadii,
    Shape, Vec2,
};
use std::fmt::Display;
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CShapeRectRounded {
    id: ClosedShapeId,
    op: COperation,
    radii: RoundedRectRadii,
    handles: (Handle, Handle, Handle, Handle, Handle, Handle),
    highlighted: bool,
    selected: bool,
}
impl CShapeRectRounded {
    fn get_rectangle_rounded(&self) -> RoundedRect {
        let (pos1, pos2) = (self.handles.0.get_pos(), self.handles.1.get_pos());
        RoundedRect::new(pos1.x, pos1.y, pos2.x, pos2.y, self.radii)
    }
    fn enforce_constraints(&self, pos: Vec2, other: Vec2, last_pos: Vec2, diameter: f64) -> Vec2 {
        let dx = (pos.x - other.x).abs();
        let dy = (pos.y - other.y).abs();
        match (dx < diameter, dy < diameter) {
            (false, false) => pos,
            (true, true) => last_pos,
            (true, false) => Vec2::new(other.x + diameter * (pos.x - other.x).signum(), pos.y),
            (false, true) => Vec2::new(pos.x, other.y + diameter * (pos.y - other.y).signum()),
        }
    }
    fn clamp_radii(&mut self) {
        let (pos1, pos2) = (self.handles.0.get_pos(), self.handles.1.get_pos());
        let r = RoundedRect::new(pos1.x, pos1.y, pos2.x, pos2.y, self.radii);
        self.radii = r.radii();
    }
}

impl Display for CShapeRectRounded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rounded rectangle")
    }
}
impl Shape for CShapeRectRounded {
    type PathElementsIter<'iter> = RoundedRectPathIter;

    fn path_elements(&self, tolerance: f64) -> RoundedRectPathIter {
        self.get_rectangle_rounded().path_elements(tolerance)
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
impl CShapes for CShapeRectRounded {
    const TOLERANCE: f64 = 0.01;

    fn new(cshid: ClosedShapeId, pos1: Vec2, pos2: Vec2) -> CShape {
        // let pos2 = pos2 + Vec2::new(20., 20.);
        let radii = RoundedRectRadii::new(2., 8., 3., 5.);
        let (tl, tr, br, bl) = (
            radii.top_left,
            radii.top_right,
            radii.bottom_right,
            radii.bottom_left,
        );
        use HandleKind::*;
        let handles = (
            Handle::new(Vec2::new(pos1.x, pos1.y), Grab, false),
            Handle::new(Vec2::new(pos2.x, pos2.y), Grab, true),
            Handle::new(Vec2::new(pos1.x + tl, pos1.y + tl), Modify, false),
            Handle::new(Vec2::new(pos2.x - tr, pos1.y + tr), Modify, false),
            Handle::new(Vec2::new(pos2.x - br, pos2.y - br), Modify, false),
            Handle::new(Vec2::new(pos1.x + bl, pos2.y - bl), Modify, false),
        );
        CShape::CRectangleRounded(CShapeRectRounded {
            id: cshid,
            op: COperation::Add,
            radii,
            handles,
            highlighted: false,
            selected: false,
        })
    }
    fn get_id(&self) -> ClosedShapeId {
        self.id
    }
    fn get_op(&self) -> COperation {
        self.op
    }
    fn save_pos(&mut self) {
        self.handles.0.save_pos();
        self.handles.1.save_pos();
        self.handles.2.save_pos();
        self.handles.3.save_pos();
        self.handles.4.save_pos();
        self.handles.5.save_pos();
    }
    fn toggle_prop(&mut self) {
        ()
    }
    fn is_near_cursor(&self, pos: Vec2, precision: f64) -> bool {
        for seg in self.get_shape_path().segments() {
            let nearest = seg.nearest(pos.to_point(), precision);
            if nearest.distance_sq < precision {
                return true;
            }
        }
        false
    }
    fn get_shape_path(&self) -> BezPath {
        self.get_rectangle_rounded().to_path(Self::TOLERANCE)
    }
    fn highlight_object(&mut self, pos: Vec2, precision: f64) {
        self.handles
            .0
            .set_highlighted(is_near_position(pos, self.handles.0.get_pos(), precision));
        self.handles
            .1
            .set_highlighted(is_near_position(pos, self.handles.1.get_pos(), precision));
        self.handles
            .2
            .set_highlighted(is_near_position(pos, self.handles.2.get_pos(), precision));
        self.handles
            .3
            .set_highlighted(is_near_position(pos, self.handles.3.get_pos(), precision));
        self.handles
            .4
            .set_highlighted(is_near_position(pos, self.handles.4.get_pos(), precision));
        self.handles
            .5
            .set_highlighted(is_near_position(pos, self.handles.5.get_pos(), precision));
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
        self.handles
            .2
            .set_selection(is_near_position(pos, self.handles.2.get_pos(), precision));
        self.handles
            .3
            .set_selection(is_near_position(pos, self.handles.3.get_pos(), precision));
        self.handles
            .4
            .set_selection(is_near_position(pos, self.handles.4.get_pos(), precision));
        self.handles
            .5
            .set_selection(is_near_position(pos, self.handles.5.get_pos(), precision));
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
        self.handles.2.set_selection(false);
        self.handles.3.set_selection(false);
        self.handles.4.set_selection(false);
        self.handles.5.set_selection(false);
    }
    fn move_position(&mut self, pos_init: Vec2, pos: Vec2) {
        let h1 = self.handles.0.get_saved_pos();
        let h2 = self.handles.1.get_saved_pos();
        let h3 = self.handles.2.get_saved_pos();
        let h4 = self.handles.3.get_saved_pos();
        let h5 = self.handles.4.get_saved_pos();
        let h6 = self.handles.5.get_saved_pos();
        let last_h1 = self.handles.0.get_last_pos();
        let last_h2 = self.handles.1.get_last_pos();
        let last_h3 = self.handles.2.get_last_pos();
        let last_h4 = self.handles.3.get_last_pos();
        let last_h5 = self.handles.4.get_last_pos();
        let last_h6 = self.handles.5.get_last_pos();
        let (tl, tr, bl, br) = (
            self.radii.top_left,
            self.radii.top_right,
            self.radii.bottom_left,
            self.radii.bottom_right,
        );
        let dpos = pos - pos_init;
        match self.get_handle_selected() {
            None => {
                if self.selected {
                    self.handles.0.set_pos(h1 + dpos);
                    self.handles.1.set_pos(h2 + dpos);
                    self.handles.2.set_pos(h3 + dpos);
                    self.handles.3.set_pos(h4 + dpos);
                    self.handles.4.set_pos(h5 + dpos);
                    self.handles.5.set_pos(h6 + dpos);
                }
            }
            Some((_handle, idx)) => {
                let biggest_diameter = 2. * get_max_radius(self.radii);
                match idx {
                    0 => {
                        let new_h1 =
                            self.enforce_constraints(h1 + dpos, h2, last_h1, biggest_diameter);
                        self.handles.0.set_pos(new_h1);

                        let (offset_x1, offset_x2) = if (h2 - new_h1).x >= 0. {
                            (0., (h2 - new_h1).x)
                        } else {
                            ((h2 - new_h1).x, 0.)
                        };
                        let (offset_y1, offset_y2) = if (h2 - new_h1).y >= 0. {
                            (0., (h2 - new_h1).y)
                        } else {
                            ((h2 - new_h1).y, 0.)
                        };

                        self.handles
                            .2
                            .set_pos(new_h1 + Vec2::new(offset_x1 + tl, offset_y1 + tl));
                        self.handles
                            .3
                            .set_pos(new_h1 + Vec2::new(offset_x2 - tr, offset_y1 + tr));
                        self.handles
                            .4
                            .set_pos(new_h1 + Vec2::new(offset_x2 - br, offset_y2 - br));
                        self.handles
                            .5
                            .set_pos(new_h1 + Vec2::new(offset_x1 + bl, offset_y2 - bl));
                    }
                    1 => {
                        let new_h2 =
                            self.enforce_constraints(h2 + dpos, h1, last_h2, biggest_diameter);
                        self.handles.1.set_pos(new_h2);

                        let (offset_x1, offset_x2) = if (h1 - new_h2).x >= 0. {
                            (0., (h1 - new_h2).x)
                        } else {
                            ((h1 - new_h2).x, 0.)
                        };
                        let (offset_y1, offset_y2) = if (h1 - new_h2).y >= 0. {
                            (0., (h1 - new_h2).y)
                        } else {
                            ((h1 - new_h2).y, 0.)
                        };

                        self.handles
                            .2
                            .set_pos(new_h2 + Vec2::new(offset_x1 + tl, offset_y1 + tl));
                        self.handles
                            .3
                            .set_pos(new_h2 + Vec2::new(offset_x2 - tr, offset_y1 + tr));

                        self.handles
                            .4
                            .set_pos(new_h2 + Vec2::new(offset_x2 - br, offset_y2 - br));
                        self.handles
                            .5
                            .set_pos(new_h2 + Vec2::new(offset_x1 + bl, offset_y2 - bl));
                    }
                    2 => {
                        let new_h3 =
                            h3 + Vec2::new((dpos.x + dpos.y) * 0.5, (dpos.x + dpos.y) * 0.5);
                        self.handles.2.set_pos(new_h3);
                        self.radii.top_left = (new_h3.x - h1.x).abs();
                        self.clamp_radii();
                        self.handles
                            .2
                            .set_pos(h1 + Vec2::new(self.radii.top_left, self.radii.top_left));
                    }
                    3 => {
                        let new_h4 =
                            h4 + Vec2::new((dpos.x - dpos.y) * 0.5, (-dpos.x + dpos.y) * 0.5);
                        self.handles.3.set_pos(new_h4);
                        self.radii.top_right = (new_h4.x - h2.x).abs();
                        self.clamp_radii();
                        self.handles.3.set_pos(Vec2::new(
                            h2.x - self.radii.top_right,
                            h1.y + self.radii.top_right,
                        ));
                    }
                    4 => (),
                    5 => (),
                    _ => unreachable!(),
                }
            }
        }
        self.update_handles_pos();
    }
    fn get_handles(&self) -> Vec<Handle> {
        vec![
            self.handles.0,
            self.handles.1,
            self.handles.2,
            self.handles.3,
            self.handles.4,
            self.handles.5,
        ]
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
        if self.handles.2.get_selection() {
            return Some((self.handles.2, 2));
        }
        if self.handles.3.get_selection() {
            return Some((self.handles.3, 3));
        }
        if self.handles.4.get_selection() {
            return Some((self.handles.4, 4));
        }
        if self.handles.5.get_selection() {
            return Some((self.handles.5, 5));
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
        if self.handles.2.is_highlighted() {
            return Some((self.handles.2, 2));
        }
        if self.handles.3.is_highlighted() {
            return Some((self.handles.3, 3));
        }
        if self.handles.4.is_highlighted() {
            return Some((self.handles.4, 4));
        }
        if self.handles.5.is_highlighted() {
            return Some((self.handles.5, 5));
        }
        None
    }
}
