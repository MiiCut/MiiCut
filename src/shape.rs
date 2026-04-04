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
use geo::{orient::Direction, BooleanOps, Coord, LineString, MultiPolygon, Point, Polygon};
use js_sys::Math::{self};
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
    dim_offsets: [f64; 6],

    ghosts: usize,
    ghosts_trans_x: f64,
    ghosts_trans_y: f64,
    ghosts_rot_center: Vec2,
    ghosts_rot_center_saved: Vec2,
    ghosts_rot: bool,
    ghosts_self_rot: bool,
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
            dim_offsets: self.dim_offsets,
            ghosts: self.ghosts,
            ghosts_trans_x: self.ghosts_trans_x,
            ghosts_trans_y: self.ghosts_trans_y,
            ghosts_rot_center: self.ghosts_rot_center,
            ghosts_rot_center_saved: self.ghosts_rot_center_saved,
            ghosts_rot: self.ghosts_rot,
            ghosts_self_rot: self.ghosts_self_rot,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GroupShape {
    children: Vec<EUId>,
    pub(crate) cached_polygon: Option<MultiPolygon<f64>>,
}
impl GroupShape {
    pub fn new(children: Vec<EUId>) -> Self {
        Self { children, cached_polygon: None }
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
        // Place radius handle horizontally to the right
        let radius = (v2 - v1).hypot();
        let v2_horiz = Vec2::new(v1.x + radius, v1.y);
        let vs = vec![Vertex::new(v1), Vertex::new(v2_horiz)];

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

        let mut shape = GeneralShape::new(
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
        )?;
        // Dimension on the opposite side (left) of the radius handle (right)
        shape.set_dim_offset(0, PI);
        Some(shape)
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

        let mut values: Vec<Vertex> = vs
            .into_iter()
            .map(|v| {
                let mut vertex = v;
                vertex.enable_radius();
                vertex
            })
            .collect();
        // Add 4 sag vertices at edge midpoints
        for i in 0..4 {
            let j = (i + 1) % 4;
            let mid = (values[i].curr() + values[j].curr()) * 0.5;
            values.push(Vertex::new(mid));
        }

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
        // Place radius handles on the horizontal, pointing outward from the axis
        let mid = (v1 + v2) * 0.5;
        let outward1 = if v1.x < mid.x {
            Vec2::new(-1.0, 0.0)
        } else {
            Vec2::new(1.0, 0.0)
        };
        let outward2 = if v2.x < mid.x {
            Vec2::new(-1.0, 0.0)
        } else {
            Vec2::new(1.0, 0.0)
        };
        // If oblong is vertical, fallback to perpendicular
        let default_r1 = if (v2.x - v1.x).abs() < 1e-6 {
            let perp = Vec2::new(-dir.y, dir.x);
            v1 + perp * 20.0
        } else {
            v1 + outward1 * 20.0
        };
        let default_r2 = if (v2.x - v1.x).abs() < 1e-6 {
            let perp = Vec2::new(-dir.y, dir.x);
            v2 + perp * 20.0
        } else {
            v2 + outward2 * 20.0
        };
        let r1 = r1.unwrap_or(default_r1);
        let r2 = r2.unwrap_or(default_r2);
        // Compute sag vertex positions at chord midpoints (sag=0)
        let r1_radius = (r1 - v1).hypot();
        let r2_radius = (r2 - v2).hypot();
        let (sag1_pos, sag2_pos) =
            if let Some((a1_s, a2_s)) = oblong_straight_angles(v1, v2, r1_radius, r2_radius) {
                (
                    oblong_side_handle(v1, v2, r1_radius, r2_radius, a2_s, 0.0, true),
                    oblong_side_handle(v1, v2, r1_radius, r2_radius, a1_s, 0.0, false),
                )
            } else {
                let mid = (v1 + v2) * 0.5;
                (mid, mid)
            };
        let vs = vec![
            Vertex::new(v1),       // v[0] = c1
            Vertex::new(v2),       // v[1] = c2
            Vertex::new(r1),       // v[2] = r1 control
            Vertex::new(r2),       // v[3] = r2 control
            Vertex::new(sag1_pos), // v[4] = sag1 handle (side at a2)
            Vertex::new(sag2_pos), // v[5] = sag2 handle (side at a1)
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

        let mut shape = GeneralShape::new(
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
        )?;
        // Axis dimension on the segment O1O2
        shape.set_dim_offset(0, 0.0);
        // Radius dimensions on the opposite side of the radius handles
        let natural1 = (r1 - v1).y.atan2((r1 - v1).x);
        let natural2 = (r2 - v2).y.atan2((r2 - v2).x);
        shape.set_dim_offset(1, natural1 + PI);
        shape.set_dim_offset(2, natural2 + PI);
        Some(shape)
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
        let n = vs.len();
        let mut values: Vec<Vertex> = vs
            .into_iter()
            .map(|v| {
                let mut vertex = Vertex::new(v);
                vertex.enable_radius();
                vertex
            })
            .collect();
        // Add N sag vertices at edge midpoints
        for i in 0..n {
            let j = (i + 1) % n;
            let mid = (values[i].curr() + values[j].curr()) * 0.5;
            values.push(Vertex::new(mid));
        }

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
            dim_offsets: [-20.0, 20.0, -20.0, -20.0, 0.0, 0.0],

            ghosts: 0,
            ghosts_trans_x: 100.0,
            ghosts_trans_y: 0.0,
            ghosts_rot: false,
            ghosts_rot_center: Vec2::ZERO,
            ghosts_rot_center_saved: Vec2::ZERO,
            ghosts_self_rot: false,
        };
        // Compute default rot center as vertex centroid
        let n = shape.vertices.len() as f64;
        if n > 0.0 {
            let sum: Vec2 = shape
                .vertices
                .iter()
                .map(|(_, v)| v.curr())
                .fold(Vec2::ZERO, |a, b| a + b);
            shape.ghosts_rot_center = sum / n;
            shape.ghosts_rot_center_saved = shape.ghosts_rot_center;
        }
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
                    let center = self.vertices.val(0).curr();
                    let curr = self.vertices.val(1).curr();
                    // Constrain to horizontal: keep same Y as center, radius = |dx|
                    let r = snap_val((curr - center).hypot(), snap);
                    let sign = if curr.x >= center.x { 1.0 } else { -1.0 };
                    self.vertices
                        .val_mut(1)
                        .set_curr(Vec2::new(center.x + sign * r, center.y));
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
                if self.vertices.len() != 6 {
                    return false;
                }
                let idx = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                    Some(i) => i,
                    None => return false,
                };
                if idx == 0 || idx == 1 {
                    // Moving a center: also move radius handle + recompute sag vertices
                    let radius_idx = if idx == 0 { 2 } else { 3 };
                    let saved_center = self.vertices.val(idx as i64).saved();
                    // Compute old sag values before the move
                    let old_c1 = self.vertices.val(0).curr();
                    let old_c2 = self.vertices.val(1).curr();
                    let old_r1 = (self.vertices.val(2).curr() - old_c1).hypot();
                    let old_r2 = (self.vertices.val(3).curr() - old_c2).hypot();
                    let old_sags =
                        oblong_straight_angles(old_c1, old_c2, old_r1, old_r2).map(|(a1, a2)| {
                            let s1 = sag_from_vertex(
                                old_c1,
                                old_c2,
                                old_r1,
                                old_r2,
                                a2,
                                self.vertices.val(4).curr(),
                            );
                            let s2 = sag_from_vertex(
                                old_c1,
                                old_c2,
                                old_r1,
                                old_r2,
                                a1,
                                self.vertices.val(5).curr(),
                            );
                            (s1, s2)
                        });

                    let tmp = self.vertices.val(idx as i64).clone();
                    self.vertices
                        .val_mut(idx as i64)
                        .set_saved(snap_vertex(tmp.saved(), snap));
                    self.vertices.val_mut(idx as i64).add(delta);
                    let tmp = self.vertices.val(idx as i64).clone();
                    self.vertices
                        .val_mut(idx as i64)
                        .set_curr(snap_vertex(tmp.curr(), snap));

                    let center_delta = self.vertices.val(idx as i64).curr() - saved_center;
                    if center_delta.hypot() > EPSILON {
                        self.vertices.val_mut(radius_idx as i64).add(center_delta);
                    }

                    // Recompute sag vertices with preserved sag values
                    if let Some((old_s1, old_s2)) = old_sags {
                        let new_c1 = self.vertices.val(0).curr();
                        let new_c2 = self.vertices.val(1).curr();
                        let new_r1 = (self.vertices.val(2).curr() - new_c1).hypot();
                        let new_r2 = (self.vertices.val(3).curr() - new_c2).hypot();
                        if let Some((na1, na2)) =
                            oblong_straight_angles(new_c1, new_c2, new_r1, new_r2)
                        {
                            let pos4 = oblong_side_handle(
                                new_c1, new_c2, new_r1, new_r2, na2, old_s1, true,
                            );
                            let pos5 = oblong_side_handle(
                                new_c1, new_c2, new_r1, new_r2, na1, old_s2, false,
                            );
                            self.vertices.val_mut(4).set_curr(pos4);
                            self.vertices.val_mut(5).set_curr(pos5);
                        }
                    }
                } else if idx == 2 || idx == 3 {
                    // Radius handle: constrain to horizontal + recompute sag vertices
                    let center_idx = if idx == 2 { 0 } else { 1 };
                    // Old sag values
                    let old_c1 = self.vertices.val(0).curr();
                    let old_c2 = self.vertices.val(1).curr();
                    let old_r1 = (self.vertices.val(2).curr() - old_c1).hypot();
                    let old_r2 = (self.vertices.val(3).curr() - old_c2).hypot();
                    let old_sags =
                        oblong_straight_angles(old_c1, old_c2, old_r1, old_r2).map(|(a1, a2)| {
                            let s1 = sag_from_vertex(
                                old_c1,
                                old_c2,
                                old_r1,
                                old_r2,
                                a2,
                                self.vertices.val(4).curr(),
                            );
                            let s2 = sag_from_vertex(
                                old_c1,
                                old_c2,
                                old_r1,
                                old_r2,
                                a1,
                                self.vertices.val(5).curr(),
                            );
                            (s1, s2)
                        });

                    self.vertices.val_mut(idx as i64).add(delta);
                    let center = self.vertices.val(center_idx).curr();
                    let curr = self.vertices.val(idx as i64).curr();
                    let r = snap_val((curr - center).hypot(), snap);
                    let sign = if curr.x >= center.x { 1.0 } else { -1.0 };
                    self.vertices
                        .val_mut(idx as i64)
                        .set_curr(Vec2::new(center.x + sign * r, center.y));

                    // Recompute sag vertices
                    if let Some((old_s1, old_s2)) = old_sags {
                        let new_c1 = self.vertices.val(0).curr();
                        let new_c2 = self.vertices.val(1).curr();
                        let new_r1 = (self.vertices.val(2).curr() - new_c1).hypot();
                        let new_r2 = (self.vertices.val(3).curr() - new_c2).hypot();
                        if let Some((na1, na2)) =
                            oblong_straight_angles(new_c1, new_c2, new_r1, new_r2)
                        {
                            let pos4 = oblong_side_handle(
                                new_c1, new_c2, new_r1, new_r2, na2, old_s1, true,
                            );
                            let pos5 = oblong_side_handle(
                                new_c1, new_c2, new_r1, new_r2, na1, old_s2, false,
                            );
                            self.vertices.val_mut(4).set_curr(pos4);
                            self.vertices.val_mut(5).set_curr(pos5);
                        }
                    }
                } else if idx == 4 || idx == 5 {
                    // Sag handle: use raw draw_pos (bypasses grid snap on pointer)
                    self.vertices.val_mut(idx as i64).set_curr(user_ui.draw_pos);
                    let c1 = self.vertices.val(0).curr();
                    let c2 = self.vertices.val(1).curr();
                    let r1 = (self.vertices.val(2).curr() - c1).hypot();
                    let r2 = (self.vertices.val(3).curr() - c2).hypot();
                    if let Some((a1_s, a2_s)) = oblong_straight_angles(c1, c2, r1, r2) {
                        let angle = if idx == 4 { a2_s } else { a1_s };
                        let mut sag = sag_from_vertex(
                            c1,
                            c2,
                            r1,
                            r2,
                            angle,
                            self.vertices.val(idx as i64).curr(),
                        );
                        // Magnet: snap to 0 (straight line)
                        if sag.abs() < 5.0 {
                            sag = 0.0;
                        } else {
                            // Magnet: snap when inter-handle distance ≈ average diameter
                            let other_idx = if idx == 4 { 5 } else { 4 };
                            let other_pos = self.vertices.val(other_idx).curr();
                            let target = r1 + r2;
                            let p1 = c1 + Vec2::new(r1 * angle.cos(), r1 * angle.sin());
                            let p2 = c2 + Vec2::new(r2 * angle.cos(), r2 * angle.sin());
                            let chord = p2 - p1;
                            let l = chord.hypot();
                            if l > 1e-9 {
                                let u = chord / l;
                                let n = Vec2::new(-u.y, u.x);
                                let mid = (p1 + p2) * 0.5;
                                let p = mid - other_pos;
                                let pn = p.x * n.x + p.y * n.y;
                                let pp = p.x * p.x + p.y * p.y;
                                let disc = 4.0 * pn * pn - 4.0 * (pp - target * target);
                                if disc >= 0.0 {
                                    let sq = disc.sqrt();
                                    let s1 = (-2.0 * pn + sq) / 2.0;
                                    let s2 = (-2.0 * pn - sq) / 2.0;
                                    let snap_sag = if (s1 - sag).abs() < (s2 - sag).abs() {
                                        s1
                                    } else {
                                        s2
                                    };
                                    if (sag - snap_sag).abs() < 5.0 {
                                        sag = snap_sag;
                                    }
                                }
                            }
                        }

                        let new_pos = oblong_side_handle(c1, c2, r1, r2, angle, sag, idx == 4);
                        self.vertices.val_mut(idx as i64).set_curr(new_pos);
                    }
                }
                self.set_bezpath();
                true
            }
            ShapeType::Square | ShapeType::Text | ShapeType::Svg | ShapeType::Voronoi => {
                let len = self.vertices.len();
                let nc = self.get_n_corners();
                if len != 4 && len != 8 {
                    return false;
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
                // Sag vertex drag (idx >= nc): constrain to edge normal, no grid snap
                if (i as usize) >= nc && nc == 4 && len == 8 {
                    use crate::helpers::math::{poly_sag_from_vertex, poly_sag_handle};
                    let edge_idx = (i as usize) - nc;
                    let edge_start = self.vertices.val(edge_idx as i64).curr();
                    let edge_end = self.vertices.val(((edge_idx + 1) % nc) as i64).curr();
                    self.vertices.val_mut(i).set_curr(user_ui.draw_pos);
                    let mut sag = poly_sag_from_vertex(edge_start, edge_end, user_ui.draw_pos);
                    if sag.abs() < 5.0 { sag = 0.0; }
                    let new_pos = poly_sag_handle(edge_start, edge_end, sag);
                    self.vertices.val_mut(i).set_curr(new_pos);
                    self.set_bezpath();
                    return true;
                }
                let rotation = self.rotation.get();
                if rotation.abs() > EPSILON {
                    // Vertices are stored in local (unrotated) space.
                    // Mouse pointer is in world (rotated) space.
                    // Un-rotate mouse around the bbox center to convert to local space.
                    let center = self.bbox_center_saved();
                    let mouse_saved_local =
                        rotate_vector(user_ui.pointer.saved() - center, -rotation) + center;
                    let mouse_curr_local =
                        rotate_vector(user_ui.pointer.curr() - center, -rotation) + center;
                    let mut local_delta = mouse_curr_local - mouse_saved_local;
                    if !user_ui.magnetized {
                        local_delta = (local_delta / snap.linear()).round() * snap.linear();
                    }
                    let i_usize = i.rem_euclid(4) as usize;
                    let dragged_saved = self.vertices.val(i).saved();
                    self.vertices
                        .val_mut(i)
                        .set_curr(dragged_saved + local_delta);
                    if i % 2 == 1 {
                        let left = (i - 1).rem_euclid(4);
                        let right = (i + 1).rem_euclid(4);
                        let left_saved = self.vertices.val(left).saved();
                        let right_saved = self.vertices.val(right).saved();
                        self.vertices
                            .val_mut(left)
                            .set_curr(left_saved + Vec2::new(local_delta.x, 0.));
                        self.vertices
                            .val_mut(right)
                            .set_curr(right_saved + Vec2::new(0., local_delta.y));
                    } else {
                        let left = (i - 1).rem_euclid(4);
                        let right = (i + 1).rem_euclid(4);
                        let left_saved = self.vertices.val(left).saved();
                        let right_saved = self.vertices.val(right).saved();
                        self.vertices
                            .val_mut(left)
                            .set_curr(left_saved + Vec2::new(0., local_delta.y));
                        self.vertices
                            .val_mut(right)
                            .set_curr(right_saved + Vec2::new(local_delta.x, 0.));
                    }
                    // Opposite vertex (opp_idx) stays implicitly at saved() — not modified.
                    let _ = i_usize;
                    self.recompute_sag_vertices(nc);
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
                self.recompute_sag_vertices(nc);
                self.set_bezpath();
                true
            }
            ShapeType::Poly => {
                let nc = self.get_n_corners();
                let idx = match self.vertices.iter().position(|(uid, _)| *uid == value_uid) {
                    Some(i) => i,
                    None => return false,
                };
                if idx >= nc && self.vertices.len() > nc {
                    // Sag vertex drag
                    use crate::helpers::math::{poly_sag_from_vertex, poly_sag_handle};
                    let edge_idx = idx - nc;
                    let edge_start = self.vertices.val(edge_idx as i64).curr();
                    let edge_end = self.vertices.val(((edge_idx + 1) % nc) as i64).curr();
                    let mut sag = poly_sag_from_vertex(edge_start, edge_end, user_ui.draw_pos);
                    if sag.abs() < 5.0 { sag = 0.0; }
                    let new_pos = poly_sag_handle(edge_start, edge_end, sag);
                    self.vertices.val_mut(idx as i64).set_curr(new_pos);
                } else {
                    // Corner vertex drag
                    let idx = idx as i64;
                    let tmp = self.vertices.val_mut(idx).saved();
                    self.vertices.val_mut(idx).set_saved(snap_vertex(tmp, snap));
                    self.vertices.val_mut(idx).add(delta);
                    self.recompute_sag_vertices(nc);
                }
                self.set_bezpath();
                true
            }
            ShapeType::Group | ShapeType::Arrow => false,
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
        // Compute incremental delta from previous position
        let prev_pos = self.vertices.val(0).curr();
        for (_, value) in self.vertices.iter_mut() {
            value.add(delta);
        }
        let new_pos = self.vertices.val(0).curr();
        let incr_delta = new_pos - prev_pos;
        self.ghosts_rot_center = self.ghosts_rot_center_saved + delta;
        self.update_properties();
        // Translate bezpath + polygon directly (skip expensive rebuild)
        self.bezpath = translate_bezpath(&self.bezpath, incr_delta.x, incr_delta.y);
        self.polygon = self.polygon.translate(incr_delta.x, incr_delta.y);
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
        self.ghosts_rot_center_saved = self.ghosts_rot_center;
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
    pub fn set_group_polygon(&mut self, poly: MultiPolygon<f64>) {
        if let Some(group) = self.group_shape_data.as_mut() {
            group.cached_polygon = Some(poly);
        }
    }
    pub fn get_group_polygon(&self) -> Option<&MultiPolygon<f64>> {
        self.group_shape_data.as_ref()?.cached_polygon.as_ref()
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

    /// Update current rotation without saving (used during live drag preview).
    pub fn set_rotation_curr(&mut self, rotation: f64) {
        self.rotation.set_curr(rotation);
        self.set_bezpath();
    }

    // ── Ghost getters / setters ──────────────────────────────────────────
    pub fn get_ghosts(&self) -> usize {
        self.ghosts
    }
    pub fn get_ghosts_trans_x(&self) -> f64 {
        self.ghosts_trans_x
    }
    pub fn get_ghosts_trans_y(&self) -> f64 {
        self.ghosts_trans_y
    }
    pub fn get_ghosts_rot(&self) -> bool {
        self.ghosts_rot
    }
    pub fn get_ghosts_self_rot(&self) -> bool {
        self.ghosts_self_rot
    }
    pub fn get_ghosts_rot_center(&self) -> Vec2 {
        self.ghosts_rot_center
    }
    pub fn set_ghosts(&mut self, v: usize) {
        self.ghosts = v;
    }
    pub fn set_ghosts_trans_x(&mut self, v: f64) {
        self.ghosts_trans_x = v;
    }
    pub fn set_ghosts_trans_y(&mut self, v: f64) {
        self.ghosts_trans_y = v;
    }
    pub fn set_ghosts_rot(&mut self, v: bool) {
        self.ghosts_rot = v;
    }
    pub fn set_ghosts_self_rot(&mut self, v: bool) {
        self.ghosts_self_rot = v;
    }
    pub fn set_ghosts_rot_center(&mut self, v: Vec2) {
        self.ghosts_rot_center = v;
    }

    /// Recompute sag vertex positions preserving their sag values after corners move.
    fn recompute_sag_vertices(&mut self, nc: usize) {
        use crate::helpers::math::{poly_sag_from_vertex, poly_sag_handle};
        if self.vertices.len() <= nc { return; }
        for i in 0..nc {
            let j = (i + 1) % nc;
            let sag_idx = (nc + i) as i64;
            let old_start = self.vertices.val(i as i64).saved();
            let old_end = self.vertices.val(j as i64).saved();
            let old_sag = poly_sag_from_vertex(old_start, old_end, self.vertices.val(sag_idx).saved());
            let new_start = self.vertices.val(i as i64).curr();
            let new_end = self.vertices.val(j as i64).curr();
            let new_pos = poly_sag_handle(new_start, new_end, old_sag);
            self.vertices.val_mut(sag_idx).set_curr(new_pos);
        }
    }

    /// Number of corner vertices (excluding sag vertices).
    pub fn get_n_corners(&self) -> usize {
        match self.shape_type {
            ShapeType::Square => {
                if self.vertices.len() == 8 { 4 } else { self.vertices.len() }
            }
            ShapeType::Poly => {
                let n = self.vertices.len();
                if n >= 6 && n % 2 == 0 { n / 2 } else { n }
            }
            _ => self.vertices.len(),
        }
    }

    /// Compute the ghost copies of the current bezpath.
    pub fn get_ghost_bezpaths(&self) -> Vec<BezPath> {
        if self.ghosts == 0 {
            return vec![];
        }
        // Use the actual shape paths, not just the bounding box bezpath
        let base_paths: Vec<BezPath> = if self.shape_type == ShapeType::Group {
            if let Some(poly) = self.get_group_polygon() {
                geo_multipolygon_to_bez_paths(poly)
            } else {
                return vec![];
            }
        } else if let Some(paths) = self.get_text_paths() {
            paths.clone()
        } else if let Some(paths) = self.get_svg_paths() {
            paths.clone()
        } else {
            vec![self.bezpath.clone()]
        };
        let mut result = Vec::new();
        if self.ghosts_rot {
            let total = self.ghosts + 1;
            let center = self.ghosts_rot_center;
            for i in 1..=self.ghosts {
                let angle = 2.0 * PI * (i as f64) / (total as f64);
                for base in &base_paths {
                    if self.ghosts_self_rot {
                        result.push(_rotate_bezpath(base, center, angle));
                    } else {
                        let bbox = base.bounding_box();
                        let shape_center =
                            Vec2::new((bbox.x0 + bbox.x1) * 0.5, (bbox.y0 + bbox.y1) * 0.5);
                        let rotated_pos = rotate_vector(shape_center - center, angle) + center;
                        let delta = rotated_pos - shape_center;
                        result.push(translate_bezpath(base, delta.x, delta.y));
                    }
                }
            }
        } else {
            for i in 1..=self.ghosts {
                let dx = self.ghosts_trans_x * (i as f64);
                let dy = self.ghosts_trans_y * (i as f64);
                for base in &base_paths {
                    result.push(translate_bezpath(base, dx, dy));
                }
            }
        }
        result
    }

    /// World-space position of the rotation handle (14 px outside v[2] from center).
    /// Returns `None` for shape types that do not support rotation via the handle.
    pub fn rotation_handle_world_pos(&self, scale: f64) -> Option<Vec2> {
        if !matches!(
            self.shape_type,
            ShapeType::Square | ShapeType::Text | ShapeType::Svg | ShapeType::Voronoi
        ) {
            return None;
        }
        if self.vertices.len() < 4 {
            return None;
        }
        let v2_world = self.vertex_display_pos(self.vertices.val(2).curr());
        let center = self.bbox_center();
        let outward = v2_world - center;
        let dist = outward.hypot();
        if dist < 1e-9 {
            return None;
        }
        const OFFSET_PX: f64 = 14.0;
        Some(v2_world + outward / dist * (OFFSET_PX / scale))
    }

    pub fn get_dim_offsets(&self) -> [f64; 6] {
        self.dim_offsets
    }
    pub fn set_dim_offset(&mut self, idx: usize, val: f64) {
        if idx < 6 {
            self.dim_offsets[idx] = val;
        }
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
                if self.vertices.len() >= 6 {
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
                        let v4 = self.vertices.val(4).curr();
                        let v5 = self.vertices.val(5).curr();
                        // Check if sag vertices are at chord midpoints (sag ≈ 0)
                        let vs1 = if let Some((_, a2_s)) = oblong_straight_angles(c1, c2, r1, r2) {
                            let sag = sag_from_vertex(c1, c2, r1, r2, a2_s, v4);
                            if sag.abs() < 1e-6 {
                                None
                            } else {
                                Some(v4)
                            }
                        } else {
                            None
                        };
                        let vs2 = if let Some((a1_s, _)) = oblong_straight_angles(c1, c2, r1, r2) {
                            let sag = sag_from_vertex(c1, c2, r1, r2, a1_s, v5);
                            if sag.abs() < 1e-6 {
                                None
                            } else {
                                Some(v5)
                            }
                        } else {
                            None
                        };
                        if let Some(p) =
                            oblong_build_path(c1, c2, r1, r2, vs1, vs2, Self::TOLERANCE)
                        {
                            path = p;
                        }
                    }
                }
                self.bezpath = path;
            }
            ShapeType::Group => {
                let apices = self.vertices.get_apices();
                self.bezpath = bezpath_from_apices(&apices);
            }
            ShapeType::Square => {
                let nc = self.get_n_corners();
                let apices = self.vertices.get_apices_n(nc);
                let has_sag = self.vertices.len() > nc;
                if has_sag {
                    let sag_verts: Vec<Option<Vec2>> = (0..nc)
                        .map(|i| Some(self.vertices.val((nc + i) as i64).curr()))
                        .collect();
                    self.bezpath = build_polygon_path_with_sag(&apices, &sag_verts, Self::TOLERANCE);
                } else {
                    self.bezpath = bezpath_from_apices(&apices);
                }
            }
            ShapeType::Poly => {
                let nc = self.get_n_corners();
                let apices = self.vertices.get_apices_n(nc);
                let has_sag = self.vertices.len() > nc;
                if has_sag {
                    let sag_verts: Vec<Option<Vec2>> = (0..nc)
                        .map(|i| Some(self.vertices.val((nc + i) as i64).curr()))
                        .collect();
                    self.bezpath = build_polygon_path_with_sag(&apices, &sag_verts, Self::TOLERANCE);
                } else {
                    self.bezpath = bezpath_from_apices(&apices);
                }
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

    pub fn bbox_center_pub(&self) -> Vec2 {
        self.bbox_center()
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
        if self.shape_type == ShapeType::Group {
            // For Groups, translate/rotate the cached MultiPolygon directly
            // (preserves holes from Difference operations)
            if let Some(base) = self.get_group_polygon().cloned() {
                let mut result = base.clone();
                if self.ghosts > 0 {
                    let ghost_polys = self.group_ghost_multipolygons(&base);
                    for gp in ghost_polys {
                        result = result.boolean_op(&gp, geo::OpType::Union);
                    }
                }
                self.polygon = result;
            }
            return;
        }
        let base_poly = bez_path_to_geo_polygon(&self.bezpath);
        let mut result = MultiPolygon::new(vec![base_poly]);
        for ghost_path in self.get_ghost_bezpaths() {
            let ghost_poly = MultiPolygon::new(vec![bez_path_to_geo_polygon(&ghost_path)]);
            result = result.boolean_op(&ghost_poly, geo::OpType::Union);
        }
        self.polygon = result;
    }

    /// Generate ghost MultiPolygons for a Group (translated/rotated copies of base).
    fn group_ghost_multipolygons(&self, base: &MultiPolygon<f64>) -> Vec<MultiPolygon<f64>> {
        let mut ghosts = Vec::new();
        if self.ghosts_rot {
            let total = self.ghosts + 1;
            let center = self.ghosts_rot_center;
            for i in 1..=self.ghosts {
                let angle = 2.0 * PI * (i as f64) / (total as f64);
                let polys: Vec<geo::Polygon<f64>> = if self.ghosts_self_rot {
                    base.0.iter().map(|p| rotate_geo_polygon(p, center, angle)).collect()
                } else {
                    let bbox = self.bezpath.bounding_box();
                    let sc = Vec2::new((bbox.x0+bbox.x1)*0.5, (bbox.y0+bbox.y1)*0.5);
                    let rp = rotate_vector(sc - center, angle) + center;
                    let delta = rp - sc;
                    base.0.iter().map(|p| translate_geo_polygon(p, delta.x, delta.y)).collect()
                };
                ghosts.push(MultiPolygon::new(polys));
            }
        } else {
            for i in 1..=self.ghosts {
                let dx = self.ghosts_trans_x * (i as f64);
                let dy = self.ghosts_trans_y * (i as f64);
                let polys: Vec<geo::Polygon<f64>> = base.0.iter()
                    .map(|p| translate_geo_polygon(p, dx, dy))
                    .collect();
                ghosts.push(MultiPolygon::new(polys));
            }
        }
        ghosts
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
        // Add ghost copies to the polygon
        if self.ghosts > 0 {
            let mut all_polys: Vec<geo::Polygon<f64>> = self.polygon.0.clone();
            for ghost_path in self.get_ghost_bezpaths() {
                all_polys.push(bez_path_to_geo_polygon(&ghost_path));
            }
            self.polygon = MultiPolygon::new(all_polys);
        }
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
        // Add ghost copies to the polygon
        if self.ghosts > 0 {
            let mut all_polys: Vec<geo::Polygon<f64>> = self.polygon.0.clone();
            for ghost_path in self.get_ghost_bezpaths() {
                all_polys.push(bez_path_to_geo_polygon(&ghost_path));
            }
            self.polygon = MultiPolygon::new(all_polys);
        }
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

fn translate_geo_polygon(poly: &geo::Polygon<f64>, dx: f64, dy: f64) -> geo::Polygon<f64> {
    use geo::{Coord, LineString, Polygon};
    let translate_ls = |ls: &LineString<f64>| -> LineString<f64> {
        LineString(ls.0.iter().map(|c| Coord { x: c.x + dx, y: c.y + dy }).collect())
    };
    Polygon::new(translate_ls(poly.exterior()), poly.interiors().iter().map(translate_ls).collect())
}

fn rotate_geo_polygon(poly: &geo::Polygon<f64>, center: Vec2, angle: f64) -> geo::Polygon<f64> {
    use geo::{Coord, LineString, Polygon};
    let rotate_ls = |ls: &LineString<f64>| -> LineString<f64> {
        LineString(ls.0.iter().map(|c| {
            let v = rotate_vector(Vec2::new(c.x, c.y) - center, angle) + center;
            Coord { x: v.x, y: v.y }
        }).collect())
    };
    Polygon::new(rotate_ls(poly.exterior()), poly.interiors().iter().map(rotate_ls).collect())
}

/// Compute the straight-tangent geometry of a 4-vertex oblong (sag=0 case).
/// Returns (a1, a2) angles or None if degenerate.
pub(crate) fn oblong_straight_angles(c1: Vec2, c2: Vec2, r1: f64, r2: f64) -> Option<(f64, f64)> {
    let d = (c2 - c1).hypot();
    if d < EPSILON || d <= (r1 - r2).abs() {
        return None;
    }
    let base = (c2 - c1).atan2();
    let ratio = ((r1 - r2) / d).clamp(-1.0, 1.0);
    let offset = ratio.acos();
    Some((base + offset, base - offset))
}

/// Compute the handle position for a side arc.
/// Uses midpoint of the straight chord + normal * sag for monotonic tracking.
pub(crate) fn oblong_side_handle(
    c1: Vec2,
    c2: Vec2,
    r1: f64,
    r2: f64,
    straight_angle: f64,
    sag: f64,
    _is_side1: bool,
) -> Vec2 {
    let p1 = c1 + Vec2::new(r1 * straight_angle.cos(), r1 * straight_angle.sin());
    let p2 = c2 + Vec2::new(r2 * straight_angle.cos(), r2 * straight_angle.sin());
    let chord = p2 - p1;
    let l = chord.hypot();
    if l < 1e-9 {
        return (p1 + p2) * 0.5;
    }
    let u = chord / l;
    let n = Vec2::new(-u.y, u.x);
    (p1 + p2) * 0.5 + n * sag
}

/// Compute the sag value from a sag vertex position by projecting onto the chord normal.
fn sag_from_vertex(
    c1: Vec2,
    c2: Vec2,
    r1: f64,
    r2: f64,
    straight_angle: f64,
    vertex_pos: Vec2,
) -> f64 {
    let p1 = c1 + Vec2::new(r1 * straight_angle.cos(), r1 * straight_angle.sin());
    let p2 = c2 + Vec2::new(r2 * straight_angle.cos(), r2 * straight_angle.sin());
    let chord = p2 - p1;
    let l = chord.hypot();
    if l < 1e-9 {
        return 0.0;
    }
    let u = chord / l;
    let n = Vec2::new(-u.y, u.x);
    let mid = (p1 + p2) * 0.5;
    let delta = vertex_pos - mid;
    delta.x * n.x + delta.y * n.y
}

/// Find all arc centers O and radii R such that the arc passes through point V
/// and is externally (or internally) tangent to both circles (c1,r1) and (c2,r2).
/// Returns all valid (O, R) solutions with R > 0.
fn arc_through_point_tangent(
    c1: Vec2,
    c2: Vec2,
    r1: f64,
    r2: f64,
    v: Vec2,
    internal: bool,
) -> Vec<(Vec2, f64)> {
    let sr1 = if internal { -r1 } else { r1 };
    let sr2 = if internal { -r2 } else { r2 };
    let u = v - c1;
    let w = v - c2;
    let det = u.x * w.y - u.y * w.x;
    if det.abs() < 1e-12 {
        return vec![];
    }
    let alpha = (sr1 * sr1 + v.x * v.x + v.y * v.y - c1.x * c1.x - c1.y * c1.y) / 2.0;
    let beta = (sr2 * sr2 + v.x * v.x + v.y * v.y - c2.x * c2.x - c2.y * c2.y) / 2.0;
    let o0 = Vec2::new(
        (w.y * alpha - u.y * beta) / det,
        (u.x * beta - w.x * alpha) / det,
    );
    let d_o = Vec2::new((w.y * sr1 - u.y * sr2) / det, (u.x * sr2 - w.x * sr1) / det);
    let pv = o0 - v;
    let a = d_o.x * d_o.x + d_o.y * d_o.y - 1.0;
    let b = 2.0 * (pv.x * d_o.x + pv.y * d_o.y);
    let c = pv.x * pv.x + pv.y * pv.y;
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return vec![];
    }
    let sq = disc.sqrt();
    let mut results = Vec::new();
    for r in [(-b + sq) / (2.0 * a), (-b - sq) / (2.0 * a)] {
        if r > 0.0 {
            results.push((o0 + d_o * r, r));
        }
    }
    results
}

/// Compute the sweep angle from p_start to p_end (around center O) that contains point V.
/// Returns either the CW or CCW sweep, whichever includes V.
fn sweep_through_point(o: Vec2, p_start: Vec2, p_end: Vec2, v: Vec2) -> f64 {
    let a_start = (p_start - o).y.atan2((p_start - o).x);
    let a_end = (p_end - o).y.atan2((p_end - o).x);
    let a_v = (v - o).y.atan2((v - o).x);

    // Two possible sweeps: CW (negative) and CCW (positive)
    let mut sweep_ccw = a_end - a_start;
    if sweep_ccw < 0.0 {
        sweep_ccw += 2.0 * PI;
    }
    let sweep_cw = sweep_ccw - 2.0 * PI; // negative

    // Check which sweep contains the angle of V
    let mut dv = a_v - a_start;
    if dv < 0.0 {
        dv += 2.0 * PI;
    }
    // dv is now in [0, 2π). If dv < sweep_ccw, V is in the CCW arc.
    if dv <= sweep_ccw { sweep_ccw } else { sweep_cw }
}

/// Build a polygon BezPath with optional arc sides (sag vertices) and filleted corners.
/// `apices`: the corner apices (from get_apices_n)
/// `sag_positions`: one sag vertex per edge (None = straight). sag[i] = edge corner[i]→corner[(i+1)%n]
pub(crate) fn build_polygon_path_with_sag(
    apices: &[ApexType],
    sag_positions: &[Option<Vec2>],
    tolerance: f64,
) -> BezPath {
    use crate::helpers::math::{circle_from_three_points, poly_sag_from_vertex};
    let n = apices.len();
    assert_eq!(sag_positions.len(), n);

    let entry_of = |apex: &ApexType| -> Vec2 {
        match *apex { ApexType::TypeVertex { a } => a, ApexType::Arc { s, .. } => s }
    };
    let exit_of = |apex: &ApexType| -> Vec2 {
        match *apex { ApexType::TypeVertex { a } => a, ApexType::Arc { e, .. } => e }
    };
    let corner_pos = |apex: &ApexType| -> Vec2 {
        match *apex { ApexType::TypeVertex { a } => a, ApexType::Arc { s, c, e } => (s + e) * 0.5 }
    };

    // Determine polygon winding direction (CW or CCW) via shoelace formula
    let mut area2 = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        let pi = corner_pos(&apices[i]);
        let pj = corner_pos(&apices[j]);
        area2 += pi.x * pj.y - pj.x * pi.y;
    }
    let winding_ccw = area2 > 0.0;

    // --- Pass 1: compute adjusted entry/exit points for each corner ---
    let mut adj_entry: Vec<Vec2> = (0..n).map(|i| entry_of(&apices[i])).collect();
    let mut adj_exit: Vec<Vec2> = (0..n).map(|i| exit_of(&apices[i])).collect();
    // Store sag arc data for each edge: (arc_center, arc_radius, sag_vertex)
    let mut edge_arcs: Vec<Option<(Vec2, f64, Vec2)>> = vec![None; n];

    for i in 0..n {
        let j = (i + 1) % n;
        let Some(v) = sag_positions[i] else { continue };
        let edge_start = exit_of(&apices[i]);
        let edge_end = entry_of(&apices[j]);
        let sag = poly_sag_from_vertex(edge_start, edge_end, v);
        if sag.abs() < 1e-6 { continue; }

        let fillet_i = match apices[i] { ApexType::Arc { c, .. } => Some((c, (exit_of(&apices[i]) - c).hypot())), _ => None };
        let fillet_j = match apices[j] { ApexType::Arc { c, .. } => Some((c, (entry_of(&apices[j]) - c).hypot())), _ => None };

        match (fillet_i, fillet_j) {
            (Some((c1, r1)), Some((c2, r2))) => {
                if let Some((ac1, ac2, o, big_r)) = solve_side(c1, c2, r1, r2, v, edge_start, edge_end) {
                    // solve_side already handles internal/external in ac1/ac2
                    adj_exit[i] = c1 + Vec2::new(r1 * ac1.cos(), r1 * ac1.sin());
                    adj_entry[j] = c2 + Vec2::new(r2 * ac2.cos(), r2 * ac2.sin());
                    edge_arcs[i] = Some((o, big_r, v));
                }
            }
            (Some((c1, r1)), None) => {
                if let Some((ac1, _, o, big_r)) = solve_side(c1, edge_end, r1, 0.0, v, edge_start, edge_end) {
                    adj_exit[i] = c1 + Vec2::new(r1 * ac1.cos(), r1 * ac1.sin());
                    edge_arcs[i] = Some((o, big_r, v));
                }
            }
            (None, Some((c2, r2))) => {
                if let Some((_, ac2, o, big_r)) = solve_side(edge_start, c2, 0.0, r2, v, edge_start, edge_end) {
                    adj_entry[j] = c2 + Vec2::new(r2 * ac2.cos(), r2 * ac2.sin());
                    edge_arcs[i] = Some((o, big_r, v));
                }
            }
            (None, None) => {
                // point-arc-point: no fillet adjustment needed, just store the circle
                if let Some((center, radius)) = circle_from_three_points(edge_start, v, edge_end) {
                    edge_arcs[i] = Some((center, radius, v));
                }
            }
        }
    }

    // --- Pass 2: draw the path ---
    let mut path = BezPath::new();
    path.move_to(adj_exit[0].to_point());

    for i in 0..n {
        let j = (i + 1) % n;

        // Draw edge i → j
        if let Some((o, big_r, v)) = edge_arcs[i] {
            let start = adj_exit[i];
            let end = adj_entry[j];
            let sweep = sweep_through_point(o, start, end, v);
            let a_start = (start - o).y.atan2((start - o).x);
            let arc = Arc::new(o.to_point(), Vec2::new(big_r, big_r), a_start, sweep, 0.0);
            let mut elems = arc.path_elements(tolerance);
            elems.next();
            path.extend(elems);
        } else {
            path.push(PathEl::LineTo(adj_entry[j].to_point()));
        }

        // Draw fillet arc at corner j — always bulges outward (same winding as polygon)
        if let ApexType::Arc { s, c, .. } = apices[j] {
            let r = (s - c).hypot();
            let a_start = (adj_entry[j] - c).y.atan2((adj_entry[j] - c).x);
            let a_end = (adj_exit[j] - c).y.atan2((adj_exit[j] - c).x);
            let mut sweep = a_end - a_start;
            if winding_ccw {
                // CCW polygon → fillet sweep must be positive (CCW)
                if sweep < 0.0 { sweep += 2.0 * PI; }
            } else {
                // CW polygon → fillet sweep must be negative (CW)
                if sweep > 0.0 { sweep -= 2.0 * PI; }
            }
            let arc = Arc::new(c.to_point(), Vec2::new(r, r), a_start, sweep, 0.0);
            arc.to_cubic_beziers(0.01, |p1, p2, p3| {
                path.push(PathEl::CurveTo(p1, p2, p3));
            });
        }
    }

    path.close_path();
    path
}

/// Find the best arc through V tangent to both circles (c1,r1) and (c2,r2).
/// chord_start/chord_end: the straight-line tangent points (for reference).
/// Returns (angle_on_c1, angle_on_c2, arc_center, arc_radius) or None.
pub(crate) fn solve_side(
    c1: Vec2, c2: Vec2, r1: f64, r2: f64, v: Vec2,
    chord_start: Vec2, chord_end: Vec2,
) -> Option<(f64, f64, Vec2, f64)> {
    let chord = chord_end - chord_start;
    let chord_len = chord.hypot();
    if chord_len < 1e-9 { return None; }
    let chord_n = Vec2::new(-chord.y / chord_len, chord.x / chord_len);
    let chord_mid = (chord_start + chord_end) * 0.5;
    let v_side = (v - chord_mid).x * chord_n.x + (v - chord_mid).y * chord_n.y;

    let axis_mid = (c1 + c2) * 0.5;
    let chord_offset = (chord_mid - axis_mid).x * chord_n.x + (chord_mid - axis_mid).y * chord_n.y;
    let v_away = v_side * chord_offset > 0.0;
    let to_chord = chord_mid - v;

    let mut best: Option<(f64, f64, Vec2, f64)> = None;
    for &try_internal in &[false, true] {
        for (o, big_r) in arc_through_point_tangent(c1, c2, r1, r2, v, try_internal) {
            let internal = (o - c1).hypot() < big_r;
            let (ac1, ac2) = if internal {
                ((c1 - o).y.atan2((c1 - o).x), (c2 - o).y.atan2((c2 - o).x))
            } else {
                ((o - c1).y.atan2((o - c1).x), (o - c2).y.atan2((o - c2).x))
            };
            let o_toward_chord = (o - v).x * to_chord.x + (o - v).y * to_chord.y;
            if o_toward_chord <= 0.0 { continue; }
            let dominated = if v_away {
                best.as_ref().map_or(false, |b| big_r <= b.3)
            } else {
                best.as_ref().map_or(false, |b| big_r >= b.3)
            };
            if !dominated {
                best = Some((ac1, ac2, o, big_r));
            }
        }
    }
    best
}

/// Build the full oblong BezPath with optional arc sides.
/// v1, v2: sag vertex positions (or None for straight sides).
pub(crate) fn oblong_build_path(
    c1: Vec2,
    c2: Vec2,
    r1: f64,
    r2: f64,
    v_side1: Option<Vec2>,
    v_side2: Option<Vec2>,
    tolerance: f64,
) -> Option<BezPath> {
    let d = (c2 - c1).hypot();
    if d < EPSILON {
        return None;
    }
    let (a1_straight, a2_straight) = oblong_straight_angles(c1, c2, r1, r2)?;

    // Side 1: arc goes from c1 to c2
    let (s1_angle_c1, s1_angle_c2, s1_arc) = if let Some(v) = v_side1 {
        let cs = c1 + Vec2::new(r1 * a2_straight.cos(), r1 * a2_straight.sin());
        let ce = c2 + Vec2::new(r2 * a2_straight.cos(), r2 * a2_straight.sin());
        if let Some((ac1, ac2, o, big_r)) = solve_side(c1, c2, r1, r2, v, cs, ce) {
            (ac1, ac2, Some((o, big_r)))
        } else {
            (a2_straight, a2_straight, None)
        }
    } else {
        (a2_straight, a2_straight, None)
    };

    // Side 2: arc goes from c2 to c1
    let (s2_angle_c2, s2_angle_c1, s2_arc) = if let Some(v) = v_side2 {
        let cs = c2 + Vec2::new(r2 * a1_straight.cos(), r2 * a1_straight.sin());
        let ce = c1 + Vec2::new(r1 * a1_straight.cos(), r1 * a1_straight.sin());
        if let Some((ac1, ac2, o, big_r)) = solve_side(c1, c2, r1, r2, v, cs, ce) {
            (ac2, ac1, Some((o, big_r)))
        } else {
            (a1_straight, a1_straight, None)
        }
    } else {
        (a1_straight, a1_straight, None)
    };

    let mut path = BezPath::new();

    // Arc at c1: from s2_angle_c1 to s1_angle_c1 (the long way, through the "outside")
    let mut sweep_c1 = s1_angle_c1 - s2_angle_c1;
    if sweep_c1 <= 0.0 {
        sweep_c1 += 2.0 * PI;
    }
    path.extend(
        Arc::new(c1.to_point(), Vec2::new(r1, r1), s2_angle_c1, sweep_c1, 0.0)
            .path_elements(tolerance),
    );

    // Side 1: from tangent point on c1 to tangent point on c2
    let p1_s1 = c1 + Vec2::new(r1 * s1_angle_c1.cos(), r1 * s1_angle_c1.sin());
    let p2_s1 = c2 + Vec2::new(r2 * s1_angle_c2.cos(), r2 * s1_angle_c2.sin());
    if let Some((o, big_r)) = s1_arc {
        let sweep = sweep_through_point(o, p1_s1, p2_s1, v_side1.unwrap_or(p2_s1));
        let a_start = (p1_s1 - o).y.atan2((p1_s1 - o).x);
        let arc = Arc::new(o.to_point(), Vec2::new(big_r, big_r), a_start, sweep, 0.0);
        let mut elems = arc.path_elements(tolerance);
        elems.next();
        path.extend(elems);
    } else {
        path.push(PathEl::LineTo(p2_s1.to_point()));
    }

    // Arc at c2: from s1_angle_c2 to s2_angle_c2
    let mut sweep_c2 = s2_angle_c2 - s1_angle_c2;
    if sweep_c2 <= 0.0 {
        sweep_c2 += 2.0 * PI;
    }
    let mut arc2 = Arc::new(c2.to_point(), Vec2::new(r2, r2), s1_angle_c2, sweep_c2, 0.0)
        .path_elements(tolerance);
    arc2.next(); // skip MoveTo
    path.extend(arc2);

    // Side 2: from tangent point on c2 to tangent point on c1
    let p1_s2 = c2 + Vec2::new(r2 * s2_angle_c2.cos(), r2 * s2_angle_c2.sin());
    let p2_s2 = c1 + Vec2::new(r1 * s2_angle_c1.cos(), r1 * s2_angle_c1.sin());
    if let Some((o, big_r)) = s2_arc {
        let sweep = sweep_through_point(o, p1_s2, p2_s2, v_side2.unwrap_or(p2_s2));
        let a_start = (p1_s2 - o).y.atan2((p1_s2 - o).x);
        let arc = Arc::new(o.to_point(), Vec2::new(big_r, big_r), a_start, sweep, 0.0);
        let mut elems = arc.path_elements(tolerance);
        elems.next();
        path.extend(elems);
    } else {
        path.push(PathEl::LineTo(p2_s2.to_point()));
    }

    path.push(PathEl::ClosePath);
    Some(path)
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

pub(crate) fn rotate_bezpath_pub(path: &BezPath, center: Vec2, angle: f64) -> BezPath {
    _rotate_bezpath(path, center, angle)
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
