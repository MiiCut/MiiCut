use super::{
    shape_custom::ShapeCustom,
    shape_disc::ShapeDisc,
    shape_oblong::ShapeOblong,
    shape_rectangle::ShapeRectangle,
    shape_rectangle_rounded::ShapeRectRounded,
    shapes::{BSKindvars, BasicShape, BoolOps},
};
use crate::{clipboard::Action, math::*, pools::HS, traits::*, IconsShapes, Pools};
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
    pub shape: BasicShape,
}
impl Action for AddShapeAction {
    fn undo(&self, pools: &mut Pools) {
        log!("Undoing shape creation: {:?}", self.shape.get_id());
        pools.sh.delete_shape(self.shape.get_id());
    }

    fn redo(&self, pools: &mut Pools) {
        log!("Redoing shape creation: {:?}", self.shape.get_id());
        pools.sh.add_shape(self.shape.clone());
    }
}

pub struct DeleteShapeAction {
    pub shapes: Vec<BasicShape>,
}
impl Action for DeleteShapeAction {
    fn undo(&self, pools: &mut Pools) {
        log!("Undoing shapes creation");
        self.shapes.iter().for_each(|shape| {
            pools.sh.add_shape(shape.clone());
        });
    }

    fn redo(&self, pools: &mut Pools) {
        log!("Redoing shapes creation");
        self.shapes.iter().for_each(|shape| {
            pools.sh.delete_shape(shape.get_id());
        });
    }
}

#[derive(Clone, Debug)]
pub struct ShapesPool {
    shapes: HashMap<BSid, BasicShape>,
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
    ) -> BasicShape {
        let shid = BSid::new();
        let shape_kind = match icon_shape {
            IconsShapes::Rectangle => ShapeRectangle::new(pos1, pos2),
            IconsShapes::RectangleRounded => ShapeRectRounded::new(pos1, pos2),
            IconsShapes::Disc => ShapeDisc::new(pos1, pos2),
            IconsShapes::Oblong => ShapeOblong::new(pos1, pos2),
            IconsShapes::Custom => ShapeCustom::new(pos1, pos2),
        };
        BasicShape::new(shid, shape_kind, boolean_op)
    }
    // Methods
    pub fn duplicate_shapes(&mut self, shapes: Vec<BasicShape>) -> Vec<BasicShape> {
        use SetEntityState::*;
        let mut new_shapes = vec![];
        for mut shape in shapes.into_iter() {
            shape.set_new_id(BSid::new());
            shape.get_kind_mut().set_state(SetSelect(true));
            self.add_shape(shape.clone());
            new_shapes.push(shape);
        }
        new_shapes
    }
    pub fn add_shape(&mut self, shape: BasicShape) {
        self.shapes.insert(shape.get_id(), shape);
    }
    pub fn delete_shape(&mut self, shid: BSid) -> Option<BasicShape> {
        self.shapes.remove(&shid)
    }
    pub fn get_shape(&self, target_shid: BSid) -> Option<&BasicShape> {
        self.shapes.get(&target_shid)
    }
    pub fn get_shape_mut(&mut self, target_shid: BSid) -> Option<&mut BasicShape> {
        self.shapes.get_mut(&target_shid)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&BSid, &BasicShape)> {
        self.shapes.iter()
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&BSid, &mut BasicShape)> {
        self.shapes.iter_mut()
    }
    pub fn values(&self) -> impl Iterator<Item = &BasicShape> {
        self.shapes.values()
    }
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut BasicShape> {
        self.shapes.values_mut()
    }

    pub fn save_vars(&mut self) {
        self.shapes.values_mut().for_each(|shape| {
            shape.get_kind_mut().save_vars();
        });
    }

    pub fn set_shapes_hs_from_pos(&mut self, pos: Vec2, snap: f64, hors: HS) -> Option<Vec2> {
        use GetEntityState::*;
        use SetEntityState::*;
        use HS::*;
        let mut res = None;
        if let Highlight = hors {
            self.shapes.values_mut().for_each(|shape| {
                shape
                    .get_kind_mut()
                    .set_state(HighlightFromPos(pos, snap, 5.0))
            });
        } else {
            // If Select, update the ShapeSelector and return the position of the selected shape
            let mut overlapping_shapes = HashSet::new();
            self.shapes.values_mut().for_each(|shape| {
                shape
                    .get_kind_mut()
                    .set_state(SelectFromPos(pos, snap, 5.0));
                if shape.get_kind().get_state(IsSelected).is_some() {
                    overlapping_shapes.insert(shape.get_id());
                }
            });
            // Update the ShapeSelector with the overlapping shapes
            self.shapes_selector.update_shapes(overlapping_shapes);

            // Clear the selection of all shapes
            self.shapes
                .values_mut()
                .for_each(|shape| _ = shape.get_kind_mut().set_state(SetSelect(false)));

            // Toggle to the next shape
            if let Some(next_shid) = self.shapes_selector.next_selection() {
                // Find and select the next shape
                if let Some(shape) = self.shapes.get_mut(&next_shid) {
                    shape
                        .get_kind_mut()
                        .set_state(SetEntityState::SetSelect(true));
                    res = Some(shape.get_kind().get_position());
                }
            }
        }
        res
    }
    pub fn set_shapes_hs(&mut self, value: bool, hors: HS) {
        use SetEntityState::*;
        match hors {
            HS::Highlight => {
                self.shapes.values_mut().for_each(|shape| {
                    shape.get_kind_mut().set_state(SetHighlight(value));
                });
            }
            HS::Select => {
                self.shapes.values_mut().for_each(|shape| {
                    shape.get_kind_mut().set_state(SetSelect(value));
                });
            }
        }
    }
    pub fn get_hs(&self, hors: HS) -> Vec<BSid> {
        use GetEntityState::*;
        let mut result = vec![];
        match hors {
            HS::Highlight => {
                for shape in self.shapes.values() {
                    if shape.get_kind().get_state(IsHighlighted).is_some() {
                        result.push(shape.get_id());
                    }
                }
            }
            HS::Select => {
                for shape in self.shapes.values() {
                    if shape.get_kind().get_state(IsSelected).is_some() {
                        result.push(shape.get_id());
                    }
                }
            }
        }
        result
    }
    pub fn get_hs_if_one(&mut self, hors: HS) -> Option<BSid> {
        let result = self.get_hs(hors);
        if result.len() == 1 {
            Some(result[0])
        } else {
            None
        }
    }
    pub fn get_hs_vars(&self, hors: HS) -> Vec<(BSid, BSKindvars)> {
        use GetEntityState::*;
        let mut result = vec![];
        match hors {
            HS::Highlight => {
                for shape in self.shapes.values() {
                    if shape.get_kind().get_state(IsHighlighted).is_some() {
                        result.push((shape.get_id(), shape.get_kind().get_vars()));
                    }
                }
            }
            HS::Select => {
                for shape in self.shapes.values() {
                    if shape.get_kind().get_state(IsSelected).is_some() {
                        result.push((shape.get_id(), shape.get_kind().get_vars()));
                    }
                }
            }
        }
        result
    }
    pub fn set_hs_from_shid(&mut self, shid: BSid, value: bool, hors: HS) {
        use SetEntityState::*;
        if let Some(shape) = self.shapes.get_mut(&shid) {
            match hors {
                HS::Highlight => {
                    shape.get_kind_mut().set_state(SetHighlight(value));
                }
                HS::Select => {
                    shape.get_kind_mut().set_state(SetSelect(value));
                }
            }
        }
    }

    pub fn set_shapes_mod_hs_from_pos(
        &mut self,
        pos: Vec2,
        snap: f64,
        _precision: f64,
        hors: HS,
    ) -> bool {
        use GetEntityState::*;
        use SetEntityState::*;
        match hors {
            HS::Highlight => {
                self.shapes.values_mut().for_each(|shape| {
                    shape
                        .get_kind_mut()
                        .set_state(HighlightFromPos(pos, snap, 5.0));
                });
            }
            HS::Select => {
                let mut overlapping_shapes = HashSet::new();
                self.shapes.values_mut().for_each(|shape| {
                    shape
                        .get_kind_mut()
                        .set_state(SelectFromPos(pos, snap, 5.0));
                    if shape.get_kind().get_state(IsSelected).is_some() {
                        overlapping_shapes.insert(shape.get_id());
                    }
                });
                self.shapes_selector.update_shapes(overlapping_shapes);
            }
        }
        for shape in self.shapes.values_mut() {
            shape
                .get_kind_mut()
                .set_state(SelectFromPos(pos, snap, 5.0));
            if shape.get_kind().get_state(IsSelected).is_some() {
                return true;
            }
        }
        false
    }
    pub fn set_hs_modifiers(&mut self, value: bool, hors: HS) {
        use SetEntityState::*;
        match hors {
            HS::Highlight => {
                self.shapes.values_mut().for_each(|shape| {
                    shape.get_kind_mut().set_state(HighlightAllModifiers(value));
                });
            }
            HS::Select => {
                self.shapes.values_mut().for_each(|shape| {
                    shape.get_kind_mut().set_state(SelectAllModifiers(value));
                });
            }
        }
    }
    pub fn get_first_selected_modifier_vars(&self) -> Option<(BSid, BSKindvars)> {
        use GetEntityState::*;
        for shape in self.shapes.values() {
            if shape.get_kind().get_state(IsAnyModifierSelected).is_some() {
                return Some((shape.get_id(), shape.get_kind().get_vars()));
            }
        }
        None
    }

    pub fn move_position(
        &mut self,
        shid: BSid,
        pos_init: Vec2,
        pos: Vec2,
        snap: f64,
        _shift_pressed: bool,
    ) -> Option<Vec2> {
        self.shapes
            .get_mut(&shid)
            .and_then(|shape| return shape.get_kind_mut().move_position(pos - pos_init, snap))
    }
    pub fn move_modifier(
        &mut self,
        shid: BSid,
        pos_init: Vec2,
        pos: Vec2,
        snap: f64,
        shift_pressed: bool,
    ) -> Option<Vec2> {
        if let Some(shape) = self.shapes.get_mut(&shid) {
            return shape
                .get_kind_mut()
                .move_modifier(pos_init, pos, snap, shift_pressed);
        }
        None
    }

    pub fn delete_selection(&mut self) -> Option<Vec<BasicShape>> {
        use GetEntityState::*;
        let mut shapes_deleted = vec![];

        for shape in self.shapes.values_mut() {
            if shape.get_kind_mut().get_state(IsSelected).is_some() {
                shapes_deleted.push(shape.clone());
            }
        }

        self.shapes
            .retain(|_, v| !v.get_kind_mut().get_state(IsSelected).is_some());

        if !shapes_deleted.is_empty() {
            Some(shapes_deleted)
        } else {
            None
        }
    }

    pub fn intersection_set(&self, shid: BSid) -> HashSet<BSid> {
        let mut result = HashSet::new();
        if let Some(shape) = self.shapes.get(&shid) {
            for (k, v) in self.shapes.iter() {
                if k == &shid {
                    // result.insert(*k);
                    continue;
                }
                if shape
                    .get_kind()
                    .get_polygon()
                    .intersects(&v.get_kind().get_polygon())
                {
                    result.insert(*k);
                }
            }
        }
        result
    }
    // BFS algorithm: https://en.wikipedia.org/wiki/Breadth-first_search
    pub fn connected_shapes(&self, start_shid: BSid) -> HashSet<BSid> {
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
        use SetEntityState::*;
        let mut res = false;
        if let Some(start_shid) = self.get_hs(HS::Select).get(0).copied() {
            let connected_shids = self.connected_shapes(start_shid);
            connected_shids.iter().for_each(|shid| {
                if let Some(shape) = self.shapes.get_mut(shid) {
                    shape.get_kind_mut().set_state(SetSelect(true));
                    res = true;
                }
            });
        }
        res
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
            .map(|shape| (shape.get_kind().get_polygon(), shape.get_boolean_op()))
            .collect();

        // let performance = window().unwrap().performance().unwrap();
        // let start_time = performance.now();

        // Apply boolean operations iteratively
        let mut multi_polygon = MultiPolygon(vec![]);
        for (idx, (polygon, op_type)) in polygons.iter().enumerate() {
            if idx == 0 {
                multi_polygon = MultiPolygon(vec![polygon.clone()]);
            } else {
                multi_polygon = multi_polygon.boolean_op(polygon, op_type.get_op());
            }
        }

        // let end_time = performance.now();
        // log!("Apply boolean operation: {:.2} ms", end_time - start_time);

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
pub struct BSid {
    id: usize,
}
impl Deref for BSid {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.id
    }
}
impl DerefMut for BSid {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.id
    }
}
impl Display for BSid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
impl NewId for BSid {
    fn new() -> Self {
        BSid {
            id: COUNTER_SHAPES.fetch_add(1, Ordering::Relaxed),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ShapeSelector {
    selectable_shapes: Vec<BSid>, // IDs of selectable shapes
    current_index: usize,         // Current index in the list
}
impl ShapeSelector {
    pub fn new() -> Self {
        Self {
            selectable_shapes: Vec::new(),
            current_index: 0,
        }
    }
    pub fn update_shapes(&mut self, new_shapes: HashSet<BSid>) {
        let current_set: HashSet<_> = self.selectable_shapes.iter().cloned().collect();
        // Compare the sets, ignoring order
        if current_set != new_shapes {
            // Reset if the set of shapes changes
            self.selectable_shapes = new_shapes.into_iter().collect();
            self.current_index = 0;
        }
    }
    pub fn next_selection(&mut self) -> Option<BSid> {
        if self.selectable_shapes.is_empty() {
            return None;
        }
        // Select the current shape and move to the next
        let selected = self.selectable_shapes[self.current_index];
        self.current_index = (self.current_index + 1) % self.selectable_shapes.len();
        Some(selected)
    }
}
