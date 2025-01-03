// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

use std::fmt::Display;

use crate::{
    canvas::{CanvasText, Pattern},
    shape_disc::{HighLightOrSelect, ShapeDisc},
    shape_oblong::ShapeOblong,
    shape_rectangle::ShapeRectangle,
    shape_rectangle_rounded::ShapeRectRounded,
    shapes_pool::Shid,
};
use geo::{OpType, Polygon};
use kurbo::{BezPath, Vec2};

#[derive(Clone, Debug, PartialEq)]
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
pub trait Shapes {
    const TOLERANCE: f64;
    const GRAB: f64;

    fn new(pos1: Vec2, pos2: Vec2) -> ShapeKind;
    fn good_size(&self) -> bool;
    fn save_pos(&mut self);
    fn toggle_prop(&mut self);

    fn hors_from_pos(&mut self, pos: Vec2, hors: HighLightOrSelect) -> bool;
    fn hors_center_from_pos(&mut self, pos: Vec2, hors: HighLightOrSelect) -> bool;
    fn hors_modifiers_from_pos(&mut self, pos: Vec2, hors: HighLightOrSelect) -> bool;
    fn set_hors(&mut self, value: bool, hors: HighLightOrSelect);
    fn set_hors_modifiers(&mut self, value: bool, hors: HighLightOrSelect);
    fn set_hors_center(&mut self, value: bool, hors: HighLightOrSelect);
    fn is_hors(&self, hors: HighLightOrSelect) -> bool;
    fn is_center_hors(&self, hors: HighLightOrSelect) -> bool;

    fn get_position(&self) -> Vec2;
    fn move_position(&mut self, pos_init: Vec2, pos: Vec2, shift_pressed: bool);

    fn get_paths_patterns(&self) -> Vec<(BezPath, Pattern)>;
    fn get_polygon(&self) -> Polygon<f64>;
    fn get_magnets_paths(&self) -> Vec<(BezPath, Pattern)>;

    fn get_pattern(&self, selected: bool, highlighted: bool) -> Pattern {
        match (selected, highlighted) {
            (false, false) => Pattern::BasicNormal,
            (false, true) => Pattern::BasicHighlighted,
            (true, false) => Pattern::BasicSelected,
            (true, true) => Pattern::BasicSelected,
        }
    }
    fn get_pattern_modifiers(&self, selected: bool, highlighted: bool) -> Pattern {
        match (selected, highlighted) {
            (false, false) => Pattern::Modifiers,
            (false, true) => Pattern::ModifiersHighlighted,
            (true, false) => Pattern::ModifiersSelected,
            (true, true) => Pattern::ModifiersSelected,
        }
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>);
}

#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    shid: Shid,
    cshape_kind: ShapeKind,
    boolean_op: OpType,
}
impl Shape {
    pub fn new(cshid: Shid, cshape_kind: ShapeKind, boolean_op: OpType) -> Shape {
        Shape {
            shid: cshid,
            cshape_kind,
            boolean_op,
        }
    }
    pub fn get_id(&self) -> Shid {
        self.shid
    }
    pub fn good_size(&self) -> bool {
        use ShapeKind::*;
        match &self.cshape_kind {
            Rectangle(sh) => sh.good_size(),
            RectangleRounded(sh) => sh.good_size(),
            Disc(sh) => sh.good_size(),
            Oblong(sh) => sh.good_size(),
        }
    }
    pub fn save_pos(&mut self) {
        use ShapeKind::*;
        match &mut self.cshape_kind {
            Rectangle(sh) => sh.save_pos(),
            RectangleRounded(sh) => sh.save_pos(),
            Disc(sh) => sh.save_pos(),
            Oblong(sh) => sh.save_pos(),
        }
    }
    pub fn toggle_boolean_op(&mut self) {
        if self.boolean_op == OpType::Union {
            self.boolean_op = OpType::Difference
        } else {
            self.boolean_op = OpType::Union
        }
    }
    pub fn get_boolean_op(&self) -> OpType {
        self.boolean_op
    }

    pub fn hors_from_pos(&mut self, pos: Vec2, hors: HighLightOrSelect) -> bool {
        use ShapeKind::*;
        match &mut self.cshape_kind {
            Rectangle(sh) => sh.hors_from_pos(pos, hors),
            RectangleRounded(sh) => sh.hors_from_pos(pos, hors),
            Disc(sh) => sh.hors_from_pos(pos, hors),
            Oblong(sh) => sh.hors_from_pos(pos, hors),
        }
    }
    pub fn hors_center_from_pos(&mut self, pos: Vec2, hors: HighLightOrSelect) -> bool {
        use ShapeKind::*;
        match &mut self.cshape_kind {
            Rectangle(sh) => sh.hors_center_from_pos(pos, hors),
            RectangleRounded(sh) => sh.hors_center_from_pos(pos, hors),
            Disc(sh) => sh.hors_center_from_pos(pos, hors),
            Oblong(sh) => sh.hors_center_from_pos(pos, hors),
        }
    }
    pub fn hors_modifiers_from_pos(&mut self, pos: Vec2, hors: HighLightOrSelect) -> bool {
        use ShapeKind::*;
        match &mut self.cshape_kind {
            Rectangle(sh) => sh.hors_modifiers_from_pos(pos, hors),
            RectangleRounded(sh) => sh.hors_modifiers_from_pos(pos, hors),
            Disc(sh) => sh.hors_modifiers_from_pos(pos, hors),
            Oblong(sh) => sh.hors_modifiers_from_pos(pos, hors),
        }
    }
    pub fn set_hors(&mut self, value: bool, hors: HighLightOrSelect) {
        use ShapeKind::*;
        match &mut self.cshape_kind {
            Rectangle(sh) => sh.set_hors(value, hors),
            RectangleRounded(sh) => sh.set_hors(value, hors),
            Disc(sh) => sh.set_hors(value, hors),
            Oblong(sh) => sh.set_hors(value, hors),
        }
    }
    pub fn set_hors_center(&mut self, value: bool, hors: HighLightOrSelect) {
        use ShapeKind::*;
        match &mut self.cshape_kind {
            Rectangle(sh) => sh.set_hors_center(value, hors),
            RectangleRounded(sh) => sh.set_hors_center(value, hors),
            Disc(sh) => sh.set_hors_center(value, hors),
            Oblong(sh) => sh.set_hors_center(value, hors),
        }
    }
    pub fn set_hors_modifiers(&mut self, value: bool, hors: HighLightOrSelect) {
        use ShapeKind::*;
        match &mut self.cshape_kind {
            Rectangle(sh) => sh.set_hors_modifiers(value, hors),
            RectangleRounded(sh) => sh.set_hors_modifiers(value, hors),
            Disc(sh) => sh.set_hors_modifiers(value, hors),
            Oblong(sh) => sh.set_hors_modifiers(value, hors),
        }
    }
    pub fn get_hors(&self, hors: HighLightOrSelect) -> bool {
        use ShapeKind::*;
        match &self.cshape_kind {
            Rectangle(sh) => sh.is_hors(hors),
            RectangleRounded(sh) => sh.is_hors(hors),
            Disc(sh) => sh.is_hors(hors),
            Oblong(sh) => sh.is_hors(hors),
        }
    }
    pub fn get_center_hors(&self, hors: HighLightOrSelect) -> bool {
        use ShapeKind::*;
        match &self.cshape_kind {
            Rectangle(sh) => sh.is_center_hors(hors),
            RectangleRounded(sh) => sh.is_center_hors(hors),
            Disc(sh) => sh.is_center_hors(hors),
            Oblong(sh) => sh.is_center_hors(hors),
        }
    }
    pub fn move_selection(&mut self, pos_init: Vec2, pos: Vec2, shift_pressed: bool) {
        use ShapeKind::*;
        match &mut self.cshape_kind {
            Rectangle(sh) => sh.move_position(pos_init, pos, shift_pressed),
            RectangleRounded(sh) => sh.move_position(pos_init, pos, shift_pressed),
            Disc(sh) => sh.move_position(pos_init, pos, shift_pressed),
            Oblong(sh) => sh.move_position(pos_init, pos, shift_pressed),
        }
    }

    pub fn get_pattern_operation(&self) -> Pattern {
        match (
            self.get_hors(HighLightOrSelect::Select),
            self.get_hors(HighLightOrSelect::Highlight),
        ) {
            (false, false) => Pattern::ComposedNormal(true),
            (false, true) => Pattern::ComposedHighlighted(true),
            (true, false) => Pattern::ComposedSelected(true),
            (true, true) => Pattern::ComposedSelected(true),
        }
    }
    pub fn get_paths(&self) -> Vec<(BezPath, Pattern)> {
        use ShapeKind::*;
        match &self.cshape_kind {
            Rectangle(sh) => sh.get_paths_patterns(),
            RectangleRounded(sh) => sh.get_paths_patterns(),
            Disc(sh) => sh.get_paths_patterns(),
            Oblong(sh) => sh.get_paths_patterns(),
        }
    }

    pub fn get_polygon(&self) -> Polygon<f64> {
        use ShapeKind::*;
        match &self.cshape_kind {
            Rectangle(sh) => sh.get_polygon(),
            RectangleRounded(sh) => sh.get_polygon(),
            Disc(sh) => sh.get_polygon(),
            Oblong(sh) => sh.get_polygon(),
        }
    }

    pub fn get_magnets_paths(&self) -> Vec<(BezPath, Pattern)> {
        use ShapeKind::*;
        match &self.cshape_kind {
            Rectangle(sh) => sh.get_magnets_paths(),
            RectangleRounded(sh) => sh.get_magnets_paths(),
            Disc(sh) => sh.get_magnets_paths(),
            Oblong(sh) => sh.get_magnets_paths(),
        }
    }
    pub fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        use ShapeKind::*;
        match &self.cshape_kind {
            Rectangle(sh) => sh.get_dimensions_paths(),
            RectangleRounded(sh) => sh.get_dimensions_paths(),
            Disc(sh) => sh.get_dimensions_paths(),
            Oblong(sh) => sh.get_dimensions_paths(),
        }
    }
}
