use crate::{
    shape_hole::ShapeHole,
    shape_oblong::ShapeOblong,
    shape_rectangle::ShapeRectangle,
    shape_rectangle_rounded::ShapeRectRounded,
    shapes::{Shape, ShapeKind, Shapes},
};
use geo::{BooleanOps, HasDimensions, MultiPolygon, OpType, Point, Polygon};
use kurbo::{BezPath, PathEl, Vec2};
use std::{
    collections::HashMap,
    fmt::Display,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
    vec,
};

pub struct CShapeBuilder;
impl CShapeBuilder {
    fn new(cshape_kind: ShapeKind, boolean_op: OpType) -> Shape {
        let cshid = Shid::new();
        Shape::new(cshid, cshape_kind, boolean_op)
    }
    pub fn new_rectangle(pos1: Vec2, pos2: Vec2, boolean_op: OpType) -> Shape {
        log!("Creating rectangle");
        let cshape_kind = ShapeRectangle::new(pos1, pos2);
        CShapeBuilder::new(cshape_kind, boolean_op)
    }
    pub fn new_rectangle_rounded(pos1: Vec2, pos2: Vec2, boolean_op: OpType) -> Shape {
        let cshape_kind = ShapeRectRounded::new(pos1, pos2);
        CShapeBuilder::new(cshape_kind, boolean_op)
    }
    pub fn new_circle(pos1: Vec2, pos2: Vec2, boolean_op: OpType) -> Shape {
        let cshape_kind = ShapeHole::new(pos1, pos2);
        CShapeBuilder::new(cshape_kind, boolean_op)
    }

    pub fn new_oblong(pos1: Vec2, pos2: Vec2, boolean_op: OpType) -> Shape {
        let cshape_kind = ShapeOblong::new(pos1, pos2);
        CShapeBuilder::new(cshape_kind, boolean_op)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct CSPool {
    cshapes: HashMap<Shid, Shape>,
}
impl CSPool {
    pub fn new() -> CSPool {
        CSPool {
            cshapes: HashMap::new(),
        }
    }
    pub fn add_shape(&mut self, cshape: Shape) {
        let cshid = cshape.get_id();
        self.cshapes.insert(cshid, cshape);
    }
    pub fn delete_shape(&mut self, cshid: Shid) -> Option<Shape> {
        self.cshapes.remove(&cshid)
    }
    pub fn get_shape(&self, target_cshid: Shid) -> Option<&Shape> {
        self.cshapes.get(&target_cshid)
    }
    pub fn get_shape_mut(&mut self, target_cshid: Shid) -> Option<&mut Shape> {
        self.cshapes.get_mut(&target_cshid)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Shid, &Shape)> {
        self.cshapes.iter()
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Shid, &mut Shape)> {
        self.cshapes.iter_mut()
    }
    pub fn values(&self) -> impl Iterator<Item = &Shape> {
        self.cshapes.values()
    }
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Shape> {
        self.cshapes.values_mut()
    }

    pub fn save_positions(&mut self) {
        self.cshapes.values_mut().for_each(|cs| {
            cs.save_pos();
        });
    }

    // Highlighting
    pub fn highlight_object(&mut self, pos: Vec2, _precision: f64) {
        let mut modifier_highlighted = false;
        self.cshapes.values_mut().for_each(|cs| {
            cs.highlight(false);
            if cs.highlight_modifiers_from_pos(pos) {
                modifier_highlighted = true;
            }
        });
        if !modifier_highlighted {
            self.cshapes.values_mut().for_each(|cs| {
                cs.highlight_from_pos(pos);
            });
        }
    }
    pub fn get_first_highlighted(&self) -> Option<Shid> {
        for cshape in self.cshapes.values() {
            if cshape.is_highlighted() {
                return Some(cshape.get_id());
            }
        }
        None
    }
    pub fn get_highlighted(&self) -> Vec<Shid> {
        let mut highlight = vec![];
        for cshape in self.cshapes.values() {
            if cshape.is_highlighted() {
                highlight.push(cshape.get_id());
            }
        }
        highlight
    }

    // Selections
    pub fn select_object(&mut self, pos: Vec2, _precision: f64) {
        let mut modifier_selected = false;
        self.cshapes.values_mut().for_each(|cs| {
            cs.select(false);
            if cs.select_modifiers_from_pos(pos) {
                modifier_selected = true;
            }
        });
        if !modifier_selected {
            self.cshapes.values_mut().for_each(|cs| {
                cs.select_from_pos(pos);
            });
        }
    }
    pub fn clear_selections(&mut self) {
        self.cshapes.values_mut().for_each(|cs| {
            cs.select(false);
        });
    }
    pub fn clear_selections_all(&mut self) {
        self.cshapes.values_mut().for_each(|cs| {
            cs.select(false);
            cs.select_modifiers(false);
        });
    }
    pub fn move_selection(&mut self, pos_dwn: Vec2, cursor_pos: Vec2) {
        self.cshapes.values_mut().for_each(|cs| {
            cs.move_selection(pos_dwn, cursor_pos);
        });
    }
    pub fn move_selection_from_shid(&mut self, cshid: Shid, pos_dwn: Vec2, cursor_pos: Vec2) {
        if let Some(cshape) = self.get_shape_mut(cshid) {
            cshape.move_selection(pos_dwn, cursor_pos);
        }
    }
    pub fn get_selection(&self) -> Vec<Shid> {
        let mut selection = vec![];
        self.cshapes.values().for_each(|cs| {
            if cs.is_selected() {
                selection.push(cs.get_id());
            }
        });
        selection
    }
    pub fn delete_objects_selected(&mut self) {
        self.cshapes.retain(|_, v| !v.is_selected());
    }

    pub fn get_full_segs(&mut self) -> Vec<BezPath> {
        // Sort by OpType, prioritizing Union over Difference
        let mut shapes = self.cshapes.clone().into_values().collect::<Vec<_>>();
        shapes.sort_by(|a, b| match (a.get_boolean_op(), b.get_boolean_op()) {
            (OpType::Union, OpType::Difference) => std::cmp::Ordering::Less,
            (OpType::Difference, OpType::Union) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        });

        let polygons: Vec<(Polygon, OpType)> = shapes
            .iter()
            .map(|cs| (cs.bez_path_to_geo_polygon(), cs.get_boolean_op()))
            .collect();

        let mut multi_polygon = MultiPolygon(vec![]);
        for (idx, p) in polygons.iter().enumerate() {
            if idx == 0 {
                multi_polygon = MultiPolygon(vec![p.0.clone()]);
            } else {
                multi_polygon = multi_polygon.boolean_op(&p.0, p.1);
            }
        }
        let mut multi_paths = Vec::new();
        multi_polygon.iter().for_each(|p| {
            multi_paths.extend(self.geo_polygon_to_bez_path(&p));
        });
        multi_paths
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
        bez_path.push(PathEl::MoveTo(kurbo::Point::new(
            exterior_points[0].x(),
            exterior_points[0].y(),
        )));
        for point in &exterior_points[1..] {
            bez_path.push(PathEl::LineTo(kurbo::Point::new(point.x(), point.y())));
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
            bez_path.push(PathEl::MoveTo(kurbo::Point::new(
                interior_points[0].x(),
                interior_points[0].y(),
            )));
            for point in &interior_points[1..] {
                bez_path.push(PathEl::LineTo(kurbo::Point::new(point.x(), point.y())));
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

static COUNTER_CLOSED_SHAPES: AtomicUsize = AtomicUsize::new(0);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd)]
pub struct Shid {
    id: usize,
}
impl Deref for Shid {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.id
    }
}
impl DerefMut for Shid {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.id
    }
}
impl Display for Shid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
impl Shid {
    pub fn new() -> Shid {
        Shid {
            id: COUNTER_CLOSED_SHAPES.fetch_add(1, Ordering::Relaxed),
        }
    }
}
