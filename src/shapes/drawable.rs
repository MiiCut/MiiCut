// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use crate::{
    math::EPSILON,
    nodes::Set,
    types::{SegBundle, Value},
};
use geo::{LineString, Polygon};
use kurbo::{Arc, BezPath, Circle, PathEl, Shape, Vec2};
use std::{
    collections::HashSet,
    f64::consts::PI,
    fmt::{Debug, Display},
    sync::atomic::{AtomicUsize, Ordering},
    vec,
};

pub trait Drawable: Clone + 'static {
    const TOLERANCE: f64;
    const GRAB_RADIUS: f64 = 5.0;

    fn op_next(&mut self);
    fn op_union(&mut self);
    fn op_force_union(&mut self);
    fn op_difference(&mut self);

    fn get_vertices(&self) -> &Vec<(ValueUId, Value<Vec2>)>;
    fn get_vertices_mut(&mut self) -> &mut Vec<(ValueUId, Value<Vec2>)>;
    fn get_vertices_highlighted(&self) -> &HashSet<ValueUId>;
    fn get_vertices_selected(&self) -> &HashSet<ValueUId>;
    fn clear_vertices_selected(&mut self);
    fn select_vertex(&mut self, position: Value<Vec2>, shift_pressed: bool) -> bool;
    fn highlight_vertex(&mut self, position: Value<Vec2>) -> bool;
    fn move_vertex(&mut self, value_uid: ValueUId, position: Value<Vec2>) -> Vec<ValueUId>;
    fn save_vertices_positions(&mut self);
    fn contains(&self, pos: Vec2) -> bool {
        self.get_bezpath().contains(pos.to_point())
    }
    fn get_bezpath(&self) -> &BezPath;
}

impl<E: Drawable> Set<E> {
    pub fn select_element_vertex(&mut self, position: Value<Vec2>, shift_pressed: bool) -> bool {
        // Select element vertices next to the position
        if !shift_pressed {
            for (_, node) in self.nodes.iter_mut() {
                node.element.clear_vertices_selected();
            }
            for (_, node) in self.nodes.iter_mut() {
                if node.element.select_vertex(position, shift_pressed) {
                    self.nodes_selected.clear();
                    return true;
                }
            }
            return false;
        } else {
            let mut select = false;
            for (_, node) in self.nodes.iter_mut() {
                select |= node.element.select_vertex(position, shift_pressed)
            }
            if select {
                self.nodes_selected.clear();
            }
            return select;
        }
    }
    pub fn select_nodes(&mut self, position: Value<Vec2>, shift_pressed: bool) {
        // Select the nodes whose element contains the position
        if !shift_pressed {
            let mut nodes_selected = HashSet::new();
            for (id, node) in &self.nodes {
                if node.element.contains(position.curr) {
                    nodes_selected.insert(*id);
                }
            }
            if nodes_selected.len() > 0 {
                self.node_selector
                    .refresh_selectable_nodes(nodes_selected.clone());
                if let Some(id) = self.node_selector.next_selection() {
                    self.nodes_selected.clear();
                    self.nodes_selected.insert(id);
                }
            } else {
                self.nodes_selected.clear();
            }
        } else {
            // Shift pressed, add to selection
            for (id, node) in &self.nodes {
                if node.element.contains(position.curr) {
                    self.nodes_selected.insert(*id);
                }
            }
        }
    }
    pub fn highlight_nodes(&mut self, position: Value<Vec2>) {
        // Select the nodes whose element contains the position
        self.nodes_highlighted.clear();
        for (id, node) in &self.nodes {
            if node.element.contains(position.curr) {
                self.nodes_highlighted.insert(*id);
            }
        }
    }
    pub fn save_nodes_positions(&mut self) {
        for (_, node) in self.nodes.iter_mut() {
            node.element.save_vertices_positions();
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    Union,
    UnionForced,
    Difference,
}
impl Operation {
    pub fn next(&mut self) {
        match self {
            Operation::Union => *self = Operation::UnionForced,
            Operation::UnionForced => *self = Operation::Difference,
            Operation::Difference => *self = Operation::Union,
        }
    }
    pub fn union(&mut self) {
        *self = Operation::Union;
    }
    pub fn force_union(&mut self) {
        *self = Operation::UnionForced;
    }
    pub fn difference(&mut self) {
        *self = Operation::Difference;
    }
}
impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Union => write!(f, "Add"),
            Operation::UnionForced => write!(f, "Add to top"),
            Operation::Difference => write!(f, "Substract"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ClosedShapeType {
    Disc,
    Oblong,
    Rectangle,
    PolyRectangle,
    Polygon,
}
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ClosedShapes {
    shape_type: ClosedShapeType,
    operation: Operation,
    vertices: Vec<(ValueUId, Value<Vec2>)>,
    vertices_highlighted: HashSet<ValueUId>,
    vertices_selected: HashSet<ValueUId>,

    bezpath: BezPath,
    polygon: Polygon<f64>,
}
impl ClosedShapes {
    pub fn new(shape_type: ClosedShapeType, mut vertices: Vec<Vec2>) -> Option<Self> {
        // Sanity check
        match shape_type {
            ClosedShapeType::Disc => {
                if vertices.len() != 2 {
                    return None;
                }
            }
            ClosedShapeType::Oblong => {
                if vertices.len() != 3 {
                    return None;
                }
            }
            ClosedShapeType::Rectangle => {
                if vertices.len() != 2 {
                    return None;
                } else {
                    let bl = vertices[0];
                    let tr = vertices[1];
                    let tl = Vec2::new(bl.x, tr.y);
                    let br = Vec2::new(tr.x, bl.y);
                    vertices = vec![bl, tl, tr, br];
                }
            }
            ClosedShapeType::PolyRectangle => {
                if vertices.len() < 4 {
                    return None;
                }
            }
            ClosedShapeType::Polygon => {
                if vertices.len() < 3 {
                    return None;
                }
            }
        }
        let mut shape = ClosedShapes {
            shape_type,
            operation: Operation::Union,
            vertices: vertices
                .iter()
                .map(|v| (ValueUId::new(), Value::new(*v)))
                .collect(),
            vertices_highlighted: HashSet::new(),
            vertices_selected: HashSet::new(),
            bezpath: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        };
        shape.set_bezpath();
        Some(shape)
    }
    fn set_bezpath(&mut self) {
        match self.shape_type {
            ClosedShapeType::Disc => {
                let center = self.vertices[0].1.curr;
                let radius = (self.vertices[1].1.curr - center).hypot();
                self.bezpath =
                    kurbo::Circle::new(center.to_point(), radius).to_path(Self::TOLERANCE);
            }
            ClosedShapeType::Oblong => {
                let e1 = self.vertices[0].1.curr;
                let side = self.vertices[1].1.curr;
                let e2 = self.vertices[2].1.curr;
                let m = (e1 + e2) * 0.5;
                let radius = (side - m).hypot();
                let angle = (e2 - e1).atan2();
                let mut dir = e2 - e1;

                let mut path = BezPath::new();

                if dir.hypot() >= EPSILON {
                    dir = dir.normalize();
                    // Perpendicular unit vector
                    let perp = Vec2::new(-dir.y, dir.x);
                    // Two points at e1 ± perp * radius
                    let pt2 = e1 - perp * radius;
                    // Two points at e2 ± perp * radius
                    let pt3 = e2 + perp * radius;

                    path.extend(
                        Arc::new(
                            e1.to_point(),
                            Vec2::new(radius, radius),
                            3. * PI / 2.,
                            -PI,
                            angle,
                        )
                        .path_elements(Self::TOLERANCE),
                    );
                    path.push(PathEl::LineTo(pt3.to_point()));
                    let mut arc2 = Arc::new(
                        e2.to_point(),
                        Vec2::new(radius, radius),
                        PI / 2.,
                        -PI,
                        angle,
                    )
                    .path_elements(Self::TOLERANCE);
                    arc2.next(); // Remove the MoveTo
                    path.extend(arc2);
                    path.push(PathEl::LineTo(pt2.to_point()));
                    path.push(PathEl::ClosePath);
                } else {
                    path.extend(Circle::new(e2.to_point(), radius).path_elements(Self::TOLERANCE));
                }
                self.bezpath = path;
            }
            ClosedShapeType::Rectangle
            | ClosedShapeType::PolyRectangle
            | ClosedShapeType::Polygon => {
                let mut path = BezPath::new();
                for (i, (_, value)) in self.vertices.iter().enumerate() {
                    if i == 0 {
                        path.move_to(value.curr.to_point());
                    } else {
                        path.line_to(value.curr.to_point());
                    }
                }
                path.close_path();
                self.bezpath = path;
            }
        }
    }
}
impl Drawable for ClosedShapes {
    const TOLERANCE: f64 = 0.01;

    fn op_next(&mut self) {
        self.operation.next();
    }
    fn op_union(&mut self) {
        self.operation.union();
    }
    fn op_force_union(&mut self) {
        self.operation.force_union();
    }
    fn op_difference(&mut self) {
        self.operation.difference();
    }

    fn get_vertices(&self) -> &Vec<(ValueUId, Value<Vec2>)> {
        &self.vertices
    }
    fn get_vertices_mut(&mut self) -> &mut Vec<(ValueUId, Value<Vec2>)> {
        &mut self.vertices
    }
    fn get_vertices_highlighted(&self) -> &HashSet<ValueUId> {
        &self.vertices_highlighted
    }
    fn get_vertices_selected(&self) -> &HashSet<ValueUId> {
        &self.vertices_selected
    }
    fn clear_vertices_selected(&mut self) {
        self.vertices_selected.clear();
    }
    fn select_vertex(&mut self, position: Value<Vec2>, shift_pressed: bool) -> bool {
        if !shift_pressed {
            self.vertices_selected.clear();
            for (uid, value) in &self.vertices {
                if (value.curr - position.curr).hypot() < Self::GRAB_RADIUS {
                    self.vertices_selected.insert(*uid);
                    return true;
                }
            }
            return false;
        } else {
            let mut select = false;
            for (uid, value) in &self.vertices {
                if (value.curr - position.curr).hypot() < Self::GRAB_RADIUS {
                    self.vertices_selected.insert(*uid);
                    log!("selecting");
                    select = true;
                }
            }
            log!(
                "select || self.vertices_selected.len() > 0 {}",
                select || self.vertices_selected.len() > 0
            );
            return select || self.vertices_selected.len() > 0;
        }
    }
    fn highlight_vertex(&mut self, _position: Value<Vec2>) -> bool {
        false
    }
    fn move_vertex(&mut self, value_uid: ValueUId, position: Value<Vec2>) -> Vec<ValueUId> {
        let delta = position.curr - position.saved;
        match self.shape_type {
            ClosedShapeType::Disc => {
                if self.vertices.len() != 2 {
                    return vec![];
                }
                // The first vertex is the center
                // The second vertex is the radius
                if self.vertices[0].0 == value_uid {
                    self.vertices[0].1.add(delta);
                    self.vertices[1].1.add(delta);
                    self.set_bezpath();
                    return vec![self.vertices[0].0, self.vertices[1].0];
                } else if self.vertices[1].0 == value_uid {
                    self.vertices[1].1.add(delta);
                    self.set_bezpath();
                    return vec![self.vertices[1].0];
                } else {
                    return vec![];
                }
            }
            ClosedShapeType::Oblong => {
                if self.vertices.len() != 3 {
                    return vec![];
                }
                if let Some(seg) = SegBundle::new(self.vertices[0].1.curr, self.vertices[2].1.curr)
                {
                    let side = self.vertices[1].1.curr;
                    let signed_d = (side - seg.m).dot(seg.n);

                    let idx = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                        Some(i) => i,
                        None => return vec![],
                    };
                    // The side is moved, this doesn't change the pos of e1, e2
                    if idx == 1 {
                        let proj_along = seg.u * delta.dot(seg.u);
                        self.vertices[1].1.add(delta - proj_along);
                        self.set_bezpath();
                        return vec![self.vertices[1].0];
                    } else {
                        // e1 or e2 is moved, this affect the position of the side
                        // Apply the move to whichever endpoint
                        if idx == 0 {
                            self.vertices[0].1.add(delta);
                        } else {
                            self.vertices[2].1.add(delta);
                        }
                        if let Some(seg) =
                            SegBundle::new(self.vertices[0].1.curr, self.vertices[2].1.curr)
                        {
                            self.vertices[1].1.curr = seg.m + seg.n * signed_d;
                            self.set_bezpath();
                            if idx == 0 {
                                return vec![self.vertices[0].0, self.vertices[1].0];
                            } else {
                                return vec![self.vertices[2].0, self.vertices[1].0];
                            }
                        } else {
                            if idx == 0 {
                                self.vertices[0].1.add(-delta);
                            } else {
                                self.vertices[2].1.add(-delta);
                            }
                            return vec![];
                        }
                    }
                } else {
                    return vec![];
                }
            }
            ClosedShapeType::Rectangle | ClosedShapeType::PolyRectangle => {
                // 0) Check length
                let len = self.vertices.len();
                if len < 4 {
                    return vec![];
                }
                // 1) find dragged corner index
                let idx = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                    Some(i) => i,
                    None => return vec![],
                };
                // 2) compute neighbor indices
                let next = (idx + 1) % len;
                let prev = (idx + len - 1) % len;

                // 3) grab positions
                let pos_m = self.vertices[idx].1.curr;
                let pos_next = self.vertices[next].1.curr;
                let pos_prev = self.vertices[prev].1.curr;
                // 4) local axes and original lengths
                let dir_next = (pos_next - pos_m).normalize();
                let dir_prev = (pos_prev - pos_m).normalize();
                let len_next = (pos_next - pos_m).hypot();
                let len_prev = (pos_prev - pos_m).hypot();
                // 5) decompose delta
                let d_next = position.curr.dot(dir_next);
                let d_prev = position.curr.dot(dir_prev);
                // 6) move main corner
                self.vertices[idx].1.curr = position.curr;
                let mut moved = vec![self.vertices[idx].0];
                // 7) for each axis, only slide if above threshold
                if d_next.abs() > EPSILON {
                    let new_1 = (pos_m + position.curr) + dir_next * (len_next + d_next);
                    let d1 = new_1 - pos_next;
                    self.vertices[next].1.add(d1);
                    moved.push(self.vertices[next].0);
                }
                if d_prev.abs() > EPSILON {
                    let new_2 = (pos_m + position.curr) + dir_prev * (len_prev + d_prev);
                    let d2 = new_2 - pos_prev;
                    self.vertices[prev].1.add(d2);
                    moved.push(self.vertices[prev].0);
                }
                self.set_bezpath();
                moved
            }
            ClosedShapeType::Polygon => {
                let idx = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                    Some(i) => i,
                    None => return vec![],
                };
                self.vertices[idx].1.curr = position.curr;
                self.set_bezpath();
                return vec![self.vertices[idx].0];
            }
        }
    }
    fn save_vertices_positions(&mut self) {
        for (_, value) in self.get_vertices_mut() {
            value.save();
        }
    }
    fn get_bezpath(&self) -> &BezPath {
        &self.bezpath
    }
}

static COUNTER_VALUE: AtomicUsize = AtomicUsize::new(0);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd)]
pub struct ValueUId {
    id: usize,
}
impl Display for ValueUId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}
impl ValueUId {
    pub fn new() -> Self {
        let id = COUNTER_VALUE.fetch_add(1, Ordering::SeqCst);
        ValueUId { id }
    }
}
