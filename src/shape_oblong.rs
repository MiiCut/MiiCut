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
    Arc, ArcAppendIter, BezPath, Line, LinePathIter, ParamCurveNearest, PathEl, Point, Rect, Shape,
    Vec2,
};
use std::{
    f64::consts::{FRAC_PI_2, PI},
    fmt::Display,
};
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CShapeOblong {
    id: ClosedShapeId,
    op: COperation,
    radius: f64,
    handles: (Handle, Handle, Handle),
    highlighted: bool,
    selected: bool,
}
impl CShapeOblong {
    fn get_arc(&self, start_arc: bool) -> Arc {
        let (pos1, pos2, r_vec) = (
            self.handles.0.get_pos(),
            self.handles.1.get_pos(),
            self.handles.2.get_pos(),
        );
        let radius = get_dist_to_line(pos1, pos2, r_vec);
        let angle = angle_between(pos1, pos2);
        if start_arc {
            Arc::new(
                pos1.to_point(),
                Vec2::new(radius, radius),
                3. * FRAC_PI_2,
                -PI,
                angle,
            )
        } else {
            Arc::new(
                pos2.to_point(),
                Vec2::new(radius, radius),
                FRAC_PI_2,
                -PI,
                angle,
            )
        }
    }
    fn get_line(&self, upper_line: bool) -> Line {
        let (pos1, pos2, r_vec) = (
            self.handles.0.get_pos(),
            self.handles.1.get_pos(),
            self.handles.2.get_pos(),
        );
        let pos1_offset = get_intersection(pos1, pos2, r_vec);
        let pos2_offset = get_intersection(pos2, pos1, r_vec);
        let r_vec_sym = symmetric_point(pos2, pos1, r_vec);
        let pos1_offset_sym = get_intersection(pos1, pos2, r_vec_sym);
        let pos2_offset_sym = get_intersection(pos2, pos1, r_vec_sym);
        if upper_line {
            Line::new(pos1_offset.to_point(), pos2_offset.to_point())
        } else {
            Line::new(pos2_offset_sym.to_point(), pos1_offset_sym.to_point())
        }
    }
    fn get_radius(&self, pos: Vec2) -> f64 {
        log!("ddd");
        let (pos1, pos2) = (self.handles.0.get_pos(), self.handles.1.get_pos());
        get_dist_to_line(pos1, pos2, pos)
    }
}
impl Display for CShapeOblong {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rounded rectangle")
    }
}
impl Shape for CShapeOblong {
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
    fn winding(&self, _pt: Point) -> i32 {
        //TODO
        0
    }
    #[inline]
    fn bounding_box(&self) -> Rect {
        //TODO
        Rect::ZERO
    }
    #[inline]
    fn contains(&self, _pt: Point) -> bool {
        //TODO
        false
    }
}
impl CShapes for CShapeOblong {
    const TOLERANCE: f64 = 0.01;

    fn new(cshid: ClosedShapeId, pos1: Vec2, pos2: Vec2) -> CShape {
        let radius = 5.;
        let pos3 = point_at_distance(pos1, pos2, radius);
        use HandleKind::*;
        let handles = (
            Handle::new(Vec2::new(pos1.x, pos1.y), Grab, false),
            Handle::new(Vec2::new(pos2.x, pos2.y), Grab, true),
            Handle::new(pos3, Modify, false),
        );
        CShape::COblong(CShapeOblong {
            id: cshid,
            op: COperation::Add,
            radius,
            handles,
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
        self.handles.0.save_pos();
        self.handles.1.save_pos();
        self.handles.2.save_pos();
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
        self.path_elements(CShapeOblong::TOLERANCE).collect()
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
    }
    fn move_position(&mut self, pos_init: Vec2, pos: Vec2) {
        let dpos = pos - pos_init;
        let h1 = self.handles.0.get_saved_pos();
        let h2 = self.handles.1.get_saved_pos();
        let h3 = self.handles.2.get_saved_pos();
        match self.get_handle_selected() {
            None => {
                if self.selected {
                    self.handles.0.set_pos(h1 + dpos);
                    self.handles.1.set_pos(h2 + dpos);
                    self.handles.2.set_pos(h3 + dpos);
                }
            }
            Some((_handle, idx)) => match idx {
                0 => {
                    if (h1 + dpos - h2).hypot() >= 1.0 {
                        self.handles.0.set_pos(h1 + dpos);
                        let pos3 = point_at_distance(
                            self.handles.0.get_pos(),
                            self.handles.1.get_pos(),
                            self.radius,
                        );
                        self.handles.2.set_pos(pos3);
                    }
                }
                1 => {
                    if (h2 + dpos - h1).hypot() >= 1.0 {
                        self.handles.1.set_pos(h2 + dpos);
                        let pos3 = point_at_distance(
                            self.handles.0.get_pos(),
                            self.handles.1.get_pos(),
                            self.radius,
                        );
                        self.handles.2.set_pos(pos3);
                    }
                }
                2 => {
                    let pos3 = projection_to_perpendicular(h1, h2, h3 + dpos);
                    let mut radius = self.get_radius(pos3);
                    if radius < 1. {
                        radius = 1.
                    }
                    let pos3 = point_at_distance(
                        self.handles.0.get_pos(),
                        self.handles.1.get_pos(),
                        radius,
                    );
                    self.handles.2.set_pos(pos3);
                    self.radius = radius;
                }
                _ => unreachable!(),
            },
        }
    }
    fn get_handles(&self) -> Vec<Handle> {
        vec![self.handles.0, self.handles.1, self.handles.2]
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
        None
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
