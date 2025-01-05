// A macro to provide `println!(..)`-style syntax for `console.log` logging.
// macro_rules! log {
//     ( $( $t:tt )* ) => {
//         web_sys::console::log_1(&format!( $( $t )* ).into());
//     }
// }

use geo::{LineString, Polygon};
use kurbo::{BezPath, Circle, CirclePathIter, Point, Rect, Shape, Vec2};
use std::fmt::Display;

use crate::{
    canvas::{CanvasText, Pattern},
    dimensions::{DimKind, Dimension},
    math::*,
    positions::Position,
    prefab::{center_path, modifiers_path},
    shapes::{ShapeKind, Shapes},
};

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum HighLightOrSelect {
    Highlight,
    Select,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeDisc {
    radius_left: Position,
    radius_right: Position,

    highlighted: bool,
    selected: bool,

    segs: BezPath,
    polygon: Polygon<f64>,
}
impl ShapeDisc {
    const MIN_RADIUS: f64 = 2.;

    fn update_polygon(&mut self) {
        log!("calc disc polygon");
        self.segs = calc_segs(self.get_paths_and_patterns());
        self.polygon = calc_polygon(&self.segs);
    }
    fn get_center(&self) -> Vec2 {
        (self.radius_left.get_pos() + self.radius_right.get_pos()) / 2.
    }
    fn get_radius(&self) -> f64 {
        (self.radius_left.get_pos() - self.radius_right.get_pos()).hypot() / 2.
    }
    fn get_circle(&self) -> Circle {
        let center = self.get_center();
        let radius = self.get_radius();
        Circle::new(center.to_point(), radius)
    }
}
impl Display for ShapeDisc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Circle")
    }
}
impl Shape for ShapeDisc {
    type PathElementsIter<'iter> = CirclePathIter;

    fn path_elements(&self, tolerance: f64) -> CirclePathIter {
        self.get_circle().path_elements(tolerance)
    }
    #[inline]
    fn area(&self) -> f64 {
        self.get_circle().area()
    }
    #[inline]
    fn perimeter(&self, accuracy: f64) -> f64 {
        self.get_circle().perimeter(accuracy)
    }
    #[inline]
    fn winding(&self, pt: Point) -> i32 {
        self.get_circle().winding(pt)
    }
    #[inline]
    fn bounding_box(&self) -> Rect {
        self.get_circle().bounding_box()
    }
    #[inline]
    fn as_circle(&self) -> Option<Circle> {
        self.get_circle().as_circle()
    }
    #[inline]
    fn contains(&self, pt: Point) -> bool {
        self.get_circle().contains(pt)
    }
}
impl Shapes for ShapeDisc {
    const TOLERANCE: f64 = 0.01;
    const GRAB: f64 = 2.;

    fn new(pos1: Vec2, _pos2: Vec2) -> ShapeKind {
        let radius_left = Position::new(pos1, true);
        let mut radius_right =
            Position::new(pos1 + Vec2::new(2. * ShapeDisc::MIN_RADIUS, 0.), true);
        radius_right.select(true);
        ShapeKind::Disc(ShapeDisc {
            radius_left,
            radius_right,
            highlighted: false,
            selected: false,
            segs: BezPath::new(),
            polygon: Polygon::new(LineString::new(vec![]), vec![]),
        })
    }

    fn good_size(&self) -> bool {
        self.get_radius() >= ShapeDisc::MIN_RADIUS
    }
    fn save_pos(&mut self) {
        self.radius_left.save_pos();
        self.radius_right.save_pos();
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
    fn hors_modifiers_from_pos(&mut self, pos: Vec2, hors: HighLightOrSelect) -> bool {
        let radius_left_hors = (pos - self.radius_left.get_pos()).hypot() < Self::GRAB;
        let radius_right_hors = (pos - self.radius_right.get_pos()).hypot() < Self::GRAB;
        match hors {
            HighLightOrSelect::Highlight => {
                self.radius_left.highlight(radius_left_hors);
                self.radius_right.highlight(radius_right_hors);
                self.radius_right.is_highlighted() || self.radius_left.is_highlighted()
            }
            HighLightOrSelect::Select => {
                self.radius_left.select(radius_left_hors);
                self.radius_right.select(radius_right_hors);
                self.radius_right.is_selected() || self.radius_left.is_selected()
            }
        }
    }
    fn set_hors(&mut self, value: bool, hors: HighLightOrSelect) {
        match hors {
            HighLightOrSelect::Highlight => self.highlighted = value,
            HighLightOrSelect::Select => self.selected = value,
        }
    }
    fn set_hors_modifiers(&mut self, value: bool, hors: HighLightOrSelect) {
        match hors {
            HighLightOrSelect::Highlight => {
                self.radius_left.highlight(value);
                self.radius_right.highlight(value);
            }
            HighLightOrSelect::Select => {
                self.radius_left.select(value);
                self.radius_right.select(value);
            }
        }
    }
    fn is_hors(&self, hors: HighLightOrSelect) -> bool {
        match hors {
            HighLightOrSelect::Highlight => self.highlighted,
            HighLightOrSelect::Select => self.selected,
        }
    }
    fn get_position(&self) -> Vec2 {
        self.get_center()
    }
    fn move_position(&mut self, pos_init: Vec2, pos: Vec2, _shift_pressed: bool) {
        let radius_left_saved = self.radius_left.get_saved_pos();
        let radius_right_saved = self.radius_right.get_saved_pos();

        let dpos = pos - pos_init;

        if self.selected {
            self.radius_left.set_pos(radius_left_saved + dpos);
            self.radius_right.set_pos(radius_right_saved + dpos);
            self.update_polygon();
        } else {
            if self.radius_left.is_selected() {
                let radius_p = radius_left_saved + Vec2::new(dpos.x, 0.);
                if radius_p.x < radius_right_saved.x {
                    let radius = (radius_p - radius_right_saved).hypot() / 2.;
                    if radius >= ShapeDisc::MIN_RADIUS {
                        self.radius_left.set_pos(radius_p);
                        self.update_polygon();
                    };
                }
            }
            if self.radius_right.is_selected() {
                let radius_p = radius_right_saved + Vec2::new(dpos.x, 0.);
                if radius_p.x > radius_left_saved.x {
                    let radius = (radius_p - radius_left_saved).hypot() / 2.;
                    if radius >= ShapeDisc::MIN_RADIUS {
                        self.radius_right.set_pos(radius_p);
                        self.update_polygon();
                    };
                }
            }
        }
    }

    fn get_dimensions_paths(&self) -> (Vec<(BezPath, Pattern)>, Vec<CanvasText>) {
        let mut paths = vec![];
        let mut texts = vec![];
        let offset = self.get_radius() / 2_f64.sqrt() + 2.;
        let end = self.get_center() + Vec2::new(offset, -offset);
        let (path, text) = Dimension::new(DimKind::Radius, self.get_center(), end).get_path();
        paths.push(path);
        texts.push(text);
        (paths, texts)
    }
    fn get_modifiers_paths(&self) -> Vec<(BezPath, Pattern)> {
        vec![
            (
                modifiers_path(self.radius_left.get_pos(), 1., ShapeDisc::GRAB),
                self.get_pattern_modifiers(
                    self.radius_left.is_selected(),
                    self.radius_left.is_highlighted(),
                ),
            ),
            (
                modifiers_path(self.radius_right.get_pos(), 1., ShapeDisc::GRAB),
                self.get_pattern_modifiers(
                    self.radius_right.is_selected(),
                    self.radius_right.is_highlighted(),
                ),
            ),
            (
                center_path(self.get_center(), 1., ShapeDisc::GRAB),
                self.get_pattern_modifiers(self.selected, self.highlighted),
            ),
        ]
    }

    fn get_paths_and_patterns(&self) -> Vec<(BezPath, Pattern)> {
        vec![(
            self.get_circle().to_path(Self::TOLERANCE),
            self.get_pattern(self.selected, self.highlighted),
        )]
    }

    fn get_polygon(&self) -> Polygon<f64> {
        self.polygon.clone()
    }
}
