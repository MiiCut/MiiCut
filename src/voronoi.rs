use kurbo::Vec2;
use spade::{DelaunayTriangulation, Point2, Triangulation};
use std::f64::consts::PI;

use crate::math::fillet_at_apex;

pub(crate) fn voronoi_cells(points: &[Vec2], excluded: &[Vec2]) -> Vec<Vec<Vec2>> {
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
                cells.push(cleaned);
            }
        }
    }

    cells
}

fn clean_ring(ring: &[Vec2], min_dist: f64) -> Vec<Vec2> {
    let mut out = Vec::new();
    for p in ring {
        if out
            .last()
            .map_or(true, |last| (*p - *last).hypot() >= min_dist)
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
