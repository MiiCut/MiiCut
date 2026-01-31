use crate::canvas::Canvas;
use kurbo::{BezPath, Rect, Shape, Vec2};

pub(crate) fn zoom_canvas_at(canvas: &mut Canvas, world_pos: Vec2, factor: f64) {
    let scale = canvas.get_scale();
    let view_size = canvas.get_canvas_size();
    let max_world_w = 3500.0;
    let max_world_h = 3500.0;
    let min_scale_x = view_size.width / max_world_w;
    let min_scale_y = view_size.height / max_world_h;
    let min_scale = min_scale_x.max(min_scale_y).max(0.01);
    let new_scale = (scale * factor).clamp(min_scale, 10.0);
    if (new_scale - scale).abs() < f64::EPSILON {
        return;
    }
    let offset = canvas.get_offset();
    let new_offset = offset + world_pos * (scale - new_scale);
    canvas.set_scale(new_scale);
    canvas.set_offset(new_offset);
}

pub(crate) fn bounds_from_paths(paths: &[BezPath]) -> Option<Rect> {
    let mut rect: Option<Rect> = None;
    for path in paths.iter() {
        if path.is_empty() {
            continue;
        }
        let bbox = path.bounding_box();
        rect = Some(match rect {
            Some(prev) => prev.union(bbox),
            None => bbox,
        });
    }
    rect
}

pub(crate) fn center_paths_canvas(canvas: &mut Canvas, paths: &[BezPath]) {
    let Some(bbox) = bounds_from_paths(paths) else {
        return;
    };
    let center = Vec2::new((bbox.x0 + bbox.x1) * 0.5, (bbox.y0 + bbox.y1) * 0.5);
    let size = canvas.get_canvas_size();
    let canvas_center = Vec2::new(size.width * 0.5, size.height * 0.5);
    let offset = canvas_center - center * canvas.get_scale();
    canvas.set_offset(offset);
}

pub(crate) fn fit_paths_canvas(canvas: &mut Canvas, paths: &[BezPath]) {
    let Some(bbox) = bounds_from_paths(paths) else {
        return;
    };
    let bbox_w = (bbox.x1 - bbox.x0).max(1.0);
    let bbox_h = (bbox.y1 - bbox.y0).max(1.0);

    let size = canvas.get_canvas_size();
    let pad = 20.0;
    let avail_w = (size.width - pad * 2.0).max(1.0);
    let avail_h = (size.height - pad * 2.0).max(1.0);

    let scale = (avail_w / bbox_w).min(avail_h / bbox_h).clamp(0.5, 10.0);
    canvas.set_scale(scale);

    let center = Vec2::new((bbox.x0 + bbox.x1) * 0.5, (bbox.y0 + bbox.y1) * 0.5);
    let canvas_center = Vec2::new(size.width * 0.5, size.height * 0.5);
    let offset = canvas_center - center * scale;
    canvas.set_offset(offset);
}

fn bounds_from_segments<T, F>(segs: &[T], mut points: F) -> Option<(f64, f64, f64, f64)>
where
    F: FnMut(&T) -> (f64, f64, f64, f64),
{
    let first = segs.first()?;
    let (x1, y1, x2, y2) = points(first);
    let mut min_x = x1.min(x2);
    let mut max_x = x1.max(x2);
    let mut min_y = y1.min(y2);
    let mut max_y = y1.max(y2);

    for s in segs.iter().skip(1) {
        let (sx1, sy1, sx2, sy2) = points(s);
        let sx_min = sx1.min(sx2);
        let sx_max = sx1.max(sx2);
        let sy_min = sy1.min(sy2);
        let sy_max = sy1.max(sy2);
        min_x = min_x.min(sx_min);
        max_x = max_x.max(sx_max);
        min_y = min_y.min(sy_min);
        max_y = max_y.max(sy_max);
    }

    Some((min_x, max_x, min_y, max_y))
}

pub(crate) fn center_segments_canvas<T, F>(canvas: &mut Canvas, segs: &[T], points: F)
where
    F: FnMut(&T) -> (f64, f64, f64, f64),
{
    let Some((min_x, max_x, min_y, max_y)) = bounds_from_segments(segs, points) else {
        return;
    };
    let center = Vec2::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
    let size = canvas.get_canvas_size();
    let canvas_center = Vec2::new(size.width * 0.5, size.height * 0.5);
    let offset = canvas_center - center * canvas.get_scale();
    canvas.set_offset(offset);
}

pub(crate) fn fit_segments_canvas<T, F>(canvas: &mut Canvas, segs: &[T], points: F)
where
    F: FnMut(&T) -> (f64, f64, f64, f64),
{
    let Some((min_x, max_x, min_y, max_y)) = bounds_from_segments(segs, points) else {
        return;
    };

    let bbox_w = (max_x - min_x).max(1.0);
    let bbox_h = (max_y - min_y).max(1.0);

    let size = canvas.get_canvas_size();
    let pad = 20.0;
    let avail_w = (size.width - pad * 2.0).max(1.0);
    let avail_h = (size.height - pad * 2.0).max(1.0);

    let scale = (avail_w / bbox_w).min(avail_h / bbox_h).clamp(0.5, 10.0);
    canvas.set_scale(scale);

    let center = Vec2::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
    let canvas_center = Vec2::new(size.width * 0.5, size.height * 0.5);
    let offset = canvas_center - center * scale;
    canvas.set_offset(offset);
}
