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
    positions::Position,
    prefab::magnet_path,
    shape_disc::HighLightOrSelect,
    shapes::{ShapeKind, Shapes},
};
use geo::{LineString, Polygon};
use kurbo::{BezPath, PathEl, Point, Rect, RectPathIter, Shape, Vec2};
use std::fmt::Display;
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeRectangle {
    tl: Position,
    tr: Position,
    br: Position,
    bl: Position,
    top: Position,
    right: Position,
    bottom: Position,
    left: Position,
    center: Position,

    highlighted: bool,
    selected: bool,

    segs: BezPath,
    polygon: Polygon<f64>,
}
impl ShapeRectangle {
    const MIN_SIZE: f64 = 10.;

    fn update_polygon(&mut self) {
        log!("calc rect polygon");
        self.segs = calc_segs(self.get_paths_patterns());
        self.polygon = calc_polygon(&self.segs);
    }
    fn update_edges_positions(&mut self) {
        self.top
            .set_pos((self.tl.get_pos() + self.tr.get_pos()) / 2.);
        self.right
            .set_pos((self.tr.get_pos() + self.br.get_pos()) / 2.);
        self.bottom
            .set_pos((self.br.get_pos() + self.bl.get_pos()) / 2.);
        self.left
            .set_pos((self.bl.get_pos() + self.tl.get_pos()) / 2.);
    }
    fn update_center(&mut self) {
        self.center
            .set_pos((self.tl.get_pos() + self.br.get_pos()) / 2.);
    }
    fn get_rectangle(&self) -> Rect {
        let tl_pos = self.tl.get_pos();
        let br_pos = self.br.get_pos();
        Rect::new(tl_pos.x, tl_pos.y, br_pos.x, br_pos.y)
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
        ShapeKind::Rectangle(ShapeRectangle {
            tl: Position::new(pos1, true),
            tr: Position::new(Vec2::new(pos2.x, pos1.y), true),
            br,
            bl: Position::new(Vec2::new(pos1.x, pos2.y), true),
            top: Position::new((pos1 + pos2) / 2., true),
            right: Position::new((pos1 + pos2) / 2., true),
            bottom: Position::new((pos1 + pos2) / 2., true),
            left: Position::new((pos1 + pos2) / 2., true),
            center: Position::new((pos1 + pos2) / 2., true),
            highlighted: false,
            selected: false,
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }
    fn good_size(&self) -> bool {
        (self.tl.get_pos().x - self.br.get_pos().x).abs() >= ShapeRectangle::MIN_SIZE
            && (self.tl.get_pos().y - self.br.get_pos().y).abs() >= ShapeRectangle::MIN_SIZE
    }
    fn save_pos(&mut self) {
        self.tl.save_pos();
        self.tr.save_pos();
        self.br.save_pos();
        self.bl.save_pos();
        self.top.save_pos();
        self.right.save_pos();
        self.bottom.save_pos();
        self.left.save_pos();
        self.center.save_pos();
    }
    fn toggle_prop(&mut self) {
        ()
    }
    fn hors_from_pos(&mut self, pos: Vec2, hors: HighLightOrSelect) -> bool {
        match hors {
            HighLightOrSelect::Highlight => {
                self.highlighted = self.contains(pos.to_point());
                self.highlighted
            }
            HighLightOrSelect::Select => {
                self.selected = self.contains(pos.to_point());
                self.selected
            }
        }
    }
    fn hors_center_from_pos(&mut self, pos: Vec2, hors: HighLightOrSelect) -> bool {
        let center_hors = (pos - self.center.get_pos()).hypot() < Self::GRAB;
        match hors {
            HighLightOrSelect::Highlight => {
                self.center.highlight(center_hors);
                self.center.is_highlighted()
            }
            HighLightOrSelect::Select => {
                self.center.select(center_hors);
                self.center.is_selected()
            }
        }
    }
    fn hors_modifiers_from_pos(&mut self, pos: Vec2, hors: HighLightOrSelect) -> bool {
        let tl_hors = (pos - self.tl.get_pos()).hypot() < Self::GRAB;
        let tr_hors = (pos - self.tr.get_pos()).hypot() < Self::GRAB;
        let br_hors = (pos - self.br.get_pos()).hypot() < Self::GRAB;
        let bl_hors = (pos - self.bl.get_pos()).hypot() < Self::GRAB;
        let top_hors = (pos - self.top.get_pos()).hypot() < Self::GRAB;
        let right_hors = (pos - self.right.get_pos()).hypot() < Self::GRAB;
        let bottom_hors = (pos - self.bottom.get_pos()).hypot() < Self::GRAB;
        let left_hors = (pos - self.left.get_pos()).hypot() < Self::GRAB;
        match hors {
            HighLightOrSelect::Highlight => {
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
            HighLightOrSelect::Select => {
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
    fn set_hors(&mut self, value: bool, hors: HighLightOrSelect) {
        match hors {
            HighLightOrSelect::Highlight => self.highlighted = value,
            HighLightOrSelect::Select => self.selected = value,
        }
    }
    fn set_hors_center(&mut self, value: bool, hors: HighLightOrSelect) {
        match hors {
            HighLightOrSelect::Highlight => self.center.highlight(value),
            HighLightOrSelect::Select => self.center.select(value),
        }
    }
    fn set_hors_modifiers(&mut self, value: bool, hors: HighLightOrSelect) {
        match hors {
            HighLightOrSelect::Highlight => {
                self.tl.highlight(value);
                self.tr.highlight(value);
                self.br.highlight(value);
                self.bl.highlight(value);
                self.top.highlight(value);
                self.right.highlight(value);
                self.bottom.highlight(value);
                self.left.highlight(value);
            }
            HighLightOrSelect::Select => {
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
    fn is_hors(&self, hors: HighLightOrSelect) -> bool {
        match hors {
            HighLightOrSelect::Highlight => self.highlighted,
            HighLightOrSelect::Select => self.selected,
        }
    }
    fn is_center_hors(&self, hors: HighLightOrSelect) -> bool {
        match hors {
            HighLightOrSelect::Highlight => self.center.is_highlighted(),
            HighLightOrSelect::Select => self.center.is_selected(),
        }
    }
    fn get_position(&self) -> Vec2 {
        self.center.get_pos()
    }
    fn move_position(&mut self, pos_init: Vec2, pos: Vec2) {
        let tl_saved = self.tl.get_saved_pos();
        let tr_saved = self.tr.get_saved_pos();
        let br_saved = self.br.get_saved_pos();
        let bl_saved = self.bl.get_saved_pos();
        let top_saved = self.top.get_saved_pos();
        let right_saved = self.right.get_saved_pos();
        let bottom_saved = self.bottom.get_saved_pos();
        let left_saved = self.left.get_saved_pos();
        let center_saved = self.center.get_saved_pos();

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
        if self.selected {
            self.center.set_pos(center_saved + dpos);
            self.tl.set_pos(tl_saved + dpos);
            self.tr.set_pos(tr_saved + dpos);
            self.br.set_pos(br_saved + dpos);
            self.bl.set_pos(bl_saved + dpos);
            self.top.set_pos(top_saved + dpos);
            self.right.set_pos(right_saved + dpos);
            self.bottom.set_pos(bottom_saved + dpos);
            self.left.set_pos(left_saved + dpos);
            self.update_polygon();
        } else {
            match (
                tl_sel, tr_sel, br_sel, bl_sel, top_sel, right_sel, bottom_sel, left_sel,
            ) {
                (true, false, false, false, false, false, false, false) => {
                    let mut tlpos = tl_saved + dpos;
                    tlpos.x = tlpos.x.min(tr_saved.x - MIN_SIZE);
                    tlpos.y = tlpos.y.min(bl_saved.y - MIN_SIZE);
                    self.tl.set_pos(tlpos);
                    self.bl.set_pos(Vec2::new(tlpos.x, bl_saved.y));
                    self.tr.set_pos(Vec2::new(tr_saved.x, tlpos.y));
                    self.update_edges_positions();
                    self.update_center();
                    self.update_polygon();
                }
                (false, true, false, false, false, false, false, false) => {
                    let mut trpos = tr_saved + dpos;
                    trpos.x = trpos.x.max(tl_saved.x + MIN_SIZE);
                    trpos.y = trpos.y.min(br_saved.y - MIN_SIZE);
                    self.tr.set_pos(trpos);
                    self.br.set_pos(Vec2::new(trpos.x, br_saved.y));
                    self.tl.set_pos(Vec2::new(tl_saved.x, trpos.y));
                    self.update_edges_positions();
                    self.update_center();
                    self.update_polygon();
                }
                (false, false, true, false, false, false, false, false) => {
                    let mut brpos = br_saved + dpos;
                    brpos.x = brpos.x.max(bl_saved.x + MIN_SIZE);
                    brpos.y = brpos.y.max(tr_saved.y + MIN_SIZE);
                    self.br.set_pos(brpos);
                    self.tr.set_pos(Vec2::new(brpos.x, tr_saved.y));
                    self.bl.set_pos(Vec2::new(bl_saved.x, brpos.y));
                    self.update_edges_positions();
                    self.update_center();
                    self.update_polygon();
                }
                (false, false, false, true, false, false, false, false) => {
                    let mut blpos = bl_saved + dpos;
                    blpos.x = blpos.x.min(br_saved.x - MIN_SIZE);
                    blpos.y = blpos.y.max(tl_saved.y + MIN_SIZE);
                    self.bl.set_pos(blpos);
                    self.tl.set_pos(Vec2::new(blpos.x, tl_saved.y));
                    self.br.set_pos(Vec2::new(br_saved.x, blpos.y));
                    self.update_edges_positions();
                    self.update_center();
                    self.update_polygon();
                }
                (false, false, false, false, true, false, false, false) => {
                    let mut toppos = top_saved + dpos;
                    toppos.y = toppos.y.min(bottom_saved.y - MIN_SIZE);
                    self.top.set_pos(toppos);
                    self.tl.set_pos(Vec2::new(tl_saved.x, toppos.y));
                    self.tr.set_pos(Vec2::new(tr_saved.x, toppos.y));
                    self.update_edges_positions();
                    self.update_center();
                    self.update_polygon();
                }
                (false, false, false, false, false, true, false, false) => {
                    let mut rightpos = right_saved + dpos;
                    rightpos.x = rightpos.x.max(left_saved.x + MIN_SIZE);
                    self.right.set_pos(rightpos);
                    self.tr.set_pos(Vec2::new(rightpos.x, tr_saved.y));
                    self.br.set_pos(Vec2::new(rightpos.x, br_saved.y));
                    self.update_edges_positions();
                    self.update_center();
                    self.update_polygon();
                }
                (false, false, false, false, false, false, true, false) => {
                    let mut bottompos = bottom_saved + dpos;
                    bottompos.y = bottompos.y.max(top_saved.y + MIN_SIZE);
                    self.bottom.set_pos(bottompos);
                    self.br.set_pos(Vec2::new(br_saved.x, bottompos.y));
                    self.bl.set_pos(Vec2::new(bl_saved.x, bottompos.y));
                    self.update_edges_positions();
                    self.update_center();
                    self.update_polygon();
                }
                (false, false, false, false, false, false, false, true) => {
                    let mut leftpos = left_saved + dpos;
                    leftpos.x = leftpos.x.min(right_saved.x - MIN_SIZE);
                    self.left.set_pos(leftpos);
                    self.bl.set_pos(Vec2::new(leftpos.x, bl_saved.y));
                    self.tl.set_pos(Vec2::new(leftpos.x, tl_saved.y));
                    self.update_edges_positions();
                    self.update_center();
                    self.update_polygon();
                }
                _ => (),
            }
        }
    }
    fn get_magnets_paths(&self) -> Vec<(BezPath, Pattern)> {
        vec![
            (
                magnet_path(self.tl.get_pos(), 1., ShapeRectangle::GRAB),
                self.get_pattern_modifiers(self.tl.is_selected(), self.tl.is_highlighted()),
            ),
            (
                magnet_path(self.tr.get_pos(), 1., ShapeRectangle::GRAB),
                self.get_pattern_modifiers(self.tr.is_selected(), self.tr.is_highlighted()),
            ),
            (
                magnet_path(self.br.get_pos(), 1., ShapeRectangle::GRAB),
                self.get_pattern_modifiers(self.br.is_selected(), self.br.is_highlighted()),
            ),
            (
                magnet_path(self.bl.get_pos(), 1., ShapeRectangle::GRAB),
                self.get_pattern_modifiers(self.bl.is_selected(), self.bl.is_highlighted()),
            ),
            (
                magnet_path(self.top.get_pos(), 1., ShapeRectangle::GRAB),
                self.get_pattern_modifiers(self.top.is_selected(), self.top.is_highlighted()),
            ),
            (
                magnet_path(self.right.get_pos(), 1., ShapeRectangle::GRAB),
                self.get_pattern_modifiers(self.right.is_selected(), self.right.is_highlighted()),
            ),
            (
                magnet_path(self.bottom.get_pos(), 1., ShapeRectangle::GRAB),
                self.get_pattern_modifiers(self.bottom.is_selected(), self.bottom.is_highlighted()),
            ),
            (
                magnet_path(self.left.get_pos(), 1., ShapeRectangle::GRAB),
                self.get_pattern_modifiers(self.left.is_selected(), self.left.is_highlighted()),
            ),
            (
                magnet_path(self.center.get_pos(), 1., ShapeRectangle::GRAB),
                self.get_pattern_modifiers(self.center.is_selected(), self.center.is_highlighted()),
            ),
        ]
    }
    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        let mut paths = vec![];
        let mut texts = vec![];
        let (path, text) =
            Dimension::new(DimKind::Horizontal, self.tl.get_pos(), self.tr.get_pos()).get_path();
        paths.push(path);
        texts.push(text);
        let (path, text) =
            Dimension::new(DimKind::Vertical, self.bl.get_pos(), self.tl.get_pos()).get_path();
        paths.push(path);
        texts.push(text);
        (paths, texts)
    }
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
    fn get_paths_patterns(&self) -> Vec<(BezPath, Pattern)> {
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
