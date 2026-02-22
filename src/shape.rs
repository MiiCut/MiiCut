use crate::helpers::math::{get_magnets_vertices, length_unit_to_mm};
use crate::types::others::{Properties, Property, PropertyValue};
use crate::types::scalar::Scalar;
use crate::types::vertex::Vertex;
use crate::voronoi::{inset_rings, lloyd_relax_points, round_rings, voronoi_cells};
use crate::{
    dom::document,
    helpers::math::*,
    inputs::UserUI,
    types::others::{EUId, SegBundle, VUId, VecRing},
};
use geo::algorithm::contains::Contains;
use geo::algorithm::orient::Orient;
use geo::algorithm::translate::Translate;
use geo::{orient::Direction, Coord, LineString, MultiPolygon, Point, Polygon};
use js_sys::Math;
use kurbo::{flatten, Arc, BezPath, Circle, PathEl, Shape, Vec2};
use std::hash::Hash;
use std::{
    f64::consts::PI,
    fmt::{Debug, Display},
    vec,
};
use svg::parser::{Event as SvgEvent, Parser as SvgParser};
use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlElement};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextFont {
    Stencilia,
    Urbanist,
}
impl TextFont {
    pub fn data(&self) -> &'static [u8] {
        match self {
            TextFont::Stencilia => include_bytes!("../assets/stencilia/Stencilia-A.ttf"),
            TextFont::Urbanist => include_bytes!("../assets/urbanist/Urbanist-Variable.ttf"),
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            TextFont::Stencilia => "Stencilia-A",
            TextFont::Urbanist => "Urbanist",
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextData {
    polygon: Option<MultiPolygon<f64>>,
    cached_paths: Option<Vec<BezPath>>,
}
impl TextData {
    pub fn new(polygon: Option<MultiPolygon<f64>>) -> Self {
        Self {
            polygon,
            cached_paths: None,
        }
    }
    fn invalidate_cache(&mut self) {
        self.polygon = None;
        self.cached_paths = None;
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SvgFillRule {
    EvenOdd,
    NonZero,
}

#[derive(Clone, Debug)]
pub struct SvgData {
    pub rings: Vec<Vec<Vec2>>,
    pub fill_rule: SvgFillRule,
    original_min: Vec2,
    original_max: Vec2,
    cached_bbox_min: Vec2,
    cached_bbox_max: Vec2,
    cached_polygon: Option<MultiPolygon<f64>>,
    cached_paths_raw: Option<Vec<BezPath>>,
    cached_paths: Option<Vec<BezPath>>,
}
impl SvgData {
    pub fn new(rings: Vec<Vec<Vec2>>, fill_rule: SvgFillRule) -> Self {
        let mut min = Vec2::new(f64::INFINITY, f64::INFINITY);
        let mut max = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for ring in rings.iter() {
            for pt in ring.iter() {
                min.x = min.x.min(pt.x);
                min.y = min.y.min(pt.y);
                max.x = max.x.max(pt.x);
                max.y = max.y.max(pt.y);
            }
        }
        if !min.x.is_finite() || !min.y.is_finite() || !max.x.is_finite() || !max.y.is_finite() {
            min = Vec2::ZERO;
            max = Vec2::ZERO;
        }
        Self {
            rings,
            fill_rule,
            original_min: min,
            original_max: max,
            cached_bbox_min: Vec2::ZERO,
            cached_bbox_max: Vec2::ZERO,
            cached_polygon: None,
            cached_paths_raw: None,
            cached_paths: None,
        }
    }
    fn invalidate_cache(&mut self) {
        self.cached_polygon = None;
        self.cached_paths_raw = None;
        self.cached_paths = None;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShapeType {
    Arrow,
    Disc,
    Square,
    Oblong,
    Poly,
    Text,
    Svg,
    Voronoi,
    Group,
    ConstrLine,
    ConstrCircle,
}
impl ShapeType {
    pub fn id(&self) -> &'static str {
        use ShapeType::*;
        match self {
            Arrow => "icon-arrow",
            Disc => "icon-disc",
            Square => "icon-square",
            Oblong => "icon-oblong",
            Poly => "icon-polygon",
            Text => "icon-text",
            Svg => "icon-svg",
            Voronoi => "icon-voronoi",
            Group => "icon-group",
            ConstrLine => "icon-constr-line",
            ConstrCircle => "icon-constr-circle",
        }
    }
    pub fn get_element(&self) -> Option<Element> {
        document().get_element_by_id(self.id())
    }
    pub fn get_html_element(&self) -> Option<HtmlElement> {
        if let Some(element) = self.get_element() {
            return element.dyn_into().ok();
        };
        None
    }
}

#[derive(Debug, Clone)]
pub struct ShapeRotation {
    rotation: f64,
    rotation_saved: f64,
}
impl Default for ShapeRotation {
    fn default() -> Self {
        Self::new()
    }
}

impl ShapeRotation {
    pub fn new() -> Self {
        Self {
            rotation: 0.,
            rotation_saved: 0.,
        }
    }
    pub fn set(&mut self, rotation: f64) {
        self.rotation_saved = self.rotation;
        self.rotation = rotation;
    }
    pub fn set_curr(&mut self, rotation: f64) {
        self.rotation = rotation;
    }
    pub fn get(&self) -> f64 {
        self.rotation
    }
    pub fn get_saved(&self) -> f64 {
        self.rotation_saved
    }
    pub fn save(&mut self) {
        self.rotation_saved = self.rotation;
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct GeneralShape {
    shape_type: ShapeType,
    shape_name: Option<String>,
    order: i32,
    vertices: VecRing<VUId>,
    operation: Operation,
    properties: Properties,

    // Data for specific shapes
    text_shape_data: Option<TextData>,
    svg_shape_data: Option<SvgData>,
    voronoi_shape_data: Option<SvgData>,
    group_shape_data: Option<GroupShape>,

    bezpath: BezPath,
    polygon: MultiPolygon<f64>,

    rotation: ShapeRotation,
}
impl Clone for GeneralShape {
    fn clone(&self) -> Self {
        let new_vertices: Vec<(VUId, Vertex)> = self
            .vertices
            .iter()
            .map(|(_, value)| (VUId::new(), value.clone()))
            .collect();
        Self {
            shape_type: self.shape_type,
            shape_name: self.shape_name.clone(),
            order: self.order,
            vertices: VecRing::from_slice(&new_vertices[..]).unwrap(),
            operation: self.operation,
            properties: self.properties.clone(),
            text_shape_data: self.text_shape_data.clone(),
            svg_shape_data: self.svg_shape_data.clone(),
            voronoi_shape_data: self.voronoi_shape_data.clone(),
            group_shape_data: self.group_shape_data.clone(),
            bezpath: self.bezpath.clone(),
            polygon: self.polygon.clone(),
            rotation: self.rotation.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GroupShape {
    children: Vec<EUId>,
}
impl GroupShape {
    pub fn new(children: Vec<EUId>) -> Self {
        Self { children }
    }
    pub fn children(&self) -> &[EUId] {
        &self.children
    }
    pub fn set_children(&mut self, children: Vec<EUId>) {
        self.children = children;
    }
}
impl GeneralShape {
    const TOLERANCE: f64 = 0.01;
    const GRAB_RADIUS: f64 = 3.0;
    const MIN_MAGNETS: usize = 6;
    const DEFAULT_MAGNETS: usize = 6;
    const MIN_RADIUS: f64 = 2.0;
    const DEFAULT_SEEDS: usize = 40;
    const MIN_SEEDS: usize = 10;
    const MAX_SEEDS: usize = 100;
    const DEFAULT_VORONOI_GAP: f64 = 0.2;
    const MIN_VORONOI_GAP: f64 = 0.0;
    const MAX_VORONOI_GAP: f64 = 0.8;
    const VORONOI_GAP_STEP: f64 = 0.01;
    const DEFAULT_VORONOI_RELAXATION: usize = 1;
    const MIN_VORONOI_RELAXATION: usize = 0;
    const MAX_VORONOI_RELAXATION: usize = 5;
    const DEFAULT_SCALE: f64 = 1.0;
    const MIN_SCALE: f64 = 0.01;
    const MAX_SCALE: f64 = 1000.0;
    const SCALE_STEP: f64 = 0.1;

    pub fn op_next(&mut self) {
        self.operation.next();
    }
    pub fn op_union(&mut self) {
        self.operation.union();
    }
    pub fn op_difference(&mut self) {
        self.operation.difference();
    }
    pub(crate) fn bb(v1: Vec2, v2: Vec2) -> Vec<Vertex> {
        let min_x = v1.x.min(v2.x);
        let max_x = v1.x.max(v2.x);
        let min_y = v1.y.min(v2.y);
        let max_y = v1.y.max(v2.y);
        let bl = Vertex::new_from_coords(min_x, max_y);
        let tr = Vertex::new_from_coords(max_x, max_y);
        let br = Vertex::new_from_coords(max_x, min_y);
        let tl = Vertex::new_from_coords(min_x, min_y);
        vec![tl, bl, tr, br]
    }

    pub fn new_shape_disc(v1: Vec2, v2: Vec2, order: i32) -> Option<Self> {
        if v1 == v2 {
            return None;
        }
        let vs = vec![Vertex::new(v1), Vertex::new(v2)];

        let mut properties = Properties::new();
        use PropertyValue::*;
        properties.add(Property::Center, Center { idx: 0, value: v1 });
        properties.add(
            Property::Radius,
            Radius {
                idx: 1,
                value: Scalar::new((v2 - v1).hypot(), Self::MIN_RADIUS, f64::INFINITY, 1.0),
            },
        );
        properties.add(
            Property::Scale,
            Scale {
                value: Scalar::new(
                    Self::DEFAULT_SCALE,
                    Self::MIN_SCALE,
                    Self::MAX_SCALE,
                    Self::SCALE_STEP,
                ),
            },
        );

        GeneralShape::new(
            ShapeType::Disc,
            vs,
            properties,
            order,
            None,
            None,
            None,
            None,
            Operation::Union,
            None,
        )
    }
    pub fn new_shape_rectangle(v1: Vec2, v2: Vec2, order: i32) -> Option<Self> {
        if v1 == v2 {
            return None;
        }
        let vs = GeneralShape::bb(v1, v2);

        use PropertyValue::*;
        let mut properties = Properties::new();
        properties.add(
            Property::BottomLeft,
            BottomLeft {
                idx: 0,
                value: vs[0].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::TopLeft,
            TopLeft {
                idx: 1,
                value: vs[1].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::TopRight,
            TopRight {
                idx: 2,
                value: vs[2].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::BottomRight,
            BottomRight {
                idx: 3,
                value: vs[3].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::Scale,
            Scale {
                value: Scalar::new(
                    Self::DEFAULT_SCALE,
                    Self::MIN_SCALE,
                    Self::MAX_SCALE,
                    Self::SCALE_STEP,
                ),
            },
        );

        let values: Vec<Vertex> = vs
            .into_iter()
            .map(|v| {
                let mut vertex = v;
                vertex.enable_radius();
                vertex
            })
            .collect();

        GeneralShape::new(
            ShapeType::Square,
            values,
            properties,
            order,
            None,
            None,
            None,
            None,
            Operation::Union,
            None,
        )
    }
    pub fn new_shape_oblong(v1: Vec2, v2: Vec2, order: i32) -> Option<Self> {
        Self::new_shape_oblong_with_radii(v1, v2, None, None, order)
    }
    pub fn new_shape_oblong_with_radii(
        v1: Vec2,
        v2: Vec2,
        r1: Option<Vec2>,
        r2: Option<Vec2>,
        order: i32,
    ) -> Option<Self> {
        if v1 == v2 {
            return None;
        }
        let dir = (v2 - v1).normalize();
        let perp = Vec2::new(-dir.y, dir.x);
        let default_r1 = v1 + perp * 20.0;
        let default_r2 = v2 + perp * 20.0;
        let r1 = r1.unwrap_or(default_r1);
        let r2 = r2.unwrap_or(default_r2);
        let vs = vec![
            Vertex::new(v1),
            Vertex::new(v2),
            Vertex::new(r1),
            Vertex::new(r2),
        ];

        use PropertyValue::*;
        let mut properties = Properties::new();
        properties.add(Property::Pt1, Pt1 { idx: 0, value: v1 });
        properties.add(Property::Pt2, Pt2 { idx: 1, value: v2 });
        properties.add(
            Property::Scale,
            Scale {
                value: Scalar::new(
                    Self::DEFAULT_SCALE,
                    Self::MIN_SCALE,
                    Self::MAX_SCALE,
                    Self::SCALE_STEP,
                ),
            },
        );

        GeneralShape::new(
            ShapeType::Oblong,
            vs,
            properties,
            order,
            None,
            None,
            None,
            None,
            Operation::Union,
            None,
        )
    }
    pub fn new_shape_text(v1: Vec2, v2: Vec2, order: i32) -> Option<Self> {
        const DEFAULT_TEXT: &str = "TEXT";
        const DEFAULT_FONT: TextFont = TextFont::Stencilia;

        if v1 == v2 {
            return None;
        }
        let vs = GeneralShape::bb(v1, v2);
        let (poly, scale) = text_to_multipolygon(DEFAULT_TEXT, &DEFAULT_FONT, v1, v2);

        use PropertyValue::*;
        let mut properties = Properties::new();
        properties.add(
            Property::BottomLeft,
            BottomLeft {
                idx: 0,
                value: vs[0].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::TopLeft,
            TopLeft {
                idx: 1,
                value: vs[1].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::TopRight,
            TopRight {
                idx: 2,
                value: vs[2].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::BottomRight,
            BottomRight {
                idx: 3,
                value: vs[3].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::Text,
            Text {
                value: DEFAULT_TEXT.to_string(),
            },
        );
        properties.add(
            Property::Font,
            Font {
                value: DEFAULT_FONT,
            },
        );
        properties.add(
            Property::Scale,
            Scale {
                value: Scalar::new(
                    Self::DEFAULT_SCALE,
                    Self::MIN_SCALE,
                    Self::MAX_SCALE,
                    Self::SCALE_STEP,
                ),
            },
        );
        let _ = scale;

        GeneralShape::new(
            ShapeType::Text,
            vs,
            properties,
            order,
            Some(TextData::new(Some(poly))),
            None,
            None,
            None,
            Operation::Union,
            None,
        )
    }
    pub fn new_shape_poly(vs: Vec<Vec2>, order: i32) -> Option<Self> {
        if vs.len() < 3 {
            return None;
        }
        use PropertyValue::*;
        let mut properties = Properties::new();
        vs.iter().enumerate().for_each(|(idx, v)| {
            properties.add(
                Property::Apex { idx },
                Apex {
                    idx,
                    value: *v,
                    radius: None,
                },
            )
        });
        properties.add(
            Property::Scale,
            Scale {
                value: Scalar::new(
                    Self::DEFAULT_SCALE,
                    Self::MIN_SCALE,
                    Self::MAX_SCALE,
                    Self::SCALE_STEP,
                ),
            },
        );
        let values: Vec<Vertex> = vs
            .into_iter()
            .map(|v| {
                let mut vertex = Vertex::new(v);
                vertex.enable_radius();
                vertex
            })
            .collect();

        GeneralShape::new(
            ShapeType::Poly,
            values,
            properties,
            order,
            None,
            None,
            None,
            None,
            Operation::Union,
            None,
        )
    }
    pub fn new_shape_voronoi(v1: Vec2, v2: Vec2, order: i32) -> Option<Self> {
        if v1 == v2 {
            return None;
        }
        let rings = build_voronoi_rings(
            v1,
            v2,
            GeneralShape::DEFAULT_SEEDS,
            Self::DEFAULT_VORONOI_GAP,
            Self::DEFAULT_VORONOI_RELAXATION,
        );
        let voronoi_svg = voronoi_rings_to_svg_data(rings);

        let vs = GeneralShape::bb(v1, v2);

        use PropertyValue::*;
        let mut properties = Properties::new();
        properties.add(
            Property::BottomLeft,
            BottomLeft {
                idx: 0,
                value: vs[0].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::TopLeft,
            TopLeft {
                idx: 1,
                value: vs[1].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::TopRight,
            TopRight {
                idx: 2,
                value: vs[2].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::BottomRight,
            BottomRight {
                idx: 3,
                value: vs[3].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::Seeds,
            Seeds {
                value: Scalar::new(
                    GeneralShape::DEFAULT_SEEDS as u64,
                    GeneralShape::MIN_SEEDS as u64,
                    GeneralShape::MAX_SEEDS as u64,
                    1,
                ),
            },
        );
        properties.add(
            Property::VoronoiGap,
            VoronoiGap {
                value: Scalar::new(
                    Self::DEFAULT_VORONOI_GAP,
                    Self::MIN_VORONOI_GAP,
                    Self::MAX_VORONOI_GAP,
                    Self::VORONOI_GAP_STEP,
                ),
            },
        );
        properties.add(
            Property::VoronoiRelaxation,
            VoronoiRelaxation {
                value: Scalar::new(
                    Self::DEFAULT_VORONOI_RELAXATION as u64,
                    Self::MIN_VORONOI_RELAXATION as u64,
                    Self::MAX_VORONOI_RELAXATION as u64,
                    1,
                ),
            },
        );
        properties.add(
            Property::Scale,
            Scale {
                value: Scalar::new(
                    Self::DEFAULT_SCALE,
                    Self::MIN_SCALE,
                    Self::MAX_SCALE,
                    Self::SCALE_STEP,
                ),
            },
        );

        GeneralShape::new(
            ShapeType::Voronoi,
            vs,
            properties,
            order,
            None,
            None,
            Some(voronoi_svg),
            None,
            Operation::Union,
            None,
        )
    }
    pub fn new_shape_svg_fit(
        order: i32,
        svg_data: String,
        combine_paths: bool,
        v1: Vec2,
        v2: Vec2,
    ) -> Option<Self> {
        let (parsed, _scale) = parse_svg(svg_data, combine_paths)?;
        let mut rings = Vec::new();
        let mut svg_min = Vec2::new(f64::INFINITY, f64::INFINITY);
        let mut svg_max = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        let fill_rule = parsed
            .first()
            .map(|shape| shape._fill_rule)
            .unwrap_or(SvgFillRule::NonZero);
        for shape in parsed {
            svg_min.x = svg_min.x.min(shape.bbox_min.x);
            svg_min.y = svg_min.y.min(shape.bbox_min.y);
            svg_max.x = svg_max.x.max(shape.bbox_max.x);
            svg_max.y = svg_max.y.max(shape.bbox_max.y);
            rings.extend(shape.rings);
        }
        if rings.is_empty()
            || !svg_min.x.is_finite()
            || !svg_min.y.is_finite()
            || !svg_max.x.is_finite()
            || !svg_max.y.is_finite()
        {
            return None;
        }
        let target_min = Vec2::new(v1.x.min(v2.x), v1.y.min(v2.y));
        let target_max = Vec2::new(v1.x.max(v2.x), v1.y.max(v2.y));
        let target_size = target_max - target_min;
        let svg_size = svg_max - svg_min;
        let center = (target_min + target_max) * 0.5;
        let fit_size = if svg_size.x > target_size.x || svg_size.y > target_size.y {
            let scale = (target_size.x / svg_size.x).min(target_size.y / svg_size.y);
            svg_size * scale
        } else {
            svg_size
        };
        let half = fit_size * 0.5;
        let fit_min = center - half;
        let fit_max = center + half;
        let svg = SvgData::new(rings, fill_rule);
        let tl = Vec2::new(fit_min.x, fit_max.y);
        let br = Vec2::new(fit_max.x, fit_min.y);
        let vs = GeneralShape::bb(tl, br);

        use PropertyValue::*;
        let mut properties = Properties::new();
        properties.add(
            Property::BottomLeft,
            BottomLeft {
                idx: 0,
                value: vs[0].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::TopLeft,
            TopLeft {
                idx: 1,
                value: vs[1].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::TopRight,
            TopRight {
                idx: 2,
                value: vs[2].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::BottomRight,
            BottomRight {
                idx: 3,
                value: vs[3].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::Scale,
            Scale {
                value: Scalar::new(
                    Self::DEFAULT_SCALE,
                    Self::MIN_SCALE,
                    Self::MAX_SCALE,
                    Self::SCALE_STEP,
                ),
            },
        );

        GeneralShape::new(
            ShapeType::Svg,
            vs,
            properties,
            order,
            None,
            Some(svg),
            None,
            None,
            Operation::Union,
            None,
        )
    }
    pub fn new_shapes_svg_fit(
        order: i32,
        svg_data: String,
        v1: Vec2,
        v2: Vec2,
    ) -> Option<Vec<Self>> {
        let (parsed, _scale) = parse_svg(svg_data, false)?;
        let mut svg_min = Vec2::new(f64::INFINITY, f64::INFINITY);
        let mut svg_max = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for shape in &parsed {
            svg_min.x = svg_min.x.min(shape.bbox_min.x);
            svg_min.y = svg_min.y.min(shape.bbox_min.y);
            svg_max.x = svg_max.x.max(shape.bbox_max.x);
            svg_max.y = svg_max.y.max(shape.bbox_max.y);
        }
        if !svg_min.x.is_finite()
            || !svg_min.y.is_finite()
            || !svg_max.x.is_finite()
            || !svg_max.y.is_finite()
        {
            return None;
        }

        let target_min = Vec2::new(v1.x.min(v2.x), v1.y.min(v2.y));
        let target_max = Vec2::new(v1.x.max(v2.x), v1.y.max(v2.y));
        let target_size = target_max - target_min;
        let svg_size = svg_max - svg_min;
        let center = (target_min + target_max) * 0.5;
        let fit_size = if svg_size.x > target_size.x || svg_size.y > target_size.y {
            let scale = (target_size.x / svg_size.x).min(target_size.y / svg_size.y);
            svg_size * scale
        } else {
            svg_size
        };
        let scale = if svg_size.x > 0.0 {
            fit_size.x / svg_size.x
        } else {
            1.0
        };
        let fit_min = center - fit_size * 0.5;

        let mut shapes = Vec::new();
        for parsed_shape in parsed {
            let transformed_rings: Vec<Vec<Vec2>> = parsed_shape
                .rings
                .into_iter()
                .map(|ring| {
                    ring.into_iter()
                        .map(|pt| (pt - svg_min) * scale + fit_min)
                        .collect()
                })
                .collect();
            let bbox_min = (parsed_shape.bbox_min - svg_min) * scale + fit_min;
            let bbox_max = (parsed_shape.bbox_max - svg_min) * scale + fit_min;
            if let Some(shape) = Self::new_svg_shape_with_rings(
                order,
                transformed_rings,
                parsed_shape._fill_rule,
                bbox_min,
                bbox_max,
            ) {
                shapes.push(shape);
            }
        }

        if shapes.is_empty() {
            return None;
        }
        Some(shapes)
    }
    pub fn new_shape_group(v1: Vec2, v2: Vec2, order: i32, children: Vec<EUId>) -> Option<Self> {
        if v1 == v2 || children.is_empty() {
            return None;
        }
        let vs = GeneralShape::bb(v1, v2);

        use PropertyValue::*;
        let mut properties = Properties::new();
        properties.add(
            Property::BottomLeft,
            BottomLeft {
                idx: 0,
                value: vs[0].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::TopLeft,
            TopLeft {
                idx: 1,
                value: vs[1].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::TopRight,
            TopRight {
                idx: 2,
                value: vs[2].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::BottomRight,
            BottomRight {
                idx: 3,
                value: vs[3].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::Scale,
            Scale {
                value: Scalar::new(
                    Self::DEFAULT_SCALE,
                    Self::MIN_SCALE,
                    Self::MAX_SCALE,
                    Self::SCALE_STEP,
                ),
            },
        );

        GeneralShape::new(
            ShapeType::Group,
            vs,
            properties,
            order,
            None,
            None,
            None,
            Some(GroupShape::new(children)),
            Operation::Union,
            None,
        )
    }
    fn new_svg_shape_with_rings(
        order: i32,
        rings: Vec<Vec<Vec2>>,
        fill_rule: SvgFillRule,
        bbox_min: Vec2,
        bbox_max: Vec2,
    ) -> Option<Self> {
        if rings.is_empty()
            || !bbox_min.x.is_finite()
            || !bbox_min.y.is_finite()
            || !bbox_max.x.is_finite()
            || !bbox_max.y.is_finite()
        {
            return None;
        }
        let svg = SvgData::new(rings, fill_rule);
        let vs = GeneralShape::bb(bbox_max, bbox_min);

        use PropertyValue::*;
        let mut properties = Properties::new();
        properties.add(
            Property::BottomLeft,
            BottomLeft {
                idx: 0,
                value: vs[0].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::TopLeft,
            TopLeft {
                idx: 1,
                value: vs[1].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::TopRight,
            TopRight {
                idx: 2,
                value: vs[2].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::BottomRight,
            BottomRight {
                idx: 3,
                value: vs[3].curr(),
                radius: None,
            },
        );
        properties.add(
            Property::Scale,
            Scale {
                value: Scalar::new(
                    Self::DEFAULT_SCALE,
                    Self::MIN_SCALE,
                    Self::MAX_SCALE,
                    Self::SCALE_STEP,
                ),
            },
        );

        GeneralShape::new(
            ShapeType::Svg,
            vs,
            properties,
            order,
            None,
            Some(svg),
            None,
            None,
            Operation::Union,
            None,
        )
    }
    pub fn new_shape_constr_line(v1: Vec2, v2: Vec2, order: i32) -> Option<Self> {
        if v1 == v2 {
            return None;
        }
        let vs = vec![Vertex::new(v1), Vertex::new(v2)];

        use PropertyValue::*;
        let mut properties = Properties::new();
        properties.add(Property::Pt1, Pt1 { idx: 0, value: v1 });
        properties.add(Property::Pt2, Pt2 { idx: 1, value: v2 });
        properties.add(
            Property::Scale,
            Scale {
                value: Scalar::new(
                    Self::DEFAULT_SCALE,
                    Self::MIN_SCALE,
                    Self::MAX_SCALE,
                    Self::SCALE_STEP,
                ),
            },
        );

        GeneralShape::new(
            ShapeType::ConstrLine,
            vs,
            properties,
            order,
            None,
            None,
            None,
            None,
            Operation::Union,
            None,
        )
    }
    pub fn new_shape_constr_circle(v1: Vec2, v2: Vec2, order: i32) -> Option<Self> {
        if v1 == v2 {
            return None;
        }

        let mut properties = Properties::new();
        use PropertyValue::*;
        properties.add(Property::Center, Center { idx: 0, value: v1 });
        properties.add(
            Property::Radius,
            Radius {
                idx: 1,
                value: Scalar::new((v2 - v1).hypot(), Self::MIN_RADIUS, f64::INFINITY, 1.0),
            },
        );
        properties.add(
            Property::Magnets,
            Magnets {
                value: Scalar::new(GeneralShape::DEFAULT_MAGNETS, 3, 20, 1),
            },
        );
        properties.add(
            Property::Scale,
            Scale {
                value: Scalar::new(1.0, 0.01, 1000.0, 0.1),
            },
        );

        let vs_magnets = get_magnets_vertices(v1, v2, GeneralShape::DEFAULT_MAGNETS);
        let mut vs = vec![v1, v2];
        vs.extend(vs_magnets);
        let vs: Vec<Vertex> = vs.into_iter().map(Vertex::new).collect();

        GeneralShape::new(
            ShapeType::ConstrCircle,
            vs,
            properties,
            order,
            None,
            None,
            None,
            None,
            Operation::Union,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        shape_type: ShapeType,
        vs: Vec<Vertex>,
        properties: Properties,
        order: i32,
        text_shape: Option<TextData>,
        svg_shape: Option<SvgData>,
        voronoi_shape: Option<SvgData>,
        group_shape: Option<GroupShape>,
        operation: Operation,
        shape_name: Option<String>,
    ) -> Option<Self> {
        let vs: Vec<Vertex> = vs.to_vec();
        if vs.is_empty() {
            return None;
        }
        if shape_type == ShapeType::Arrow {
            return None;
        }

        let vertices = &vs
            .iter()
            .map(|v| (VUId::new(), v.clone()))
            .collect::<Vec<_>>()[..];

        let mut shape = GeneralShape {
            shape_type,
            vertices: VecRing::from_slice(vertices).unwrap(),
            order,
            operation,
            shape_name,
            properties,

            text_shape_data: text_shape,
            svg_shape_data: svg_shape,
            voronoi_shape_data: voronoi_shape,
            group_shape_data: group_shape,

            bezpath: BezPath::new(),
            polygon: MultiPolygon::new(vec![]),

            rotation: ShapeRotation::new(),
        };
        shape.set_bezpath();
        Some(shape)
    }

    pub fn get_vertex(&self, value_uid: &VUId) -> Option<&Vertex> {
        self.vertices.iter().find_map(
            |(uid, value)| {
                if uid == value_uid {
                    Some(value)
                } else {
                    None
                }
            },
        )
    }
    pub fn get_vertex_mut(&mut self, value_uid: &VUId) -> Option<&mut Vertex> {
        self.vertices.iter_mut().find_map(
            |(uid, value)| {
                if uid == value_uid {
                    Some(value)
                } else {
                    None
                }
            },
        )
    }
    pub fn get_vertices(&self) -> &VecRing<VUId> {
        &self.vertices
    }
    pub fn get_vertices_mut(&mut self) -> &mut VecRing<VUId> {
        &mut self.vertices
    }
    pub fn select_vertex(&mut self, draw_pos: Vec2) -> Option<VUId> {
        for (idx, (uid, value)) in self.vertices.iter().enumerate() {
            if matches!(self.shape_type, ShapeType::ConstrCircle) && idx >= 2 {
                continue;
            }
            let pos = self.vertex_display_pos(value.curr());
            if (pos - draw_pos).hypot() < Self::GRAB_RADIUS {
                return Some(*uid);
            }
        }
        None
    }
    pub fn highlight_vertex(&mut self, draw_pos: Vec2) -> Option<VUId> {
        for (idx, (uid, value)) in self.vertices.iter().enumerate() {
            if matches!(self.shape_type, ShapeType::ConstrCircle) && idx >= 2 {
                continue;
            }
            let pos = self.vertex_display_pos(value.curr());
            if (pos - draw_pos).hypot() < Self::GRAB_RADIUS {
                return Some(*uid);
            }
        }
        None
    }
    fn move_vertex(&mut self, value_uid: VUId, user_ui: &UserUI) -> bool {
        let snap = user_ui.snap;
        let mut delta = user_ui.pointer.curr() - user_ui.pointer.saved();
        if !user_ui.magnetized {
            delta = (delta / snap.linear()).round() * snap.linear();
        }
        match self.shape_type {
            ShapeType::Disc | ShapeType::ConstrCircle => {
                if self.vertices.len() < 2 {
                    return false;
                }
                if let ShapeType::Disc = self.shape_type {
                    if self.vertices.len() != 2 {
                        return false;
                    }
                }

                // The first vertex is the center
                // The second vertex is the radius
                if self.vertices.key(0) == &value_uid {
                    if let Some(seg) =
                        SegBundle::new(self.vertices.val(0).saved(), self.vertices.val(1).saved())
                    {
                        // Snap the saved center
                        let snap_c = snap_vertex(seg.s, snap);
                        // Snap the radius relative to the saved center (keep same radius)
                        let snap_r = seg.e + snap_c - seg.s;
                        // Snap the angle
                        let r = (snap_r - snap_c).hypot();
                        let a = snap_angle((snap_r - snap_c).atan2(), snap);

                        self.vertices.val_mut(0).set_saved(snap_c);
                        self.vertices
                            .val_mut(1)
                            .set_saved(snap_c + Vec2::new(r * a.cos(), r * a.sin()));

                        // Then move all
                        self.vertices.val_mut(0).add(delta);
                        self.vertices.val_mut(1).add(delta);
                        self.set_bezpath();
                        true
                    } else {
                        self.vertices.val_mut(0).add(delta);
                        self.set_bezpath();
                        true
                    }
                } else if self.vertices.key(1) == &value_uid {
                    let try_curr = self.vertices.val(1).saved() + delta;
                    if (try_curr - self.vertices.val(0).curr()).hypot() < 2.0 {
                        log!("Radius too small");
                        return false;
                    }
                    self.vertices.val_mut(1).add(delta);
                    if let Some(seg) =
                        SegBundle::new(self.vertices.val(0).saved(), self.vertices.val(1).curr())
                    {
                        let r = snap_val(seg.len, snap);
                        let a = snap_angle(seg.a, snap);
                        self.vertices
                            .val_mut(1)
                            .set_curr(seg.s + Vec2::new(r * a.cos(), r * a.sin()));
                    }
                    self.set_bezpath();
                    true
                } else {
                    false
                }
            }
            ShapeType::ConstrLine => {
                if self.vertices.len() != 2 {
                    return false;
                }
                let idx = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                    Some(i) => i,
                    None => return false,
                };
                self.vertices.val_mut(idx as i64).add(delta);

                self.set_bezpath();
                true
            }
            ShapeType::Oblong => {
                if self.vertices.len() != 4 {
                    return false;
                }
                log!("Moving oblong vertex {:?}", value_uid);
                let idx = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                    Some(i) => i,
                    None => return false,
                };
                if idx == 0 || idx == 1 {
                    let radius_idx = if idx == 0 { 2 } else { 3 };
                    let saved_center = self.vertices.val(idx as i64).saved();
                    if user_ui.keys_states.shift_pressed {
                        self.vertices.val_mut(idx as i64).add(delta);
                        let other_idx = if idx == 0 { 1 } else { 0 };
                        if let Some(seg) = SegBundle::new(
                            self.vertices.val(other_idx).saved(),
                            self.vertices.val(idx as i64).curr(),
                        ) {
                            let r = snap_val(seg.len, snap);
                            let a = snap_angle(seg.a, snap);
                            self.vertices
                                .val_mut(idx as i64)
                                .set_curr(seg.e - Vec2::new(r * a.cos(), r * a.sin()));
                        }
                    } else {
                        let tmp = self.vertices.val(idx as i64).clone();
                        self.vertices
                            .val_mut(idx as i64)
                            .set_saved(snap_vertex(tmp.saved(), snap));
                        self.vertices.val_mut(idx as i64).add(delta);
                        let tmp = self.vertices.val(idx as i64).clone();
                        self.vertices
                            .val_mut(idx as i64)
                            .set_curr(snap_vertex(tmp.curr(), snap));
                    }
                    let center_delta = self.vertices.val(idx as i64).curr() - saved_center;
                    if center_delta.hypot() > EPSILON {
                        self.vertices.val_mut(radius_idx as i64).add(center_delta);
                    }
                } else {
                    let tmp = self.vertices.val(idx as i64).clone();
                    self.vertices
                        .val_mut(idx as i64)
                        .set_saved(snap_vertex(tmp.saved(), snap));
                    self.vertices.val_mut(idx as i64).add(delta);
                    let tmp = self.vertices.val(idx as i64).clone();
                    self.vertices
                        .val_mut(idx as i64)
                        .set_curr(snap_vertex(tmp.curr(), snap));
                }
                self.set_bezpath();
                true
            }
            ShapeType::Square | ShapeType::Text | ShapeType::Svg | ShapeType::Voronoi => {
                let len = self.vertices.len();
                if len != 4 {
                    return false;
                }
                let center = self.bbox_center_saved();
                if user_ui.keys_states.shift_pressed {
                    let start = user_ui.draw_pos_down() - center;
                    let curr = user_ui.draw_pos() - center;
                    if start.hypot() < EPSILON || curr.hypot() < EPSILON {
                        return false;
                    }
                    let delta_a = angle_from(start, curr);
                    self.rotation
                        .set_curr(snap_angle(self.rotation.get_saved() + delta_a, snap));
                    log!(
                        "Rotation: {:.2}",
                        self.rotation.get() / std::f64::consts::PI * 180.
                    );
                    self.set_bezpath();
                    return true;
                }
                if let Some(text_data) = &mut self.text_shape_data {
                    text_data.invalidate_cache();
                }
                if let Some(svg_shape_data) = &mut self.svg_shape_data {
                    svg_shape_data.invalidate_cache();
                }
                if let Some(voronoi_shape_data) = &mut self.voronoi_shape_data {
                    voronoi_shape_data.invalidate_cache();
                }

                let i = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                    Some(i) => i as i64,
                    None => return false,
                };
                let rotation = self.rotation.get();
                if rotation.abs() > EPSILON {
                    log!("Rotation: {:.2}", rotation / std::f64::consts::PI * 180.);
                    let opp_idx = (i + 2).rem_euclid(4) as usize;
                    let pivot = self.vertices.val(opp_idx as i64).saved();
                    let saved = rotate_vector(user_ui.pointer.saved() - pivot, -rotation) + pivot;
                    let curr = rotate_vector(user_ui.pointer.curr() - pivot, -rotation) + pivot;
                    let mut local_delta = curr - saved;
                    if !user_ui.magnetized {
                        local_delta = (local_delta / snap.linear()).round() * snap.linear();
                    }
                    let mut local_saved: [Vec2; 4] = [Vec2::ZERO; 4];
                    let mut local_curr: [Vec2; 4] = [Vec2::ZERO; 4];
                    for idx in 0..4 {
                        let pos = self.vertices.val(idx as i64).saved();
                        let local = rotate_vector(pos - pivot, -rotation) + pivot;
                        local_saved[idx] = local;
                        local_curr[idx] = local;
                    }
                    let i_usize = i.rem_euclid(4) as usize;
                    local_curr[i_usize] = local_saved[i_usize] + local_delta;
                    let tmp = local_saved[i_usize];
                    if i % 2 == 1 {
                        let left = (i - 1).rem_euclid(4) as usize;
                        let right = (i + 1).rem_euclid(4) as usize;
                        local_saved[left].x = tmp.x;
                        local_curr[left] = local_saved[left] + Vec2::new(local_delta.x, 0.);
                        local_saved[right].y = tmp.y;
                        local_curr[right] = local_saved[right] + Vec2::new(0., local_delta.y);
                    } else {
                        let left = (i - 1).rem_euclid(4) as usize;
                        let right = (i + 1).rem_euclid(4) as usize;
                        local_saved[left].y = tmp.y;
                        local_curr[left] = local_saved[left] + Vec2::new(0., local_delta.y);
                        local_saved[right].x = tmp.x;
                        local_curr[right] = local_saved[right] + Vec2::new(local_delta.x, 0.);
                    }
                    for idx in 0..4 {
                        let curr = rotate_vector(local_curr[idx] - pivot, rotation) + pivot;
                        self.vertices.val_mut(idx as i64).set_curr(curr);
                    }
                    self.set_bezpath();
                    return true;
                }

                let mut local_delta = user_ui.pointer.curr() - user_ui.pointer.saved();
                if !user_ui.magnetized {
                    local_delta = (local_delta / snap.linear()).round() * snap.linear();
                }
                if !user_ui.magnetized {
                    let tmp = self.vertices.val(i).saved();
                    self.vertices.val_mut(i).set_saved(snap_vertex(tmp, snap));
                }
                self.vertices.val_mut(i).add(local_delta);
                let tmp = self.vertices.val_mut(i).saved();
                if i % 2 == 1 {
                    self.vertices.val_mut(i - 1).set_saved_x(tmp.x);
                    self.vertices
                        .val_mut(i - 1)
                        .add(Vec2::new(local_delta.x, 0.));
                    self.vertices.val_mut(i + 1).set_saved_y(tmp.y);
                    self.vertices
                        .val_mut(i + 1)
                        .add(Vec2::new(0., local_delta.y));
                } else {
                    self.vertices.val_mut(i - 1).set_saved_y(tmp.y);
                    self.vertices
                        .val_mut(i - 1)
                        .add(Vec2::new(0., local_delta.y));
                    self.vertices.val_mut(i + 1).set_saved_x(tmp.x);
                    self.vertices
                        .val_mut(i + 1)
                        .add(Vec2::new(local_delta.x, 0.));
                }
                self.set_bezpath();
                true
            }
            ShapeType::Poly => {
                let idx = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                    Some(i) => i as i64,
                    None => return false,
                };
                let tmp = self.vertices.val_mut(idx).saved();
                self.vertices.val_mut(idx).set_saved(snap_vertex(tmp, snap));
                self.vertices.val_mut(idx).add(delta);
                self.set_bezpath();
                true
            }
            ShapeType::Group => false,

            ShapeType::Arrow => {
                // Arrow is not a closed shape, so we don't move it
                false
            }
        }
    }
    pub fn move_vertex_by_mouse(&mut self, value_uid: VUId, user_ui: &UserUI) -> Option<()> {
        if self.move_vertex(value_uid, user_ui) {
            self.update_properties();
        }
        None
    }

    pub fn move_vertex_by_props(&mut self, prop: &Property, mut user_ui: UserUI) -> Option<()> {
        let prop_val = self.properties.get(prop)?.clone();
        use PropertyValue::*;
        match prop_val {
            Center { idx, value, .. }
            | BottomLeft { idx, value, .. }
            | TopLeft { idx, value, .. }
            | TopRight { idx, value, .. }
            | BottomRight { idx, value, .. }
            | Pt1 { idx, value, .. }
            | Pt2 { idx, value, .. }
            | Apex { idx, value, .. } => {
                let vertex_uid = *self.vertices.key(idx as i64);
                let current = self.vertices.val(idx as i64).curr();
                self.save_vertices_positions();
                user_ui.magnetized = true;
                user_ui.pointer.set_saved(current);
                user_ui.pointer.set_curr(value);
                log!("Moved vertex {:?} to {:?}", vertex_uid, value);
                if self.move_vertex(vertex_uid, &user_ui) {
                    self.update_properties();
                }
                Some(())
            }
            _ => None,
        }
    }
    pub fn move_shape(&mut self, delta: Vec2) {
        for (_, value) in self.vertices.iter_mut() {
            value.add(delta);
        }
        self.update_properties();
        self.set_bezpath();
    }

    pub fn update_from_property(&mut self, prop: &Property) -> Option<()> {
        let prop_val = self.properties.get(prop)?.clone();
        use PropertyValue::*;
        match prop_val {
            Radius { idx, value } => {
                let center = self
                    .properties
                    .get(&Property::Center)
                    .and_then(PropertyValue::as_vec2)
                    .unwrap_or_else(|| self.vertices.val(0).curr());
                let current = self.vertices.val(idx as i64).curr();
                let mut dir = current - center;
                if dir.hypot() < EPSILON {
                    dir = Vec2::new(1.0, 0.0);
                } else {
                    dir = dir.normalize();
                }
                let new_pos = center + dir * value.curr();
                let vertex = self.vertices.val_mut(idx as i64);
                vertex.set_curr(new_pos);
                vertex.set_saved(new_pos);
                if matches!(self.shape_type, ShapeType::ConstrCircle) {
                    let count = self
                        .properties
                        .get(&Property::Magnets)
                        .map(|value| match value {
                            PropertyValue::Magnets { value } => value.curr(),
                            _ => 0,
                        })
                        .unwrap_or(0);
                    if count > 0 {
                        if self.vertices.len() != 2 + count {
                            self.set_magnets_number(count);
                        } else {
                            let v0 = self.vertices.val(0).curr();
                            let v1 = self.vertices.val(1).curr();
                            let magnets = get_magnets_vertices(v0, v1, count);
                            for (i, pos) in magnets.iter().enumerate() {
                                let idx = 2 + i as i64;
                                let vertex = self.vertices.val_mut(idx);
                                vertex.set_curr(*pos);
                                vertex.set_saved(*pos);
                            }
                        }
                    }
                }
            }
            Magnets { value } => {
                if matches!(self.shape_type, ShapeType::ConstrCircle) {
                    self.set_magnets_number(value.curr());
                }
            }
            Seeds { .. } | VoronoiGap { .. } | VoronoiRelaxation { .. } => {
                if matches!(self.shape_type, ShapeType::Voronoi) {
                    self.rebuild_voronoi_from_properties();
                }
            }
            _ => {}
        }
        self.update_properties();
        self.set_bezpath();
        Some(())
    }
    fn rebuild_voronoi_from_properties(&mut self) {
        let mut min = Vec2::new(f64::INFINITY, f64::INFINITY);
        let mut max = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (_, vertex) in self.vertices.iter() {
            let pos = vertex.curr();
            min.x = min.x.min(pos.x);
            min.y = min.y.min(pos.y);
            max.x = max.x.max(pos.x);
            max.y = max.y.max(pos.y);
        }
        if !min.x.is_finite() || !min.y.is_finite() || !max.x.is_finite() || !max.y.is_finite() {
            return;
        }
        let seeds = self
            .properties
            .get(&Property::Seeds)
            .and_then(|value| match value {
                PropertyValue::Seeds { value } => Some(value.curr() as usize),
                _ => None,
            })
            .unwrap_or(Self::DEFAULT_SEEDS);
        let gap = self
            .properties
            .get(&Property::VoronoiGap)
            .and_then(|value| match value {
                PropertyValue::VoronoiGap { value } => Some(value.curr()),
                _ => None,
            })
            .unwrap_or(Self::DEFAULT_VORONOI_GAP);
        let relaxation = self
            .properties
            .get(&Property::VoronoiRelaxation)
            .and_then(|value| match value {
                PropertyValue::VoronoiRelaxation { value } => Some(value.curr() as usize),
                _ => None,
            })
            .unwrap_or(Self::DEFAULT_VORONOI_RELAXATION);
        let rings = build_voronoi_rings(min, max, seeds, gap, relaxation);
        self.voronoi_shape_data = Some(voronoi_rings_to_svg_data(rings));
    }
    pub fn save_vertices_positions(&mut self) {
        for (_, value) in self.vertices.iter_mut() {
            value.save();
        }
        self.rotation.save();
    }
    pub fn get_properties(&self) -> &Properties {
        &self.properties
    }
    pub fn get_properties_mut(&mut self) -> &mut Properties {
        &mut self.properties
    }
    pub fn update_properties(&mut self) {
        use PropertyValue::*;
        for (_, prop_val) in self.properties.iter_mut() {
            match prop_val {
                Center { idx, value } => {
                    let vertex = self.vertices.val_mut(*idx as i64);
                    *value = vertex.curr();
                }
                Radius { idx, value } => {
                    let vertex = self.vertices.val_mut(*idx as i64);
                    *value = Scalar::new(
                        (vertex.curr() - self.vertices.val(0).curr()).hypot(),
                        Self::MIN_RADIUS,
                        f64::INFINITY,
                        1.0,
                    );
                }
                BottomLeft { idx, value, .. } => {
                    let vertex = self.vertices.val_mut(*idx as i64);
                    *value = vertex.curr();
                }
                TopLeft { idx, value, .. } => {
                    let vertex = self.vertices.val_mut(*idx as i64);
                    *value = vertex.curr();
                }
                TopRight { idx, value, .. } => {
                    let vertex = self.vertices.val_mut(*idx as i64);
                    *value = vertex.curr();
                }
                BottomRight { idx, value, .. } => {
                    let vertex = self.vertices.val_mut(*idx as i64);
                    *value = vertex.curr();
                }
                Pt1 { idx, value } => {
                    let vertex = self.vertices.val_mut(*idx as i64);
                    *value = vertex.curr();
                }
                Pt2 { idx, value } => {
                    let vertex = self.vertices.val_mut(*idx as i64);
                    *value = vertex.curr();
                }
                Apex { idx, value, .. } => {
                    let vertex = self.vertices.val_mut(*idx as i64);
                    *value = vertex.curr();
                }
                _ => {}
            }
        }

        if matches!(self.shape_type, ShapeType::ConstrCircle) {
            let Some(PropertyValue::Magnets { value, .. }) =
                self.properties.get(&Property::Magnets)
            else {
                return;
            };
            let count = value.curr();
            if self.vertices.len() != 2 + count {
                self.set_magnets_number(count);
                return;
            }
            let v0 = self.vertices.val(0).curr();
            let v1 = self.vertices.val(1).curr();
            let magnets = get_magnets_vertices(v0, v1, count);
            for (i, pos) in magnets.iter().enumerate() {
                let idx = 2 + i as i64;
                let vertex = self.vertices.val_mut(idx);
                vertex.set_curr(*pos);
                vertex.set_saved(*pos);
            }
        }
    }

    pub fn contains(&self, pos: Vec2) -> bool {
        if let ShapeType::ConstrLine = self.shape_type {
            let mut iter = self.vertices.iter();
            let Some((_, v0)) = iter.next() else {
                return false;
            };
            let Some((_, v1)) = iter.next() else {
                return false;
            };
            return distance_to_segment(v0.curr(), v1.curr(), pos, Self::GRAB_RADIUS)
                <= Self::GRAB_RADIUS;
        }
        if let ShapeType::ConstrCircle = self.shape_type {
            let mut iter = self.vertices.iter();
            let Some((_, center)) = iter.next() else {
                return false;
            };
            let Some((_, edge)) = iter.next() else {
                return false;
            };
            let radius = (edge.curr() - center.curr()).hypot();
            if radius < EPSILON {
                return false;
            }
            let distance = (pos - center.curr()).hypot();
            let tolerance = Self::GRAB_RADIUS * 2.0;
            return (distance - radius).abs() <= tolerance;
        }
        if matches!(
            self.shape_type,
            ShapeType::Text
                | ShapeType::Svg
                | ShapeType::Voronoi
                | ShapeType::Square
                | ShapeType::Group
        ) {
            if self.rotation.get().abs() > EPSILON {
                return self.get_bezpath_rotated().contains(pos.to_point());
            }
            let bbox = self.bezpath.bounding_box();
            return bbox.contains(pos.to_point());
        }
        self.get_bezpath().contains(pos.to_point())
    }
    pub fn get_shape_type(&self) -> ShapeType {
        self.shape_type
    }
    pub fn get_operation(&self) -> Operation {
        self.operation
    }
    pub fn get_order(&self) -> i32 {
        self.order
    }
    pub fn set_order(&mut self, order: i32) {
        self.order = order;
    }
    pub fn get_name(&self) -> Option<&str> {
        self.shape_name.as_deref()
    }
    pub fn set_name(&mut self, name: Option<String>) {
        let trimmed = name.and_then(|value| {
            let trimmed = value.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        });
        self.shape_name = trimmed;
    }
    pub fn get_polygon(&self) -> &MultiPolygon<f64> {
        &self.polygon
    }
    pub fn get_bezpath(&self) -> &BezPath {
        &self.bezpath
    }
    pub fn is_group(&self) -> bool {
        matches!(self.shape_type, ShapeType::Group)
    }
    pub fn get_group_children(&self) -> Option<&[EUId]> {
        self.group_shape_data.as_ref().map(GroupShape::children)
    }
    pub fn set_group_children(&mut self, children: Vec<EUId>) {
        if let Some(group) = self.group_shape_data.as_mut() {
            group.set_children(children);
        }
    }

    pub fn get_text(&self) -> Option<String> {
        if let Some(PropertyValue::Text { value }) = self.properties.get(&Property::Text) {
            return Some(value.clone());
        }
        None
    }
    pub fn set_text(&mut self, text: String) {
        let Some(PropertyValue::Text { value: texte }) = self.properties.get_mut(&Property::Text)
        else {
            return;
        };
        *texte = text.clone();
        self.set_bezpath();
    }

    pub fn get_svg(&self) -> Option<&SvgData> {
        if let Some(svg_data) = &self.svg_shape_data {
            return Some(svg_data);
        }
        if let Some(svg_data) = &self.voronoi_shape_data {
            return Some(svg_data);
        }
        None
    }
    pub fn get_svg_paths(&self) -> Option<&Vec<BezPath>> {
        if let Some(data) = &self.svg_shape_data {
            return data.cached_paths.as_ref();
        }
        self.voronoi_shape_data
            .as_ref()
            .and_then(|data| data.cached_paths.as_ref())
    }

    pub fn get_text_paths(&self) -> Option<&Vec<BezPath>> {
        self.text_shape_data
            .as_ref()
            .and_then(|data| data.cached_paths.as_ref())
    }

    pub fn get_magnets_number(&self) -> Option<usize> {
        if let Some(PropertyValue::Magnets { value, .. }) = self.properties.get(&Property::Magnets)
        {
            return Some(value.curr());
        }
        None
    }
    pub fn set_magnets_number(&mut self, count: usize) {
        if !matches!(self.shape_type, ShapeType::ConstrCircle) {
            return;
        }
        let count = count.max(GeneralShape::MIN_MAGNETS);
        if let Some(PropertyValue::Magnets { value, .. }) =
            self.properties.get_mut(&Property::Magnets)
        {
            value.set_curr(count);

            // Update the pool of vertices
            let mut vs = vec![self.vertices.val(0).curr(), self.vertices.val(1).curr()];
            let vs_magnets = get_magnets_vertices(vs[0], vs[1], count);
            vs.extend(vs_magnets);
            let vertices = &vs
                .iter()
                .map(|v| (VUId::new(), Vertex::new(*v)))
                .collect::<Vec<_>>()[..];
            self.vertices = VecRing::from_slice(vertices).unwrap();

            self.set_bezpath();
        }
    }

    pub fn vertex_display_pos(&self, pos: Vec2) -> Vec2 {
        if matches!(
            self.shape_type,
            ShapeType::Square | ShapeType::Text | ShapeType::Svg | ShapeType::Voronoi
        ) && self.rotation.get().abs() > EPSILON
        {
            let center = self.bbox_center();
            return rotate_vector(pos - center, self.rotation.get()) + center;
        }
        pos
    }

    pub(crate) fn get_bezpath_rotated(&self) -> BezPath {
        let rotation = self.rotation.get();
        if rotation.abs() <= EPSILON
            || !matches!(
                self.shape_type,
                ShapeType::Square
                    | ShapeType::Text
                    | ShapeType::Svg
                    | ShapeType::Voronoi
                    | ShapeType::Group
            )
        {
            return self.bezpath.clone();
        }
        let bbox = self.bezpath.bounding_box();
        let center = Vec2::new((bbox.x0 + bbox.x1) * 0.5, (bbox.y0 + bbox.y1) * 0.5);
        _rotate_bezpath(&self.bezpath, center, rotation)
    }

    pub(crate) fn get_text_paths_rotated(&self) -> Option<Vec<BezPath>> {
        let paths = self.get_text_paths()?.clone();
        let rotation = self.rotation.get();
        if rotation.abs() <= EPSILON {
            return Some(paths);
        }
        let bbox = self.bezpath.bounding_box();
        let center = Vec2::new((bbox.x0 + bbox.x1) * 0.5, (bbox.y0 + bbox.y1) * 0.5);
        Some(_rotate_bezpaths(&paths, center, rotation))
    }

    pub(crate) fn get_svg_paths_rotated(&self) -> Option<Vec<BezPath>> {
        let paths = self.get_svg_paths()?.clone();
        let rotation = self.rotation.get();
        if rotation.abs() <= EPSILON {
            return Some(paths);
        }
        let bbox = self.bezpath.bounding_box();
        let center = Vec2::new((bbox.x0 + bbox.x1) * 0.5, (bbox.y0 + bbox.y1) * 0.5);
        Some(_rotate_bezpaths(&paths, center, rotation))
    }
    pub fn get_rotation(&self) -> f64 {
        self.rotation.get()
    }
    pub fn set_rotation(&mut self, rotation: f64) {
        self.rotation.set(rotation);
        self.rotation.save();
        self.set_bezpath();
    }

    pub fn set_bezpath(&mut self) {
        let mut bezpath_only = true;
        match self.shape_type {
            ShapeType::Disc | ShapeType::ConstrCircle => {
                let center = self.vertices.val(0).curr();
                let radius = (self.vertices.val(1).curr() - center).hypot();
                self.bezpath =
                    kurbo::Circle::new(center.to_point(), radius).to_path(Self::TOLERANCE);
            }
            ShapeType::ConstrLine => {
                let p1 = self.vertices.val(0).curr();
                let p2 = self.vertices.val(1).curr();
                let mut path = BezPath::new();
                path.move_to(p1.to_point());
                path.line_to(p2.to_point());
                self.bezpath = path;
            }
            ShapeType::Oblong => {
                let mut path = BezPath::new();
                if self.vertices.len() >= 4 {
                    let c1 = self.vertices.val(0).curr();
                    let c2 = self.vertices.val(1).curr();
                    let r1 = (self.vertices.val(2).curr() - c1).hypot();
                    let r2 = (self.vertices.val(3).curr() - c2).hypot();
                    let d = (c2 - c1).hypot();

                    let (center, radius) = if r1 >= r2 { (c1, r1) } else { (c2, r2) };
                    if d < EPSILON || d <= (r1 - r2).abs() {
                        path.extend(
                            Circle::new(center.to_point(), radius).path_elements(Self::TOLERANCE),
                        );
                    } else {
                        let base = (c2 - c1).atan2();
                        let ratio = ((r1 - r2) / d).clamp(-1.0, 1.0);
                        let offset = ratio.acos();
                        let sweep1 = 2.0 * (PI - offset);
                        let sweep2 = 2.0 * offset;
                        let a1 = base + offset;
                        let a2 = base - offset;

                        let t1_a = c1 + Vec2::new(r1 * a1.cos(), r1 * a1.sin());
                        let t2_b = c2 + Vec2::new(r2 * a2.cos(), r2 * a2.sin());

                        path.extend(
                            Arc::new(c1.to_point(), Vec2::new(r1, r1), a1, sweep1, 0.0)
                                .path_elements(Self::TOLERANCE),
                        );
                        path.push(PathEl::LineTo(t2_b.to_point()));
                        let mut arc2 = Arc::new(c2.to_point(), Vec2::new(r2, r2), a2, sweep2, 0.0)
                            .path_elements(Self::TOLERANCE);
                        arc2.next();
                        path.extend(arc2);
                        path.push(PathEl::LineTo(t1_a.to_point()));
                        path.push(PathEl::ClosePath);
                    }
                } else if self.vertices.len() == 3 {
                    let e1 = self.vertices.val(0).curr();
                    let e2 = self.vertices.val(1).curr();
                    let side = self.vertices.val(2).curr();
                    let m = (e1 + e2) * 0.5;
                    let radius = (side - m).hypot();
                    let angle = (e2 - e1).atan2();
                    let mut dir = e2 - e1;
                    if dir.hypot() >= EPSILON {
                        dir = dir.normalize();
                        let perp = Vec2::new(-dir.y, dir.x);
                        let pt2 = e1 - perp * radius;
                        let pt3 = e2 + perp * radius;
                        path.extend(
                            Arc::new(
                                e1.to_point(),
                                Vec2::new(radius, radius),
                                3.0 * PI / 2.0,
                                -PI,
                                angle,
                            )
                            .path_elements(Self::TOLERANCE),
                        );
                        path.push(PathEl::LineTo(pt3.to_point()));
                        let mut arc2 = Arc::new(
                            e2.to_point(),
                            Vec2::new(radius, radius),
                            PI / 2.0,
                            -PI,
                            angle,
                        )
                        .path_elements(Self::TOLERANCE);
                        arc2.next();
                        path.extend(arc2);
                        path.push(PathEl::LineTo(pt2.to_point()));
                        path.push(PathEl::ClosePath);
                    } else {
                        path.extend(
                            Circle::new(e2.to_point(), radius).path_elements(Self::TOLERANCE),
                        );
                    }
                }
                self.bezpath = path;
            }
            ShapeType::Square | ShapeType::Group => {
                let apices = self.vertices.get_apices();
                self.bezpath = bezpath_from_apices(&apices);
            }
            ShapeType::Poly => {
                let apices = self.vertices.get_apices();
                self.bezpath = bezpath_from_apices(&apices);
            }
            ShapeType::Text => {
                let apices = self.vertices.get_apices();
                self.bezpath = bezpath_from_apices(&apices);
                self.update_text_polygon();
                bezpath_only = false;
            }
            ShapeType::Svg | ShapeType::Voronoi => {
                let apices = self.vertices.get_apices();
                self.bezpath = bezpath_from_apices(&apices);
                self.update_svg_polygon();
                bezpath_only = false;
            }
            ShapeType::Arrow => return,
        }

        if bezpath_only {
            self.update_polygon();
        }
    }

    fn bbox_center(&self) -> Vec2 {
        let mut min = Vec2::new(f64::INFINITY, f64::INFINITY);
        let mut max = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (_, value) in self.vertices.iter() {
            min.x = min.x.min(value.curr().x);
            min.y = min.y.min(value.curr().y);
            max.x = max.x.max(value.curr().x);
            max.y = max.y.max(value.curr().y);
        }
        if !min.x.is_finite() || !min.y.is_finite() || !max.x.is_finite() || !max.y.is_finite() {
            return Vec2::ZERO;
        }
        (min + max) * 0.5
    }

    pub(crate) fn rotate_polygon_if_needed(
        &self,
        polygon: &MultiPolygon<f64>,
    ) -> MultiPolygon<f64> {
        let rotation = self.rotation.get();
        if rotation.abs() <= EPSILON
            || !matches!(
                self.shape_type,
                ShapeType::Square
                    | ShapeType::Text
                    | ShapeType::Svg
                    | ShapeType::Voronoi
                    | ShapeType::Group
            )
        {
            return polygon.clone();
        }
        let bbox = self.bezpath.bounding_box();
        let center = Vec2::new((bbox.x0 + bbox.x1) * 0.5, (bbox.y0 + bbox.y1) * 0.5);
        _rotate_multipolygon(polygon, center, rotation)
    }
    fn bbox_center_saved(&self) -> Vec2 {
        let mut min = Vec2::new(f64::INFINITY, f64::INFINITY);
        let mut max = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (_, value) in self.vertices.iter() {
            min.x = min.x.min(value.saved().x);
            min.y = min.y.min(value.saved().y);
            max.x = max.x.max(value.saved().x);
            max.y = max.y.max(value.saved().y);
        }
        if !min.x.is_finite() || !min.y.is_finite() || !max.x.is_finite() || !max.y.is_finite() {
            return Vec2::ZERO;
        }
        (min + max) * 0.5
    }

    fn update_polygon(&mut self) {
        let poly = bez_path_to_geo_polygon(&self.bezpath);
        self.polygon = MultiPolygon::new(vec![poly]);
    }

    fn update_text_polygon(&mut self) -> Option<()> {
        self.polygon = MultiPolygon::new(vec![]);

        let mut min = Vec2::new(f64::INFINITY, f64::INFINITY);
        let mut max = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (_, value) in self.vertices.iter() {
            let curr = value.curr();
            min.x = min.x.min(curr.x);
            min.y = min.y.min(curr.y);
            max.x = max.x.max(curr.x);
            max.y = max.y.max(curr.y);
        }
        if !min.x.is_finite() || !min.y.is_finite() || !max.x.is_finite() || !max.y.is_finite() {
            return None;
        }
        let v1 = min;
        let v2 = max;

        let text = self.get_text()?;
        let text_data = self.text_shape_data.as_mut()?;
        let font = self.properties.get(&Property::Font)?.as_font()?;

        let (poly, _scale) = text_to_multipolygon(&text, font, v1, v2);
        self.polygon = poly.clone();
        text_data.polygon = Some(poly.clone());
        text_data.cached_paths = Some(geo_multipolygon_to_bez_paths(&poly));
        Some(())
    }

    fn update_svg_polygon(&mut self) {
        let svg = match self.shape_type {
            ShapeType::Svg => self.svg_shape_data.as_mut(),
            ShapeType::Voronoi => self.voronoi_shape_data.as_mut(),
            _ => None,
        };
        let Some(svg) = svg else {
            self.polygon = MultiPolygon::new(vec![]);
            return;
        };
        if self.vertices.len() < 4 || self.bezpath.is_empty() || svg.rings.is_empty() {
            self.polygon = MultiPolygon::new(vec![]);
            svg.invalidate_cache();
            return;
        }
        let bbox = self.bezpath.bounding_box();
        if bbox.is_zero_area() {
            self.polygon = MultiPolygon::new(vec![]);
            svg.invalidate_cache();
            return;
        }
        let bbox_min = Vec2::new(bbox.x0, bbox.y0);
        let bbox_max = Vec2::new(bbox.x1, bbox.y1);

        if let Some(cached) = svg.cached_polygon.as_ref() {
            if svg.cached_bbox_min == bbox_min && svg.cached_bbox_max == bbox_max {
                self.polygon = cached.clone();
                return;
            }
            let cached_size = svg.cached_bbox_max - svg.cached_bbox_min;
            let bbox_size = bbox_max - bbox_min;
            if cached_size == bbox_size {
                let dx = bbox_min.x - svg.cached_bbox_min.x;
                let dy = bbox_min.y - svg.cached_bbox_min.y;
                let translated = cached.translate(dx, dy);
                self.polygon = translated.clone();
                svg.cached_polygon = Some(translated);
                if let Some(paths) = svg.cached_paths_raw.as_ref() {
                    let translated_paths = translate_bezpaths(paths, dx, dy);
                    svg.cached_paths_raw = Some(translated_paths.clone());
                    svg.cached_paths = Some(translated_paths);
                }
                svg.cached_bbox_min = bbox_min;
                svg.cached_bbox_max = bbox_max;
                return;
            }
        }

        let orig_min = svg.original_min;
        let orig_max = svg.original_max;
        let orig_w = (orig_max.x - orig_min.x).max(1e-6);
        let orig_h = (orig_max.y - orig_min.y).max(1e-6);
        let bbox_w = (bbox_max.x - bbox_min.x).max(1e-6);
        let bbox_h = (bbox_max.y - bbox_min.y).max(1e-6);
        let scale = (bbox_w / orig_w).min(bbox_h / orig_h);
        let dx = bbox_min.x + (bbox_w - orig_w * scale) * 0.5 - orig_min.x * scale;
        let dy = bbox_min.y + (bbox_h - orig_h * scale) * 0.5 - orig_min.y * scale;

        let mut rings: Vec<Vec<Vec2>> = Vec::new();
        for ring in svg.rings.iter() {
            let mut pts = Vec::with_capacity(ring.len());
            for pt in ring.iter() {
                pts.push(Vec2::new(pt.x * scale + dx, pt.y * scale + dy));
            }
            let pts = normalize_svg_ring(pts);
            if pts.len() >= 3 {
                rings.push(pts);
            }
        }
        if rings.is_empty() {
            self.polygon = MultiPolygon::new(vec![]);
            return;
        }

        let hole_flags = svg_compute_hole_flags(&rings, svg.fill_rule);
        let mut outers: Vec<Vec<Vec2>> = Vec::new();
        let mut holes: Vec<Vec<Vec2>> = Vec::new();
        for (ring, is_hole) in rings.into_iter().zip(hole_flags.into_iter()) {
            if is_hole {
                holes.push(ring);
            } else {
                outers.push(ring);
            }
        }
        if outers.is_empty() {
            outers = holes;
            holes = Vec::new();
        }

        let mut polys = Vec::new();
        for outer in outers.iter() {
            let outer_poly = Polygon::new(vec2_to_linestring(outer), vec![]);
            let mut inner_lines = Vec::new();
            for hole in holes.iter() {
                if let Some(pt) = hole.first() {
                    if outer_poly.contains(&Point::new(pt.x, pt.y)) {
                        inner_lines.push(vec2_to_linestring(hole));
                    }
                }
            }
            polys.push(Polygon::new(vec2_to_linestring(outer), inner_lines));
        }

        self.polygon = MultiPolygon::new(polys);
        svg.cached_polygon = Some(self.polygon.clone());
        let raw_paths = geo_multipolygon_to_bez_paths(&self.polygon);
        svg.cached_paths_raw = Some(raw_paths.clone());
        svg.cached_paths = Some(raw_paths);
        svg.cached_bbox_min = bbox_min;
        svg.cached_bbox_max = bbox_max;
    }
}

// SVG Parsing
#[derive(Clone, Debug)]
struct ParsedSvgShape {
    rings: Vec<Vec<Vec2>>,
    _fill_rule: SvgFillRule,
    bbox_min: Vec2,
    bbox_max: Vec2,
}

fn rings_bbox(rings: &[Vec<Vec2>]) -> Option<(Vec2, Vec2)> {
    let mut min = Vec2::new(f64::INFINITY, f64::INFINITY);
    let mut max = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for ring in rings {
        for pt in ring {
            min.x = min.x.min(pt.x);
            min.y = min.y.min(pt.y);
            max.x = max.x.max(pt.x);
            max.y = max.y.max(pt.y);
        }
    }
    if !min.x.is_finite() || !min.y.is_finite() || !max.x.is_finite() || !max.y.is_finite() {
        return None;
    }
    Some((min, max))
}

fn parse_svg(svg_data: String, combine_paths: bool) -> Option<(Vec<ParsedSvgShape>, f64)> {
    let mut paths = Vec::new();
    let mut view_box: Option<(f64, f64, f64, f64)> = None;
    let mut width: Option<(f64, String)> = None;
    let mut height: Option<(f64, String)> = None;
    let parser = SvgParser::new(svg_data.as_str());
    let mut svg_fill_rule = SvgFillRule::NonZero;
    for event in parser {
        if let SvgEvent::Tag(tag, _, attributes) = event {
            if tag == "svg" {
                if let Some(val) = attributes.get("viewBox") {
                    view_box = parse_view_box(val.as_ref());
                }
                if let Some(val) = attributes.get("width") {
                    width = parse_svg_length(val.as_ref());
                }
                if let Some(val) = attributes.get("height") {
                    height = parse_svg_length(val.as_ref());
                }
                svg_fill_rule = fill_rule_from_attrs(&attributes, svg_fill_rule);
            }
            if tag == "path" {
                if let Some(d) = attributes.get("d") {
                    if let Ok(path) = BezPath::from_svg(d.as_ref()) {
                        if !path.is_empty() {
                            let fill_rule = fill_rule_from_attrs(&attributes, svg_fill_rule);
                            paths.push((path, fill_rule));
                        }
                    }
                }
            }
        }
    }

    let mut scale = 1.0;
    if let (Some((w, wu)), Some((h, hu))) = (width, height) {
        if let (Some(wx), Some(hx)) = (length_unit_to_mm(&wu), length_unit_to_mm(&hu)) {
            let view = view_box.unwrap_or((0.0, 0.0, w, h));
            let view_w = view.2.max(1.0);
            let view_h = view.3.max(1.0);
            let sx = (w * wx) / view_w;
            let sy = (h * hx) / view_h;
            scale = sx.min(sy);
        }
    }

    let mut shape_rings: Vec<ParsedSvgShape> = Vec::new();
    let mut all_rings = Vec::new();
    for (path, fill_rule) in paths {
        let rings = bezpath_to_rings(&path, 0.5);
        if rings.is_empty() {
            continue;
        }
        let normalized_rings: Vec<Vec<Vec2>> = rings.into_iter().map(normalize_ring).collect();

        if combine_paths {
            all_rings.extend(normalized_rings);
        } else if let Some((bbox_min, bbox_max)) = rings_bbox(&normalized_rings) {
            shape_rings.push(ParsedSvgShape {
                rings: normalized_rings,
                _fill_rule: fill_rule,
                bbox_min,
                bbox_max,
            });
        }
    }
    if combine_paths && !all_rings.is_empty() {
        if let Some((bbox_min, bbox_max)) = rings_bbox(&all_rings) {
            shape_rings.push(ParsedSvgShape {
                rings: all_rings,
                _fill_rule: svg_fill_rule,
                bbox_min,
                bbox_max,
            });
        }
    }
    if shape_rings.is_empty() {
        return None;
    }

    Some((shape_rings, scale))
}
fn normalize_ring(mut ring: Vec<Vec2>) -> Vec<Vec2> {
    if ring.len() > 1 {
        let first = ring[0];
        let last = ring[ring.len() - 1];
        if (first - last).hypot() < 1e-6 {
            ring.pop();
        }
    }
    ring
}
fn bezpath_to_rings(path: &BezPath, tolerance: f64) -> Vec<Vec<Vec2>> {
    let mut rings = Vec::new();
    let mut flattened = BezPath::new();
    flatten(path, tolerance, |el| flattened.push(el));
    let mut current_ring: Vec<Vec2> = Vec::new();

    for el in flattened.elements() {
        match el {
            PathEl::MoveTo(p) => {
                if current_ring.len() >= 3 {
                    rings.push(current_ring);
                }
                current_ring = vec![Vec2::new(p.x, p.y)];
            }
            PathEl::LineTo(p) => current_ring.push(Vec2::new(p.x, p.y)),
            PathEl::ClosePath => {
                if current_ring.len() >= 3 {
                    rings.push(current_ring.clone());
                }
                current_ring.clear();
            }
            _ => {}
        }
    }
    if current_ring.len() >= 3 {
        rings.push(current_ring);
    }
    rings
}
fn fill_rule_from_attrs(attrs: &svg::node::Attributes, fallback: SvgFillRule) -> SvgFillRule {
    let fill_rule = attrs
        .get("fill-rule")
        .map(|val| val.to_string())
        .or_else(|| {
            attrs
                .get("style")
                .and_then(|style| parse_style_value(style.as_ref(), "fill-rule"))
        })
        .unwrap_or_else(|| "nonzero".to_string());
    match fill_rule.as_str() {
        "evenodd" => SvgFillRule::EvenOdd,
        _ => fallback,
    }
}
fn parse_style_value(style: &str, key: &str) -> Option<String> {
    style
        .split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .find_map(|entry| {
            let mut parts = entry.splitn(2, ':');
            let name = parts.next()?.trim();
            let value = parts.next()?.trim();
            if name == key {
                Some(value.to_string())
            } else {
                None
            }
        })
}
fn parse_view_box(value: &str) -> Option<(f64, f64, f64, f64)> {
    let parts: Vec<f64> = value
        .split(|ch: char| ch.is_whitespace() || ch == ',')
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<f64>().ok())
        .collect();
    if parts.len() != 4 {
        return None;
    }
    Some((parts[0], parts[1], parts[2], parts[3]))
}
fn parse_svg_length(value: &str) -> Option<(f64, String)> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut number = String::new();
    let mut unit = String::new();
    for ch in trimmed.chars() {
        if ch.is_ascii_digit() || ch == '.' || ch == '-' || ch == '+' {
            if unit.is_empty() {
                number.push(ch);
            } else {
                return None;
            }
        } else {
            unit.push(ch);
        }
    }
    let value = number.parse::<f64>().ok()?;
    let unit = if unit.is_empty() { "px" } else { unit.as_str() };
    Some((value, unit.to_string()))
}

// TEXT
fn text_to_multipolygon(
    text: &str,
    font: &TextFont,
    v1: Vec2,
    v2: Vec2,
) -> (MultiPolygon<f64>, f64) {
    let Ok(face) = ttf_parser::Face::parse(font.data(), 0) else {
        return (MultiPolygon::new(vec![]), 0.0);
    };
    let bbox_w = (v2.x - v1.x).max(EPSILON);
    let bbox_h = (v2.y - v1.y).max(EPSILON);

    let asc = face.ascender() as f64;
    let desc = face.descender() as f64;
    let text_height = (asc - desc).max(1.0);

    let mut advances: Vec<(ttf_parser::GlyphId, f64)> = Vec::new();
    let mut advance_total = 0.0;

    for ch in text.chars() {
        if ch == '\n' {
            continue;
        }
        if let Some(gid) = face.glyph_index(ch) {
            advances.push((gid, advance_total));
            let adv = face.glyph_hor_advance(gid).unwrap_or(0) as f64;
            advance_total += adv;
        } else {
            let adv = face.units_per_em() as f64 * 0.5;
            advance_total += adv;
        }
    }
    if advance_total <= EPSILON {
        return (MultiPolygon::new(vec![]), 0.0);
    }

    let scale = (bbox_w / advance_total).min(bbox_h / text_height);
    let scaled_w = advance_total * scale;
    let scaled_h = text_height * scale;
    let top_y = v1.y + (bbox_h - scaled_h) * 0.5;
    let offset_y = top_y + asc * scale;
    let offset_x = v1.x + (bbox_w - scaled_w) * 0.5;

    let mut rings: Vec<LineString<f64>> = Vec::new();
    for (gid, advance_x) in advances {
        let offset = Vec2::new(offset_x + advance_x * scale, offset_y);
        let mut builder = GlyphBuilder::new(scale, offset);
        face.outline_glyph(gid, &mut builder);
        let contours = builder.finish();
        for contour in contours {
            let poly = bez_path_to_geo_polygon(&contour);
            let ring = poly.exterior().clone();
            if ring.0.len() >= 4 {
                rings.push(ring);
            }
        }
    }

    (rings_to_multipolygon(rings), scale)
}

// VORONOI
fn build_voronoi_rings(
    p1: Vec2,
    p2: Vec2,
    seeds: usize,
    gap_factor: f64,
    relaxation: usize,
) -> Vec<Vec<Vec2>> {
    let min = Vec2::new(p1.x.min(p2.x), p1.y.min(p2.y));
    let max = Vec2::new(p1.x.max(p2.x), p1.y.max(p2.y));
    let size = max - min;
    if size.x.abs() < 1e-6 || size.y.abs() < 1e-6 {
        return Vec::new();
    }

    let guard_pad_x = size.x;
    let guard_pad_y = size.y;
    let guard_min = min - Vec2::new(guard_pad_x, guard_pad_y);
    let guard_max = max + Vec2::new(guard_pad_x, guard_pad_y);
    let guard_steps = 20;
    let mut guard_points = Vec::new();
    for i in 0..=guard_steps {
        let t = i as f64 / guard_steps as f64;
        let x = guard_min.x + (guard_max.x - guard_min.x) * t;
        guard_points.push(Vec2::new(x, guard_min.y));
    }
    for i in 1..=guard_steps {
        let t = i as f64 / guard_steps as f64;
        let y = guard_min.y + (guard_max.y - guard_min.y) * t;
        guard_points.push(Vec2::new(guard_max.x, y));
    }
    for i in 1..=guard_steps {
        let t = i as f64 / guard_steps as f64;
        let x = guard_max.x + (guard_min.x - guard_max.x) * t;
        guard_points.push(Vec2::new(x, guard_max.y));
    }
    for i in 1..guard_steps {
        let t = i as f64 / guard_steps as f64;
        let y = guard_max.y + (guard_min.y - guard_max.y) * t;
        guard_points.push(Vec2::new(guard_min.x, y));
    }
    let count = seeds;
    let mut points = Vec::with_capacity(count + guard_points.len());
    let grid_x = (count as f64).sqrt().ceil() as usize;
    let grid_y = count.div_ceil(grid_x).max(1);
    let step_x = size.x / grid_x as f64;
    let step_y = size.y / grid_y as f64;
    let cell = step_x.min(step_y);
    let jitter = cell * 0.5;

    'outer: for iy in 0..grid_y {
        for ix in 0..grid_x {
            if points.len() >= count {
                break 'outer;
            }
            let base_x = min.x + (ix as f64 + 0.5) * step_x;
            let base_y = min.y + (iy as f64 + 0.5) * step_y;
            let jx = (Math::random() * 2.0 - 1.0) * jitter;
            let jy = (Math::random() * 2.0 - 1.0) * jitter;
            let x = (base_x + jx).clamp(min.x, max.x);
            let y = (base_y + jy).clamp(min.y, max.y);
            points.push(Vec2::new(x, y));
        }
    }

    lloyd_relax_points(&mut points, &guard_points, min, max, relaxation);
    points.extend_from_slice(&guard_points);
    let rings = voronoi_cells(&points, &guard_points, min, max);
    let rings = inset_rings(rings, cell * gap_factor.clamp(0.0, 0.95));
    round_rings(rings, cell * 0.1, 3)
}

fn voronoi_rings_to_svg_data(rings: Vec<Vec<Vec2>>) -> SvgData {
    let mut normalized = Vec::new();
    for ring in rings {
        let mut ring = normalize_svg_ring(ring);
        if ring.len() < 3 {
            continue;
        }
        if ring_area(&ring) < 0.0 {
            ring.reverse();
        }
        normalized.push(ring);
    }
    SvgData::new(normalized, SvgFillRule::NonZero)
}

// OTHER HELPERS
fn normalize_svg_ring(mut ring: Vec<Vec2>) -> Vec<Vec2> {
    if ring.len() > 1 {
        let first = ring[0];
        let last = ring[ring.len() - 1];
        if (first - last).hypot() < 1e-6 {
            ring.pop();
        }
    }
    ring
}

fn translate_bezpaths(paths: &[BezPath], dx: f64, dy: f64) -> Vec<BezPath> {
    paths
        .iter()
        .map(|path| translate_bezpath(path, dx, dy))
        .collect()
}

fn translate_bezpath(path: &BezPath, dx: f64, dy: f64) -> BezPath {
    let mut out = BezPath::new();
    for elem in path.iter() {
        match elem {
            PathEl::MoveTo(pt) => {
                out.push(PathEl::MoveTo(kurbo::Point::new(pt.x + dx, pt.y + dy)));
            }
            PathEl::LineTo(pt) => {
                out.push(PathEl::LineTo(kurbo::Point::new(pt.x + dx, pt.y + dy)));
            }
            PathEl::QuadTo(pt1, pt2) => {
                out.push(PathEl::QuadTo(
                    kurbo::Point::new(pt1.x + dx, pt1.y + dy),
                    kurbo::Point::new(pt2.x + dx, pt2.y + dy),
                ));
            }
            PathEl::CurveTo(pt1, pt2, pt3) => {
                out.push(PathEl::CurveTo(
                    kurbo::Point::new(pt1.x + dx, pt1.y + dy),
                    kurbo::Point::new(pt2.x + dx, pt2.y + dy),
                    kurbo::Point::new(pt3.x + dx, pt3.y + dy),
                ));
            }
            PathEl::ClosePath => out.push(PathEl::ClosePath),
        }
    }
    out
}

fn _rotate_bezpaths(paths: &[BezPath], center: Vec2, angle: f64) -> Vec<BezPath> {
    paths
        .iter()
        .map(|path| _rotate_bezpath(path, center, angle))
        .collect()
}

fn _rotate_bezpath(path: &BezPath, center: Vec2, angle: f64) -> BezPath {
    let mut out = BezPath::new();
    for elem in path.iter() {
        match elem {
            PathEl::MoveTo(pt) => {
                out.push(PathEl::MoveTo(_rotate_point(pt, center, angle)));
            }
            PathEl::LineTo(pt) => {
                out.push(PathEl::LineTo(_rotate_point(pt, center, angle)));
            }
            PathEl::QuadTo(pt1, pt2) => {
                out.push(PathEl::QuadTo(
                    _rotate_point(pt1, center, angle),
                    _rotate_point(pt2, center, angle),
                ));
            }
            PathEl::CurveTo(pt1, pt2, pt3) => {
                out.push(PathEl::CurveTo(
                    _rotate_point(pt1, center, angle),
                    _rotate_point(pt2, center, angle),
                    _rotate_point(pt3, center, angle),
                ));
            }
            PathEl::ClosePath => out.push(PathEl::ClosePath),
        }
    }
    out
}

fn _rotate_point(point: kurbo::Point, center: Vec2, angle: f64) -> kurbo::Point {
    let v = Vec2::new(point.x, point.y);
    let rotated = rotate_vector(v - center, angle) + center;
    kurbo::Point::new(rotated.x, rotated.y)
}

fn _rotate_multipolygon(
    polygon: &MultiPolygon<f64>,
    center: Vec2,
    angle: f64,
) -> MultiPolygon<f64> {
    let mut polys = Vec::with_capacity(polygon.0.len());
    for poly in polygon.0.iter() {
        let exterior = _rotate_linestring(poly.exterior(), center, angle);
        let interiors = poly
            .interiors()
            .iter()
            .map(|ring| _rotate_linestring(ring, center, angle))
            .collect();
        polys.push(Polygon::new(exterior, interiors));
    }
    MultiPolygon::new(polys)
}

fn _rotate_linestring(line: &LineString<f64>, center: Vec2, angle: f64) -> LineString<f64> {
    let coords: Vec<Coord<f64>> = line
        .points()
        .map(|pt| {
            let v = Vec2::new(pt.x(), pt.y());
            let rotated = rotate_vector(v - center, angle) + center;
            Coord {
                x: rotated.x,
                y: rotated.y,
            }
        })
        .collect();
    LineString::from(coords)
}

fn vec2_to_linestring(points: &[Vec2]) -> LineString<f64> {
    let mut coords: Vec<geo::Coord<f64>> = points
        .iter()
        .map(|pt| geo::Coord { x: pt.x, y: pt.y })
        .collect();
    if coords.len() >= 2 && coords.first() != coords.last() {
        coords.push(coords[0]);
    }
    LineString::from(coords)
}

fn ring_area(points: &[Vec2]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..points.len() {
        let p1 = points[i];
        let p2 = points[(i + 1) % points.len()];
        area += (p1.x * p2.y) - (p2.x * p1.y);
    }
    area * 0.5
}

fn svg_compute_hole_flags(rings: &[Vec<Vec2>], fill_rule: SvgFillRule) -> Vec<bool> {
    let mut flags = vec![false; rings.len()];
    match fill_rule {
        SvgFillRule::NonZero => {
            for (idx, ring) in rings.iter().enumerate() {
                flags[idx] = ring_area(ring) < 0.0;
            }
        }
        SvgFillRule::EvenOdd => {
            let polys: Vec<Polygon<f64>> = rings
                .iter()
                .map(|ring| Polygon::new(vec2_to_linestring(ring), vec![]))
                .collect();
            for (idx, ring) in rings.iter().enumerate() {
                let Some(probe) = ring.first() else {
                    continue;
                };
                let mut depth = 0;
                for (j, poly) in polys.iter().enumerate() {
                    if idx == j {
                        continue;
                    }
                    if poly.contains(&Point::new(probe.x, probe.y)) {
                        depth += 1;
                    }
                }
                flags[idx] = depth % 2 == 1;
            }
        }
    }
    flags
}

struct GlyphBuilder {
    contours: Vec<BezPath>,
    current: Option<BezPath>,
    scale: f64,
    offset: Vec2,
}
impl GlyphBuilder {
    fn new(scale: f64, offset: Vec2) -> Self {
        Self {
            contours: Vec::new(),
            current: None,
            scale,
            offset,
        }
    }
    fn finish(mut self) -> Vec<BezPath> {
        if let Some(path) = self.current.take() {
            if !path.is_empty() {
                self.contours.push(path);
            }
        }
        self.contours
    }
    fn pt(&self, x: f32, y: f32) -> kurbo::Point {
        kurbo::Point::new(
            self.offset.x + (x as f64) * self.scale,
            self.offset.y - (y as f64) * self.scale,
        )
    }
}
impl ttf_parser::OutlineBuilder for GlyphBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        if let Some(path) = self.current.take() {
            if !path.is_empty() {
                self.contours.push(path);
            }
        }
        let mut path = BezPath::new();
        path.push(PathEl::MoveTo(self.pt(x, y)));
        self.current = Some(path);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let pt = self.pt(x, y);
        if let Some(path) = self.current.as_mut() {
            path.push(PathEl::LineTo(pt));
        }
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let pt1 = self.pt(x1, y1);
        let pt2 = self.pt(x, y);
        if let Some(path) = self.current.as_mut() {
            path.push(PathEl::QuadTo(pt1, pt2));
        }
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let pt1 = self.pt(x1, y1);
        let pt2 = self.pt(x2, y2);
        let pt3 = self.pt(x, y);
        if let Some(path) = self.current.as_mut() {
            path.push(PathEl::CurveTo(pt1, pt2, pt3));
        }
    }
    fn close(&mut self) {
        if let Some(path) = self.current.as_mut() {
            path.push(PathEl::ClosePath);
        }
    }
}

fn rings_to_multipolygon(rings: Vec<LineString<f64>>) -> MultiPolygon<f64> {
    if rings.is_empty() {
        return MultiPolygon::new(vec![]);
    }
    let mut outers: Vec<usize> = Vec::new();
    let mut holes: Vec<(usize, LineString<f64>)> = Vec::new();
    let mut areas: Vec<f64> = Vec::new();

    for ring in &rings {
        areas.push(ring_signed_area(ring).abs());
    }

    for (i, ring) in rings.iter().enumerate() {
        let pt = ring.0.first().copied().unwrap_or_default();
        let mut container: Option<usize> = None;
        for (j, outer) in rings.iter().enumerate() {
            if i == j {
                continue;
            }
            let poly = Polygon::new(outer.clone(), vec![]);
            if poly.contains(&Point::new(pt.x, pt.y))
                && (container.is_none() || areas[j] < areas[container.unwrap()])
            {
                container = Some(j);
            }
        }
        if let Some(idx) = container {
            holes.push((idx, ring.clone()));
        } else {
            outers.push(i);
        }
    }

    let mut polys = Vec::new();
    for outer_idx in outers {
        let mut inner = Vec::new();
        for (idx, ring) in holes.iter() {
            if *idx == outer_idx {
                inner.push(ring.clone());
            }
        }
        let poly = Polygon::new(rings[outer_idx].clone(), inner).orient(Direction::Default);
        polys.push(poly);
    }

    MultiPolygon::new(polys)
}

fn ring_signed_area(ring: &LineString<f64>) -> f64 {
    let pts = &ring.0;
    if pts.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..pts.len() - 1 {
        area += pts[i].x * pts[i + 1].y - pts[i + 1].x * pts[i].y;
    }
    area * 0.5
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Operation {
    Union,
    Difference,
}
impl Operation {
    pub fn next(&mut self) {
        match self {
            Operation::Union => *self = Operation::Difference,
            Operation::Difference => *self = Operation::Union,
        }
    }
    pub fn union(&mut self) {
        *self = Operation::Union;
    }
    pub fn difference(&mut self) {
        *self = Operation::Difference;
    }
}
impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Union => write!(f, "Add"),
            Operation::Difference => write!(f, "Substract"),
        }
    }
}
