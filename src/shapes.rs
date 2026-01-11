use crate::dom::ShapeType;
use crate::inputs::UserUI;
use crate::math::{geo_multipolygon_to_bez_paths, snap_vertex};
use crate::shape::{GeneralShape, Operation};
use crate::types::{EUId, Snap, VUId, Value};
use geo::algorithm::unary_union;
use geo::{BooleanOps, Coord, LineString, MultiPolygon, Polygon};
use kurbo::{flatten, stroke, BezPath, Cap, Join, PathEl, Shape, Stroke, StrokeOpts, Vec2};
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

fn ensure_orientation(points: &mut Vec<Vec2>, want_cw: bool) {
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

#[derive(Debug)]
pub struct DataSet {
    pub shapes: HashMap<EUId, GeneralShape>,
    pub shapes_selected: HashSet<EUId>,
    pub shapes_highlighted: HashSet<EUId>,
    pub shapes_selector: ShapeSelector,
    pub vertices_selected: HashSet<(EUId, VUId)>,
    pub vertices_highlighted: HashSet<(EUId, VUId)>,
    pub last_vertex_selected: Option<(EUId, VUId)>,

    pub final_polygon: MultiPolygon<f64>,
    pub final_paths: Vec<BezPath>,
    pub final_polygon_dirty: bool,
}
impl DataSet {
    pub fn new() -> Self {
        DataSet {
            shapes: HashMap::new(),
            shapes_selected: HashSet::new(),
            shapes_highlighted: HashSet::new(),
            shapes_selector: ShapeSelector::new(),
            vertices_selected: HashSet::new(),
            vertices_highlighted: HashSet::new(),
            last_vertex_selected: None,
            final_polygon: MultiPolygon::new(vec![]),
            final_paths: Vec::new(),
            final_polygon_dirty: true,
        }
    }
    pub fn mark_final_polygon_dirty(&mut self) {
        self.final_polygon_dirty = true;
    }
    pub fn push_element(&mut self, elem: GeneralShape) {
        let affects_final = !matches!(
            elem.get_shape_type(),
            ShapeType::ConstrLine | ShapeType::ConstrCircle
        );
        self.shapes.insert(EUId::new(), elem);
        if affects_final {
            self.final_polygon_dirty = true;
        }
    }
    pub fn pop_element(&mut self, eid: EUId) -> Option<GeneralShape> {
        // remove the shape
        let removed = self.shapes.remove(&eid);

        if removed.is_some() {
            // clear element-level state
            self.shapes_selected.remove(&eid);
            self.shapes_highlighted.remove(&eid);

            // drop all (EUId, VUId) matching this EUId
            self.vertices_selected
                .retain(|(sel_eid, _)| sel_eid != &eid);
            self.vertices_highlighted
                .retain(|(high_eid, _)| high_eid != &eid);
            // also for the last
            if let Some((last_eid, _)) = self.last_vertex_selected {
                if last_eid == eid {
                    self.last_vertex_selected = None;
                }
            }
            if let Some(shape) = removed.as_ref() {
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
        elem.get_vertices().dist_ok(
            elem.get_vertices().get_idx(&vid1_sel)?,
            elem.get_vertices().get_idx(&vid2_sel)?,
            1,
        )?;
        let v1_sel = elem.get_vertex(&vid1_sel)?;
        let v2_sel = elem.get_vertex(&vid2_sel)?;

        match elem.get_shape_type() {
            ShapeType::Poly => {
                // Create new vertex between the selected and highlighted vertices
                let new_v = (
                    VUId::new(),
                    Value::new(snap_vertex((v1_sel.curr + v2_sel.curr) / 2.0, Snap::new())),
                );
                elem.get_vertices_mut()
                    .insert_one_between(&vid1_sel, &vid2_sel, new_v);
                elem.set_bezpath();
                self.final_polygon_dirty = true;
                return Some(());
            }
            _ => return None,
        }
    }
    pub fn delete_vertex(&mut self, eid_sel: EUId, vid_sel: VUId) -> bool {
        if let Some(elem) = self.get_element_mut(eid_sel) {
            match elem.get_shape_type() {
                ShapeType::Poly => {
                    if elem.get_vertices().len() < 4 {
                        return false;
                    }
                    if let Some(idx_sel) = elem.get_vertices().get_idx(&vid_sel) {
                        elem.get_vertices_mut().remove(&idx_sel);
                        elem.set_bezpath();
                        self.final_polygon_dirty = true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub fn select_vertices(&mut self, draw_pos: Vec2) -> bool {
        let mut selection_changed = false;
        for (eid, element) in self.shapes.iter_mut() {
            if let Some(vid_sel) = element.select_vertex(draw_pos) {
                // Check if element was already in the vertices_selected set
                // if self.vertices_selected.contains(&(*eid, vid_sel)) {
                //     // Element is already selected, remove it
                //     self.vertices_selected.remove(&(*eid, vid_sel));
                //     self.last_vertex_selected = None;
                //     selection_changed = true;
                // } else {
                self.vertices_selected.insert((*eid, vid_sel));
                self.last_vertex_selected = Some((*eid, vid_sel));
                selection_changed = true;
                // }
            }
        }
        if !selection_changed {
            self.vertices_selected.clear();
            self.last_vertex_selected = None;
        } else {
            self.shapes_selected.clear();
        }
        selection_changed

        // match (self.vertices_selected, vsel_new) {
        //     (None, None) => return false,
        //     (None, Some(vselnew)) => {
        //         self.vertices_selected = Some(vselnew);
        //         return true;
        //     }
        //     (Some(_), None) => {
        //         self.vertices_selected = None;
        //         return false;
        //     }
        //     (Some(vsel), Some(vselnew)) => {
        //         match (
        //             userui.keys_states.ctrl_cmd_pressed,
        //             userui.keys_states.shift_pressed,
        //         ) {
        //             (false, false) => {
        //                 self.vertices_selected = Some(vselnew);
        //                 return true;
        //             }
        //             (false, true) => {
        //                 if self.bind_unbind_vertices(vsel.0, vsel.1, vselnew.0, vselnew.1) {
        //                     return true;
        //                 } else {
        //                     self.vertices_selected = None;
        //                     return true;
        //                 }
        //             }
        //             (true, false) => {
        //                 self.create_vertices_between(vsel.0, vsel.1, vselnew.0, vselnew.1);
        //                 return true;
        //             }
        //             (true, true) => {
        //                 self.delete_vertices_between(vsel.0, vsel.1, vselnew.0, vselnew.1);
        //                 return true;
        //             }
        //         }
        //     }
        // }
    }
    pub fn highlight_vertices(&mut self, draw_pos: Vec2) -> bool {
        self.vertices_highlighted.clear();
        let mut highlight_changed = false;
        for (eid, element) in self.shapes.iter_mut() {
            if let Some(vid_sel) = element.highlight_vertex(draw_pos) {
                self.vertices_highlighted.insert((*eid, vid_sel));
                highlight_changed = true;
            }
        }
        highlight_changed
    }

    pub fn select_elements(&mut self, userui: &UserUI) {
        // Select the nodes whose element contains the position
        if !userui.keys_states.shift_pressed {
            let mut candidates: Vec<(EUId, f64)> = Vec::new();
            for (eid, elem) in &self.shapes {
                if elem.contains(userui.draw_pos) {
                    let bbox = elem.get_bezpath().bounding_box();
                    let area = (bbox.x1 - bbox.x0).abs() * (bbox.y1 - bbox.y0).abs();
                    candidates.push((*eid, area));
                }
            }
            if !candidates.is_empty() {
                candidates
                    .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                let ordered: Vec<EUId> = candidates.iter().map(|(eid, _)| *eid).collect();
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
            // Shift pressed, add to selection
            for (id, node) in &self.shapes {
                if node.contains(userui.draw_pos) {
                    self.shapes_selected.insert(*id);
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
                self.vertices_selected.clear();
                self.vertices_highlighted.clear();
                self.shapes_selector
                    .refresh_selectable_elems(HashSet::new());
                deleted = true;
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
        if let Some((eid, _)) = self.last_vertex_selected {
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
    }

    pub fn move_elements(&mut self, userui: &UserUI) -> bool {
        let delta = userui.pointer.curr - userui.pointer.saved;
        let mut moved = false;
        let mut affects_final = false;
        let mut sel_and_bind = self.shapes_selected.clone();

        if !userui.keys_states.shift_pressed {
            for eid in &self.shapes_selected {
                if let Some(_) = self.shapes.get_mut(eid) {
                    sel_and_bind.extend(self.get_binded_elements(*eid));
                }
            }
            let mut other_binds: HashSet<EUId> = HashSet::new();
            for eid in self.shapes.keys() {
                if let Some(s) = self.shapes.get(eid) {
                    s.get_binded_elements().iter().for_each(|bind_eid| {
                        if sel_and_bind.contains(bind_eid) {
                            other_binds.extend(s.get_binded_elements().iter());
                        }
                    });
                }
            }
            sel_and_bind.extend(other_binds);
        }
        // Move all selected elements and their binded elements
        for eid in sel_and_bind {
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

    pub fn refresh_svg_cache(&mut self) {
        for (_, shape) in self.shapes.iter_mut() {
            if shape.get_shape_type() == ShapeType::Svg {
                shape.set_bezpath();
            }
        }
    }
    pub fn get_binded_elements(&self, eid: EUId) -> HashSet<EUId> {
        if let Some(element) = self.get_element(eid) {
            return element.get_binded_elements();
        }
        HashSet::new()
    }
    pub fn bind_unbind_vertices(
        &mut self,
        eid_sel: EUId,
        vid_sel: VUId,
        eid_hig: EUId,
        vid_hig: VUId,
    ) -> bool {
        // Ensure vertices belong to different elements
        if eid_sel == eid_hig {
            log!("Cannot bind vertices from the same element");
            return false;
        }

        let old_bind_sel = self
            .get_element(eid_sel)
            .and_then(|elem| elem.get_vertex(&vid_sel))
            .map(|vertex| vertex.bind.clone())
            .unwrap_or_else(HashSet::new); // Default to empty HashSet if not found

        let old_bind_hig = self
            .get_element(eid_hig)
            .and_then(|elem| elem.get_vertex(&vid_hig))
            .map(|vertex| vertex.bind.clone())
            .unwrap_or_else(HashSet::new);

        let unbound = if old_bind_sel.contains(&(eid_hig, vid_hig))
            && old_bind_hig.contains(&(eid_sel, vid_sel))
        {
            true
        } else {
            false
        };
        if let Some(elem_sel) = self.get_element_mut(eid_sel) {
            if let Some(vertex_sel) = elem_sel.get_vertex_mut(&vid_sel) {
                if unbound {
                    log!("Unbind {} to {}", vid_hig, vid_sel);
                    vertex_sel.bind.remove(&(eid_hig, vid_hig));
                } else {
                    log!("Binding {} to {}", vid_hig, vid_sel);
                    vertex_sel.bind.insert((eid_hig, vid_hig));
                }
            }
        }
        if let Some(elem_hig) = self.get_element_mut(eid_hig) {
            if let Some(vertex_hig) = elem_hig.get_vertex_mut(&vid_hig) {
                if unbound {
                    log!("Unbind {} to {}", vid_sel, vid_hig);
                    vertex_hig.bind.remove(&(eid_sel, vid_sel));
                } else {
                    log!("Binding {} to {}", vid_sel, vid_hig);
                    vertex_hig.bind.insert((eid_sel, vid_sel));
                }
            }
        }
        true
    }

    pub fn calc_final_polygon(&mut self) {
        if !self.final_polygon_dirty {
            return;
        }
        let mut unions: Vec<geo::Polygon<f64>> = Vec::new();
        let mut diffs: Vec<geo::Polygon<f64>> = Vec::new();
        for s in self.shapes.values() {
            if matches!(
                s.get_shape_type(),
                ShapeType::ConstrLine | ShapeType::ConstrCircle
            ) {
                continue;
            }
            match s.get_operation() {
                Operation::Union => {
                    for poly in s.get_polygon().iter() {
                        unions.push(normalize_polygon_orientation(poly));
                    }
                }
                Operation::Difference => {
                    for poly in s.get_polygon().iter() {
                        diffs.push(normalize_polygon_orientation(poly));
                    }
                }
            }
        }
        let poly_union = unary_union(&unions);
        let poly_diff = unary_union(&diffs);
        self.final_polygon = if diffs.len() > 0 {
            poly_union.boolean_op(&poly_diff, geo::OpType::Difference)
        } else {
            poly_union
        };
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
