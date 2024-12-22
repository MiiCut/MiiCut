// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

use crate::{
    canvas_core::{Layer, Pattern},
    prefab,
    shapes_pool::{CSPool, CShapeKind, CShid},
};
use kurbo::{BezPath, Vec2};
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GlobalCompositeOperation {
    SourceOver(&'static str),
    SourceIn(&'static str),
    SourceOut(&'static str),
    SourceATop(&'static str),
    DestinationOver(&'static str),
    DeqstinationIn(&'static str),
    DestinationOut(&'static str),
    DestinationAtop(&'static str),
    Lighter(&'static str),
    Copy(&'static str),
    Xor(&'static str),
    Multiply(&'static str),
    Screen(&'static str),
    Overlay(&'static str),
    Darken(&'static str),
    Lighten(&'static str),
    ColorDodge(&'static str),
    ColorBurn(&'static str),
    HardLight(&'static str),
    SoftLight(&'static str),
    Difference(&'static str),
    Exclusion(&'static str),
    Hue(&'static str),
    Saturation(&'static str),
    Color(&'static str),
    Luminosity(&'static str),
}
impl GlobalCompositeOperation {
    pub fn new_source_over() -> GlobalCompositeOperation {
        GlobalCompositeOperation::SourceOver("source-over")
    }
    pub fn new_source_in() -> GlobalCompositeOperation {
        GlobalCompositeOperation::SourceIn("source-in")
    }
    pub fn new_source_out() -> GlobalCompositeOperation {
        GlobalCompositeOperation::SourceOut("source-out")
    }
    pub fn new_source_atop() -> GlobalCompositeOperation {
        GlobalCompositeOperation::SourceATop("source-atop")
    }
    pub fn new_destination_over() -> GlobalCompositeOperation {
        GlobalCompositeOperation::DestinationOver("destination-over")
    }
    pub fn new_destination_in() -> GlobalCompositeOperation {
        GlobalCompositeOperation::DeqstinationIn("destination-in")
    }
    pub fn new_destination_out() -> GlobalCompositeOperation {
        GlobalCompositeOperation::DestinationOut("destination-out")
    }
    pub fn new_destination_atop() -> GlobalCompositeOperation {
        GlobalCompositeOperation::DestinationAtop("destination-atop")
    }
    pub fn new_lighter() -> GlobalCompositeOperation {
        GlobalCompositeOperation::Lighter("lighter")
    }
    pub fn new_copy() -> GlobalCompositeOperation {
        GlobalCompositeOperation::Copy("copy")
    }
    pub fn new_xor() -> GlobalCompositeOperation {
        GlobalCompositeOperation::Xor("xor")
    }
    pub fn new_multiply() -> GlobalCompositeOperation {
        GlobalCompositeOperation::Multiply("multiply")
    }
    pub fn new_screen() -> GlobalCompositeOperation {
        GlobalCompositeOperation::Screen("screen")
    }
    pub fn new_overlay() -> GlobalCompositeOperation {
        GlobalCompositeOperation::Overlay("overlay")
    }
    pub fn new_darken() -> GlobalCompositeOperation {
        GlobalCompositeOperation::Darken("darken")
    }
    pub fn new_lighten() -> GlobalCompositeOperation {
        GlobalCompositeOperation::Lighten("lighten")
    }
    pub fn new_color_dodge() -> GlobalCompositeOperation {
        GlobalCompositeOperation::ColorDodge("color-dodge")
    }
    pub fn new_color_burn() -> GlobalCompositeOperation {
        GlobalCompositeOperation::ColorBurn("color-burn")
    }
    pub fn new_hard_light() -> GlobalCompositeOperation {
        GlobalCompositeOperation::HardLight("hard-light")
    }
    pub fn new_soft_light() -> GlobalCompositeOperation {
        GlobalCompositeOperation::SoftLight("soft-light")
    }
    pub fn new_difference() -> GlobalCompositeOperation {
        GlobalCompositeOperation::Difference("difference")
    }
    pub fn new_exclusion() -> GlobalCompositeOperation {
        GlobalCompositeOperation::Exclusion("exclusion")
    }
    pub fn new_hue() -> GlobalCompositeOperation {
        GlobalCompositeOperation::Hue("hue")
    }
    pub fn new_saturation() -> GlobalCompositeOperation {
        GlobalCompositeOperation::Saturation("saturation")
    }
    pub fn new_color() -> GlobalCompositeOperation {
        GlobalCompositeOperation::Color("color")
    }
    pub fn new_luminosity() -> GlobalCompositeOperation {
        GlobalCompositeOperation::Luminosity("luminosity")
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            GlobalCompositeOperation::SourceOver(s) => s,
            GlobalCompositeOperation::SourceIn(s) => s,
            GlobalCompositeOperation::SourceOut(s) => s,
            GlobalCompositeOperation::SourceATop(s) => s,
            GlobalCompositeOperation::DestinationOver(s) => s,
            GlobalCompositeOperation::DeqstinationIn(s) => s,
            GlobalCompositeOperation::DestinationOut(s) => s,
            GlobalCompositeOperation::DestinationAtop(s) => s,
            GlobalCompositeOperation::Lighter(s) => s,
            GlobalCompositeOperation::Copy(s) => s,
            GlobalCompositeOperation::Xor(s) => s,
            GlobalCompositeOperation::Multiply(s) => s,
            GlobalCompositeOperation::Screen(s) => s,
            GlobalCompositeOperation::Overlay(s) => s,
            GlobalCompositeOperation::Darken(s) => s,
            GlobalCompositeOperation::Lighten(s) => s,
            GlobalCompositeOperation::ColorDodge(s) => s,
            GlobalCompositeOperation::ColorBurn(s) => s,
            GlobalCompositeOperation::HardLight(s) => s,
            GlobalCompositeOperation::SoftLight(s) => s,
            GlobalCompositeOperation::Difference(s) => s,
            GlobalCompositeOperation::Exclusion(s) => s,
            GlobalCompositeOperation::Hue(s) => s,
            GlobalCompositeOperation::Saturation(s) => s,
            GlobalCompositeOperation::Color(s) => s,
            GlobalCompositeOperation::Luminosity(s) => s,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HandleKind {
    Grab,
    Modify,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Handle {
    saved_pos: Vec2,
    old_pos: Vec2,
    pos: Vec2,
    kind: HandleKind,
    highlighted: bool,
    selected: bool,
}
impl Handle {
    pub fn new(pos: Vec2, kind: HandleKind, selected: bool) -> Handle {
        Handle {
            saved_pos: pos,
            old_pos: pos,
            pos,
            kind,
            highlighted: false,
            selected,
        }
    }
    pub fn is_highlighted(&self) -> bool {
        self.highlighted
    }
    pub fn set_highlighted(&mut self, highlighted: bool) {
        self.highlighted = highlighted;
    }
    pub fn get_selection(&self) -> bool {
        self.selected
    }
    pub fn set_selection(&mut self, selected: bool) {
        self.selected = selected;
    }
    pub fn get_pos(&self) -> Vec2 {
        self.pos
    }
    pub fn get_last_pos(&self) -> Vec2 {
        self.old_pos
    }
    pub fn get_saved_pos(&self) -> Vec2 {
        self.saved_pos
    }
    pub fn set_pos(&mut self, pos: Vec2) {
        self.old_pos = self.pos;
        self.pos = pos;
    }
    pub fn save_pos(&mut self) {
        self.saved_pos = self.pos;
        self.old_pos = self.pos;
    }
    pub fn get_kind(&self) -> HandleKind {
        self.kind
    }
    pub fn get_path(&self, scale: f64) -> (Pattern, BezPath) {
        let grab_path = prefab::handle_grab_path(self.pos, scale);
        let modify_path = prefab::handle_modify_path(self.pos, scale);

        match self.kind {
            HandleKind::Grab => match (self.selected, self.highlighted) {
                (false, false) => (Pattern::Normal, grab_path),
                (false, true) => (Pattern::Highlighted, grab_path),
                (true, false) => (Pattern::Selected, grab_path),
                (true, true) => (Pattern::Highlighted, grab_path),
            },
            HandleKind::Modify => {
                if self.highlighted {
                    (Pattern::Highlighted, modify_path)
                } else {
                    (Pattern::Light, modify_path)
                }
            }
        }
    }
}

pub trait CShapes {
    const TOLERANCE: f64;

    fn new(pos1: Vec2, pos2: Vec2) -> CShapeKind;
    fn save_pos(&mut self);
    fn toggle_prop(&mut self);
    fn get_shape_path(&self) -> BezPath;
    fn highlight_object(&mut self, pos: Vec2, precision: f64);
    fn select_object(&mut self, pos: Vec2, precision: f64);
    fn is_selected(&self) -> bool;
    fn is_highlighted(&self) -> bool;
    fn clear_selection(&mut self);
    fn clear_selection_all(&mut self);
    fn get_position(&self) -> Vec2;
    fn move_position(&mut self, pos_init: Vec2, pos: Vec2);
    fn get_handles(&self) -> Vec<Handle>;
    // Trait implementation must call this function after each move
    fn update_handles_pos(&mut self);
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
    op: GlobalCompositeOperation,
}
impl CShape {
    pub fn new(
        cshid: CShid,
        cshape_kind: CShapeKind,
        parent: Option<CShid>,
        layer: Layer,
        op: GlobalCompositeOperation,
    ) -> CShape {
        CShape {
            cshid,
            cshape_kind,
            parent,
            children: CSPool::new(),
            layer,
            op,
        }
    }
    pub fn get_id(&self) -> CShid {
        self.cshid
    }
    pub fn get_parent(&self) -> Option<CShid> {
        self.parent
    }
    pub fn get_layer(&self) -> Layer {
        self.layer
    }
    pub fn get_op(&self) -> GlobalCompositeOperation {
        self.op
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
    pub fn get_shape_path(&self) -> BezPath {
        use CShapeKind::*;
        match self.cshape_kind {
            CRectangle(sh) => sh.get_shape_path(),
            CRectangleRounded(sh) => sh.get_shape_path(),
            CHole(sh) => sh.get_shape_path(),
            COblong(sh) => sh.get_shape_path(),
        }
    }
    pub fn highlight_object(&mut self, pos: Vec2, precision: f64) {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.highlight_object(pos, precision),
            CRectangleRounded(sh) => sh.highlight_object(pos, precision),
            CHole(sh) => sh.highlight_object(pos, precision),
            COblong(sh) => sh.highlight_object(pos, precision),
        }
    }
    pub fn set_selection(&mut self, pos: Vec2, precision: f64) {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.select_object(pos, precision),
            CRectangleRounded(sh) => sh.select_object(pos, precision),
            CHole(sh) => sh.select_object(pos, precision),
            COblong(sh) => sh.select_object(pos, precision),
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
        match self.cshape_kind {
            CRectangle(sh) => sh.get_handles(),
            CRectangleRounded(sh) => sh.get_handles(),
            CHole(sh) => sh.get_handles(),
            COblong(sh) => sh.get_handles(),
        }
    }
}
