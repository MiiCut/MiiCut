// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

use crate::{
    math::*,
    shapes::{CShapes, Handle, HandleKind},
    shapes_pool::CShapeKind,
};
use kurbo::{Arc, ArcAppendIter, BezPath, Line, LinePathIter, PathEl, Point, Rect, Shape, Vec2};
use std::{
    f64::consts::{FRAC_PI_2, PI},
    fmt::Display,
};
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CShapeOblong {
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
    fn get_rectangle_vertices(&self) -> [Vec2; 4] {
        let (pos1, pos2, r_vec) = (
            self.handles.0.get_pos(),
            self.handles.1.get_pos(),
            self.handles.2.get_pos(),
        );
        let v0 = get_intersection(pos1, pos2, r_vec);
        let v1 = get_intersection(pos2, pos1, r_vec);
        let r_vec_sym = symmetric_point(pos2, pos1, r_vec);
        let v3 = get_intersection(pos1, pos2, r_vec_sym);
        let v2 = get_intersection(pos2, pos1, r_vec_sym);
        [v0, v1, v2, v3]
    }
    fn get_line(&self, upper_line: bool) -> Line {
        let [v0, v1, v2, v3] = self.get_rectangle_vertices();
        if upper_line {
            Line::new(v0.to_point(), v1.to_point())
        } else {
            Line::new(v2.to_point(), v3.to_point())
        }
    }

    fn get_radius_from_point(&self, pos: Vec2) -> f64 {
        let (pos1, pos2) = (self.handles.0.get_pos(), self.handles.1.get_pos());
        get_dist_to_line(pos1, pos2, pos)
    }
    fn is_point_in_rectangle(&self, pos: Vec2) -> bool {
        let rect_vertices = self.get_rectangle_vertices();

        let mut signs = vec![];

        // Compute signed distances to each edge of the rectangle
        for i in 0..4 {
            let a = rect_vertices[i];
            let b = rect_vertices[(i + 1) % 4];
            let distance = signed_distance(pos, a, b);
            signs.push(distance);
        }

        // Check if all distances have the same sign
        let all_positive = signs.iter().all(|&d| d > 0.0);
        let all_negative = signs.iter().all(|&d| d < 0.0);

        all_positive || all_negative
    }

    fn is_point_in_circle(&self, point: Point, start_arc: bool) -> bool {
        let vertices = self.get_rectangle_vertices(); // Call once
        let (center, radius_squared) = if start_arc {
            let center = self.handles.0.get_pos();
            (center, (vertices[0] - center).hypot2())
        } else {
            let center = self.handles.1.get_pos();
            (center, (vertices[2] - center).hypot2())
        };

        let dist_squared = (point.x - center.x).powi(2) + (point.y - center.y).powi(2);
        dist_squared <= radius_squared
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
    fn winding(&self, pt: Point) -> i32 {
        if self.is_point_in_rectangle(pt.to_vec2())
            || self.is_point_in_circle(pt, true)
            || self.is_point_in_circle(pt, false)
        {
            1
        } else {
            0
        }
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
impl CShapes for CShapeOblong {
    const TOLERANCE: f64 = 0.01;

    fn new(pos1: Vec2, pos2: Vec2) -> CShapeKind {
        let radius = 5.;
        let pos3 = point_at_distance(pos1, pos2, radius);
        use HandleKind::*;
        let handles = (
            Handle::new(Vec2::new(pos1.x, pos1.y), Grab, false),
            Handle::new(Vec2::new(pos2.x, pos2.y), Grab, true),
            Handle::new(pos3, Modify, false),
        );
        CShapeKind::COblong(CShapeOblong {
            radius,
            handles,
            highlighted: false,
            selected: false,
        })
    }
    fn save_pos(&mut self) {
        self.handles.0.save_pos();
        self.handles.1.save_pos();
        self.handles.2.save_pos();
    }
    fn toggle_prop(&mut self) {
        ()
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
        self.handles
            .2
            .set_selection(is_near_position(pos, self.handles.2.get_pos(), precision));
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
        self.handles.2.set_selection(false);
    }
    fn get_position(&self) -> Vec2 {
        (self.handles.0.get_pos() + self.handles.1.get_pos()) / 2.
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
                    let mut radius = self.get_radius_from_point(pos3);
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
