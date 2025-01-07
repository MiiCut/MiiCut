use crate::{
    math::*,
    positions::HS,
    shape_disc::ShapeDisc,
    shape_oblong::ShapeOblong,
    shape_rectangle::ShapeRectangle,
    shape_rectangle_rounded::ShapeRectRounded,
    shapes::{BoolOps, Shape, ShapeKindFuncs, ShapeKindvars, Shapes},
    undo_redo::Action,
    IconsShapes,
};
use geo::{BooleanOps, Intersects, MultiPolygon, Polygon};
use kurbo::{BezPath, Vec2};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Display,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
    vec,
};

pub struct AddShapeAction {
    pub shape: Shape,
}
impl Action for AddShapeAction {
    fn undo(&self, pool: &mut ShapesPool) {
        log!("Undoing shape creation: {:?}", self.shape.get_id());
        pool.delete_shape(self.shape.get_id());
    }

    fn redo(&self, pool: &mut ShapesPool) {
        log!("Redoing shape creation: {:?}", self.shape.get_id());
        pool.add_shape(self.shape.clone());
    }
}
pub struct RemoveShapeAction {
    pub shapes: Vec<Shape>,
}
impl Action for RemoveShapeAction {
    fn undo(&self, pool: &mut ShapesPool) {
        log!("Undoing shapes creation");
        self.shapes.iter().for_each(|shape| {
            pool.add_shape(shape.clone());
        });
    }

    fn redo(&self, pool: &mut ShapesPool) {
        log!("Redoing shapes creation");
        self.shapes.iter().for_each(|shape| {
            pool.delete_shape(shape.get_id());
        });
    }
}

#[derive(Clone, Debug)]
pub struct ShapesPool {
    shapes: HashMap<Shid, Shape>,
    shapes_selector: ShapeSelector,
    full_segs: Vec<BezPath>,
}
impl ShapesPool {
    // Static methods
    pub fn new() -> ShapesPool {
        ShapesPool {
            shapes: HashMap::new(),
            shapes_selector: ShapeSelector::new(),
            full_segs: Vec::new(),
        }
    }
    pub fn new_shape(
        icon_shape: IconsShapes,
        pos1: Vec2,
        pos2: Vec2,
        boolean_op: BoolOps,
    ) -> Shape {
        let shid = Shid::new();
        let shape_kind = match icon_shape {
            IconsShapes::Rectangle => ShapeRectangle::new(pos1, pos2),
            IconsShapes::RectangleRounded => ShapeRectRounded::new(pos1, pos2),
            IconsShapes::Disc => ShapeDisc::new(pos1, pos2),
            IconsShapes::Oblong => ShapeOblong::new(pos1, pos2),
        };
        Shape::new(shid, shape_kind, boolean_op)
    }
    pub fn new_shape_cloned_from(shape: &Shape) -> Shape {
        let shid = Shid::new();
        let shape_kind = shape.clone_kind();
        Shape::new(shid, shape_kind, shape.get_boolean_op())
    }

    // Methods
    pub fn add_shape(&mut self, shape: Shape) {
        self.shapes.insert(shape.get_id(), shape);
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
        self.shapes.values_mut().for_each(|shape| {
            shape.kind_mut().save_vars();
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
                if shape
                    .kind()
                    .get_polygon()
                    .intersects(&v.kind().get_polygon())
                {
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

        if let Some(start_shid) = self.get_hs(HS::Select).get(0).copied() {
            let connected_shids = self.connected_shapes(start_shid);
            connected_shids.iter().for_each(|shid| {
                if let Some(shape) = self.shapes.get_mut(shid) {
                    shape.kind_mut().set_hs(true, HS::Select);
                    res = true;
                }
            });
        }
        res
    }

    pub fn set_hs_from_pos(&mut self, pos: Vec2, hors: HS) -> bool {
        use HS::*;
        if let Highlight = hors {
            let mut res = false;
            self.shapes.values_mut().for_each(|shape| {
                res |= shape.kind_mut().set_hs_from_pos(pos, Highlight);
            });
            return res;
        } else {
            let mut overlapping_shapes = HashSet::new();
            self.shapes.values_mut().for_each(|shape| {
                if shape.kind_mut().set_hs_from_pos(pos, Select) {
                    overlapping_shapes.insert(shape.get_id());
                }
            });
            // Update the ShapeSelector with the overlapping shapes
            self.shapes_selector.update_shapes(overlapping_shapes);

            // Toggle to the next shape
            self.shapes
                .values_mut()
                .for_each(|shape| shape.kind_mut().set_hs(false, Select));

            if let Some(next_shid) = self.shapes_selector.next_selection() {
                // Find and select the next shape
                if let Some(shape) = self.shapes.get_mut(&next_shid) {
                    shape.kind_mut().set_hs(true, Select);
                    return true;
                }
            }
            false
        }
    }
    pub fn set_hs(&mut self, value: bool, hors: HS) {
        self.shapes.values_mut().for_each(|shape| {
            shape.kind_mut().set_hs(value, hors);
        });
    }

    pub fn set_hs_modifiers_from_pos(&mut self, pos: Vec2, _precision: f64, hors: HS) -> bool {
        let mut setted = false;
        self.shapes.values_mut().for_each(|shape| {
            setted |= shape.kind_mut().set_hs_modifiers_from_pos(pos, hors);
        });
        setted
    }
    pub fn set_hs_modifiers(&mut self, value: bool, hors: HS) {
        self.shapes.values_mut().for_each(|shape| {
            shape.kind_mut().set_hs_modifiers(value, hors);
        });
    }

    pub fn move_positions(
        &mut self,
        shid_sel: Vec<Shid>,
        pos_init: Vec2,
        pos: Vec2,
        _shift_pressed: bool,
    ) {
        shid_sel.into_iter().for_each(|shid| {
            if let Some(shape) = self.shapes.get_mut(&shid) {
                shape.kind_mut().move_position(pos - pos_init);
            }
        });
    }
    pub fn move_modifier(&mut self, shid: Shid, pos_init: Vec2, pos: Vec2, shift_pressed: bool) {
        if let Some(shape) = self.shapes.get_mut(&shid) {
            shape.kind_mut().move_modifier(pos_init, pos, shift_pressed);
        }
    }
    pub fn get_hs(&self, hors: HS) -> Vec<Shid> {
        let mut result = vec![];
        for shape in self.shapes.values() {
            if shape.kind().get_hs(hors) {
                result.push(shape.get_id());
            }
        }
        result
    }
    pub fn get_hs_vars(&self, hors: HS) -> Vec<(Shid, ShapeKindvars)> {
        let mut result = vec![];
        for shape in self.shapes.values() {
            if shape.kind().get_hs(hors) {
                result.push((shape.get_id(), shape.kind().get_vars()));
            }
        }
        result
    }
    pub fn get_first_selected_modifier(&self) -> Option<Shid> {
        for shape in self.shapes.values() {
            if shape.kind().get_hs_modifiers(HS::Select) {
                return Some(shape.get_id());
            }
        }
        None
    }
    pub fn set_hs_from_shid(&mut self, shid: Shid, value: bool, hors: HS) {
        if let Some(shape) = self.shapes.get_mut(&shid) {
            shape.kind_mut().set_hs(value, hors);
        }
    }

    pub fn delete_shapes_selected(&mut self) -> Vec<Shape> {
        let shapes_deleted: Vec<Shape> = self
            .shapes
            .iter()
            .filter(|(_, shape)| shape.kind().get_hs(HS::Select))
            .map(|(_, shape)| shape.clone())
            .collect();
        self.shapes.retain(|_, v| !v.kind().get_hs(HS::Select));
        shapes_deleted
    }

    pub fn recalc_full_segs(&mut self) {
        // Sort shapes by OpType, prioritizing Union over Difference
        let mut shapes: Vec<_> = self.shapes.values().collect();

        shapes.sort_by(|a, b| {
            let priority = |op: &BoolOps| match op {
                BoolOps::Union => 0,
                BoolOps::Difference => 1,
                BoolOps::UnionForced => 2,
            };

            priority(&a.get_boolean_op()).cmp(&priority(&b.get_boolean_op()))
        });

        // Convert shapes to polygons with their boolean operations
        let polygons: Vec<(Polygon, BoolOps)> = shapes
            .iter()
            .map(|shape| (shape.kind().get_polygon(), shape.get_boolean_op()))
            .collect();

        // Apply boolean operations iteratively
        let mut multi_polygon = MultiPolygon(vec![]);
        for (idx, (polygon, op_type)) in polygons.iter().enumerate() {
            if idx == 0 {
                multi_polygon = MultiPolygon(vec![polygon.clone()]);
            } else {
                multi_polygon = multi_polygon.boolean_op(polygon, op_type.get_op());
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
