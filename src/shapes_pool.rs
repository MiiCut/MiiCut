use crate::{
    shape_disc::{HighLightOrSelect, ShapeDisc},
    shape_oblong::ShapeOblong,
    shape_rectangle::ShapeRectangle,
    shape_rectangle_rounded::ShapeRectRounded,
    shapes::{Shape, Shapes},
    IconsShapes,
};
use geo::{BooleanOps, HasDimensions, Intersects, MultiPolygon, OpType, Point, Polygon};
use kurbo::{BezPath, PathEl, Vec2};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Display,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
    vec,
};

#[derive(Clone, Debug, PartialEq)]
struct ShapeSelector {
    selectable_shapes: Vec<Shid>, // IDs of selectable shapes
    current_index: usize,         // Current index in the list
}
impl ShapeSelector {
    pub fn new() -> Self {
        Self {
            selectable_shapes: Vec::new(),
            current_index: 0,
        }
    }
    pub fn update_shapes(&mut self, new_shapes: HashSet<Shid>) {
        let current_set: HashSet<_> = self.selectable_shapes.iter().cloned().collect();
        // Compare the sets, ignoring order
        if current_set != new_shapes {
            // Reset if the set of shapes changes
            self.selectable_shapes = new_shapes.into_iter().collect();
            self.current_index = 0;
        }
    }
    pub fn next_selection(&mut self) -> Option<Shid> {
        if self.selectable_shapes.is_empty() {
            return None;
        }
        // Select the current shape and move to the next
        let selected = self.selectable_shapes[self.current_index];
        self.current_index = (self.current_index + 1) % self.selectable_shapes.len();
        Some(selected)
    }
}

pub struct CShapeBuilder;
impl CShapeBuilder {
    pub fn new(icon_shape: IconsShapes, pos1: Vec2, pos2: Vec2, boolean_op: OpType) -> Shape {
        let cshid = Shid::new();
        let shape_kind = match icon_shape {
            IconsShapes::Rectangle => ShapeRectangle::new(pos1, pos2),
            IconsShapes::RectangleRounded => ShapeRectRounded::new(pos1, pos2),
            IconsShapes::Disc => ShapeDisc::new(pos1, pos2),
            IconsShapes::Oblong => ShapeOblong::new(pos1, pos2),
        };
        Shape::new(cshid, shape_kind, boolean_op)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CSPool {
    shapes: HashMap<Shid, Shape>,
    shapes_selector: ShapeSelector,
}
impl CSPool {
    pub fn new() -> CSPool {
        CSPool {
            shapes: HashMap::new(),
            shapes_selector: ShapeSelector::new(),
        }
    }
    pub fn add_shape(&mut self, cshape: Shape) {
        let cshid = cshape.get_id();
        self.shapes.insert(cshid, cshape);
    }
    pub fn delete_shape(&mut self, cshid: Shid) -> Option<Shape> {
        self.shapes.remove(&cshid)
    }
    pub fn get_shape(&self, target_cshid: Shid) -> Option<&Shape> {
        self.shapes.get(&target_cshid)
    }
    pub fn get_shape_mut(&mut self, target_cshid: Shid) -> Option<&mut Shape> {
        self.shapes.get_mut(&target_cshid)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Shid, &Shape)> {
        self.shapes.iter()
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Shid, &mut Shape)> {
        self.shapes.iter_mut()
    }
    pub fn values(&self) -> impl Iterator<Item = &Shape> {
        self.shapes.values()
    }
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Shape> {
        self.shapes.values_mut()
    }

    pub fn save_positions(&mut self) {
        self.shapes.values_mut().for_each(|cs| {
            cs.save_pos();
        });
    }

    pub fn intersection_set(&self, shid: Shid) -> HashSet<Shid> {
        let mut result = HashSet::new();
        if let Some(shape) = self.get_shape(shid) {
            for (k, v) in self.shapes.iter() {
                if k == &shid {
                    // result.insert(*k);
                    continue;
                }
                if shape.get_polygon().intersects(&v.get_polygon()) {
                    result.insert(*k);
                }
            }
        }
        result
    }
    pub fn connected_shapes(&self, start_shid: Shid) -> HashSet<Shid> {
        let mut visited = HashSet::new(); // Tracks visited shapes
        let mut to_visit = VecDeque::new(); // Queue for BFS

        // Start with the initial shape
        to_visit.push_back(start_shid);
        visited.insert(start_shid);

        while let Some(current_shid) = to_visit.pop_front() {
            // Retrieve shapes intersecting the current shape
            let intersecting_shapes = self.intersection_set(current_shid);

            for neighbor in intersecting_shapes {
                // If the neighbor hasn't been visited yet
                if !visited.contains(&neighbor) {
                    visited.insert(neighbor); // Mark it as visited
                    to_visit.push_back(neighbor); // Add it to the queue for further exploration
                }
            }
        }

        visited
    }

    pub fn select_all_connected(&mut self) -> bool {
        let mut res = false;

        if let Some(start_shid) = self.get_hors(HighLightOrSelect::Select).get(0).copied() {
            let connected_shids = self.connected_shapes(start_shid);
            log!("Connected shapes: {:?}", connected_shids);
            connected_shids.iter().for_each(|shid| {
                if let Some(cshape) = self.get_shape_mut(*shid) {
                    cshape.set_hors(true, HighLightOrSelect::Select);
                    res = true;
                }
            });
        }
        res
    }
    pub fn set_hors_from_pos(&mut self, pos: Vec2, hors: HighLightOrSelect) -> bool {
        let mut res = false;
        self.shapes.values_mut().for_each(|cs| {
            res |= cs.hors_from_pos(pos, hors);
        });
        res
    }
    pub fn get_hors(&self, hors: HighLightOrSelect) -> Vec<Shid> {
        let mut result = vec![];
        for cshape in self.shapes.values() {
            if cshape.get_hors(hors) {
                result.push(cshape.get_id());
            }
        }
        result
    }
    pub fn get_center_hors(&self, hors: HighLightOrSelect) -> Vec<Shid> {
        let mut result = vec![];
        for cshape in self.shapes.values() {
            if cshape.get_center_hors(hors) {
                result.push(cshape.get_id());
            }
        }
        result
    }

    pub fn set_center_hors_from_pos(
        &mut self,
        pos: Vec2,
        _precision: f64,
        hors: HighLightOrSelect,
    ) -> bool {
        let mut setted = false;
        self.shapes.values_mut().for_each(|cs| {
            setted |= cs.hors_center_from_pos(pos, hors);
        });
        setted
    }
    pub fn set_hors_modifiers_from_pos(
        &mut self,
        pos: Vec2,
        _precision: f64,
        hors: HighLightOrSelect,
    ) -> bool {
        let mut setted = false;
        self.shapes.values_mut().for_each(|cs| {
            setted |= cs.hors_modifiers_from_pos(pos, hors);
        });
        setted
    }
    pub fn set_hors(&mut self, value: bool, hors: HighLightOrSelect) {
        self.shapes.values_mut().for_each(|cs| {
            cs.set_hors(value, hors);
        });
    }
    pub fn set_hors_centers(&mut self, value: bool, hors: HighLightOrSelect) {
        self.shapes.values_mut().for_each(|cs| {
            cs.set_hors_center(value, hors);
        });
    }
    pub fn set_hors_modifiers(&mut self, value: bool, hors: HighLightOrSelect) {
        self.shapes.values_mut().for_each(|cs| {
            cs.set_hors_modifiers(value, hors);
        });
    }
    pub fn move_selection(&mut self, pos_dwn: Vec2, cursor_pos: Vec2) {
        self.shapes.values_mut().for_each(|cs| {
            cs.move_selection(pos_dwn, cursor_pos);
        });
    }
    pub fn move_selection_from_shid(&mut self, cshid: Shid, pos_dwn: Vec2, cursor_pos: Vec2) {
        if let Some(cshape) = self.get_shape_mut(cshid) {
            cshape.move_selection(pos_dwn, cursor_pos);
        }
    }

    pub fn delete_objects_selected(&mut self) {
        self.shapes
            .retain(|_, v| !v.get_hors(HighLightOrSelect::Select));
    }

    pub fn get_full_segs(&mut self) -> Vec<BezPath> {
        // Sort by OpType, prioritizing Union over Difference
        let mut shapes = self.shapes.clone().into_values().collect::<Vec<_>>();
        shapes.sort_by(|a, b| match (a.get_boolean_op(), b.get_boolean_op()) {
            (OpType::Union, OpType::Difference) => std::cmp::Ordering::Less,
            (OpType::Difference, OpType::Union) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        });

        let polygons: Vec<(Polygon, OpType)> = shapes
            .iter()
            .map(|cs| (cs.get_polygon(), cs.get_boolean_op()))
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
