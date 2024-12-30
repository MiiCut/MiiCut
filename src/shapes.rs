// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

use std::fmt::Display;

use crate::{
    canvas_core::Pattern, shape_hole::ShapeHole, shape_oblong::ShapeOblong,
    shape_rectangle::ShapeRectangle, shape_rectangle_rounded::ShapeRectRounded, shapes_pool::Shid,
};
use geo::{OpType, Polygon};
use kurbo::{flatten, BezPath, PathEl, Vec2};

#[derive(Clone, Debug, PartialEq)]
pub enum ShapeKind {
    Rectangle(ShapeRectangle),
    RectangleRounded(ShapeRectRounded),
    Hole(ShapeHole),
    Oblong(ShapeOblong),
}
impl Display for ShapeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ShapeKind::*;
        match self {
            Rectangle(sh) => write!(f, "{sh}"),
            RectangleRounded(sh) => write!(f, "{sh}"),
            Hole(sh) => write!(f, "{sh}"),
            Oblong(sh) => write!(f, "{sh}"),
        }
    }
}
pub trait Shapes {
    const TOLERANCE: f64;

    fn new(pos1: Vec2, pos2: Vec2) -> ShapeKind;
    fn good_size(&self) -> bool;
    fn save_pos(&mut self);
    fn toggle_prop(&mut self);

    fn highlight_from_pos(&mut self, pos: Vec2) -> bool;
    fn highlight_modifiers_from_pos(&mut self, pos: Vec2) -> bool;
    fn highlight(&mut self, value: bool);
    fn highlight_modifiers(&mut self, value: bool);
    fn is_highlighted(&self) -> bool;

    fn select_from_pos(&mut self, pos: Vec2) -> bool;
    fn select_modifiers_from_pos(&mut self, pos: Vec2) -> bool;
    fn select(&mut self, value: bool);
    fn select_modifiers(&mut self, value: bool);
    fn is_selected(&self) -> bool;

    fn get_position(&self) -> Vec2;
    fn move_position(&mut self, pos_init: Vec2, pos: Vec2);

    fn get_shape_paths(&self) -> Vec<(BezPath, Pattern)>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    shid: Shid,
    cshape_kind: ShapeKind,
    boolean_op: OpType,
}
impl Shape {
    pub fn new(cshid: Shid, cshape_kind: ShapeKind, boolean_op: OpType) -> Shape {
        Shape {
            shid: cshid,
            cshape_kind,
            boolean_op,
        }
    }
    pub fn get_id(&self) -> Shid {
        self.shid
    }
    pub fn good_size(&self) -> bool {
        use ShapeKind::*;
        match &self.cshape_kind {
            Rectangle(sh) => sh.good_size(),
            RectangleRounded(sh) => sh.good_size(),
            Hole(sh) => sh.good_size(),
            Oblong(sh) => sh.good_size(),
        }
    }
    pub fn save_pos(&mut self) {
        use ShapeKind::*;
        match &mut self.cshape_kind {
            Rectangle(sh) => sh.save_pos(),
            RectangleRounded(sh) => sh.save_pos(),
            Hole(sh) => sh.save_pos(),
            Oblong(sh) => sh.save_pos(),
        }
    }
    pub fn toggle_boolean_op(&mut self) {
        if self.boolean_op == OpType::Union {
            self.boolean_op = OpType::Difference
        } else {
            self.boolean_op = OpType::Union
        }
    }

    pub fn highlight_from_pos(&mut self, pos: Vec2) -> bool {
        use ShapeKind::*;
        match &mut self.cshape_kind {
            Rectangle(sh) => sh.highlight_from_pos(pos),
            RectangleRounded(sh) => sh.highlight_from_pos(pos),
            Hole(sh) => sh.highlight_from_pos(pos),
            Oblong(sh) => sh.highlight_from_pos(pos),
        }
    }
    pub fn highlight_modifiers_from_pos(&mut self, pos: Vec2) -> bool {
        use ShapeKind::*;
        match &mut self.cshape_kind {
            Rectangle(sh) => sh.highlight_modifiers_from_pos(pos),
            RectangleRounded(sh) => sh.highlight_modifiers_from_pos(pos),
            Hole(sh) => sh.highlight_modifiers_from_pos(pos),
            Oblong(sh) => sh.highlight_modifiers_from_pos(pos),
        }
    }
    pub fn highlight(&mut self, value: bool) {
        use ShapeKind::*;
        match &mut self.cshape_kind {
            Rectangle(sh) => sh.highlight(value),
            RectangleRounded(sh) => sh.highlight(value),
            Hole(sh) => sh.highlight(value),
            Oblong(sh) => sh.highlight(value),
        }
    }
    pub fn highlight_modifiers(&mut self, value: bool) {
        use ShapeKind::*;
        match &mut self.cshape_kind {
            Rectangle(sh) => sh.highlight_modifiers(value),
            RectangleRounded(sh) => sh.highlight_modifiers(value),
            Hole(sh) => sh.highlight_modifiers(value),
            Oblong(sh) => sh.highlight_modifiers(value),
        }
    }
    pub fn is_highlighted(&self) -> bool {
        use ShapeKind::*;
        match &self.cshape_kind {
            Rectangle(sh) => sh.is_highlighted(),
            RectangleRounded(sh) => sh.is_highlighted(),
            Hole(sh) => sh.is_highlighted(),
            Oblong(sh) => sh.is_highlighted(),
        }
    }

    pub fn select_from_pos(&mut self, pos: Vec2) -> bool {
        use ShapeKind::*;
        match &mut self.cshape_kind {
            Rectangle(sh) => sh.select_from_pos(pos),
            RectangleRounded(sh) => sh.select_from_pos(pos),
            Hole(sh) => sh.select_from_pos(pos),
            Oblong(sh) => sh.select_from_pos(pos),
        }
    }
    pub fn select_modifiers_from_pos(&mut self, pos: Vec2) -> bool {
        use ShapeKind::*;
        match &mut self.cshape_kind {
            Rectangle(sh) => sh.select_modifiers_from_pos(pos),
            RectangleRounded(sh) => sh.select_modifiers_from_pos(pos),
            Hole(sh) => sh.select_modifiers_from_pos(pos),
            Oblong(sh) => sh.select_modifiers_from_pos(pos),
        }
    }
    pub fn select(&mut self, value: bool) {
        use ShapeKind::*;
        match &mut self.cshape_kind {
            Rectangle(sh) => sh.select(value),
            RectangleRounded(sh) => sh.select(value),
            Hole(sh) => sh.select(value),
            Oblong(sh) => sh.select(value),
        }
    }
    pub fn select_modifiers(&mut self, value: bool) {
        use ShapeKind::*;
        match &mut self.cshape_kind {
            Rectangle(sh) => sh.select_modifiers(value),
            RectangleRounded(sh) => sh.select_modifiers(value),
            Hole(sh) => sh.select_modifiers(value),
            Oblong(sh) => sh.select_modifiers(value),
        }
    }
    pub fn is_selected(&self) -> bool {
        use ShapeKind::*;
        match &self.cshape_kind {
            Rectangle(sh) => sh.is_selected(),
            RectangleRounded(sh) => sh.is_selected(),
            Hole(sh) => sh.is_selected(),
            Oblong(sh) => sh.is_selected(),
        }
    }

    pub fn move_selection(&mut self, pos_init: Vec2, pos: Vec2) {
        use ShapeKind::*;
        match &mut self.cshape_kind {
            Rectangle(sh) => sh.move_position(pos_init, pos),
            RectangleRounded(sh) => sh.move_position(pos_init, pos),
            Hole(sh) => sh.move_position(pos_init, pos),
            Oblong(sh) => sh.move_position(pos_init, pos),
        }
    }

    pub fn get_boolean_op(&self) -> OpType {
        self.boolean_op
    }

    pub fn get_pattern_operation(&self) -> Pattern {
        match (self.is_selected(), self.is_highlighted()) {
            (false, false) => Pattern::ComposedNormal(true),
            (false, true) => Pattern::ComposedHighlighted(true),
            (true, false) => Pattern::ComposedSelected(true),
            (true, true) => Pattern::ComposedSelected(true),
        }
    }
    pub fn get_paths(&self) -> Vec<(BezPath, Pattern)> {
        use ShapeKind::*;
        match &self.cshape_kind {
            Rectangle(sh) => sh.get_shape_paths(),
            RectangleRounded(sh) => sh.get_shape_paths(),
            Hole(sh) => sh.get_shape_paths(),
            Oblong(sh) => sh.get_shape_paths(),
        }
    }
    pub fn get_segs(&self) -> BezPath {
        let mut segs = BezPath::new();
        for path in self.get_paths() {
            flatten(path.0, 0.25, |s| segs.push(s));
        }
        segs
    }
    pub fn bez_path_to_geo_polygon(&self) -> Polygon<f64> {
        let bez_path = self.get_segs();

        let mut points = Vec::new();
        for element in bez_path.elements() {
            match element {
                PathEl::MoveTo(p) | PathEl::LineTo(p) => points.push((p.x, p.y)),
                PathEl::ClosePath => {
                    // Le polygone doit être fermé
                    if points.first() != points.last() {
                        points.push(points[0]);
                    }
                }
                _ => log!("Error: Non-linear path elements found."),
            }
        }

        if points.len() < 3 {
            unreachable!()
        } else {
            Polygon::new(points.into(), vec![])
        }
    }

    // fn _geo_polygon_to_bez_path(&self, polygon: &Polygon<f64>) -> Option<BezPath> {
    //     let mut bez_path = BezPath::new();
    //     let exterior = polygon.exterior();
    //     if exterior.is_empty() {
    //         return None; // Return None if the exterior is empty
    //     }
    //     // Iterate over the exterior points
    //     let points: Vec<Point> = exterior
    //         .coords()
    //         .map(|coord| Point::new(coord.x, coord.y))
    //         .collect();
    //     if points.len() < 2 {
    //         return None; // Not enough points to form a path
    //     }
    //     // Start with MoveTo
    //     bez_path.push(PathEl::MoveTo(points[0]));
    //     // Add LineTo for each segment
    //     for point in &points[1..] {
    //         bez_path.push(PathEl::LineTo(*point));
    //     }
    //     // Close the path if the polygon is closed
    //     if points.first() == points.last() {
    //         bez_path.push(PathEl::ClosePath);
    //     }
    //     Some(bez_path)
    // }

    // fn geo_polygon_to_bez_path(&self, polygon: &Polygon<f64>) -> Vec<BezPath> {
    //     let mut vec_bez_path: Vec<BezPath> = vec![];
    //     let mut bez_path = BezPath::new();
    //     // Convert exterior ring to bezier path
    //     let exterior = polygon.exterior();
    //     if exterior.is_empty() {
    //         return vec_bez_path; // Return None if the exterior is empty
    //     }
    //     let exterior_points: Vec<Point> = exterior
    //         .coords()
    //         .map(|coord| Point::new(coord.x, coord.y))
    //         .collect();
    //     if exterior_points.len() < 2 {
    //         return vec_bez_path; // Not enough points to form a path
    //     }
    //     // Add exterior points to path
    //     bez_path.push(PathEl::MoveTo(exterior_points[0]));
    //     for point in &exterior_points[1..] {
    //         bez_path.push(PathEl::LineTo(*point));
    //     }
    //     if exterior_points.first() == exterior_points.last() {
    //         bez_path.push(PathEl::ClosePath);
    //     }
    //     vec_bez_path.push(bez_path);
    //     bez_path = BezPath::new();
    //     // Convert interior rings (holes) to bezier paths
    //     for interior in polygon.interiors() {
    //         let interior_points: Vec<Point> = interior
    //             .coords()
    //             .map(|coord| Point::new(coord.x, coord.y))
    //             .collect();
    //         if interior_points.len() < 2 {
    //             continue; // Skip invalid rings
    //         }
    //         bez_path.push(PathEl::MoveTo(interior_points[0]));
    //         for point in &interior_points[1..] {
    //             bez_path.push(PathEl::LineTo(*point));
    //         }
    //         if interior_points.first() == interior_points.last() {
    //             bez_path.push(PathEl::ClosePath);
    //         }

    //         vec_bez_path.push(bez_path);
    //         bez_path = BezPath::new();
    //     }
    //     vec_bez_path
    // }
}
