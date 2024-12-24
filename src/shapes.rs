// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

use crate::{
    canvas_core::{Layer, Pattern},
    handles::Handle,
    shapes_pool::{CSPool, CShapeKind, CShid},
};
use kurbo::{BezPath, Vec2};

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

pub trait CShapes {
    const TOLERANCE: f64;

    fn new(pos1: Vec2, pos2: Vec2) -> CShapeKind;
    fn save_pos(&mut self);
    fn toggle_prop(&mut self);
    fn get_shape_path(&self) -> BezPath;

    fn highlight_object(&mut self, pos: Vec2, precision: f64);
    fn set_highlight(&mut self, value: bool);
    fn is_highlighted(&self) -> bool;

    fn select_object(&mut self, pos: Vec2, precision: f64);
    fn set_selection(&mut self, value: bool);
    fn is_selected(&self) -> bool;
    fn clear_selection(&mut self);
    fn clear_selection_all(&mut self);

    fn get_position(&self) -> Vec2;
    fn move_position(&mut self, pos_init: Vec2, pos: Vec2);
    fn get_handles(&self) -> Vec<Handle>;
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
    pub fn get_op(&self) -> GlobalCompositeOperation {
        self.op
    }
    pub fn get_children(&self) -> &CSPool {
        &self.children
    }
    pub fn get_children_mut(&mut self) -> &mut CSPool {
        &mut self.children
    }
    pub fn add_child(&mut self, cshape: CShape) {
        self.children.add_shape(cshape)
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
    pub fn toggle_op(&mut self) {
        if self.op == GlobalCompositeOperation::new_source_over() {
            self.op = GlobalCompositeOperation::new_destination_out();
        } else {
            self.op = GlobalCompositeOperation::new_source_over();
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
    pub fn set_highlight(&mut self, value: bool) {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.set_highlight(value),
            CRectangleRounded(sh) => sh.set_highlight(value),
            CHole(sh) => sh.set_highlight(value),
            COblong(sh) => sh.set_highlight(value),
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

    pub fn select_object(&mut self, pos: Vec2, precision: f64) {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.select_object(pos, precision),
            CRectangleRounded(sh) => sh.select_object(pos, precision),
            CHole(sh) => sh.select_object(pos, precision),
            COblong(sh) => sh.select_object(pos, precision),
        }
    }
    pub fn set_selection(&mut self, value: bool) {
        use CShapeKind::*;
        match &mut self.cshape_kind {
            CRectangle(sh) => sh.set_selection(value),
            CRectangleRounded(sh) => sh.set_selection(value),
            CHole(sh) => sh.set_selection(value),
            COblong(sh) => sh.set_selection(value),
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
    pub fn get_layer(&self) -> Layer {
        self.layer
    }
    pub fn get_path(&self) -> BezPath {
        use CShapeKind::*;
        match self.cshape_kind {
            CRectangle(sh) => sh.get_shape_path(),
            CRectangleRounded(sh) => sh.get_shape_path(),
            CHole(sh) => sh.get_shape_path(),
            COblong(sh) => sh.get_shape_path(),
        }
    }
    pub fn get_pattern(&self) -> Pattern {
        match (self.is_selected(), self.is_highlighted()) {
            (false, false) => Pattern::Normal(true),
            (false, true) => Pattern::Highlighted(true),
            (true, false) => Pattern::Selected(true),
            (true, true) => Pattern::Selected(true),
        }
    }
}
