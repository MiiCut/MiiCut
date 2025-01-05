use crate::{
    math::*,
    shape_disc::{HighLightOrSelect, ShapeDisc},
    shape_oblong::ShapeOblong,
    shape_rectangle::ShapeRectangle,
    shape_rectangle_rounded::ShapeRectRounded,
    shapes::{Shape, Shapes},
    IconsShapes,
};
use geo::{BooleanOps, Intersects, MultiPolygon, OpType, Polygon};
use kurbo::{BezPath, Vec2};
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
        let shid = Shid::new();
        let shape_kind = match icon_shape {
            IconsShapes::Rectangle => ShapeRectangle::new(pos1, pos2),
            IconsShapes::RectangleRounded => ShapeRectRounded::new(pos1, pos2),
            IconsShapes::Disc => ShapeDisc::new(pos1, pos2),
            IconsShapes::Oblong => ShapeOblong::new(pos1, pos2),
        };
        Shape::new(shid, shape_kind, boolean_op)
    }
    // Clone an existing shape
    pub fn clone(cshape: &Shape) -> (Shid, Shape) {
        let shid = Shid::new();
        let shape_kind = cshape.clone();
        (shid, Shape::new(shid, shape_kind, cshape.get_boolean_op()))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapesPool {
    shapes: HashMap<Shid, Shape>,
    shapes_selector: ShapeSelector,
    full_segs: Vec<BezPath>,
}
impl ShapesPool {
    pub fn new() -> ShapesPool {
        ShapesPool {
            shapes: HashMap::new(),
            shapes_selector: ShapeSelector::new(),
            full_segs: Vec::new(),
        }
    }
    pub fn add_shape(&mut self, cshape: Shape) {
        let shid = cshape.get_id();
        self.shapes.insert(shid, cshape);
    }
    pub fn delete_shape(&mut self, shid: Shid) -> Option<Shape> {
        self.shapes.remove(&shid)
    }

    pub fn get_shape(&self, target_shid: Shid) -> Option<&Shape> {
        self.shapes.get(&target_shid)
    }
    pub fn get_shape_mut(&mut self, target_shid: Shid) -> Option<&mut Shape> {
        self.shapes.get_mut(&target_shid)
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
            cs.save_positions();
        });
    }
    pub fn intersection_set(&self, shid: Shid) -> HashSet<Shid> {
        let mut result = HashSet::new();
        if let Some(shape) = self.shapes.get(&shid) {
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
    // BFS algorithm: https://en.wikipedia.org/wiki/Breadth-first_search
    pub fn connected_shapes(&self, start_shid: Shid) -> HashSet<Shid> {
        // Tracks visited shapes
        let mut visited = HashSet::new();
        // Queue for BFS
        let mut to_visit = VecDeque::new();
        // Start with the initial shape
        to_visit.push_back(start_shid);
        visited.insert(start_shid);
        while let Some(current_shid) = to_visit.pop_front() {
            // Retrieve shapes intersecting the current shape
            let intersecting_shapes = self.intersection_set(current_shid);
            for neighbor in intersecting_shapes {
                // If the neighbor hasn't been visited yet
                if !visited.contains(&neighbor) {
                    // Mark it as visited
                    visited.insert(neighbor);
                    // Add it to the queue for further exploration
                    to_visit.push_back(neighbor);
                }
            }
        }
        visited
    }
    pub fn select_all_connected(&mut self) -> bool {
        let mut res = false;

        if let Some(start_shid) = self.get_hors(HighLightOrSelect::Select).get(0).copied() {
            let connected_shids = self.connected_shapes(start_shid);
            connected_shids.iter().for_each(|shid| {
                if let Some(cshape) = self.shapes.get_mut(shid) {
                    cshape.set_hors(true, HighLightOrSelect::Select);
                    res = true;
                }
            });
        }
        res
    }

    pub fn set_hors_from_pos(&mut self, pos: Vec2, hors: HighLightOrSelect) -> bool {
        use HighLightOrSelect::*;
        if let Highlight = hors {
            let mut res = false;
            self.shapes.values_mut().for_each(|cs| {
                res |= cs.set_hors_from_pos(pos, Highlight);
            });
            return res;
        } else {
            let mut overlapping_shapes = HashSet::new();
            self.shapes.values_mut().for_each(|cs| {
                if cs.set_hors_from_pos(pos, Select) {
                    overlapping_shapes.insert(cs.get_id());
                }
            });
            // Update the ShapeSelector with the overlapping shapes
            self.shapes_selector.update_shapes(overlapping_shapes);

            // Toggle to the next shape
            self.shapes
                .values_mut()
                .for_each(|shape| shape.set_hors(false, Select));

            if let Some(next_shid) = self.shapes_selector.next_selection() {
                // Find and select the next shape
                if let Some(shape) = self.shapes.get_mut(&next_shid) {
                    log!("Selecting shapeee {}", next_shid);
                    shape.set_hors(true, Select);
                    return true;
                }
            }
            false
        }
    }
    pub fn set_hors_modifiers_from_pos(
        &mut self,
        pos: Vec2,
        _precision: f64,
        hors: HighLightOrSelect,
    ) -> bool {
        let mut setted = false;
        self.shapes.values_mut().for_each(|cs| {
            setted |= cs.set_hors_modifiers_from_pos(pos, hors);
        });
        setted
    }
    pub fn set_hors(&mut self, value: bool, hors: HighLightOrSelect) {
        self.shapes.values_mut().for_each(|cs| {
            cs.set_hors(value, hors);
        });
    }
    pub fn set_hors_modifiers(&mut self, value: bool, hors: HighLightOrSelect) {
        self.shapes.values_mut().for_each(|cs| {
            cs.set_hors_modifiers(value, hors);
        });
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
    pub fn set_hors_from_shid(&mut self, shid: Shid, value: bool, hors: HighLightOrSelect) {
        if let Some(shape) = self.shapes.get_mut(&shid) {
            shape.set_hors(value, hors);
        }
    }

    pub fn move_selection(&mut self, pos_dwn: Vec2, cursor_pos: Vec2, shift_pressed: bool) {
        self.shapes.values_mut().for_each(|shape| {
            shape.move_selection(pos_dwn, cursor_pos, shift_pressed);
        });
    }
    pub fn move_selection_from_shid(
        &mut self,
        shid: Shid,
        pos_dwn: Vec2,
        cursor_pos: Vec2,
        shift_pressed: bool,
    ) {
        if let Some(shape) = self.shapes.get_mut(&shid) {
            shape.move_selection(pos_dwn, cursor_pos, shift_pressed);
        }
    }

    pub fn delete_objects_selected(&mut self) {
        self.shapes
            .retain(|_, v| !v.get_hors(HighLightOrSelect::Select));
    }

    pub fn recalc_full_segs(&mut self) {
        // Sort shapes by OpType, prioritizing Union over Difference

        let mut shapes: Vec<_> = self.shapes.values().collect();

        shapes.sort_by(|a, b| match (a.get_boolean_op(), b.get_boolean_op()) {
            (OpType::Union, OpType::Difference) => std::cmp::Ordering::Less,
            (OpType::Difference, OpType::Union) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        });

        // Convert shapes to polygons with their boolean operations
        let polygons: Vec<(Polygon, OpType)> = shapes
            .iter()
            .map(|cs| (cs.get_polygon(), cs.get_boolean_op()))
            .collect();

        // Apply boolean operations iteratively

        let mut multi_polygon = MultiPolygon(vec![]);
        for (idx, (polygon, op_type)) in polygons.iter().enumerate() {
            if idx == 0 {
                multi_polygon = MultiPolygon(vec![polygon.clone()]);
            } else {
                multi_polygon = multi_polygon.boolean_op(polygon, *op_type);
            }
        }

        // Convert the resulting MultiPolygon to BezPath
        self.full_segs = multi_polygon
            .iter()
            .flat_map(|polygon| geo_polygon_to_bez_path(polygon))
            .collect();
    }
    pub fn get_full_segs(&mut self) -> Vec<BezPath> {
        self.full_segs.clone()
    }
}

static COUNTER_SHAPES: AtomicUsize = AtomicUsize::new(0);
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
            id: COUNTER_SHAPES.fetch_add(1, Ordering::Relaxed),
        }
    }
}
