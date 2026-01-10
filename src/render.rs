use crate::app::RefAV;
use crate::canvas::{Canvas, CanvasKind, Color, Pattern};
use crate::dimensions::dim_hv;
use crate::gcode::Seg;
use crate::prefab::{get_vertices_colors, line_path, point_path};
use crate::shape::{ClosedShape, TextFont};
use crate::shapes::{toolpath_to_plasma_gcode, Toolpath};
use crate::status::{begin_render, end_render, update_status_bar};
use crate::types::{Binding, Couple, EUId, SegBundle, VUId};
use kurbo::{Arc, BezPath, PathEl, Point, Rect, Shape, Vec2};

pub(crate) fn draw_reset_origin(av: RefAV) {
    av.borrow_mut().get_user_canvas_mut().reset_origin();
}

pub(crate) fn draw_grid_and_rules(av: RefAV) {
    let mut avb = av.borrow_mut();
    let draw_scale = avb.canvases[CanvasKind::Draw.idx()].get_scale();
    let draw_offset = avb.canvases[CanvasKind::Draw.idx()].get_offset();
    let grid_canvas = &mut avb.canvases[CanvasKind::Grid.idx()];
    grid_canvas.draw_rules_only(draw_scale, draw_offset);
}

pub(crate) fn render_draw_view(av: RefAV) {
    begin_render(av.clone(), "Draw");
    draw_grid_and_rules(av.clone());
    update_status_bar(av.clone());

    let mut avb = av.borrow_mut();
    let svg_bbox_only = true;
    let element_on_creation = avb.element_on_creation.clone();
    let canvas_draw = &mut avb.canvases[CanvasKind::Draw.idx()];

    canvas_draw.clear();

    canvas_draw.draw_closed_path(Pattern::Composed(true), Color::Black, Color::Gray20, vec![]);

    if !canvas_draw.get_user_ui().keys_states.alt_pressed {
        canvas_draw.draw_paths_sets_with_svg_bbox(svg_bbox_only);

        canvas_draw.may_be_draw_radiuses();

        let binds: Binding<(EUId, VUId)> = canvas_draw.draw_vertices();
        for Couple((eid1, vid1), (eid2, vid2)) in binds.iter() {
            if let (Some(e1), Some(e2)) = (
                canvas_draw.dataset.get_element(*eid1),
                canvas_draw.dataset.get_element(*eid2),
            ) {
                if let (Some(v1), Some(v2)) = (e1.get_vertex(vid1), e2.get_vertex(vid2)) {
                    let p1 = e1.vertex_display_pos(v1.curr);
                    let p2 = e2.vertex_display_pos(v2.curr);
                    if let Some(seg) = SegBundle::new(p1, p2) {
                        let (path, pattern, colors, text) =
                            dim_hv(seg, canvas_draw.get_canvas_infos());
                        canvas_draw.draw_path(
                            &path,
                            pattern,
                            colors.fill_color,
                            colors.stroke_color,
                            text,
                        );
                    }
                }
            }
        }
    }

    if let Some((cs, mut vs)) = element_on_creation {
        match cs {
            crate::dom::ShapeType::Disc
            | crate::dom::ShapeType::Square
            | crate::dom::ShapeType::Oblong => {
                if vs.len() == 1 {
                    vs.push(canvas_draw.get_user_ui().pointer.curr);
                    if let Some(e) = ClosedShape::new(cs, &vs) {
                        canvas_draw.draw_paths_creation(&e);
                        canvas_draw.draw_vs(&e);
                        canvas_draw.draw_dimensions(&e);
                    }
                }
            }
            crate::dom::ShapeType::Text => {
                if vs.len() == 1 {
                    vs.push(canvas_draw.get_user_ui().pointer.curr);
                    if let Some(e) =
                        ClosedShape::new_text("TEXT".to_string(), TextFont::Stencilia, vs[0], vs[1])
                    {
                        canvas_draw.draw_paths_creation(&e);
                        canvas_draw.draw_vs(&e);
                        canvas_draw.draw_dimensions(&e);
                    }
                }
            }
            crate::dom::ShapeType::Poly => {
                if vs.len() > 2 {
                    vs.push(canvas_draw.get_user_ui().pointer.curr);
                    if let Some(e) = ClosedShape::new(cs, &vs) {
                        canvas_draw.draw_paths_creation(&e);
                        canvas_draw.draw_vs(&e);
                        canvas_draw.draw_dimensions(&e);
                    }
                } else {
                    vs.push(canvas_draw.get_user_ui().pointer.curr);
                    if vs.len() >= 2 {
                        let colors = get_vertices_colors(false, false);
                        for (i, v) in vs.iter().enumerate() {
                            if i < vs.len() - 1 {
                                canvas_draw.draw_path(
                                    &line_path(vs[i], vs[i + 1]),
                                    Pattern::OnCreation,
                                    colors.fill_color,
                                    colors.stroke_color,
                                    vec![],
                                );
                                if let Some(seg) = SegBundle::new(vs[i], vs[i + 1]) {
                                    let (path, pattern, colors, text) =
                                        dim_hv(seg, canvas_draw.get_canvas_infos());
                                    canvas_draw.draw_path(
                                        &path,
                                        pattern,
                                        colors.fill_color,
                                        colors.stroke_color,
                                        text,
                                    );
                                }
                            }
                            canvas_draw.draw_path(
                                &point_path(*v, 1.),
                                Pattern::Point,
                                colors.fill_color,
                                colors.stroke_color,
                                vec![],
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }

    canvas_draw.draw_pointer(canvas_draw.get_user_ui().pointer.curr);
    avb.update_draw_cursor();

    drop(avb);
    end_render(av.clone());
    update_status_bar(av);
}

pub(crate) fn render_gcode_view(av: RefAV) {
    begin_render(av.clone(), "G-code");
    update_status_bar(av.clone());
    let mut avb = av.borrow_mut();
    if avb.last_gcode.is_none() {
        avb.refresh_toolpath_cache();
        let toolpath = avb.toolpath.clone().unwrap_or(Toolpath::new(Vec::new()));
        let gcode = toolpath_to_plasma_gcode(&toolpath, &avb.toolpath_params);
        avb.last_gcode = Some(gcode);
        avb.refresh_gcode_cache();
    }
    let gcode = avb.last_gcode.as_deref().unwrap_or("");

    if let Some(el) = avb.document.get_element_by_id("gcode-text") {
        el.set_text_content(Some(gcode));
    }

    let gcode_action = if avb.gcode_auto_fit {
        avb.gcode_auto_center = false;
        avb.gcode_auto_fit = false;
        Some(true)
    } else if avb.gcode_auto_center {
        avb.gcode_auto_center = false;
        Some(false)
    } else {
        None
    };

    if let Some(do_fit) = gcode_action {
        let segs = avb.gcode_segments.clone();
        let canvas_gcode = &mut avb.canvases[CanvasKind::Gcode.idx()];
        if do_fit {
            fit_gcode_canvas(canvas_gcode, &segs);
        } else {
            center_gcode_canvas(canvas_gcode, &segs);
        }
    }
    let toolpath = avb.toolpath.clone();
    let lead_ins = avb.toolpath_lead_ins.clone();
    let lead_outs = avb.toolpath_lead_outs.clone();
    let path = avb.gcode_cut_path.clone();
    let canvas_gcode = &mut avb.canvases[CanvasKind::Gcode.idx()];
    canvas_gcode.clear();

    if let Some(toolpath) = toolpath.as_ref() {
        for contour in toolpath.contours.iter() {
            if let Some(path) = toolpath_points_to_path(&contour.points) {
                let color = if contour.is_hole {
                    Color::Purple55
                } else {
                    Color::Black
                };
                canvas_gcode.draw_path(&path, Pattern::Dim, Color::Transparent, color, vec![]);
            }
        }
        for path in lead_ins.iter() {
            canvas_gcode.draw_path(
                path,
                Pattern::Dim,
                Color::Transparent,
                Color::Green40,
                vec![],
            );
        }
        for path in lead_outs.iter() {
            canvas_gcode.draw_path(path, Pattern::Dim, Color::Transparent, Color::Red60, vec![]);
        }
    } else if let Some(path) = path.as_ref() {
        canvas_gcode.draw_path(path, Pattern::Dim, Color::Transparent, Color::Black, vec![]);
    }
    drop(avb);
    end_render(av.clone());
    update_status_bar(av);
}

pub(crate) fn render_machine_view(av: RefAV) {
    begin_render(av.clone(), "Machine");
    update_status_bar(av.clone());
    let _ = av.borrow_mut().ensure_machine_view(av.clone());
    end_render(av.clone());
    update_status_bar(av);
}

pub(crate) fn toolpath_points_to_path(points: &[Vec2]) -> Option<BezPath> {
    if points.len() < 2 {
        return None;
    }
    let mut path = BezPath::new();
    path.push(PathEl::MoveTo(points[0].to_point()));
    for pt in points.iter().skip(1) {
        path.push(PathEl::LineTo(pt.to_point()));
    }
    path.push(PathEl::ClosePath);
    Some(path)
}

pub(crate) fn toolpath_arc_path(
    center: Vec2,
    start: Vec2,
    end: Vec2,
    ccw: bool,
) -> Option<BezPath> {
    let v0 = start - center;
    let v1 = end - center;
    let r0 = v0.hypot();
    let r1 = v1.hypot();
    if r0 == 0.0 || r1 == 0.0 {
        return None;
    }
    let r = 0.5 * (r0 + r1);
    let a0 = v0.y.atan2(v0.x);
    let a1 = v1.y.atan2(v1.x);
    let mut sweep = a1 - a0;
    if ccw {
        while sweep <= 0.0 {
            sweep += std::f64::consts::PI * 2.0;
        }
    } else {
        while sweep >= 0.0 {
            sweep -= std::f64::consts::PI * 2.0;
        }
    }

    let mut path = BezPath::new();
    path.push(PathEl::MoveTo(start.to_point()));
    let arc = Arc {
        center: Point::new(center.x, center.y),
        radii: Vec2::new(r, r),
        start_angle: a0,
        sweep_angle: sweep,
        x_rotation: 0.0,
    };
    arc.to_cubic_beziers(0.01, |p1, p2, p3| {
        path.push(PathEl::CurveTo(p1, p2, p3));
    });
    Some(path)
}

pub(crate) fn toolpath_arrowhead_path(start: Vec2, dir: Vec2, size: f64) -> Option<BezPath> {
    if dir.hypot() <= 0.0001 {
        return None;
    }
    let dir = dir.normalize();
    let perp = Vec2::new(-dir.y, dir.x);
    let tip = start + dir * size;
    let left = start + perp * (size * 0.5);
    let right = start - perp * (size * 0.5);

    let mut path = BezPath::new();
    path.push(PathEl::MoveTo(tip.to_point()));
    path.push(PathEl::LineTo(left.to_point()));
    path.push(PathEl::LineTo(right.to_point()));
    path.push(PathEl::ClosePath);
    Some(path)
}

pub(crate) fn render_toolpath_view(av: RefAV) {
    begin_render(av.clone(), "Toolpath");
    update_status_bar(av.clone());
    let mut avb = av.borrow_mut();

    let action = if avb.toolpath_auto_fit {
        avb.toolpath_auto_center = false;
        avb.toolpath_auto_fit = false;
        Some(true)
    } else if avb.toolpath_auto_center {
        avb.toolpath_auto_center = false;
        Some(false)
    } else {
        None
    };

    if let Some(do_fit) = action {
        let mut paths = avb.toolpath_paths.clone();
        paths.extend(avb.toolpath_lead_ins.iter().cloned());
        paths.extend(avb.toolpath_lead_outs.iter().cloned());
        paths.extend(avb.toolpath_travels.iter().cloned());
        paths.extend(avb.toolpath_travel_arrows.iter().cloned());
        paths.extend(avb.toolpath_arrows.iter().cloned());
        let canvas_toolpath = &mut avb.canvases[CanvasKind::Toolpath.idx()];
        if do_fit {
            fit_paths_canvas(canvas_toolpath, &paths);
        } else {
            center_paths_canvas(canvas_toolpath, &paths);
        }
    }

    let original_paths = avb.toolpath_original_paths.clone();
    let paths = avb.toolpath_paths.clone();
    let lead_ins = avb.toolpath_lead_ins.clone();
    let lead_outs = avb.toolpath_lead_outs.clone();
    let travels = avb.toolpath_travels.clone();
    let travel_arrows = avb.toolpath_travel_arrows.clone();
    let arrows = avb.toolpath_arrows.clone();
    let starts = avb.toolpath_starts.clone();
    let ends = avb.toolpath_ends.clone();
    let canvas_toolpath = &mut avb.canvases[CanvasKind::Toolpath.idx()];
    canvas_toolpath.clear();

    for path in original_paths.iter() {
        canvas_toolpath.draw_path(
            path,
            Pattern::Dim,
            Color::Transparent,
            Color::Gray20,
            vec![],
        );
    }
    for path in travels.iter() {
        canvas_toolpath.draw_path(
            path,
            Pattern::Dim,
            Color::Transparent,
            Color::Purple55,
            vec![],
        );
    }
    for path in travel_arrows.iter() {
        canvas_toolpath.draw_path(
            path,
            Pattern::Point,
            Color::Purple55,
            Color::Purple55,
            vec![],
        );
    }
    for path in paths.iter() {
        canvas_toolpath.draw_path(path, Pattern::Dim, Color::Transparent, Color::Black, vec![]);
    }
    for path in lead_ins.iter() {
        canvas_toolpath.draw_path(
            path,
            Pattern::Dim,
            Color::Transparent,
            Color::Green40,
            vec![],
        );
    }
    for path in lead_outs.iter() {
        canvas_toolpath.draw_path(path, Pattern::Dim, Color::Transparent, Color::Red60, vec![]);
    }
    for path in arrows.iter() {
        canvas_toolpath.draw_path(path, Pattern::Point, Color::Green40, Color::Green40, vec![]);
    }
    let scale = canvas_toolpath.get_scale();
    for pos in starts.iter() {
        canvas_toolpath.draw_path(
            &point_path(*pos, scale),
            Pattern::Point,
            Color::Transparent,
            Color::Green40,
            vec![],
        );
    }
    for pos in ends.iter() {
        canvas_toolpath.draw_path(
            &point_path(*pos, scale),
            Pattern::Point,
            Color::Transparent,
            Color::Red60,
            vec![],
        );
    }
    drop(avb);
    end_render(av.clone());
    update_status_bar(av);
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

pub(crate) fn center_gcode_canvas(canvas: &mut Canvas, segs: &[Seg]) {
    if segs.is_empty() {
        return;
    }

    let mut min_x = segs[0].x1.min(segs[0].x2);
    let mut max_x = segs[0].x1.max(segs[0].x2);
    let mut min_y = segs[0].y1.min(segs[0].y2);
    let mut max_y = segs[0].y1.max(segs[0].y2);

    for s in segs.iter() {
        let sx_min = s.x1.min(s.x2);
        let sx_max = s.x1.max(s.x2);
        let sy_min = s.y1.min(s.y2);
        let sy_max = s.y1.max(s.y2);
        min_x = min_x.min(sx_min);
        max_x = max_x.max(sx_max);
        min_y = min_y.min(sy_min);
        max_y = max_y.max(sy_max);
    }

    let center = Vec2::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
    let size = canvas.get_canvas_size();
    let canvas_center = Vec2::new(size.width * 0.5, size.height * 0.5);
    let offset = canvas_center - center * canvas.get_scale();
    canvas.set_offset(offset);
}

pub(crate) fn fit_gcode_canvas(canvas: &mut Canvas, segs: &[Seg]) {
    if segs.is_empty() {
        return;
    }

    let mut min_x = segs[0].x1.min(segs[0].x2);
    let mut max_x = segs[0].x1.max(segs[0].x2);
    let mut min_y = segs[0].y1.min(segs[0].y2);
    let mut max_y = segs[0].y1.max(segs[0].y2);

    for s in segs.iter() {
        let sx_min = s.x1.min(s.x2);
        let sx_max = s.x1.max(s.x2);
        let sy_min = s.y1.min(s.y2);
        let sy_max = s.y1.max(s.y2);
        min_x = min_x.min(sx_min);
        max_x = max_x.max(sx_max);
        min_y = min_y.min(sy_min);
        max_y = max_y.max(sy_max);
    }

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
