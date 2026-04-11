use crate::helpers::math::*;
use crate::inputs::UserUI;
use crate::shape::{GeneralShape, Operation, ShapeType};
use crate::types::vertex::Vertex;
use crate::types::others::{EUId, Property, PropertyValue, Snap, VUId};
use geo::algorithm::unary_union;
use geo::{BooleanOps, Coord, LineString, MultiPolygon, Polygon};
use kurbo::{flatten, stroke, BezPath, Cap, Join, PathEl, Rect, Shape, Stroke, StrokeOpts, Vec2};
use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;

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

// Ensure exterior CCW and holes CW before boolean ops.
fn normalize_polygon_orientation(poly: &Polygon<f64>) -> Polygon<f64> {
    let mut exterior = poly.exterior().clone();
    if ring_signed_area(&exterior) < 0.0 {
        exterior.0.reverse();
    }
    let mut interiors = Vec::new();
    for ring in poly.interiors() {
        let mut inner = ring.clone();
        if ring_signed_area(&inner) > 0.0 {
            inner.0.reverse();
        }
        interiors.push(inner);
    }
    Polygon::new(exterior, interiors)
}

pub struct CutParams {
    pub feed_xy: f64,               // mm/min
    pub pierce_delay_s: f64,        // secondes (0.0 si none)
    pub torch_on_m3: &'static str,  // "M3" ou "M3 Sxxx" selon contrôleur
    pub torch_off_m5: &'static str, // "M5"
}

#[derive(Clone, Debug)]
pub struct ToolpathParams {
    pub feed_xy: f64,        // mm/min
    pub travel_feed_xy: f64, // mm/min (déplacements sans coupe)
    pub pierce_delay_s: f64, // secondes (0.0 si none)
    pub lead_in_mm: f64,     // lead-in
    pub lead_out_mm: f64,    // lead-out
    pub kerf_mm: f64,        // largeur de jet
    pub torch_on_m3: String,
    pub torch_off_m5: String,
}

#[derive(Clone, Debug)]
pub struct ToolpathContour {
    pub points: Vec<Vec2>,
    pub is_hole: bool,
    pub lead_in: Option<LeadArc>,
    pub lead_out: Option<LeadArc>,
    pub pierce_delay_s: f64,
}

#[derive(Clone, Debug)]
pub struct Toolpath {
    pub contours: Vec<ToolpathContour>,
}

#[derive(Clone, Copy, Debug)]
pub struct LeadArc {
    pub center: Vec2,
    pub start: Vec2,
    pub end: Vec2,
    pub ccw: bool,
}

impl Toolpath {
    pub fn new(contours: Vec<ToolpathContour>) -> Self {
        Toolpath { contours }
    }
}

fn ring_to_points(ring: &LineString<f64>) -> Vec<(f64, f64)> {
    let coords = ring.0.as_slice();
    if coords.is_empty() {
        return vec![];
    }

    let mut pts: Vec<(f64, f64)> = coords.iter().map(|c| (c.x, c.y)).collect();
    if pts.len() >= 2 && pts.first() == pts.last() {
        pts.pop();
    }
    pts
}

fn ring_to_vec2(ring: &LineString<f64>) -> Vec<Vec2> {
    ring_to_points(ring)
        .into_iter()
        .map(|(x, y)| Vec2::new(x, y))
        .collect()
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

fn points_to_bezpath(points: &[Vec2]) -> BezPath {
    let mut path = BezPath::new();
    if let Some(first) = points.first() {
        path.push(PathEl::MoveTo(first.to_point()));
        for pt in points.iter().skip(1) {
            path.push(PathEl::LineTo(pt.to_point()));
        }
        path.push(PathEl::ClosePath);
    }
    path
}

fn vec2_to_linestring(points: &[Vec2]) -> LineString<f64> {
    let mut coords: Vec<Coord<f64>> = points.iter().map(|pt| Coord { x: pt.x, y: pt.y }).collect();
    if coords.len() >= 2 && coords.first() != coords.last() {
        coords.push(coords[0]);
    }
    LineString::from(coords)
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

fn bez_path_to_rings(path: &BezPath, tolerance: f64) -> Vec<Vec<Vec2>> {
    let mut flat = BezPath::new();
    flatten(path, tolerance, |seg| flat.push(seg));

    let mut rings = Vec::new();
    let mut current: Vec<Vec2> = Vec::new();

    for element in flat.elements() {
        match element {
            PathEl::MoveTo(p) => {
                if !current.is_empty() {
                    rings.push(current);
                    current = Vec::new();
                }
                current.push(Vec2::new(p.x, p.y));
            }
            PathEl::LineTo(p) => {
                current.push(Vec2::new(p.x, p.y));
            }
            PathEl::ClosePath => {
                if !current.is_empty() {
                    rings.push(current);
                    current = Vec::new();
                }
            }
            _ => {}
        }
    }
    if !current.is_empty() {
        rings.push(current);
    }

    rings
}

fn offset_contour(points: &[Vec2], kerf_mm: f64, outside: bool) -> Vec<Vec2> {
    if points.len() < 2 || kerf_mm <= 0.0 {
        return points.to_vec();
    }
    let path = points_to_bezpath(points);
    let stroke_style = Stroke::new(kerf_mm)
        .with_join(Join::Round)
        .with_caps(Cap::Round);
    let stroked = stroke(&path, &stroke_style, &StrokeOpts::default(), 0.01);
    let rings = bez_path_to_rings(&stroked, 0.05);
    if rings.is_empty() {
        return points.to_vec();
    }

    let mut original = points.to_vec();
    ensure_orientation(&mut original, false);
    let original_poly = Polygon::new(vec2_to_linestring(&original), vec![]);

    let mut polys = Vec::new();
    for ring in rings {
        let ring = normalize_ring(ring);
        if ring.len() < 3 {
            continue;
        }
        polys.push(Polygon::new(vec2_to_linestring(&ring), vec![]));
    }
    if polys.is_empty() {
        return points.to_vec();
    }

    let unioned = unary_union(&polys);
    let clipped = if outside {
        unioned.difference(&original_poly)
    } else {
        unioned.intersection(&original_poly)
    };
    let mut best: Option<(f64, Vec<Vec2>)> = None;

    if outside {
        for poly in clipped.0.iter() {
            let ring = poly.exterior();
            if ring.0.len() < 3 {
                continue;
            }
            let mut pts: Vec<Vec2> = ring.0.iter().map(|c| Vec2::new(c.x, c.y)).collect();
            pts = normalize_ring(pts);
            let area = ring_area(&pts).abs();
            match best {
                None => best = Some((area, pts)),
                Some((best_area, _)) if area > best_area => best = Some((area, pts)),
                Some(_) => {}
            }
        }
    } else {
        for poly in clipped.0.iter() {
            for inner in poly.interiors().iter() {
                if inner.0.len() < 3 {
                    continue;
                }
                let mut pts: Vec<Vec2> = inner.0.iter().map(|c| Vec2::new(c.x, c.y)).collect();
                pts = normalize_ring(pts);
                let area = ring_area(&pts).abs();
                match best {
                    None => best = Some((area, pts)),
                    Some((best_area, _)) if area > best_area => best = Some((area, pts)),
                    Some(_) => {}
                }
            }
        }
        if best.is_none() {
            for poly in clipped.0.iter() {
                let ring = poly.exterior();
                if ring.0.len() < 3 {
                    continue;
                }
                let mut pts: Vec<Vec2> = ring.0.iter().map(|c| Vec2::new(c.x, c.y)).collect();
                pts = normalize_ring(pts);
                let area = ring_area(&pts).abs();
                match best {
                    None => best = Some((area, pts)),
                    Some((best_area, _)) if area > best_area => best = Some((area, pts)),
                    Some(_) => {}
                }
            }
        }
    }

    best.map(|(_, ring)| ring)
        .unwrap_or_else(|| points.to_vec())
}

fn shift_lead_to_mid(points: &[Vec2]) -> Vec<Vec2> {
    let count = points.len();
    if count < 2 {
        return points.to_vec();
    }

    let mut best_idx = 0usize;
    let mut best_len = 0.0;
    for i in 0..count {
        let a = points[i];
        let b = points[(i + 1) % count];
        let seg_len = (b - a).hypot();
        if seg_len > best_len {
            best_len = seg_len;
            best_idx = i;
        }
    }

    let a = points[best_idx];
    let b = points[(best_idx + 1) % count];
    let mid = (a + b) * 0.5;
    let start_idx = (best_idx + 1) % count;
    let mut shifted = Vec::with_capacity(count + 1);
    shifted.push(mid);
    for k in 0..count {
        let idx = (start_idx + k) % count;
        shifted.push(points[idx]);
    }
    shifted
}

fn ensure_orientation(points: &mut [Vec2], want_cw: bool) {
    let area = ring_area(points);
    let is_cw = area < 0.0;
    if is_cw != want_cw {
        points.reverse();
    }
}

fn build_leads(
    points: &[Vec2],
    lead_in: f64,
    lead_out: f64,
    is_hole: bool,
) -> (Option<LeadArc>, Option<LeadArc>) {
    if points.len() < 2 {
        return (None, None);
    }
    let lead_in_radius = lead_in * 2.0 / PI;
    let lead_out_radius = lead_out * 2.0 / PI;
    let p0 = points[0];
    let p1 = points[1];
    let mut lead_in_seg = None;
    let area = ring_area(points);
    let is_cw = area < 0.0;
    let v = p1 - p0;
    let mut normal_in = Vec2::ZERO;
    if v.hypot() > 0.0001 {
        let dir = v.normalize();
        let left = Vec2::new(-dir.y, dir.x);
        let interior = if is_cw { -left } else { left };
        normal_in = if is_hole { interior } else { -interior };
    }
    if lead_in_radius > 0.0 && normal_in.hypot() > 0.0001 {
        let dir = v.normalize();
        let center = p0 + normal_in * lead_in_radius;
        let start = center - dir * lead_in_radius;
        let end = p0;
        let rvec_end = end - center;
        let ccw = rvec_end.cross(dir) > 0.0;
        lead_in_seg = Some(LeadArc {
            center,
            start,
            end,
            ccw,
        });
    }

    let mut lead_out_seg = None;
    let plast = points[points.len() - 1];
    let v = p0 - plast;
    let mut normal_out = Vec2::ZERO;
    if v.hypot() > 0.0001 {
        let dir = v.normalize();
        let left = Vec2::new(-dir.y, dir.x);
        let interior = if is_cw { -left } else { left };
        normal_out = if is_hole { interior } else { -interior };
    }
    if normal_in.hypot() > 0.0001 && normal_out.hypot() > 0.0001 && normal_in.dot(normal_out) < 0.0
    {
        normal_out = -normal_out;
    }
    if lead_out_radius > 0.0 && normal_out.hypot() > 0.0001 {
        let dir = v.normalize();
        let center = p0 + normal_out * lead_out_radius;
        let start = p0;
        let end = center + dir * lead_out_radius;
        let rvec_start = start - center;
        let ccw = rvec_start.cross(dir) > 0.0;
        lead_out_seg = Some(LeadArc {
            center,
            start,
            end,
            ccw,
        });
    }

    (lead_in_seg, lead_out_seg)
}

pub fn multipolygon_to_toolpath(mp: &MultiPolygon<f64>, p: &ToolpathParams) -> Toolpath {
    let mut contours = Vec::new();

    for poly in mp.0.iter() {
        for hole in poly.interiors() {
            let mut points = ring_to_vec2(hole);
            if points.len() < 2 {
                continue;
            }
            ensure_orientation(&mut points, true);
            let mut points = offset_contour(&points, p.kerf_mm, false);
            ensure_orientation(&mut points, true);
            let points = shift_lead_to_mid(&points);
            let (lead_in, lead_out) = build_leads(&points, p.lead_in_mm, p.lead_out_mm, true);
            contours.push(ToolpathContour {
                points,
                is_hole: true,
                lead_in,
                lead_out,
                pierce_delay_s: p.pierce_delay_s,
            });
        }

        let mut points = ring_to_vec2(poly.exterior());
        if points.len() < 2 {
            continue;
        }
        ensure_orientation(&mut points, false);
        let mut points = offset_contour(&points, p.kerf_mm, true);
        ensure_orientation(&mut points, false);
        let points = shift_lead_to_mid(&points);
        let (lead_in, lead_out) = build_leads(&points, p.lead_in_mm, p.lead_out_mm, false);
        contours.push(ToolpathContour {
            points,
            is_hole: false,
            lead_in,
            lead_out,
            pierce_delay_s: p.pierce_delay_s,
        });
    }

    Toolpath::new(contours)
}

pub fn toolpath_to_plasma_gcode(tp: &Toolpath, p: &ToolpathParams) -> String {
    let mut out = String::new();
    out.push_str("G21 ; mm\n");
    out.push_str("G17 ; XY plane\n");
    out.push_str("G90 ; absolute\n");
    out.push_str("G94 ; feed per minute\n");
    out.push_str(&p.torch_off_m5);
    out.push('\n');

    for contour in tp.contours.iter() {
        if contour.points.len() < 2 {
            continue;
        }
        let p0 = contour.points[0];
        let move_start = contour.lead_in.map(|lead| lead.start).unwrap_or(p0);
        out.push_str(&p.torch_off_m5);
        out.push('\n');
        if p.travel_feed_xy > 0.0 {
            out.push_str(&format!("G1 F{:.1}\n", p.travel_feed_xy));
            out.push_str(&format!("G1 X{:.3} Y{:.3}\n", move_start.x, move_start.y));
        } else {
            out.push_str(&format!("G0 X{:.3} Y{:.3}\n", move_start.x, move_start.y));
        }
        out.push_str(&p.torch_on_m3);
        out.push('\n');
        if contour.pierce_delay_s > 0.0 {
            out.push_str(&format!("G4 P{:.3}\n", contour.pierce_delay_s));
        }

        out.push_str(&format!("G1 F{:.1}\n", p.feed_xy));
        if let Some(lead_in) = contour.lead_in {
            let cmd = if lead_in.ccw { "G3" } else { "G2" };
            let i = lead_in.center.x - lead_in.start.x;
            let j = lead_in.center.y - lead_in.start.y;
            out.push_str(&format!(
                "{} X{:.3} Y{:.3} I{:.3} J{:.3}\n",
                cmd, lead_in.end.x, lead_in.end.y, i, j
            ));
        }
        for pt in contour.points.iter().skip(1) {
            out.push_str(&format!("G1 X{:.3} Y{:.3}\n", pt.x, pt.y));
        }
        out.push_str(&format!("G1 X{:.3} Y{:.3}\n", p0.x, p0.y));

        if let Some(lead_out) = contour.lead_out {
            let cmd = if lead_out.ccw { "G3" } else { "G2" };
            let i = lead_out.center.x - lead_out.start.x;
            let j = lead_out.center.y - lead_out.start.y;
            out.push_str(&format!(
                "{} X{:.3} Y{:.3} I{:.3} J{:.3}\n",
                cmd, lead_out.end.x, lead_out.end.y, i, j
            ));
        }

        out.push_str(&p.torch_off_m5);
        out.push('\n');
    }

    out.push_str(&p.torch_off_m5);
    out.push('\n');
    out.push_str("M2\n");
    out
}

// ── User-placed dimensions ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimKind {
    Horizontal, // distance H between two vertices
    Vertical,   // distance V between two vertices
    Aligned,    // distance // segment between two vertices
    Radius,     // radius dimension on a single vertex (Radius/SagHandle/Apex+r/BBox+r)
}

impl DimKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Horizontal => "h",
            Self::Vertical => "v",
            Self::Aligned => "aligned",
            Self::Radius => "radius",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "v" => Self::Vertical,
            "aligned" => Self::Aligned,
            "radius" => Self::Radius,
            _ => Self::Horizontal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimToolMode {
    None,
    Horizontal,
    Vertical,
    Aligned,
    Radius,
}

#[derive(Debug, Clone)]
pub struct UserDimension {
    pub id: usize,
    pub kind: DimKind,
    pub eid1: EUId,
    pub vid1: VUId,
    /// For H/V/Aligned: the second vertex. For Radius: unused.
    pub eid2: EUId,
    pub vid2: VUId,
    pub offset: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct RadiusInfo {
    pub center: Vec2,
    pub radius: f64,
}

// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct DataSet {
    pub shapes: HashMap<EUId, GeneralShape>,
    pub grouped_shapes: HashMap<EUId, GeneralShape>,
    pub shapes_selected: HashSet<EUId>,
    pub shapes_highlighted: HashSet<EUId>,
    pub shapes_selector: ShapeSelector,
    pub vertex_selected: Option<(EUId, VUId)>,
    pub vertex_highlighted: Option<(EUId, VUId)>,
    next_order: i32,

    pub final_polygon: MultiPolygon<f64>,
    pub final_paths: Vec<BezPath>,
    pub final_polygon_dirty: bool,

    pub user_dims: Vec<UserDimension>,
    user_dim_next_id: usize,
}
impl Default for DataSet {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSet {
    pub fn new() -> Self {
        DataSet {
            shapes: HashMap::new(),
            grouped_shapes: HashMap::new(),
            shapes_selected: HashSet::new(),
            shapes_highlighted: HashSet::new(),
            shapes_selector: ShapeSelector::new(),
            vertex_selected: None,
            vertex_highlighted: None,
            final_polygon: MultiPolygon::new(vec![]),
            final_paths: Vec::new(),
            final_polygon_dirty: true,
            next_order: 0,
            user_dims: Vec::new(),
            user_dim_next_id: 0,
        }
    }
    pub fn mark_final_polygon_dirty(&mut self) {
        self.final_polygon_dirty = true;
    }
    pub fn add_user_dim(&mut self, dim: UserDimension) -> usize {
        let id = self.user_dim_next_id;
        self.user_dim_next_id += 1;
        self.user_dims.push(UserDimension { id, ..dim });
        id
    }
    pub fn remove_user_dim(&mut self, id: usize) {
        self.user_dims.retain(|d| d.id != id);
    }
    /// Remove user dimensions referencing missing shapes/vertices, or radius dims whose
    /// vertex no longer carries a radius.
    pub fn cleanup_user_dims(&mut self) {
        let valid: Vec<bool> = self
            .user_dims
            .iter()
            .map(|d| {
                let v1_ok = self.resolve_vertex_pos(d.eid1, d.vid1).is_some();
                if d.kind == DimKind::Radius {
                    v1_ok && self.resolve_radius_info(d.eid1, d.vid1).is_some()
                } else {
                    v1_ok && self.resolve_vertex_pos(d.eid2, d.vid2).is_some()
                }
            })
            .collect();
        let mut i = 0;
        self.user_dims.retain(|_| {
            let keep = valid[i];
            i += 1;
            keep
        });
    }
    /// Resolve a vertex reference to its current world position.
    pub fn resolve_vertex_pos(&self, eid: EUId, vid: VUId) -> Option<Vec2> {
        let shape = self.shapes.get(&eid)
            .or_else(|| self.grouped_shapes.get(&eid))?;
        let vertex = shape.get_vertex(&vid)?;
        Some(shape.vertex_display_pos(vertex.curr()))
    }
    /// Compute the (p1, p2) endpoints of the dimension line in world space for a given dim.
    /// Returns (line_p1, line_p2) — the actual dimension line, not the reference vertices.
    /// For Radius: returns (center, edge_point) along the radius direction.
    pub fn user_dim_line(&self, dim: &UserDimension) -> Option<(Vec2, Vec2)> {
        if dim.kind == DimKind::Radius {
            let info = self.resolve_radius_info(dim.eid1, dim.vid1)?;
            // Line from center to a point on the circle (using offset as the angle for handle)
            let angle = dim.offset; // for radius, offset stores the display angle
            let edge =
                info.center + Vec2::new(info.radius * angle.cos(), info.radius * angle.sin());
            return Some((info.center, edge));
        }
        let v1 = self.resolve_vertex_pos(dim.eid1, dim.vid1)?;
        let v2 = self.resolve_vertex_pos(dim.eid2, dim.vid2)?;
        match dim.kind {
            DimKind::Horizontal => {
                let y = (v1.y + v2.y) * 0.5 + dim.offset;
                Some((Vec2::new(v1.x, y), Vec2::new(v2.x, y)))
            }
            DimKind::Vertical => {
                let x = (v1.x + v2.x) * 0.5 + dim.offset;
                Some((Vec2::new(x, v1.y), Vec2::new(x, v2.y)))
            }
            DimKind::Aligned => {
                let seg = v2 - v1;
                let len = seg.hypot();
                if len < 1e-6 {
                    return None;
                }
                let n = Vec2::new(-seg.y / len, seg.x / len);
                Some((v1 + n * dim.offset, v2 + n * dim.offset))
            }
            DimKind::Radius => None,
        }
    }
    /// Resolve radius info for a vertex that represents (or controls) a radius.
    /// Returns (center, radius) for shapes where this vertex is meaningful as a radius source.
    pub fn resolve_radius_info(&self, eid: EUId, vid: VUId) -> Option<RadiusInfo> {
        use crate::shape::ShapeType;
        use crate::types::vertex::VertexRole;
        let shape = self
            .shapes
            .get(&eid)
            .or_else(|| self.grouped_shapes.get(&eid))?;
        let vertex = shape.get_vertex(&vid)?;
        match vertex.role {
            VertexRole::Center | VertexRole::Radius => {
                // Disc / Oblong / ConstrCircle: vertex 0 = center, vertex 1 = radius handle
                // Or for Oblong: 0/1 = centers, 2/3 = radii.
                match shape.get_shape_type() {
                    ShapeType::Disc | ShapeType::ConstrCircle => {
                        let c = shape.get_vertices().val(0).curr();
                        let r = shape.get_vertices().val(1).curr();
                        Some(RadiusInfo { center: c, radius: (r - c).hypot() })
                    }
                    ShapeType::Oblong => {
                        // The center associated with this vertex
                        let idx = shape
                            .get_vertices()
                            .iter()
                            .position(|(uid, _)| *uid == vid)?;
                        // 0/2 pair, 1/3 pair
                        let (c_idx, r_idx) = match idx {
                            0 => (0, 2),
                            1 => (1, 3),
                            2 => (0, 2),
                            3 => (1, 3),
                            _ => return None,
                        };
                        let c = shape.get_vertices().val(c_idx as i64).curr();
                        let r = shape.get_vertices().val(r_idx as i64).curr();
                        Some(RadiusInfo { center: c, radius: (r - c).hypot() })
                    }
                    _ => None,
                }
            }
            VertexRole::Apex | VertexRole::BBoxCorner => {
                // Polygon apex / Rectangle corner with rounded radius
                let r = vertex.get_radius()?;
                let center = vertex.curr();
                Some(RadiusInfo { center, radius: r as f64 })
            }
            VertexRole::SagHandle => {
                // Sag handle of polygon edge — the radius is the arc radius of that edge
                // For now, fall through (would need solve_side); skip
                None
            }
            _ => None,
        }
    }
    /// Hit-test: returns the user dim id whose line is within `hit_radius` of `pos`.
    pub fn hit_test_user_dim(&self, pos: Vec2, hit_radius: f64) -> Option<usize> {
        let mut best: Option<(usize, f64)> = None;
        for dim in &self.user_dims {
            let Some((p1, p2)) = self.user_dim_line(dim) else {
                continue;
            };
            let seg = p2 - p1;
            let len2 = seg.x * seg.x + seg.y * seg.y;
            if len2 < 1e-9 {
                continue;
            }
            let t = ((pos.x - p1.x) * seg.x + (pos.y - p1.y) * seg.y) / len2;
            let t = t.clamp(0.0, 1.0);
            let proj = p1 + seg * t;
            let dist = (proj - pos).hypot();
            if dist < hit_radius && best.as_ref().map_or(true, |(_, d)| dist < *d) {
                best = Some((dim.id, dist));
            }
        }
        best.map(|(id, _)| id)
    }
    pub fn push_element(&mut self, mut elem: GeneralShape) -> EUId {
        let affects_final = !matches!(
            elem.get_shape_type(),
            ShapeType::ConstrLine | ShapeType::ConstrCircle
        );
        elem.set_order(self.next_order);
        self.next_order = self.next_order.saturating_add(1);
        let eid = EUId::new();
        self.shapes.insert(eid, elem);
        if affects_final {
            self.final_polygon_dirty = true;
        }
        eid
    }
    fn is_groupable_shape(shape: &GeneralShape) -> bool {
        !matches!(
            shape.get_shape_type(),
            ShapeType::ConstrLine | ShapeType::ConstrCircle
        )
    }
    fn group_bbox_from_children(&self, children: &[EUId]) -> Option<(Vec2, Vec2)> {
        let mut min = Vec2::new(f64::INFINITY, f64::INFINITY);
        let mut max = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for eid in children {
            let shape = self
                .shapes
                .get(eid)
                .or_else(|| self.grouped_shapes.get(eid))?;
            let bbox = shape.get_bezpath().bounding_box();
            if bbox.is_zero_area() {
                continue;
            }
            min.x = min.x.min(bbox.x0);
            min.y = min.y.min(bbox.y0);
            max.x = max.x.max(bbox.x1);
            max.y = max.y.max(bbox.y1);
        }
        if !min.x.is_finite() || !min.y.is_finite() || !max.x.is_finite() || !max.y.is_finite() {
            return None;
        }
        Some((min, max))
    }
    pub fn group_selected(&mut self) -> Option<EUId> {
        let selected: Vec<EUId> = self.shapes_selected.iter().copied().collect();
        if selected.len() < 2 {
            return None;
        }
        let mut children = Vec::new();
        let mut order = i32::MAX;
        for eid in selected {
            let Some(shape) = self.shapes.get(&eid) else {
                continue;
            };
            if !Self::is_groupable_shape(shape) {
                continue;
            }
            order = order.min(shape.get_order());
            children.push(eid);
        }
        if children.len() < 2 {
            return None;
        }
        let (min, max) = self.group_bbox_from_children(&children)?;
        let group = GeneralShape::new_shape_group(min, max, order, children.clone())?;
        for eid in &children {
            if let Some(shape) = self.shapes.remove(eid) {
                self.grouped_shapes.insert(*eid, shape);
            }
        }
        let group_id = EUId::new();
        self.shapes.insert(group_id, group);
        self.shapes_selected.clear();
        self.shapes_selected.insert(group_id);
        self.shapes_highlighted.clear();
        self.vertex_selected = None;
        self.vertex_highlighted = None;
        self.final_polygon_dirty = true;
        Some(group_id)
    }
    pub fn ungroup_selected(&mut self) -> Option<Vec<EUId>> {
        let selected: Vec<EUId> = self.shapes_selected.iter().copied().collect();
        if selected.len() != 1 {
            return None;
        }
        let group_id = selected[0];
        let children = self.shapes.get(&group_id)?.get_group_children()?.to_vec();
        let group = self.shapes.remove(&group_id)?;
        if !group.is_group() {
            self.shapes.insert(group_id, group);
            return None;
        }
        let mut restored = Vec::new();
        for eid in children.iter() {
            if let Some(child) = self.grouped_shapes.remove(eid) {
                self.shapes.insert(*eid, child);
                restored.push(*eid);
            }
        }
        self.shapes_selected = restored.iter().copied().collect();
        self.shapes_highlighted.clear();
        self.vertex_selected = None;
        self.vertex_highlighted = None;
        self.final_polygon_dirty = true;
        Some(restored)
    }
    pub fn sync_next_order(&mut self) {
        let mut max_order = -1;
        for shape in self.shapes.values() {
            max_order = max_order.max(shape.get_order());
        }
        self.next_order = max_order.saturating_add(1);
    }
    pub fn ordered_shapes(&self) -> Vec<EUId> {
        let mut ordered: Vec<(i32, EUId)> = self
            .shapes
            .iter()
            .filter(|(_, shape)| {
                !matches!(
                    shape.get_shape_type(),
                    ShapeType::ConstrLine | ShapeType::ConstrCircle
                )
            })
            .map(|(eid, shape)| (shape.get_order(), *eid))
            .collect();
        ordered.sort_by(|(order_a, eid_a), (order_b, eid_b)| {
            order_a.cmp(order_b).then_with(|| eid_a.cmp(eid_b))
        });
        ordered.into_iter().map(|(_, eid)| eid).collect()
    }
    pub fn order_info(&self, eid: EUId) -> Option<(usize, usize, i32)> {
        let ordered = self.ordered_shapes();
        let total = ordered.len();
        let idx = ordered.iter().position(|id| *id == eid)?;
        let order = self.shapes.get(&eid)?.get_order();
        Some((idx + 1, total, order))
    }
    pub fn shift_order(&mut self, eid: EUId, delta: i32) -> bool {
        let ordered = self.ordered_shapes();
        let total = ordered.len();
        if total < 2 {
            return false;
        }
        let idx = match ordered.iter().position(|id| *id == eid) {
            Some(idx) => idx as i32,
            None => return false,
        };
        let new_idx = (idx + delta).clamp(0, total as i32 - 1);
        if new_idx == idx {
            return false;
        }
        let other_id = ordered[new_idx as usize];
        let order_a = self.shapes.get(&eid).map(|s| s.get_order());
        let order_b = self.shapes.get(&other_id).map(|s| s.get_order());
        let (Some(order_a), Some(order_b)) = (order_a, order_b) else {
            return false;
        };
        if let Some(shape) = self.shapes.get_mut(&eid) {
            shape.set_order(order_b);
        }
        if let Some(shape) = self.shapes.get_mut(&other_id) {
            shape.set_order(order_a);
        }
        true
    }
    pub fn set_order_sequence(&mut self, ordered: &[EUId]) {
        for (idx, eid) in ordered.iter().enumerate() {
            if let Some(shape) = self.shapes.get_mut(eid) {
                shape.set_order(idx as i32);
            }
        }
        let mut max_order = -1;
        for shape in self.shapes.values() {
            max_order = max_order.max(shape.get_order());
        }
        self.next_order = max_order.saturating_add(1);
    }
    pub fn select_only(&mut self, eid: EUId) {
        self.shapes_selected.clear();
        self.shapes_highlighted.clear();
        self.vertex_selected = None;
        self.vertex_highlighted = None;
        self.shapes_selected.insert(eid);
    }
    pub fn pop_element(&mut self, eid: EUId) -> Option<GeneralShape> {
        // remove the shape
        let removed = self.shapes.remove(&eid);

        if removed.is_some() {
            // clear element-level state
            self.shapes_selected.remove(&eid);
            self.shapes_highlighted.remove(&eid);

            // drop all (EUId, VUId) matching this EUId
            if let Some((sel_eid, _)) = self.vertex_selected {
                if sel_eid == eid {
                    self.vertex_selected = None;
                }
            }
            if let Some((high_eid, _)) = self.vertex_highlighted {
                if high_eid == eid {
                    self.vertex_highlighted = None;
                }
            }
            if let Some(shape) = removed.as_ref() {
                if shape.is_group() {
                    if let Some(children) = shape.get_group_children() {
                        for child in children {
                            self.grouped_shapes.remove(child);
                        }
                    }
                }
                if !matches!(
                    shape.get_shape_type(),
                    ShapeType::ConstrLine | ShapeType::ConstrCircle
                ) {
                    self.final_polygon_dirty = true;
                }
            }
        }

        removed
    }
    pub fn get_element(&self, eid: EUId) -> Option<&GeneralShape> {
        self.shapes.get(&eid)
    }
    pub fn get_element_mut(&mut self, eid: EUId) -> Option<&mut GeneralShape> {
        self.shapes.get_mut(&eid)
    }

    pub fn find_constr_circle_at(&self, pos: Vec2) -> Option<(EUId, usize)> {
        self.shapes
            .iter()
            .filter_map(|(eid, elem)| {
                if !matches!(elem.get_shape_type(), ShapeType::ConstrCircle) || !elem.contains(pos)
                {
                    return None;
                }
                let vertices = elem.get_vertices();
                if vertices.len() < 5 {
                    return None;
                }
                let center = vertices.val(0).curr();
                let edge = vertices.val(1).curr();
                let radius = (edge - center).hypot();
                let distance = (pos - center).hypot();
                let delta = (distance - radius).abs();
                let count = elem.get_magnets_number().unwrap_or(3);
                Some((*eid, delta, count))
            })
            .min_by(|(_, a_delta, _), (_, b_delta, _)| a_delta.total_cmp(b_delta))
            .map(|(eid, _, count)| (eid, count))
    }

    pub fn create_vertices_between(
        &mut self,
        eid1_sel: EUId,
        vid1_sel: VUId,
        eid2_sel: EUId,
        vid2_sel: VUId,
    ) -> Option<()> {
        if eid1_sel != eid2_sel {
            return None;
        }
        let elem = self.get_element_mut(eid1_sel)?;
        let v1_sel = elem.get_vertex(&vid1_sel)?;
        let v2_sel = elem.get_vertex(&vid2_sel)?;

        match elem.get_shape_type() {
            ShapeType::Poly => {
                let nc = elem.get_n_corners();
                let idx1 = elem.get_vertices().get_idx(&vid1_sel)? as usize;
                let idx2 = elem.get_vertices().get_idx(&vid2_sel)? as usize;
                // Only allow insertion between two adjacent corner vertices
                if idx1 >= nc || idx2 >= nc {
                    return None;
                }
                // Check adjacency among corners (ring of N, not of 2N)
                let corner_dist = {
                    let diff = (idx1 as i64 - idx2 as i64).unsigned_abs() as usize;
                    diff.min(nc - diff)
                };
                if corner_dist != 1 {
                    return None;
                }
                // Determine the insert position within corners: after the
                // smaller index (in ring order).
                let insert_at = if (idx1 + 1) % nc == idx2 {
                    idx1 + 1
                } else {
                    idx2 + 1
                };
                // New corner at midpoint
                let mid = snap_vertex(
                    (v1_sel.curr() + v2_sel.curr()) / 2.0,
                    Snap::new(),
                );
                use crate::types::vertex::VertexRole;
                let mut new_corner = Vertex::new_with_role(mid, VertexRole::Apex);
                new_corner.enable_radius();
                // Insert corner at the correct position
                elem.get_vertices_mut()
                    .insert_at(insert_at, (VUId::new(), new_corner));
                // Insert a sag vertex at the end of the sag block (index nc+1,
                // since nc already shifted +1 after the corner insert)
                let new_nc = nc + 1;
                elem.get_vertices_mut()
                    .insert_at(new_nc + nc, (VUId::new(), Vertex::new_with_role(mid, VertexRole::SagHandle)));
                // Compute winding for poly_effective_edge
                let ccw = crate::shape::poly_winding_ccw(elem.get_vertices(), new_nc);
                // Recompute all sag vertex positions to effective edge midpoints (reset to straight)
                for i in 0..new_nc {
                    let j = (i + 1) % new_nc;
                    let ci = elem.get_vertices().val(i as i64).curr();
                    let cj = elem.get_vertices().val(j as i64).curr();
                    let ri = elem.get_vertices().val(i as i64).get_radius();
                    let rj = elem.get_vertices().val(j as i64).get_radius();
                    let ci_conv = crate::shape::poly_is_convex(elem.get_vertices(), new_nc, i, ccw);
                    let cj_conv = crate::shape::poly_is_convex(elem.get_vertices(), new_nc, j, ccw);
                    let (es, ee) = crate::shape::poly_effective_edge(ci, cj, ri, rj, ccw, ci_conv, cj_conv);
                    let edge_mid = (es + ee) * 0.5;
                    elem.get_vertices_mut()
                        .val_mut((new_nc + i) as i64)
                        .set_curr(edge_mid);
                    elem.get_vertices_mut()
                        .val_mut((new_nc + i) as i64)
                        .set_saved(edge_mid);
                }
                elem.set_bezpath();
                self.final_polygon_dirty = true;
                Some(())
            }
            _ => {
                // Non-poly shapes: use original adjacency check on the full ring
                elem.get_vertices().dist_ok(
                    elem.get_vertices().get_idx(&vid1_sel)?,
                    elem.get_vertices().get_idx(&vid2_sel)?,
                    1,
                )?;
                None
            }
        }
    }
    pub fn delete_vertex(&mut self, eid_sel: EUId, vid_sel: VUId) -> bool {
        if let Some(elem) = self.get_element_mut(eid_sel) {
            if elem.get_shape_type() == ShapeType::Poly {
                let nc = elem.get_n_corners();
                // Need at least 3 corners after deletion
                if nc < 4 {
                    return false;
                }
                let Some(idx_sel) = elem.get_vertices().get_idx(&vid_sel) else {
                    return false;
                };
                let idx = idx_sel as usize;
                // Don't delete sag vertices directly
                if idx >= nc {
                    return false;
                }
                // Remove the corner vertex
                elem.get_vertices_mut().remove(&idx_sel);
                // Remove the corresponding sag vertex.
                // After removing the corner, sag vertices start at index (nc - 1).
                // The sag for the removed corner was at original index (nc + idx),
                // which is now at index (nc - 1 + idx).
                let sag_idx = (nc - 1 + idx) as i64;
                elem.get_vertices_mut().remove(&sag_idx);
                // Recompute all sag vertex positions (reset to effective edge midpoints)
                let new_nc = nc - 1;
                let ccw = crate::shape::poly_winding_ccw(elem.get_vertices(), new_nc);
                for i in 0..new_nc {
                    let j = (i + 1) % new_nc;
                    let ci = elem.get_vertices().val(i as i64).curr();
                    let cj = elem.get_vertices().val(j as i64).curr();
                    let ri = elem.get_vertices().val(i as i64).get_radius();
                    let rj = elem.get_vertices().val(j as i64).get_radius();
                    let ci_conv = crate::shape::poly_is_convex(elem.get_vertices(), new_nc, i, ccw);
                    let cj_conv = crate::shape::poly_is_convex(elem.get_vertices(), new_nc, j, ccw);
                    let (es, ee) = crate::shape::poly_effective_edge(ci, cj, ri, rj, ccw, ci_conv, cj_conv);
                    let mid = (es + ee) * 0.5;
                    elem.get_vertices_mut()
                        .val_mut((new_nc + i) as i64)
                        .set_curr(mid);
                    elem.get_vertices_mut()
                        .val_mut((new_nc + i) as i64)
                        .set_saved(mid);
                }
                elem.set_bezpath();
                self.final_polygon_dirty = true;
            }
        }
        false
    }

    pub fn select_vertices(&mut self, draw_pos: Vec2) -> bool {
        let mut selection_changed = false;
        for (eid, element) in self.shapes.iter_mut() {
            if let Some(vid_sel) = element.select_vertex(draw_pos) {
                self.vertex_selected = Some((*eid, vid_sel));
                selection_changed = true;
            }
        }
        if !selection_changed {
            self.vertex_selected = None;
        } else {
            self.shapes_selected.clear();
        }
        selection_changed
    }
    pub fn highlight_vertices(&mut self, draw_pos: Vec2) -> bool {
        self.vertex_highlighted = None;
        let mut highlight_changed = false;
        for (eid, element) in self.shapes.iter_mut() {
            if let Some(vid_sel) = element.highlight_vertex(draw_pos) {
                self.vertex_highlighted = Some((*eid, vid_sel));
                highlight_changed = true;
            }
        }
        highlight_changed
    }

    fn has_element_in_rect(shape_rect: Rect, window_rect: Rect, select_touching: bool) -> bool {
        if select_touching {
            !(shape_rect.x1 < window_rect.x0
                || window_rect.x1 < shape_rect.x0
                || shape_rect.y1 < window_rect.y0
                || window_rect.y1 < shape_rect.y0)
        } else {
            shape_rect.x0 >= window_rect.x0
                && shape_rect.y0 >= window_rect.y0
                && shape_rect.x1 <= window_rect.x1
                && shape_rect.y1 <= window_rect.y1
        }
    }

    fn normalize_selection_rect(start: Vec2, end: Vec2) -> Rect {
        Rect::new(
            start.x.min(end.x),
            start.y.min(end.y),
            start.x.max(end.x),
            start.y.max(end.y),
        )
    }

    pub fn has_element_at(&self, draw_pos: Vec2) -> bool {
        self.shapes.values().any(|shape| shape.contains(draw_pos))
    }

    pub fn has_selected_element_at(&self, draw_pos: Vec2) -> bool {
        self.shapes_selected.iter().any(|eid| {
            self.shapes
                .get(eid)
                .map(|shape| shape.contains(draw_pos))
                .unwrap_or(false)
        })
    }

    pub fn select_elements_in_window(
        &mut self,
        start: Vec2,
        end: Vec2,
        select_touching: bool,
        append: bool,
    ) {
        let window_rect = Self::normalize_selection_rect(start, end);
        let selected_now: HashSet<EUId> = self
            .shapes
            .iter()
            .filter_map(|(eid, shape)| {
                let shape_rect = shape.get_bezpath_rotated().bounding_box();
                if Self::has_element_in_rect(shape_rect, window_rect, select_touching) {
                    Some(*eid)
                } else {
                    None
                }
            })
            .collect();

        if append {
            for eid in selected_now {
                self.shapes_selected.insert(eid);
            }
        } else {
            self.shapes_selected = selected_now;
        }
        self.vertex_selected = None;
        self.shapes_selector
            .refresh_selectable_elems(self.shapes_selected.clone());
    }

    pub fn select_elements(&mut self, userui: &UserUI) {
        // Select the nodes whose element contains the position
        fn ordered_candidates(shapes: &HashMap<EUId, GeneralShape>, pos: Vec2) -> Vec<EUId> {
            let mut candidates: Vec<(EUId, f64)> = Vec::new();
            for (eid, elem) in shapes {
                if elem.contains(pos) {
                    let bbox = elem.get_bezpath().bounding_box();
                    let area = (bbox.x1 - bbox.x0).abs() * (bbox.y1 - bbox.y0).abs();
                    candidates.push((*eid, area));
                }
            }
            candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            candidates.iter().map(|(eid, _)| *eid).collect()
        }

        if !userui.keys_states.shift_pressed {
            let ordered = ordered_candidates(&self.shapes, userui.draw_pos);
            if !ordered.is_empty() {
                self.shapes_selector
                    .refresh_selectable_elems_ordered(ordered);
                if let Some(eid) = self.shapes_selector.next_selection() {
                    self.shapes_selected.clear();
                    self.shapes_selected.insert(eid);
                }
            } else {
                self.shapes_selected.clear();
            }
        } else {
            // Shift pressed, toggle current candidate and keep existing selection
            let ordered = ordered_candidates(&self.shapes, userui.draw_pos);
            if ordered.is_empty() {
                return;
            }
            self.shapes_selector
                .refresh_selectable_elems_ordered(ordered);
            if let Some(eid) = self.shapes_selector.next_selection() {
                if self.shapes_selected.contains(&eid) {
                    self.shapes_selected.remove(&eid);
                } else {
                    self.shapes_selected.insert(eid);
                }
            }
        }
    }
    pub fn highlight_elements(&mut self, userui: &UserUI) {
        self.shapes_highlighted.clear();
        for (eid, elem) in &self.shapes {
            if elem.contains(userui.draw_pos) {
                self.shapes_highlighted.insert(*eid);
            }
        }
    }
    pub fn delete_selected_elements(&mut self) -> bool {
        let mut deleted = false;
        let mut affects_final = false;
        for eid in &self.shapes_selected.clone() {
            if let Some(shape) = self.shapes.remove(eid) {
                self.shapes_highlighted.remove(eid);
                self.shapes_selected.remove(eid);
                self.vertex_selected = None;
                self.vertex_highlighted = None;
                self.shapes_selector
                    .refresh_selectable_elems(HashSet::new());
                deleted = true;
                if shape.is_group() {
                    if let Some(children) = shape.get_group_children() {
                        for child in children {
                            self.grouped_shapes.remove(child);
                        }
                    }
                }
                if !matches!(
                    shape.get_shape_type(),
                    ShapeType::ConstrLine | ShapeType::ConstrCircle
                ) {
                    affects_final = true;
                }
            }
        }
        if deleted && affects_final {
            self.final_polygon_dirty = true;
        }
        deleted
    }

    pub fn selection_affects_final_polygon(&self) -> bool {
        if let Some((eid, _)) = self.vertex_selected {
            if let Some(e) = self.shapes.get(&eid) {
                if !matches!(
                    e.get_shape_type(),
                    ShapeType::ConstrLine | ShapeType::ConstrCircle
                ) {
                    return true;
                }
            }
        }
        for eid in &self.shapes_selected {
            if let Some(e) = self.shapes.get(eid) {
                if !matches!(
                    e.get_shape_type(),
                    ShapeType::ConstrLine | ShapeType::ConstrCircle
                ) {
                    return true;
                }
            }
        }
        false
    }

    pub fn save_elements_positions(&mut self) {
        for (_, elem) in self.shapes.iter_mut() {
            elem.save_vertices_positions();
        }
        for (_, elem) in self.grouped_shapes.iter_mut() {
            elem.save_vertices_positions();
        }
    }

    pub fn move_elements(&mut self, userui: &UserUI) -> bool {
        let delta = userui.pointer.curr() - userui.pointer.saved();
        let mut moved = false;
        let mut affects_final = false;
        for eid in self.shapes_selected.clone() {
            let is_group = self
                .shapes
                .get(&eid)
                .map(|shape| shape.is_group())
                .unwrap_or(false);
            if is_group {
                if self.move_group_any(eid, delta).is_some() {
                    moved = true;
                    affects_final = true;
                }
                continue;
            }
            if let Some(e) = self.shapes.get_mut(&eid) {
                e.move_shape(delta);
                moved = true;
                if !matches!(
                    e.get_shape_type(),
                    ShapeType::ConstrLine | ShapeType::ConstrCircle
                ) {
                    affects_final = true;
                }
            }
        }
        if moved && affects_final {
            self.final_polygon_dirty = true;
        }
        moved
    }

    fn move_group_any(&mut self, group_id: EUId, delta: Vec2) -> Option<()> {
        let (children, in_shapes) = if let Some(group) = self.shapes.get(&group_id) {
            (
                group.get_group_children().unwrap_or_default().to_vec(),
                true,
            )
        } else {
            let group = self.grouped_shapes.get(&group_id)?;
            (
                group.get_group_children().unwrap_or_default().to_vec(),
                false,
            )
        };
        for child_id in children {
            let is_group = self
                .grouped_shapes
                .get(&child_id)
                .map(GeneralShape::is_group)
                .unwrap_or(false);
            if is_group {
                let _ = self.move_group_any(child_id, delta);
                continue;
            }
            if let Some(child) = self.grouped_shapes.get_mut(&child_id) {
                child.move_shape(delta);
            }
        }
        if in_shapes {
            self.shapes.get_mut(&group_id)?.move_shape(delta);
        } else {
            self.grouped_shapes.get_mut(&group_id)?.move_shape(delta);
        }
        Some(())
    }

    fn apply_uniform_scale_to_shape(shape: &mut GeneralShape, origin: Vec2, scale: f64) {
        let shape_type = shape.get_shape_type();
        if matches!(shape_type, ShapeType::Disc | ShapeType::ConstrCircle) {
            if shape.get_vertices().len() < 2 {
                return;
            }
            let center_saved = shape.get_vertices().val(0).saved();
            let edge_saved = shape.get_vertices().val(1).saved();
            let mut dir = edge_saved - center_saved;
            if dir.hypot() < EPSILON {
                dir = Vec2::new(1.0, 0.0);
            } else {
                dir = dir.normalize();
            }
            let new_center = origin + (center_saved - origin) * scale;
            let new_edge = new_center + dir * (edge_saved - center_saved).hypot() * scale;
            let v0 = shape.get_vertices_mut().val_mut(0);
            v0.set_curr(new_center);
            let v1 = shape.get_vertices_mut().val_mut(1);
            v1.set_curr(new_edge);
            if matches!(shape_type, ShapeType::ConstrCircle) {
                if let Some(PropertyValue::Magnets { value, .. }) =
                    shape.get_properties().get(&Property::Magnets).cloned()
                {
                    let count = value.curr();
                    let v1 = shape.get_vertices().val(0).curr();
                    let v2 = shape.get_vertices().val(1).curr();
                    let vs_magnets = get_magnets_vertices(v1, v2, count);
                    for (idx, (_, v)) in shape.get_vertices_mut().iter_mut().skip(2).enumerate() {
                        v.set_curr(vs_magnets[idx]);
                    }
                }
            }
        } else {
            for (_, vertex) in shape.get_vertices_mut().iter_mut() {
                let saved = vertex.saved();
                let new_pos = origin + (saved - origin) * scale;
                vertex.set_curr(new_pos);
            }
        }
        if let Some(PropertyValue::Scale { value, .. }) =
            shape.get_properties_mut().get_mut(&Property::Scale)
        {
            let new_value = value.curr() * scale;
            value.set_curr(new_value);
        }
        shape.update_properties();
        shape.set_bezpath();
    }

    fn scale_group_by_factor(&mut self, group_id: EUId, origin: Vec2, scale: f64) -> Option<()> {
        let children = if let Some(group) = self.shapes.get(&group_id) {
            group.get_group_children()?.to_vec()
        } else {
            let group = self.grouped_shapes.get(&group_id)?;
            group.get_group_children()?.to_vec()
        };
        for child_id in children.iter() {
            if let Some(child) = self.grouped_shapes.get(child_id) {
                if child.is_group() {
                    let _ = self.scale_group_by_factor(*child_id, origin, scale);
                    continue;
                }
            }
            if let Some(child) = self.grouped_shapes.get_mut(child_id) {
                Self::apply_uniform_scale_to_shape(child, origin, scale);
            }
        }
        let vertices_saved = if let Some(group) = self.shapes.get(&group_id) {
            group
                .get_vertices()
                .iter()
                .map(|(_, v)| v.saved())
                .collect::<Vec<_>>()
        } else {
            let group = self.grouped_shapes.get(&group_id)?;
            group
                .get_vertices()
                .iter()
                .map(|(_, v)| v.saved())
                .collect::<Vec<_>>()
        };
        let group = if let Some(group) = self.shapes.get_mut(&group_id) {
            group
        } else {
            self.grouped_shapes.get_mut(&group_id)?
        };
        for (idx, saved) in vertices_saved.iter().enumerate() {
            let new_pos = origin + (*saved - origin) * scale;
            let vertex = group.get_vertices_mut().val_mut(idx as i64);
            vertex.set_curr(new_pos);
        }
        if let Some(PropertyValue::Scale { value, .. }) =
            group.get_properties_mut().get_mut(&Property::Scale)
        {
            let new_value = value.curr() * scale;
            value.set_curr(new_value);
        }
        group.update_properties();
        group.set_bezpath();
        Some(())
    }

    pub fn scale_group_by_vertex(&mut self, group_id: EUId, vid: VUId, userui: &UserUI) -> bool {
        let (opposite_pos, drag_saved) = {
            let group = match self.shapes.get(&group_id) {
                Some(group) => group,
                None => return false,
            };
            if !group.is_group() {
                return false;
            }
            let drag_idx = match group.get_vertices().get_idx(&vid) {
                Some(idx) => idx,
                None => return false,
            };
            let opposite_idx = match drag_idx {
                0 => 2,
                1 => 3,
                2 => 0,
                3 => 1,
                _ => return false,
            };
            let drag_saved = group.get_vertices().val(drag_idx).saved();
            let opposite_pos = group.get_vertices().val(opposite_idx as i64).saved();
            (opposite_pos, drag_saved)
        };

        let mut delta = userui.pointer.curr() - userui.pointer.saved();
        if !userui.magnetized {
            let snap = userui.snap;
            delta = (delta / snap.linear()).round() * snap.linear();
        }
        let new_pos = drag_saved + delta;
        let dx0 = drag_saved.x - opposite_pos.x;
        let dy0 = drag_saved.y - opposite_pos.y;
        let dx1 = new_pos.x - opposite_pos.x;
        let dy1 = new_pos.y - opposite_pos.y;
        let sx = if dx0.abs() > EPSILON { dx1 / dx0 } else { 1.0 };
        let sy = if dy0.abs() > EPSILON { dy1 / dy0 } else { 1.0 };
        let scale = if dx1.abs() >= dy1.abs() { sx } else { sy };
        if scale <= 0.0 {
            return false;
        }

        let _ = self.scale_group_by_factor(group_id, opposite_pos, scale);
        self.final_polygon_dirty = true;
        true
    }

    pub fn refresh_svg_cache(&mut self) {
        for (_, shape) in self.shapes.iter_mut() {
            if matches!(shape.get_shape_type(), ShapeType::Svg | ShapeType::Voronoi) {
                shape.set_bezpath();
            }
        }
        for (_, shape) in self.grouped_shapes.iter_mut() {
            if matches!(shape.get_shape_type(), ShapeType::Svg | ShapeType::Voronoi) {
                shape.set_bezpath();
            }
        }
    }

    fn calc_group_polygon(&self, children: &[EUId]) -> MultiPolygon<f64> {
        let mut ordered: Vec<(i32, EUId)> = children
            .iter()
            .filter_map(|eid| {
                self.grouped_shapes
                    .get(eid)
                    .map(|shape| (shape.get_order(), *eid))
            })
            .collect();
        ordered.sort_by(|(order_a, eid_a), (order_b, eid_b)| {
            order_a.cmp(order_b).then_with(|| eid_a.cmp(eid_b))
        });

        let mut result = MultiPolygon::new(vec![]);
        for (_, eid) in ordered {
            let Some(shape) = self.grouped_shapes.get(&eid) else {
                continue;
            };
            let (shape_poly, op) = if shape.is_group() {
                let children = shape.get_group_children().unwrap_or_default();
                let poly = self.calc_group_polygon(children);
                (shape.rotate_polygon_if_needed(&poly), shape.get_operation())
            } else {
                let mut polys = Vec::new();
                for poly in shape.get_polygon().iter() {
                    polys.push(normalize_polygon_orientation(poly));
                }
                let poly = MultiPolygon::new(polys);
                (shape.rotate_polygon_if_needed(&poly), shape.get_operation())
            };
            if shape_poly.0.is_empty() {
                continue;
            }
            result = match op {
                Operation::Union => {
                    if result.0.is_empty() {
                        shape_poly
                    } else {
                        result.boolean_op(&shape_poly, geo::OpType::Union)
                    }
                }
                Operation::Difference => {
                    if result.0.is_empty() {
                        result
                    } else {
                        result.boolean_op(&shape_poly, geo::OpType::Difference)
                    }
                }
            };
        }
        result
    }

    pub fn calc_final_polygon(&mut self) {
        if !self.final_polygon_dirty {
            return;
        }
        let ordered = self.ordered_shapes();
        let mut result = MultiPolygon::new(vec![]);
        for eid in ordered {
            let Some(shape) = self.shapes.get(&eid) else {
                continue;
            };
            let (shape_poly, op) = if shape.is_group() {
                let children = shape.get_group_children().unwrap_or_default();
                let poly = self.calc_group_polygon(children);
                let op = shape.get_operation();
                // Cache group polygon + rebuild with ghosts
                if let Some(group_shape) = self.shapes.get_mut(&eid) {
                    group_shape.set_group_polygon(poly);
                    group_shape.set_bezpath(); // triggers update_polygon with ghosts
                }
                let shape = self.shapes.get(&eid).unwrap();
                let full_poly = shape.get_polygon().clone();
                (shape.rotate_polygon_if_needed(&full_poly), op)
            } else {
                let mut polys = Vec::new();
                for poly in shape.get_polygon().iter() {
                    polys.push(normalize_polygon_orientation(poly));
                }
                let poly = MultiPolygon::new(polys);
                (shape.rotate_polygon_if_needed(&poly), shape.get_operation())
            };
            if shape_poly.0.is_empty() {
                continue;
            }
            result = match op {
                Operation::Union => {
                    if result.0.is_empty() {
                        shape_poly
                    } else {
                        result.boolean_op(&shape_poly, geo::OpType::Union)
                    }
                }
                Operation::Difference => {
                    if result.0.is_empty() {
                        result
                    } else {
                        result.boolean_op(&shape_poly, geo::OpType::Difference)
                    }
                }
            };
        }
        self.final_polygon = result;
        self.calc_final_paths();
        self.final_polygon_dirty = false;
    }
    fn calc_final_paths(&mut self) {
        self.final_paths = geo_multipolygon_to_bez_paths(&self.final_polygon);
    }
    pub fn get_final_paths(&self) -> &Vec<BezPath> {
        &self.final_paths
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub struct ShapeSelector {
    selectable_elems: Vec<EUId>, // IDs of selectable elements
    current_index: usize,        // Current index in the list
}
#[allow(dead_code)]
impl Default for ShapeSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl ShapeSelector {
    pub fn new() -> Self {
        Self {
            selectable_elems: Vec::new(),
            current_index: 0,
        }
    }
    pub fn refresh_selectable_elems(&mut self, new_elems: HashSet<EUId>) {
        let current_set: HashSet<_> = self.selectable_elems.iter().cloned().collect();
        // Compare the sets, ignoring order
        if current_set != new_elems {
            // Reset if the set of nodes changes
            self.selectable_elems = new_elems.into_iter().collect();
            self.current_index = 0;
        }
    }
    pub fn refresh_selectable_elems_ordered(&mut self, ordered: Vec<EUId>) {
        let current_set: HashSet<_> = self.selectable_elems.iter().cloned().collect();
        let new_set: HashSet<_> = ordered.iter().cloned().collect();
        if current_set != new_set || self.selectable_elems != ordered {
            self.selectable_elems = ordered;
            self.current_index = 0;
        }
    }
    pub fn next_selection(&mut self) -> Option<EUId> {
        if self.selectable_elems.is_empty() {
            return None;
        }
        // Select the current node and move to the next
        let selected = self.selectable_elems[self.current_index];
        self.current_index = (self.current_index + 1) % self.selectable_elems.len();
        Some(selected)
    }
}
