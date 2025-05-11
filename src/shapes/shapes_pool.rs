use super::shapes::{BoolOps, MiiShape, ShapeKind};
use crate::{
    clipboard::Action,
    math::*,
    pools::{PoolsFunctions, HS},
    traits::*,
    KeysStates, Pointer, Pools,
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
    pub shape: MiiShape,
}
impl Action for AddShapeAction {
    fn undo(&self, pools: &mut Pools) {
        log!("Undoing shape creation: {:?}", self.shape.get_id());
        pools.shapes.delete(self.shape.get_id());
        pools.shapes.create_magnet_points();
    }

    fn redo(&self, pools: &mut Pools) {
        log!("Redoing shape creation: {:?}", self.shape.get_id());
        pools.shapes.add(self.shape.clone());
        pools.shapes.create_magnet_points();
    }
}

pub struct DeleteShapeAction {
    pub shapes: Vec<MiiShape>,
}
impl Action for DeleteShapeAction {
    fn undo(&self, pools: &mut Pools) {
        log!("Undoing shapes creation");
        self.shapes.iter().for_each(|shape| {
            pools.shapes.add(shape.clone());
            pools.shapes.create_magnet_points();
        });
    }

    fn redo(&self, pools: &mut Pools) {
        log!("Redoing shapes creation");
        self.shapes.iter().for_each(|shape| {
            pools.shapes.delete(shape.get_id());
            pools.shapes.create_magnet_points();
        });
    }
}

#[derive(Clone, Debug)]
pub struct ShapesPool {
    shapes: HashMap<BSid, MiiShape>,
    shapes_selector: ShapeSelector,
    full_segs: Vec<BezPath>,
    magnet_points: Vec<Vec2>,
}
impl ShapesPool {
    const MAGNET_RADIUS: f64 = 10.;

    pub fn create_magnet_points(&mut self) {
        self.magnet_points = vec![];
        for shape in self.shapes.values() {
            match shape.get_kind() {
                ShapeKind::KindDisc(shape_disc) => {
                    self.magnet_points.push(shape_disc.get_seg_bdl().s());
                }
                ShapeKind::KindPolygon(shape_poly) => {
                    shape_poly.get_hes().iter().for_each(|he| {
                        if he.is_vertex_magnetic() {
                            self.magnet_points.push(he.get_vertex().pos);
                        }
                    });
                    if let Some(hes_prim) = shape_poly.get_hes_prim() {
                        hes_prim.iter().for_each(|he| {
                            if he.is_vertex_magnetic() {
                                self.magnet_points.push(he.get_vertex().pos);
                            }
                        });
                    }
                }
            }
        }
    }
    pub fn magnet_to_points(&self, pointer: &mut Pointer, keys_states: KeysStates) {
        for point in self.magnet_points.iter() {
            if (pointer.pos() - *point).hypot() < Self::MAGNET_RADIUS {
                if !keys_states.alt_pressed {
                    pointer.set_pos(*point);
                    pointer.set_magnetized(true);
                    return;
                }
            }
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
                    .get_geo_polygon()
                    .intersects(&v.get_kind().get_geo_polygon())
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
        use HS::*;
        let mut res = false;
        if let Some(start_shid) = self.get_state(HS::Select).get(0).copied() {
            log!("select_all_shapes_connected");
            let connected_shids = self.connected_shapes(start_shid);
            connected_shids.iter().for_each(|shid| {
                if let Some(shape) = self.shapes.get_mut(shid) {
                    shape.get_kind_mut().set_state(SetHS(Select, true));
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
            .map(|shape| (shape.get_kind().get_geo_polygon(), shape.get_boolean_op()))
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
    pub fn get_polygon_on_select(&mut self) -> Option<BSid> {
        use GetEntityState::*;
        use HS::*;
        for (shid, shape) in self.shapes.iter_mut() {
            if let ShapeKind::KindPolygon(shape_custom) = shape.get_kind_mut() {
                if shape_custom.get_state(IsAControlHS(Select)) {
                    return Some(*shid);
                }
            }
        }
        None
    }
}

impl PoolsFunctions for ShapesPool {
    type Id = BSid;
    type Pool = ShapesPool;
    type Object = MiiShape;
    type ObjectKindvars = ShapeKind;

    // Static methods
    fn new() -> ShapesPool {
        ShapesPool {
            shapes: HashMap::new(),
            shapes_selector: ShapeSelector::new(),
            full_segs: vec![],
            magnet_points: vec![],
        }
    }
    // Methods
    fn tab(&mut self) -> bool {
        for shape in self.shapes.values_mut() {
            if shape.get_kind_mut().tab() {
                return true;
            }
        }
        false
    }
    fn duplicate(&mut self, shapes: Vec<MiiShape>) -> Vec<MiiShape> {
        use SetEntityState::*;
        let mut new_shapes = vec![];
        for mut shape in shapes.into_iter() {
            shape.set_new_id(BSid::new());
            shape.get_kind_mut().set_state(SetHS(HS::Select, true));
            self.add(shape.clone());
            new_shapes.push(shape);
        }
        new_shapes
    }
    fn add(&mut self, shape: MiiShape) {
        self.shapes.insert(shape.get_id(), shape);
    }
    fn delete(&mut self, shid: BSid) -> Option<MiiShape> {
        self.shapes.remove(&shid)
    }
    fn get(&self, target_shid: BSid) -> Option<&MiiShape> {
        self.shapes.get(&target_shid)
    }
    fn get_mut(&mut self, target_shid: BSid) -> Option<&mut MiiShape> {
        self.shapes.get_mut(&target_shid)
    }
    fn iter(&self) -> impl Iterator<Item = (&BSid, &MiiShape)> {
        self.shapes.iter()
    }
    fn iter_mut(&mut self) -> impl Iterator<Item = (&BSid, &mut MiiShape)> {
        self.shapes.iter_mut()
    }
    fn values(&self) -> impl Iterator<Item = &MiiShape> {
        self.shapes.values()
    }
    fn values_mut(&mut self) -> impl Iterator<Item = &mut MiiShape> {
        self.shapes.values_mut()
    }

    fn save_vars(&mut self) {
        self.shapes.values_mut().for_each(|shape| {
            shape.get_kind_mut().save_vars();
        });
    }

    fn get_state(&mut self, hs: HS) -> Vec<BSid> {
        use GetEntityState::*;
        let mut result = vec![];
        for shape in self.shapes.values_mut() {
            if shape.get_kind_mut().get_state(IsHS(hs)) {
                result.push(shape.get_id());
            }
        }
        result
    }
    fn set_state(&mut self, set_hs: SetEntityState) {
        self.shapes.values_mut().for_each(|shape| {
            shape.get_kind_mut().set_state(set_hs);
        });
    }
    fn set_states_from_pos(
        &mut self,
        pointer: &mut Pointer,
        keys_states: KeysStates,
        ses_hs: SetEntityStateFromPos,
    ) -> bool {
        use SetEntityState::*;
        use SetEntityStateFromPos::*;
        use HS::*;
        match ses_hs {
            SetControlHSFromPos(hs) => {
                for shape in self.shapes.values_mut() {
                    if shape.get_kind_mut().set_state_from_pos(
                        pointer,
                        keys_states,
                        SetControlHSFromPos(hs),
                    ) {
                        return true;
                    }
                }
                false
            }
            SetHSFromPos(hs) => {
                match hs {
                    Highlight => {
                        for shape in self.shapes.values_mut() {
                            if shape.get_kind_mut().set_state_from_pos(
                                pointer,
                                keys_states,
                                SetHSFromPos(Highlight),
                            ) {
                                return true;
                            }
                        }
                        false
                    }
                    Select => {
                        //
                        let mut overlapping_shapes = HashSet::new();
                        self.shapes.values_mut().for_each(|shape| {
                            if shape.get_kind().contains_pointer(pointer) {
                                overlapping_shapes.insert(shape.get_id());
                            }
                        });
                        // Update the ShapeSelector with the overlapping shapes
                        self.shapes_selector.update_shapes(overlapping_shapes);

                        // Clear the selection of all shapes
                        self.shapes.values_mut().for_each(|shape| {
                            _ = shape.get_kind_mut().set_state(SetHS(Select, false))
                        });

                        // Toggle to the next shape
                        if let Some(next_shid) = self.shapes_selector.next_selection() {
                            // Find and select the next shape
                            if let Some(shape) = self.shapes.get_mut(&next_shid) {
                                shape.get_kind_mut().set_state(SetHS(Select, true));
                                return true;
                            }
                        }
                        false
                    }
                }
            }
        }
    }

    fn get_state_if_one(&mut self, hs: HS) -> Option<BSid> {
        let result = self.get_state(hs);
        if result.len() == 1 {
            Some(result[0])
        } else {
            None
        }
    }
    fn get_state_and_vars(&mut self, hs: HS) -> Vec<(BSid, ShapeKind)> {
        use GetEntityState::*;
        let mut result = vec![];
        for shape in self.shapes.values_mut() {
            if shape.get_kind_mut().get_state(IsHS(hs)) {
                result.push((shape.get_id(), shape.get_kind().get_vars()));
            }
        }
        result
    }

    fn get_first_selected_control_vars(&mut self) -> Option<(BSid, ShapeKind)> {
        use GetEntityState::*;
        use HS::*;
        for shape in self.shapes.values_mut() {
            if shape.get_kind_mut().get_state(IsAControlHS(Select)) {
                return Some((shape.get_id(), shape.get_kind().get_vars()));
            }
        }
        None
    }

    fn move_position(
        &mut self,
        shid: BSid,
        pointer: &mut Pointer,
        keys_states: KeysStates,
    ) -> bool {
        if let Some(shape) = self.shapes.get_mut(&shid) {
            return shape.get_kind_mut().move_position(pointer, keys_states);
        }
        false
    }
    fn move_control(&mut self, shid: BSid, pointer: &mut Pointer, keys_states: KeysStates) -> bool {
        if let Some(shape) = self.shapes.get_mut(&shid) {
            return shape.get_kind_mut().move_controls(pointer, keys_states);
        }
        false
    }
    fn delete_selection(&mut self) -> Option<Vec<MiiShape>> {
        use GetEntityState::*;
        use HS::*;
        let mut shapes_deleted = vec![];

        for shape in self.shapes.values_mut() {
            if shape.get_kind_mut().get_state(IsHS(Select)) {
                shapes_deleted.push(shape.clone());
            }
        }

        self.shapes
            .retain(|_, v| !v.get_kind_mut().get_state(IsHS(Select)));

        if !shapes_deleted.is_empty() {
            Some(shapes_deleted)
        } else {
            None
        }
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
