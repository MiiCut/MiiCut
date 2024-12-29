// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

use std::fmt::Display;

use crate::{
    canvas_core::{Layer, Pattern},
    shape_hole::CShapeHole,
    shape_oblong::CShapeOblong,
    shape_rectangle::CShapeRectangle,
    shape_rectangle_rounded::CShapeRectRounded,
    shapes_pool::{CSPool, CShid},
};
use geo::{BooleanOps, HasDimensions, OpType, Polygon};
use kurbo::{flatten, BezPath, PathEl, Point, Vec2};

#[derive(Clone, Debug, PartialEq)]
pub enum CShapeKind {
    CRectangle(CShapeRectangle),
    CRectangleRounded(CShapeRectRounded),
    CHole(CShapeHole),
    COblong(CShapeOblong),
}
impl Display for CShapeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use CShapeKind::*;
        match self {
            CRectangle(sh) => write!(f, "{sh}"),
            CRectangleRounded(sh) => write!(f, "{sh}"),
            CHole(sh) => write!(f, "{sh}"),
            COblong(sh) => write!(f, "{sh}"),
        }
    }
}
pub trait CShapes {
    const TOLERANCE: f64;

    fn new(pos1: Vec2, pos2: Vec2) -> CShapeKind;
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
pub struct CShape {
    cshid: CShid,
    cshape_kind: CShapeKind,
    boolean_op: OpType,
    parent: Option<CShid>,
    children: CSPool,
    layer: Layer,
}
impl CShape {
    pub fn new(
        cshid: CShid,
        cshape_kind: CShapeKind,
        boolean_op: OpType,
        parent: Option<CShid>,
        layer: Layer,
    ) -> CShape {
        CShape {
            cshid,
            cshape_kind,
            boolean_op,
            parent,
            children: CSPool::new(),
            layer,
        }
    }
    pub fn get_id(&self) -> CShid {
        self.cshid
    }
    pub fn get_parent(&self) -> Option<CShid> {
        self.parent
    }
    pub fn get_children(&self) -> &CSPool {
        &self.children
    }
    pub fn get_children_mut(&mut self) -> &mut CSPool {
        &mut self.children
    }
    pub fn add_child(&mut self, cshape: CShape) {
        self.children.add_shape(cshape)
    }
    pub fn save_pos(&mut self) {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.save_pos(),
            CRectangleRounded(sh) => sh.save_pos(),
            CHole(sh) => sh.save_pos(),
            COblong(sh) => sh.save_pos(),
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
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.highlight_from_pos(pos),
            CRectangleRounded(sh) => sh.highlight_from_pos(pos),
            CHole(sh) => sh.highlight_from_pos(pos),
            COblong(sh) => sh.highlight_from_pos(pos),
        }
    }
    pub fn highlight_modifiers_from_pos(&mut self, pos: Vec2) -> bool {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.highlight_modifiers_from_pos(pos),
            CRectangleRounded(sh) => sh.highlight_modifiers_from_pos(pos),
            CHole(sh) => sh.highlight_modifiers_from_pos(pos),
            COblong(sh) => sh.highlight_modifiers_from_pos(pos),
        }
    }
    pub fn highlight(&mut self, value: bool) {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.highlight(value),
            CRectangleRounded(sh) => sh.highlight(value),
            CHole(sh) => sh.highlight(value),
            COblong(sh) => sh.highlight(value),
        }
    }
    pub fn highlight_modifiers(&mut self, value: bool) {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.highlight_modifiers(value),
            CRectangleRounded(sh) => sh.highlight_modifiers(value),
            CHole(sh) => sh.highlight_modifiers(value),
            COblong(sh) => sh.highlight_modifiers(value),
        }
    }
    pub fn is_highlighted(&self) -> bool {
        use CShapeKind::*;
        match &self.cshape_kind {
            CRectangle(sh) => sh.is_highlighted(),
            CRectangleRounded(sh) => sh.is_highlighted(),
            CHole(sh) => sh.is_highlighted(),
            COblong(sh) => sh.is_highlighted(),
        }
    }

    pub fn select_from_pos(&mut self, pos: Vec2) -> bool {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.select_from_pos(pos),
            CRectangleRounded(sh) => sh.select_from_pos(pos),
            CHole(sh) => sh.select_from_pos(pos),
            COblong(sh) => sh.select_from_pos(pos),
        }
    }
    pub fn select_modifiers_from_pos(&mut self, pos: Vec2) -> bool {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.select_modifiers_from_pos(pos),
            CRectangleRounded(sh) => sh.select_modifiers_from_pos(pos),
            CHole(sh) => sh.select_modifiers_from_pos(pos),
            COblong(sh) => sh.select_modifiers_from_pos(pos),
        }
    }
    pub fn select(&mut self, value: bool) {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.select(value),
            CRectangleRounded(sh) => sh.select(value),
            CHole(sh) => sh.select(value),
            COblong(sh) => sh.select(value),
        }
    }
    pub fn select_modifiers(&mut self, value: bool) {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.select_modifiers(value),
            CRectangleRounded(sh) => sh.select_modifiers(value),
            CHole(sh) => sh.select_modifiers(value),
            COblong(sh) => sh.select_modifiers(value),
        }
    }
    pub fn is_selected(&self) -> bool {
        use CShapeKind::*;
        match &self.cshape_kind {
            CRectangle(sh) => sh.is_selected(),
            CRectangleRounded(sh) => sh.is_selected(),
            CHole(sh) => sh.is_selected(),
            COblong(sh) => sh.is_selected(),
        }
    }

    pub fn move_selection(&mut self, pos_init: Vec2, pos: Vec2) {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.move_position(pos_init, pos),
            CRectangleRounded(sh) => sh.move_position(pos_init, pos),
            CHole(sh) => sh.move_position(pos_init, pos),
            COblong(sh) => sh.move_position(pos_init, pos),
        }
    }

    pub fn get_layer(&self) -> Layer {
        self.layer
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

    pub fn get_full_segs(&self) -> Vec<BezPath> {
        // Init: we take the root polygon
        let mut polygons = vec![self.bez_path_to_geo_polygon()];
        let childs: Vec<(Polygon, OpType)> = self.childs_bez_path_to_geo_polygons();

        for (child_polygon, boolean_op) in childs {
            let mut result_polygons = vec![];
            polygons
                .iter()
                .for_each(|p| result_polygons.extend(p.boolean_op(&child_polygon, boolean_op)));
            polygons = result_polygons;
        }

        let mut multi_paths = Vec::new();

        polygons.iter().for_each(|p| {
            multi_paths.extend(self.geo_polygon_to_bez_path(&p));
        });
        multi_paths
    }

    fn childs_bez_path_to_geo_polygons(&self) -> Vec<(Polygon<f64>, OpType)> {
        // Récupérer les segments des enfants
        let mut childs_segs = Vec::new();
        self.children
            .values()
            .for_each(|c| childs_segs.push((c.bez_path_to_geo_polygon(), c.get_boolean_op())));
        // Sort by OpType, prioritizing Union over Difference
        childs_segs.sort_by(|a, b| match (&a.1, &b.1) {
            (OpType::Union, OpType::Difference) => std::cmp::Ordering::Less,
            (OpType::Difference, OpType::Union) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        });
        childs_segs
    }

    pub fn get_paths(&self) -> Vec<(BezPath, Pattern)> {
        use CShapeKind::*;
        match &self.cshape_kind {
            CRectangle(sh) => sh.get_shape_paths(),
            CRectangleRounded(sh) => sh.get_shape_paths(),
            CHole(sh) => sh.get_shape_paths(),
            COblong(sh) => sh.get_shape_paths(),
        }
    }

    pub fn get_segs(&self) -> BezPath {
        let mut segs = BezPath::new();
        for path in self.get_paths() {
            flatten(path.0, 0.25, |s| segs.push(s));
        }
        segs
    }

    fn bez_path_to_geo_polygon(&self) -> Polygon<f64> {
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

    fn _geo_polygon_to_bez_path(&self, polygon: &Polygon<f64>) -> Option<BezPath> {
        let mut bez_path = BezPath::new();

        let exterior = polygon.exterior();
        if exterior.is_empty() {
            return None; // Return None if the exterior is empty
        }

        // Iterate over the exterior points
        let points: Vec<Point> = exterior
            .coords()
            .map(|coord| Point::new(coord.x, coord.y))
            .collect();
        if points.len() < 2 {
            return None; // Not enough points to form a path
        }
        // Start with MoveTo
        bez_path.push(PathEl::MoveTo(points[0]));
        // Add LineTo for each segment
        for point in &points[1..] {
            bez_path.push(PathEl::LineTo(*point));
        }
        // Close the path if the polygon is closed
        if points.first() == points.last() {
            bez_path.push(PathEl::ClosePath);
        }
        Some(bez_path)
    }

    fn geo_polygon_to_bez_path(&self, polygon: &Polygon<f64>) -> Vec<BezPath> {
        let mut vec_bez_path: Vec<BezPath> = vec![];
        let mut bez_path = BezPath::new();
        // Convert exterior ring to bezier path
        let exterior = polygon.exterior();
        if exterior.is_empty() {
            return vec_bez_path; // Return None if the exterior is empty
        }
        let exterior_points: Vec<Point> = exterior
            .coords()
            .map(|coord| Point::new(coord.x, coord.y))
            .collect();
        if exterior_points.len() < 2 {
            return vec_bez_path; // Not enough points to form a path
        }
        // Add exterior points to path
        bez_path.push(PathEl::MoveTo(exterior_points[0]));
        for point in &exterior_points[1..] {
            bez_path.push(PathEl::LineTo(*point));
        }
        if exterior_points.first() == exterior_points.last() {
            bez_path.push(PathEl::ClosePath);
        }

        vec_bez_path.push(bez_path);
        bez_path = BezPath::new();

        // Convert interior rings (holes) to bezier paths
        for interior in polygon.interiors() {
            let interior_points: Vec<Point> = interior
                .coords()
                .map(|coord| Point::new(coord.x, coord.y))
                .collect();
            if interior_points.len() < 2 {
                continue; // Skip invalid rings
            }
            bez_path.push(PathEl::MoveTo(interior_points[0]));
            for point in &interior_points[1..] {
                bez_path.push(PathEl::LineTo(*point));
            }
            if interior_points.first() == interior_points.last() {
                bez_path.push(PathEl::ClosePath);
            }

            vec_bez_path.push(bez_path);
            bez_path = BezPath::new();
        }
        vec_bez_path
    }
}
