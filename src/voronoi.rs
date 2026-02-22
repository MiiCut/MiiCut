use kurbo::Vec2;
use spade::{DelaunayTriangulation, Point2, Triangulation};
use std::f64::consts::PI;

use crate::helpers::math::fillet_at_apex;

pub(crate) fn voronoi_cells(
    points: &[Vec2],
    excluded: &[Vec2],
    clip_min: Vec2,
    clip_max: Vec2,
) -> Vec<Vec<Vec2>> {
    voronoi_cells_with_sites(points, excluded, clip_min, clip_max)
        .into_iter()
        .map(|(_, cell)| cell)
        .collect()
}

pub(crate) fn lloyd_relax_points(
    points: &mut [Vec2],
    excluded: &[Vec2],
    clip_min: Vec2,
    clip_max: Vec2,
    iterations: usize,
) {
    if points.is_empty() || iterations == 0 {
        return;
    }

    for _ in 0..iterations {
        let mut all_points = Vec::with_capacity(points.len() + excluded.len());
        all_points.extend_from_slice(points);
        all_points.extend_from_slice(excluded);
        let cells = voronoi_cells_with_sites(&all_points, excluded, clip_min, clip_max);
        if cells.is_empty() {
            break;
        }

        let mut next = points.to_vec();
        for (idx, point) in points.iter().enumerate() {
            let mut best: Option<(f64, Vec2)> = None;
            for (site, cell) in &cells {
                let Some(centroid) = polygon_centroid(cell) else {
                    continue;
                };
                let dist = (*site - *point).hypot();
                if best.is_none_or(|(best_dist, _)| dist < best_dist) {
                    best = Some((dist, centroid));
                }
            }
            if let Some((_, centroid)) = best {
                next[idx] = Vec2::new(
                    centroid.x.clamp(clip_min.x, clip_max.x),
                    centroid.y.clamp(clip_min.y, clip_max.y),
                );
            }
        }
        points.copy_from_slice(&next);
    }
}

fn voronoi_cells_with_sites(
    points: &[Vec2],
    excluded: &[Vec2],
    clip_min: Vec2,
    clip_max: Vec2,
) -> Vec<(Vec2, Vec<Vec2>)> {
    if points.len() < 3 {
        return Vec::new();
    }

    let sites: Vec<Point2<f64>> = points.iter().map(|p| Point2::new(p.x, p.y)).collect();
    let triangulation: DelaunayTriangulation<Point2<f64>> =
        match DelaunayTriangulation::bulk_load(sites) {
            Ok(triangulation) => triangulation,
            Err(_) => return Vec::new(),
        };

    let mut cells = Vec::new();
    for face in triangulation.voronoi_faces() {
        let site = face.as_delaunay_vertex().position();
        let site = Vec2::new(site.x, site.y);
        if excluded.iter().any(|pt| (*pt - site).hypot() < 1e-6) {
            continue;
        }
        let mut ring = Vec::new();
        let mut finite = true;
        for edge in face.adjacent_edges() {
            match edge.from().position() {
                Some(pos) => ring.push(Vec2::new(pos.x, pos.y)),
                None => {
                    finite = false;
                    break;
                }
            }
        }

        if finite && ring.len() >= 3 {
            let cleaned = clean_ring(&ring, 2.0);
            if cleaned.len() >= 3 {
                let clipped = clip_ring_to_rect(&cleaned, clip_min, clip_max);
                let clipped_cleaned = clean_ring(&clipped, 1e-6);
                if clipped_cleaned.len() >= 3 {
                    cells.push((site, clipped_cleaned));
                }
            }
        }
    }

    cells
}

fn clip_ring_to_rect(ring: &[Vec2], min: Vec2, max: Vec2) -> Vec<Vec2> {
    let clipped_left = clip_polygon(
        ring,
        |p| p.x >= min.x,
        |a, b| line_x_intersection(a, b, min.x),
    );
    let clipped_right = clip_polygon(
        &clipped_left,
        |p| p.x <= max.x,
        |a, b| line_x_intersection(a, b, max.x),
    );
    let clipped_bottom = clip_polygon(
        &clipped_right,
        |p| p.y >= min.y,
        |a, b| line_y_intersection(a, b, min.y),
    );
    clip_polygon(
        &clipped_bottom,
        |p| p.y <= max.y,
        |a, b| line_y_intersection(a, b, max.y),
    )
}

fn clip_polygon<FInside, FIntersect>(
    input: &[Vec2],
    inside: FInside,
    intersect: FIntersect,
) -> Vec<Vec2>
where
    FInside: Fn(Vec2) -> bool,
    FIntersect: Fn(Vec2, Vec2) -> Vec2,
{
    if input.is_empty() {
        return Vec::new();
    }

    let mut output = Vec::new();
    let mut prev = *input.last().unwrap_or(&Vec2::ZERO);
    let mut prev_inside = inside(prev);
    for curr in input {
        let curr = *curr;
        let curr_inside = inside(curr);
        if curr_inside {
            if !prev_inside {
                output.push(intersect(prev, curr));
            }
            output.push(curr);
        } else if prev_inside {
            output.push(intersect(prev, curr));
        }
        prev = curr;
        prev_inside = curr_inside;
    }
    output
}

fn line_x_intersection(a: Vec2, b: Vec2, x: f64) -> Vec2 {
    let dx = b.x - a.x;
    if dx.abs() < 1e-9 {
        return Vec2::new(x, a.y);
    }
    let t = ((x - a.x) / dx).clamp(0.0, 1.0);
    Vec2::new(x, a.y + (b.y - a.y) * t)
}

fn line_y_intersection(a: Vec2, b: Vec2, y: f64) -> Vec2 {
    let dy = b.y - a.y;
    if dy.abs() < 1e-9 {
        return Vec2::new(a.x, y);
    }
    let t = ((y - a.y) / dy).clamp(0.0, 1.0);
    Vec2::new(a.x + (b.x - a.x) * t, y)
}

fn clean_ring(ring: &[Vec2], min_dist: f64) -> Vec<Vec2> {
    let mut out = Vec::new();
    for p in ring {
        if out
            .last()
            .is_none_or(|last| (*p - *last).hypot() >= min_dist)
        {
            out.push(*p);
        }
    }
    if out.len() > 1 && (out[0] - out[out.len() - 1]).hypot() < min_dist {
        out.pop();
    }
    if out.len() < 3 {
        return out;
    }
    let mut filtered = Vec::new();
    let n = out.len();
    for i in 0..n {
        let prev = out[(i + n - 1) % n];
        let curr = out[i];
        let next = out[(i + 1) % n];
        let v1 = curr - prev;
        let v2 = next - curr;
        if v1.hypot() <= 1e-6 || v2.hypot() <= 1e-6 {
            continue;
        }
        let cross = v1.cross(v2).abs();
        if cross <= 1e-6 {
            continue;
        }
        filtered.push(curr);
    }
    filtered
}
pub(crate) fn inset_rings(rings: Vec<Vec<Vec2>>, gap: f64) -> Vec<Vec<Vec2>> {
    if gap <= 0.0 {
        return rings;
    }
    let mut out = Vec::new();
    for ring in rings {
        let Some(center) = ring_centroid(&ring) else {
            continue;
        };
        let mut inset = Vec::with_capacity(ring.len());
        for p in ring {
            let v = p - center;
            let len = v.hypot();
            if len <= gap {
                continue;
            }
            inset.push(center + v * ((len - gap) / len));
        }
        if inset.len() >= 3 {
            out.push(inset);
        }
    }
    out
}

pub(crate) fn round_rings(rings: Vec<Vec<Vec2>>, radius: f64, segments: usize) -> Vec<Vec<Vec2>> {
    if radius <= 0.0 || segments == 0 {
        return rings;
    }
    let mut out = Vec::new();
    for ring in rings {
        if ring.len() < 3 {
            continue;
        }
        let rounded = round_ring_vertices(&ring, radius, segments);
        if rounded.len() >= 3 {
            out.push(rounded);
        }
    }
    out
}

fn ring_centroid(ring: &[Vec2]) -> Option<Vec2> {
    if ring.is_empty() {
        return None;
    }
    let mut sum = Vec2::ZERO;
    for p in ring {
        sum += *p;
    }
    Some(sum / ring.len() as f64)
}

fn polygon_centroid(ring: &[Vec2]) -> Option<Vec2> {
    if ring.len() < 3 {
        return None;
    }
    let mut twice_area = 0.0;
    let mut cx = 0.0;
    let mut cy = 0.0;
    for i in 0..ring.len() {
        let p0 = ring[i];
        let p1 = ring[(i + 1) % ring.len()];
        let cross = p0.x * p1.y - p1.x * p0.y;
        twice_area += cross;
        cx += (p0.x + p1.x) * cross;
        cy += (p0.y + p1.y) * cross;
    }
    if twice_area.abs() <= 1e-9 {
        return ring_centroid(ring);
    }
    let factor = 1.0 / (3.0 * twice_area);
    Some(Vec2::new(cx * factor, cy * factor))
}

fn round_ring_vertices(ring: &[Vec2], radius: f64, segments: usize) -> Vec<Vec2> {
    let n = ring.len();
    let mut out = Vec::new();
    for i in 0..n {
        let a = ring[(i + n - 1) % n];
        let b = ring[i];
        let c = ring[(i + 1) % n];

        let Some((start, center, end)) = fillet_at_apex(a, b, c, radius) else {
            log!("Could not fillet at apex");
            out.push(b);
            continue;
        };

        let a0 = (start - center).atan2();
        let a1 = (end - center).atan2();
        let mut delta = a1 - a0;
        while delta > PI {
            delta -= 2.0 * PI;
        }
        while delta < -PI {
            delta += 2.0 * PI;
        }

        out.push(start);
        for s in 1..segments {
            let t = s as f64 / segments as f64;
            let a = a0 + delta * t;
            out.push(center + Vec2::new(a.cos(), a.sin()) * radius);
        }
        out.push(end);
    }
    out
}
