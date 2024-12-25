// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

use crate::{
    canvas_core::{Layer, Pattern},
    handles::Handle,
    shapes_pool::{CSPool, CShapeKind, CShid},
};
use geo::{BooleanOps, HasDimensions, OpType, Polygon};
use kurbo::{flatten, BezPath, PathEl, Point, Vec2};

pub trait CShapes {
    const TOLERANCE: f64;

    fn new(pos1: Vec2, pos2: Vec2) -> CShapeKind;
    fn save_pos(&mut self);
    fn toggle_prop(&mut self);
    fn get_shape_path(&self) -> BezPath;

    fn highlight_handles(&mut self, pos: Vec2, precision: f64) -> bool;
    fn highlight_shape(&mut self, pos: Vec2) -> bool;
    fn set_highlight(&mut self, value: bool);
    fn is_highlighted(&self) -> bool;

    fn select_handles(&mut self, pos: Vec2, precision: f64) -> bool;
    fn select_shape(&mut self, pos: Vec2) -> bool;
    fn set_selection(&mut self, value: bool);
    fn is_selected(&self) -> bool;
    fn clear_selection(&mut self);
    fn clear_selection_all(&mut self);

    fn get_position(&self) -> Vec2;
    fn move_position(&mut self, pos_init: Vec2, pos: Vec2);
    fn get_handles(&self) -> Vec<Handle>;
    // Return the first handle selected found or None
    fn get_handle_selected(&self) -> Option<(Handle, usize)>;
    // Return the first handle highlighted found or None
    fn get_handle_highlighted(&self) -> Option<(Handle, usize)>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct CShape {
    cshid: CShid,
    cshape_kind: CShapeKind,
    parent: Option<CShid>,
    children: CSPool,
    layer: Layer,
}
impl CShape {
    pub fn new(
        cshid: CShid,
        cshape_kind: CShapeKind,
        parent: Option<CShid>,
        layer: Layer,
    ) -> CShape {
        CShape {
            cshid,
            cshape_kind,
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
    pub fn toggle_prop(&mut self) {
        ()
    }

    pub fn highlight_handles(&mut self, pos: Vec2, precision: f64) -> bool {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.highlight_handles(pos, precision),
            CRectangleRounded(sh) => sh.highlight_handles(pos, precision),
            CHole(sh) => sh.highlight_handles(pos, precision),
            COblong(sh) => sh.highlight_handles(pos, precision),
        }
    }
    pub fn highlight_shape(&mut self, pos: Vec2) -> bool {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.highlight_shape(pos),
            CRectangleRounded(sh) => sh.highlight_shape(pos),
            CHole(sh) => sh.highlight_shape(pos),
            COblong(sh) => sh.highlight_shape(pos),
        }
    }
    pub fn set_highlight(&mut self, value: bool) {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.set_highlight(value),
            CRectangleRounded(sh) => sh.set_highlight(value),
            CHole(sh) => sh.set_highlight(value),
            COblong(sh) => sh.set_highlight(value),
        }
    }
    pub fn is_highlighted(&self) -> bool {
        use CShapeKind::*;
        match self.cshape_kind {
            CRectangle(sh) => sh.is_highlighted(),
            CRectangleRounded(sh) => sh.is_highlighted(),
            CHole(sh) => sh.is_highlighted(),
            COblong(sh) => sh.is_highlighted(),
        }
    }

    pub fn select_handles(&mut self, pos: Vec2, precision: f64) -> bool {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.select_handles(pos, precision),
            CRectangleRounded(sh) => sh.select_handles(pos, precision),
            CHole(sh) => sh.select_handles(pos, precision),
            COblong(sh) => sh.select_handles(pos, precision),
        }
    }
    pub fn select_shape(&mut self, pos: Vec2) -> bool {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.select_shape(pos),
            CRectangleRounded(sh) => sh.select_shape(pos),
            CHole(sh) => sh.select_shape(pos),
            COblong(sh) => sh.select_shape(pos),
        }
    }
    pub fn set_selection(&mut self, value: bool) {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.set_selection(value),
            CRectangleRounded(sh) => sh.set_selection(value),
            CHole(sh) => sh.set_selection(value),
            COblong(sh) => sh.set_selection(value),
        }
    }
    pub fn is_selected(&self) -> bool {
        use CShapeKind::*;
        match self.cshape_kind {
            CRectangle(sh) => sh.is_selected(),
            CRectangleRounded(sh) => sh.is_selected(),
            CHole(sh) => sh.is_selected(),
            COblong(sh) => sh.is_selected(),
        }
    }
    pub fn clear_selection(&mut self) {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.clear_selection(),
            CRectangleRounded(sh) => sh.clear_selection(),
            CHole(sh) => sh.clear_selection(),
            COblong(sh) => sh.clear_selection(),
        }
    }
    pub fn clear_selection_all(&mut self) {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.clear_selection_all(),
            CRectangleRounded(sh) => sh.clear_selection_all(),
            CHole(sh) => sh.clear_selection_all(),
            COblong(sh) => sh.clear_selection_all(),
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

    pub fn get_handles(&self) -> Vec<Handle> {
        use CShapeKind::*;
        let mut handles = match self.cshape_kind {
            CRectangle(sh) => sh.get_handles(),
            CRectangleRounded(sh) => sh.get_handles(),
            CHole(sh) => sh.get_handles(),
            COblong(sh) => sh.get_handles(),
        };
        self.children.values().for_each(|child| {
            handles.extend(child.get_handles());
        });
        handles
    }
    pub fn get_layer(&self) -> Layer {
        self.layer
    }
    pub fn get_pattern(&self) -> Pattern {
        match (self.is_selected(), self.is_highlighted()) {
            (false, false) => Pattern::BasicNormal,
            (false, true) => Pattern::BasicHighlighted,
            (true, false) => Pattern::BasicSelected,
            (true, true) => Pattern::BasicSelected,
        }
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
        let mut multi_paths = Vec::new();

        // Init: we take the root polygon
        let mut polygons = vec![self.bez_path_to_geo_polygon()];

        for child_polygon in self.get_child_geo_polygons() {
            // for each polygon in polygons, we boolean op with each child
            let mut result_polygons = vec![];
            for polygon in polygons.iter() {
                result_polygons.extend(polygon.boolean_op(&child_polygon, OpType::Difference));
            }
            polygons = result_polygons;
        }
        for polygon in polygons {
            if let Some(path) = self.geo_polygon_to_bez_path(&polygon) {
                multi_paths.push(path);
            }
        }

        multi_paths
    }

    fn get_child_geo_polygons(&self) -> Vec<Polygon<f64>> {
        // Récupérer les segments des enfants
        let mut childs_segs: Vec<Polygon<f64>> = Vec::new();
        for child in self.children.values() {
            childs_segs.push(child.bez_path_to_geo_polygon());
        }
        childs_segs
    }

    pub fn get_path(&self) -> BezPath {
        use CShapeKind::*;
        match self.cshape_kind {
            CRectangle(sh) => sh.get_shape_path(),
            CRectangleRounded(sh) => sh.get_shape_path(),
            CHole(sh) => sh.get_shape_path(),
            COblong(sh) => sh.get_shape_path(),
        }
    }

    pub fn get_segs(&self) -> BezPath {
        let mut segs = BezPath::new();
        flatten(self.get_path(), 0.25, |s| segs.push(s));
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

    fn geo_polygon_to_bez_path(&self, polygon: &Polygon<f64>) -> Option<BezPath> {
        let exterior = polygon.exterior();
        if exterior.is_empty() {
            return None; // Return None if the exterior is empty
        }

        let mut bez_path = BezPath::new();

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
}
