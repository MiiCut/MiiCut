// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

use crate::{
    canvas::{CanvasText, Pattern},
    dimensions::{DimKind, Dimension},
    math::*,
    positions::{Modifier, Position, HS},
    prefab::{center_path, modifiers_path},
    shapes::{ShapeKind, ShapeKindFuncs, ShapeKindvars, Shapes},
};
use geo::{LineString, Polygon};
use kurbo::{BezPath, PathEl, Point, Rect, RectPathIter, Shape, Vec2};
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq)]
pub struct ShapeRectangle {
    tl: Position,
    br: Position,

    tr: Modifier,
    bl: Modifier,
    top: Modifier,
    right: Modifier,
    bottom: Modifier,
    left: Modifier,

    highlighted: bool,
    selected: bool,

    segs: BezPath,
    polygon: Polygon<f64>,
}
impl ShapeRectangle {
    const MIN_SIZE: f64 = 10.;

    fn update_polygon(&mut self) {
        log!("calc rect polygon");
        self.segs = calc_segs(self.get_paths_and_patterns());
        self.polygon = calc_polygon(&self.segs);
    }

    fn get_rectangle(&self) -> Rect {
        let tl_pos = self.tl.get_pos();
        let br_pos = self.br.get_pos();
        Rect::new(tl_pos.x, tl_pos.y, br_pos.x, br_pos.y)
    }

    fn get_tr_modifier(&self) -> Vec2 {
        Vec2::new(self.br.get_pos().x, self.tl.get_pos().y)
    }
    fn get_bl_modifier(&self) -> Vec2 {
        Vec2::new(self.tl.get_pos().x, self.br.get_pos().y)
    }

    fn get_top_modifier(&self) -> Vec2 {
        (self.tl.get_pos() + self.get_tr_modifier()) / 2.
    }
    fn get_right_modifier(&self) -> Vec2 {
        (self.get_tr_modifier() + self.br.get_pos()) / 2.
    }
    fn get_bottom_modifier(&self) -> Vec2 {
        (self.br.get_pos() + self.get_bl_modifier()) / 2.
    }
    fn get_left_modifier(&self) -> Vec2 {
        (self.get_bl_modifier() + self.tl.get_pos()) / 2.
    }

    fn get_tr_saved_modifier(&self) -> Vec2 {
        Vec2::new(self.br.get_saved_pos().x, self.tl.get_saved_pos().y)
    }
    fn get_bl_saved_modifier(&self) -> Vec2 {
        Vec2::new(self.tl.get_saved_pos().x, self.br.get_saved_pos().y)
    }

    fn get_top_saved_modifier(&self) -> Vec2 {
        (self.tl.get_saved_pos() + self.get_tr_saved_modifier()) / 2.
    }
    fn get_right_saved_modifier(&self) -> Vec2 {
        (self.get_tr_saved_modifier() + self.br.get_saved_pos()) / 2.
    }
    fn get_bottom_saved_modifier(&self) -> Vec2 {
        (self.br.get_saved_pos() + self.get_bl_saved_modifier()) / 2.
    }
    fn get_left_saved_modifier(&self) -> Vec2 {
        (self.get_bl_saved_modifier() + self.tl.get_saved_pos()) / 2.
    }
}
impl Display for ShapeRectangle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rectangle")
    }
}
impl Shape for ShapeRectangle {
    type PathElementsIter<'iter> = CShapeRectangleIter;

    fn path_elements(&self, tolerance: f64) -> CShapeRectangleIter {
        CShapeRectangleIter {
            rect_path_iter: self.get_rectangle().path_elements(tolerance),
        }
    }
    #[inline]
    fn area(&self) -> f64 {
        self.get_rectangle().area()
    }
    #[inline]
    fn perimeter(&self, accuracy: f64) -> f64 {
        self.get_rectangle().perimeter(accuracy)
    }
    #[inline]
    fn winding(&self, pt: Point) -> i32 {
        self.get_rectangle().winding(pt)
    }
    #[inline]
    fn bounding_box(&self) -> Rect {
        self.get_rectangle().bounding_box()
    }
    #[inline]
    fn as_rect(&self) -> Option<Rect> {
        self.get_rectangle().as_rect()
    }
    #[inline]
    fn contains(&self, pt: Point) -> bool {
        self.get_rectangle().abs().contains(pt)
    }
}
impl Shapes for ShapeRectangle {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 2.;

    fn new(pos1: Vec2, pos2: Vec2) -> ShapeKind {
        let mut br = Position::new(pos2, true);
        br.select(true);
        let tl = Position::new(pos1, true);

        ShapeKind::Rectangle(ShapeRectangle {
            tl,
            br,
            tr: Modifier::new(),
            bl: Modifier::new(),
            top: Modifier::new(),
            right: Modifier::new(),
            bottom: Modifier::new(),
            left: Modifier::new(),
            highlighted: false,
            selected: false,
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
}
impl ShapeKindFuncs for ShapeRectangle {
    fn save_vars(&mut self) {
        self.tl.save_pos();
        self.br.save_pos();
    }
    fn restore_saved(&mut self) {
        self.tl.restore_saved();
        self.br.restore_saved();
        self.update_polygon();
    }
    fn get_vars(&self) -> ShapeKindvars {
        ShapeKindvars::Rectangle(self.tl, self.br)
    }
    fn set_vars(&mut self, vars: &ShapeKindvars) {
        if let ShapeKindvars::Rectangle(tl, br) = vars {
            self.tl = tl.clone();
            self.br = br.clone();
            self.update_polygon();
        }
    }

    fn good_size(&self) -> bool {
        (self.tl.get_pos().x - self.br.get_pos().x).abs() >= ShapeRectangle::MIN_SIZE
            && (self.tl.get_pos().y - self.br.get_pos().y).abs() >= ShapeRectangle::MIN_SIZE
    }

    fn set_hs_from_pos(&mut self, pos: Vec2, hors: HS) -> bool {
        match hors {
            HS::Highlight => {
                self.highlighted = self.contains(pos.to_point());
                self.highlighted
            }
            HS::Select => {
                self.selected = self.contains(pos.to_point());
                self.selected
            }
        }
    }
    fn set_hs(&mut self, value: bool, hors: HS) {
        match hors {
            HS::Highlight => self.highlighted = value,
            HS::Select => self.selected = value,
        }
    }
    fn get_hs(&self, hors: HS) -> bool {
        match hors {
            HS::Highlight => self.highlighted,
            HS::Select => self.selected,
        }
    }

    fn set_hs_modifiers_from_pos(&mut self, pos: Vec2, hors: HS) -> bool {
        let tl_hors = (pos - self.tl.get_pos()).hypot() < Self::GRAB;
        let tr_hors = (pos - self.get_tr_modifier()).hypot() < Self::GRAB;
        let br_hors = (pos - self.br.get_pos()).hypot() < Self::GRAB;
        let bl_hors = (pos - self.get_bl_modifier()).hypot() < Self::GRAB;
        let top_hors = (pos - self.get_top_modifier()).hypot() < Self::GRAB;
        let right_hors = (pos - self.get_right_modifier()).hypot() < Self::GRAB;
        let bottom_hors = (pos - self.get_bottom_modifier()).hypot() < Self::GRAB;
        let left_hors = (pos - self.get_left_modifier()).hypot() < Self::GRAB;
        match hors {
            HS::Highlight => {
                self.tl.highlight(tl_hors);
                self.tr.highlight(tr_hors);
                self.br.highlight(br_hors);
                self.bl.highlight(bl_hors);
                self.top.highlight(top_hors);
                self.right.highlight(right_hors);
                self.bottom.highlight(bottom_hors);
                self.left.highlight(left_hors);
                self.tl.is_highlighted()
                    || self.tr.is_highlighted()
                    || self.br.is_highlighted()
                    || self.bl.is_highlighted()
                    || self.top.is_highlighted()
                    || self.right.is_highlighted()
                    || self.bottom.is_highlighted()
                    || self.left.is_highlighted()
            }
            HS::Select => {
                self.tl.select(tl_hors);
                self.tr.select(tr_hors);
                self.br.select(br_hors);
                self.bl.select(bl_hors);
                self.top.select(top_hors);
                self.right.select(right_hors);
                self.bottom.select(bottom_hors);
                self.left.select(left_hors);
                self.tl.is_selected()
                    || self.tr.is_selected()
                    || self.br.is_selected()
                    || self.bl.is_selected()
                    || self.top.is_selected()
                    || self.right.is_selected()
                    || self.bottom.is_selected()
                    || self.left.is_selected()
            }
        }
    }
    fn set_hs_modifiers(&mut self, value: bool, hors: HS) {
        match hors {
            HS::Highlight => {
                self.tl.highlight(value);
                self.tr.highlight(value);
                self.br.highlight(value);
                self.bl.highlight(value);
                self.top.highlight(value);
                self.right.highlight(value);
                self.bottom.highlight(value);
                self.left.highlight(value);
            }
            HS::Select => {
                self.tl.select(value);
                self.tr.select(value);
                self.br.select(value);
                self.bl.select(value);
                self.top.select(value);
                self.right.select(value);
                self.bottom.select(value);
                self.left.select(value);
            }
        }
    }
    fn get_hs_modifiers(&self, hors: HS) -> bool {
        match hors {
            HS::Highlight => {
                self.tl.is_highlighted()
                    || self.tr.is_highlighted()
                    || self.br.is_highlighted()
                    || self.bl.is_highlighted()
                    || self.top.is_highlighted()
                    || self.right.is_highlighted()
                    || self.bottom.is_highlighted()
                    || self.left.is_highlighted()
            }
            HS::Select => {
                self.tl.is_selected()
                    || self.tr.is_selected()
                    || self.br.is_selected()
                    || self.bl.is_selected()
                    || self.top.is_selected()
                    || self.right.is_selected()
                    || self.bottom.is_selected()
                    || self.left.is_selected()
            }
        }
    }

    fn toggle_prop(&mut self) {
        ()
    }

    fn move_position(&mut self, dpos: Vec2) {
        self.tl.set_pos(self.tl.get_saved_pos() + dpos);
        self.br.set_pos(self.br.get_saved_pos() + dpos);
        self.update_polygon();
    }
    fn move_modifier(&mut self, pos_init: Vec2, pos: Vec2, _shift_pressed: bool) -> bool {
        let tl_saved = self.tl.get_saved_pos();
        let br_saved = self.br.get_saved_pos();
        let tr_saved = self.get_tr_saved_modifier();
        let bl_saved = self.get_bl_saved_modifier();
        let top_saved = self.get_top_saved_modifier();
        let right_saved = self.get_right_saved_modifier();
        let bottom_saved = self.get_bottom_saved_modifier();
        let left_saved = self.get_left_saved_modifier();

        let tl_sel = self.tl.is_selected();
        let tr_sel = self.tr.is_selected();
        let br_sel = self.br.is_selected();
        let bl_sel = self.bl.is_selected();
        let top_sel = self.top.is_selected();
        let right_sel = self.right.is_selected();
        let bottom_sel = self.bottom.is_selected();
        let left_sel = self.left.is_selected();

        let dpos = pos - pos_init;
        const MIN_SIZE: f64 = ShapeRectangle::MIN_SIZE;

        let modified = match (tl_sel, tr_sel, br_sel, bl_sel) {
            (true, false, false, false) => {
                let mut tlpos = tl_saved + dpos;
                tlpos.x = tlpos.x.min(br_saved.x - MIN_SIZE);
                tlpos.y = tlpos.y.min(br_saved.y - MIN_SIZE);
                self.tl.set_pos(tlpos);
                true
            }
            (false, true, false, false) => {
                let mut trpos = tr_saved + dpos;
                trpos.x = trpos.x.max(tl_saved.x + MIN_SIZE);
                trpos.y = trpos.y.min(br_saved.y - MIN_SIZE);
                self.br.set_pos(Vec2::new(trpos.x, br_saved.y));
                self.tl.set_pos(Vec2::new(tl_saved.x, trpos.y));
                true
            }
            (false, false, true, false) => {
                let mut brpos = br_saved + dpos;
                brpos.x = brpos.x.max(bl_saved.x + MIN_SIZE);
                brpos.y = brpos.y.max(tr_saved.y + MIN_SIZE);
                self.br.set_pos(brpos);
                true
            }
            (false, false, false, true) => {
                let mut blpos = bl_saved + dpos;
                blpos.x = blpos.x.min(br_saved.x - MIN_SIZE);
                blpos.y = blpos.y.max(tl_saved.y + MIN_SIZE);
                self.tl.set_pos(Vec2::new(blpos.x, tl_saved.y));
                self.br.set_pos(Vec2::new(br_saved.x, blpos.y));
                true
            }
            _ => false,
        };
        if modified {
            self.update_polygon();
            return true;
        }
        let modified = match (top_sel, right_sel, bottom_sel, left_sel) {
            (true, false, false, false) => {
                let mut toppos = top_saved + dpos;
                toppos.y = toppos.y.min(bottom_saved.y - MIN_SIZE);
                self.tl.set_pos(Vec2::new(tl_saved.x, toppos.y));
                true
            }
            (false, true, false, false) => {
                let mut rightpos = right_saved + dpos;
                rightpos.x = rightpos.x.max(left_saved.x + MIN_SIZE);
                self.br.set_pos(Vec2::new(rightpos.x, br_saved.y));
                true
            }
            (false, false, true, false) => {
                let mut bottompos = bottom_saved + dpos;
                bottompos.y = bottompos.y.max(top_saved.y + MIN_SIZE);
                self.br.set_pos(Vec2::new(br_saved.x, bottompos.y));
                true
            }
            (false, false, false, true) => {
                let mut leftpos = left_saved + dpos;
                leftpos.x = leftpos.x.min(right_saved.x - MIN_SIZE);
                self.tl.set_pos(Vec2::new(leftpos.x, tl_saved.y));
                true
            }
            _ => false,
        };
        if modified {
            self.update_polygon();
            return true;
        }
        false
    }
    fn get_position(&self) -> Vec2 {
        (self.tl.get_pos() + self.br.get_pos()) / 2.
    }

    fn get_modifiers_paths(&self) -> Vec<(BezPath, Pattern)> {
        vec![
            (
                modifiers_path(self.tl.get_pos(), 1., ShapeRectangle::GRAB),
                self.get_pattern_modifiers(self.tl.is_selected(), self.tl.is_highlighted()),
            ),
            (
                modifiers_path(self.get_tr_modifier(), 1., ShapeRectangle::GRAB),
                self.get_pattern_modifiers(self.tr.is_selected(), self.tr.is_highlighted()),
            ),
            (
                modifiers_path(self.br.get_pos(), 1., ShapeRectangle::GRAB),
                self.get_pattern_modifiers(self.br.is_selected(), self.br.is_highlighted()),
            ),
            (
                modifiers_path(self.get_bl_modifier(), 1., ShapeRectangle::GRAB),
                self.get_pattern_modifiers(self.bl.is_selected(), self.bl.is_highlighted()),
            ),
            (
                modifiers_path(self.get_top_modifier(), 1., ShapeRectangle::GRAB),
                self.get_pattern_modifiers(self.top.is_selected(), self.top.is_highlighted()),
            ),
            (
                modifiers_path(self.get_right_modifier(), 1., ShapeRectangle::GRAB),
                self.get_pattern_modifiers(self.right.is_selected(), self.right.is_highlighted()),
            ),
            (
                modifiers_path(self.get_bottom_modifier(), 1., ShapeRectangle::GRAB),
                self.get_pattern_modifiers(self.bottom.is_selected(), self.bottom.is_highlighted()),
            ),
            (
                modifiers_path(self.get_left_modifier(), 1., ShapeRectangle::GRAB),
                self.get_pattern_modifiers(self.left.is_selected(), self.left.is_highlighted()),
            ),
            (
                center_path(
                    (self.tl.get_pos() + self.br.get_pos()) / 2.,
                    1.,
                    ShapeRectangle::GRAB,
                ),
                self.get_pattern_modifiers(self.selected, self.highlighted),
            ),
        ]
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        let mut paths = vec![];
        let mut texts = vec![];
        let (path, text) = Dimension::new(
            DimKind::Horizontal,
            self.tl.get_pos(),
            self.get_tr_modifier(),
        )
        .get_path();
        paths.push(path);
        texts.push(text);
        let (path, text) =
            Dimension::new(DimKind::Vertical, self.get_bl_modifier(), self.tl.get_pos()).get_path();
        paths.push(path);
        texts.push(text);
        (paths, texts)
    }
    fn get_paths_and_patterns(&self) -> Vec<(BezPath, Pattern)> {
        if self.good_size() {
            vec![(
                self.get_rectangle().to_path(Self::TOLERANCE),
                self.get_pattern(self.selected, self.highlighted),
            )]
        } else {
            vec![(BezPath::new(), Pattern::BasicNormal)]
        }
    }
    fn get_polygon(&self) -> Polygon<f64> {
        self.polygon.clone()
    }
}

pub struct CShapeRectangleIter {
    rect_path_iter: RectPathIter,
}
impl Iterator for CShapeRectangleIter {
    type Item = PathEl;

    fn next(&mut self) -> Option<PathEl> {
        self.rect_path_iter.next()
    }
}
