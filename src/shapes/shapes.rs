// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }
use super::shape_disc::ShapeDisc;
use super::shape_oblong::ShapeOblong;
use super::shape_rectangle::ShapeRectangle;
use super::shape_rectangle_rounded::ShapeRectRounded;
use super::shapes_pool::Shid;
use crate::canvas::CanvasText;
use crate::canvas::Pattern;
use crate::Action;
use crate::Pools;
use crate::Position;
use crate::Value;
use crate::HS;
use geo::{OpType, Polygon};
use kurbo::{BezPath, Vec2};
use std::fmt::Debug;
use std::fmt::Display;

pub struct MoveShapesAction {
    pub shids_vars: Vec<(Shid, ShapeKindvars)>,
}
impl Action for MoveShapesAction {
    fn undo(&self, pools: &mut Pools) {
        log!("Undoing last shapes move");
        for (shid, vars) in &self.shids_vars {
            if let Some(shape) = pools.sh.get_shape_mut(*shid) {
                shape.get_kind_mut().set_vars(vars);
                shape.get_kind_mut().restore_saved();
            }
        }
    }

    fn redo(&self, pools: &mut Pools) {
        log!("Redoing last shapes move");
        for (shid, vars) in &self.shids_vars {
            if let Some(shape) = pools.sh.get_shape_mut(*shid) {
                shape.get_kind_mut().set_vars(vars);
            }
        }
    }
}

pub struct ToogleBoolOpsShapesAction {
    pub shid_toogle: (Shid, BoolOps),
}
impl Action for ToogleBoolOpsShapesAction {
    fn undo(&self, pools: &mut Pools) {
        log!("Undoing last shapes toogle");
        if let Some(shape) = pools.sh.get_shape_mut(self.shid_toogle.0) {
            shape.set_boolean_op(self.shid_toogle.1);
        }
    }

    fn redo(&self, pools: &mut Pools) {
        log!("Redoing last shapes toogle");
        if let Some(shape) = pools.sh.get_shape_mut(self.shid_toogle.0) {
            let mut toogle = self.shid_toogle.1;
            toogle.toggle();
            shape.set_boolean_op(toogle);
        }
    }
}

pub enum ShapeKindvars {
    Rectangle(Position, Position),
    RectangleRounded(Position, Position, Value, Value, Value, Value),
    Disc(Position, Value),
    Oblong(Position, Position, Value),
}

pub trait ShapeKindFuncs: Debug + Clone {
    const TOLERANCE: f64;
    const GRAB: f64;

    fn save_vars(&mut self);
    fn restore_saved(&mut self);
    fn get_vars(&self) -> ShapeKindvars;
    fn set_vars(&mut self, vars: &ShapeKindvars);
    fn good_size(&self) -> bool;

    fn set_hs_from_pos(&mut self, pos: Vec2, hors: HS) -> bool;
    fn set_hs(&mut self, value: bool, hors: HS);
    fn get_hs(&self, hors: HS) -> bool;
    fn get_hhss(&self) -> (bool, bool);

    fn set_hs_modifiers_from_pos(&mut self, pos: Vec2, hors: HS) -> bool;
    fn set_hs_modifiers(&mut self, value: bool, hors: HS);
    fn get_hs_modifiers(&self, hors: HS) -> bool;

    fn toggle_prop(&mut self);

    fn move_position(&mut self, dpos: Vec2);
    fn move_modifier(&mut self, pos_init: Vec2, pos: Vec2, _shift_pressed: bool) -> bool;
    fn get_position(&self) -> Vec2;

    fn get_modifiers_paths(&self) -> Vec<(BezPath, Pattern)>;
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>);
    fn get_paths(&self) -> Vec<BezPath>;

    fn get_polygon(&self) -> Polygon<f64>;
    fn get_pattern_modifiers(&self, selected: bool, highlighted: bool) -> Pattern {
        match (selected, highlighted) {
            (false, false) => Pattern::Modifiers,
            (false, true) => Pattern::ModifiersHighlighted,
            (true, false) => Pattern::ModifiersSelected,
            (true, true) => Pattern::ModifiersSelected,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ShapeKind {
    Rectangle(ShapeRectangle),
    RectangleRounded(ShapeRectRounded),
    Disc(ShapeDisc),
    Oblong(ShapeOblong),
}
impl Display for ShapeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ShapeKind::*;
        match self {
            Rectangle(sh) => write!(f, "{sh}"),
            RectangleRounded(sh) => write!(f, "{sh}"),
            Disc(sh) => write!(f, "{sh}"),
            Oblong(sh) => write!(f, "{sh}"),
        }
    }
}

impl ShapeKindFuncs for ShapeKind {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 2.;

    fn save_vars(&mut self) {
        use ShapeKind::*;
        match self {
            Rectangle(sh) => sh.save_vars(),
            RectangleRounded(sh) => sh.save_vars(),
            Disc(sh) => sh.save_vars(),
            Oblong(sh) => sh.save_vars(),
        }
    }
    fn restore_saved(&mut self) {
        use ShapeKind::*;
        match self {
            Rectangle(sh) => sh.restore_saved(),
            RectangleRounded(sh) => sh.restore_saved(),
            Disc(sh) => sh.restore_saved(),
            Oblong(sh) => sh.restore_saved(),
        }
    }
    fn get_vars(&self) -> ShapeKindvars {
        use ShapeKind::*;
        match &self {
            Rectangle(sh) => sh.get_vars(),
            RectangleRounded(sh) => sh.get_vars(),
            Disc(sh) => sh.get_vars(),
            Oblong(sh) => sh.get_vars(),
        }
    }
    fn set_vars(&mut self, vars: &ShapeKindvars) {
        use ShapeKind::*;
        match self {
            Rectangle(sh) => sh.set_vars(vars),
            RectangleRounded(sh) => sh.set_vars(vars),
            Disc(sh) => sh.set_vars(vars),
            Oblong(sh) => sh.set_vars(vars),
        }
    }
    fn good_size(&self) -> bool {
        use ShapeKind::*;
        match &self {
            Rectangle(sh) => sh.good_size(),
            RectangleRounded(sh) => sh.good_size(),
            Disc(sh) => sh.good_size(),
            Oblong(sh) => sh.good_size(),
        }
    }

    fn set_hs_from_pos(&mut self, pos: Vec2, hors: HS) -> bool {
        use ShapeKind::*;
        match self {
            Rectangle(sh) => sh.set_hs_from_pos(pos, hors),
            RectangleRounded(sh) => sh.set_hs_from_pos(pos, hors),
            Disc(sh) => sh.set_hs_from_pos(pos, hors),
            Oblong(sh) => sh.set_hs_from_pos(pos, hors),
        }
    }
    fn set_hs(&mut self, value: bool, hors: HS) {
        use ShapeKind::*;
        match self {
            Rectangle(sh) => sh.set_hs(value, hors),
            RectangleRounded(sh) => sh.set_hs(value, hors),
            Disc(sh) => sh.set_hs(value, hors),
            Oblong(sh) => sh.set_hs(value, hors),
        }
    }
    fn get_hs(&self, hors: HS) -> bool {
        use ShapeKind::*;
        match &self {
            Rectangle(sh) => sh.get_hs(hors),
            RectangleRounded(sh) => sh.get_hs(hors),
            Disc(sh) => sh.get_hs(hors),
            Oblong(sh) => sh.get_hs(hors),
        }
    }
    fn get_hhss(&self) -> (bool, bool) {
        use ShapeKind::*;
        match &self {
            Rectangle(sh) => sh.get_hhss(),
            RectangleRounded(sh) => sh.get_hhss(),
            Disc(sh) => sh.get_hhss(),
            Oblong(sh) => sh.get_hhss(),
        }
    }

    fn set_hs_modifiers_from_pos(&mut self, pos: Vec2, hors: HS) -> bool {
        use ShapeKind::*;
        match self {
            Rectangle(sh) => sh.set_hs_modifiers_from_pos(pos, hors),
            RectangleRounded(sh) => sh.set_hs_modifiers_from_pos(pos, hors),
            Disc(sh) => sh.set_hs_modifiers_from_pos(pos, hors),
            Oblong(sh) => sh.set_hs_modifiers_from_pos(pos, hors),
        }
    }
    fn set_hs_modifiers(&mut self, value: bool, hors: HS) {
        use ShapeKind::*;
        match self {
            Rectangle(sh) => sh.set_hs_modifiers(value, hors),
            RectangleRounded(sh) => sh.set_hs_modifiers(value, hors),
            Disc(sh) => sh.set_hs_modifiers(value, hors),
            Oblong(sh) => sh.set_hs_modifiers(value, hors),
        }
    }
    fn get_hs_modifiers(&self, hors: HS) -> bool {
        use ShapeKind::*;
        match &self {
            Rectangle(sh) => sh.get_hs_modifiers(hors),
            RectangleRounded(sh) => sh.get_hs_modifiers(hors),
            Disc(sh) => sh.get_hs_modifiers(hors),
            Oblong(sh) => sh.get_hs_modifiers(hors),
        }
    }

    fn toggle_prop(&mut self) {
        use ShapeKind::*;
        match self {
            Rectangle(sh) => sh.toggle_prop(),
            RectangleRounded(sh) => sh.toggle_prop(),
            Disc(sh) => sh.toggle_prop(),
            Oblong(sh) => sh.toggle_prop(),
        }
    }

    fn move_position(&mut self, dpos: Vec2) {
        use ShapeKind::*;
        match self {
            Rectangle(sh) => sh.move_position(dpos),
            RectangleRounded(sh) => sh.move_position(dpos),
            Disc(sh) => sh.move_position(dpos),
            Oblong(sh) => sh.move_position(dpos),
        }
    }
    fn move_modifier(&mut self, pos_init: Vec2, pos: Vec2, _shift_pressed: bool) -> bool {
        use ShapeKind::*;
        match self {
            Rectangle(sh) => sh.move_modifier(pos_init, pos, _shift_pressed),
            RectangleRounded(sh) => sh.move_modifier(pos_init, pos, _shift_pressed),
            Disc(sh) => sh.move_modifier(pos_init, pos, _shift_pressed),
            Oblong(sh) => sh.move_modifier(pos_init, pos, _shift_pressed),
        }
    }
    fn get_position(&self) -> Vec2 {
        use ShapeKind::*;
        match self {
            Rectangle(sh) => sh.get_position(),
            RectangleRounded(sh) => sh.get_position(),
            Disc(sh) => sh.get_position(),
            Oblong(sh) => sh.get_position(),
        }
    }

    fn get_paths(&self) -> Vec<BezPath> {
        use ShapeKind::*;
        match self {
            Rectangle(sh) => sh.get_paths(),
            RectangleRounded(sh) => sh.get_paths(),
            Disc(sh) => sh.get_paths(),
            Oblong(sh) => sh.get_paths(),
        }
    }
    fn get_modifiers_paths(&self) -> Vec<(BezPath, Pattern)> {
        use ShapeKind::*;
        match self {
            Rectangle(sh) => sh.get_modifiers_paths(),
            RectangleRounded(sh) => sh.get_modifiers_paths(),
            Disc(sh) => sh.get_modifiers_paths(),
            Oblong(sh) => sh.get_modifiers_paths(),
        }
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        use ShapeKind::*;
        match self {
            Rectangle(sh) => sh.get_dimensions_paths(),
            RectangleRounded(sh) => sh.get_dimensions_paths(),
            Disc(sh) => sh.get_dimensions_paths(),
            Oblong(sh) => sh.get_dimensions_paths(),
        }
    }
    fn get_polygon(&self) -> Polygon<f64> {
        use ShapeKind::*;
        match self {
            Rectangle(sh) => sh.get_polygon(),
            RectangleRounded(sh) => sh.get_polygon(),
            Disc(sh) => sh.get_polygon(),
            Oblong(sh) => sh.get_polygon(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum BoolOps {
    Union,
    UnionForced,
    Difference,
}
impl Display for BoolOps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoolOps::Union => write!(f, "Add"),
            BoolOps::UnionForced => write!(f, "Add to top"),
            BoolOps::Difference => write!(f, "Substract"),
        }
    }
}
impl BoolOps {
    pub fn toggle(&mut self) {
        match self {
            BoolOps::Union => *self = BoolOps::UnionForced,
            BoolOps::UnionForced => *self = BoolOps::Difference,
            BoolOps::Difference => *self = BoolOps::Union,
        }
    }
    pub fn union(&mut self) {
        *self = BoolOps::Union;
    }
    pub fn force_union(&mut self) {
        *self = BoolOps::UnionForced;
    }
    pub fn difference(&mut self) {
        *self = BoolOps::Difference;
    }
    pub fn get_op(&self) -> OpType {
        match self {
            BoolOps::Union => OpType::Union,
            BoolOps::Difference => OpType::Difference,
            BoolOps::UnionForced => OpType::Union,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Shape {
    enabled: bool,
    shid: Shid,
    shape_kind: ShapeKind,
    boolean_op: BoolOps,
}
impl Shape {
    pub fn new(cshid: Shid, shape_kind: ShapeKind, boolean_op: BoolOps) -> Shape {
        Shape {
            enabled: true,
            shid: cshid,
            shape_kind,
            boolean_op,
        }
    }
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    pub fn get_kind(&self) -> &ShapeKind {
        &self.shape_kind
    }
    pub fn get_kind_mut(&mut self) -> &mut ShapeKind {
        &mut self.shape_kind
    }
    pub fn get_id(&self) -> Shid {
        self.shid
    }
    pub fn set_new_id(&mut self, new_id: Shid) {
        self.shid = new_id;
    }
    pub fn get_boolean_op(&self) -> BoolOps {
        self.boolean_op
    }

    pub fn toggle_boolean_op(&mut self) {
        self.boolean_op.toggle();
    }

    pub fn set_boolean_op(&mut self, bool_ops: BoolOps) {
        self.boolean_op = bool_ops;
    }

    pub fn get_paths_and_patterns(&self) -> Vec<(BezPath, Pattern)> {
        let hs = self.shape_kind.get_hhss();
        let pattern = match (hs.0, hs.1, self.boolean_op) {
            (false, false, BoolOps::UnionForced) => Pattern::BasicNormalDark,
            (false, true, BoolOps::UnionForced) => Pattern::BasicHighlightedDark,
            (true, false, BoolOps::UnionForced) => Pattern::BasicSelectedDark,
            (true, true, BoolOps::UnionForced) => Pattern::BasicSelectedDark,
            (false, false, _) => Pattern::BasicNormal,
            (false, true, _) => Pattern::BasicHighlighted,
            (true, false, _) => Pattern::BasicSelected,
            (true, true, _) => Pattern::BasicSelected,
        };

        let mut paths = self.shape_kind.get_paths();
        let result = paths
            .iter_mut()
            .map(|path| (path.clone(), pattern))
            .collect();
        result
    }
}
