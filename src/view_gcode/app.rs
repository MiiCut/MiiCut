use crate::{
    app::{with_av_mut, AppVars, RefAV},
    canvas::{CanvasKind, Color, Pattern},
    helpers::canvas_fit::{center_segments_canvas, fit_segments_canvas, zoom_canvas_at},
    helpers::prefab::toolpath_points_to_path,
    inputs::SystemMouse,
    shapes::{toolpath_to_plasma_gcode, Toolpath},
    status::{begin_render, end_render, update_status_bar},
    view_draw::app::update,
};
use kurbo::{BezPath, PathEl, Point};
use wasm_bindgen::{closure::Closure, prelude::JsValue, JsCast};
use web_sys::{Event, MouseEvent, WheelEvent};

impl AppVars {
    pub(crate) fn refresh_gcode_cache(&mut self) {
        let gcode = self.last_gcode.as_deref().unwrap_or("");
        let segs = gcode_to_segments(gcode);
        let mut path = BezPath::new();
        for s in segs.iter().filter(|s| s.cut) {
            path.push(PathEl::MoveTo(Point::new(s.x1, s.y1)));
            path.push(PathEl::LineTo(Point::new(s.x2, s.y2)));
        }
        self.gcode_segments = segs;
        self.gcode_cut_path = if path.is_empty() { None } else { Some(path) };
    }
}

pub(crate) fn on_gcode_context_menu(_av: RefAV, event: Event) {
    event.prevent_default();
}

pub(crate) fn init_gcode_canvas(av: RefAV) -> Result<(), JsValue> {
    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_gcode_mouse_move(pa_cloneprim.clone(), event);
    });
    let c_gcode = pa_cloneme.borrow().canvases[CanvasKind::Gcode.idx()]
        .get_canvas()
        .clone();
    c_gcode.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())?;
    closure.forget();

    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_gcode_mouse_down(pa_cloneprim.clone(), event);
    });
    let c_gcode = pa_cloneme.borrow().canvases[CanvasKind::Gcode.idx()]
        .get_canvas()
        .clone();
    c_gcode.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())?;
    closure.forget();

    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_gcode_mouse_up(pa_cloneprim.clone(), event);
    });
    let c_gcode = pa_cloneme.borrow().canvases[CanvasKind::Gcode.idx()]
        .get_canvas()
        .clone();
    c_gcode.add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref())?;
    closure.forget();

    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_gcode_mouse_wheel(pa_cloneprim.clone(), event);
    });
    let c_gcode = pa_cloneme.borrow().canvases[CanvasKind::Gcode.idx()]
        .get_canvas()
        .clone();
    c_gcode.add_event_listener_with_callback("wheel", closure.as_ref().unchecked_ref())?;
    closure.forget();

    let pa_cloneme = av.clone();
    let pa_cloneprim = av.clone();
    let closure = Closure::<dyn FnMut(_)>::new(move |event: Event| {
        on_gcode_context_menu(pa_cloneprim.clone(), event);
    });
    let c_gcode = pa_cloneme.borrow().canvases[CanvasKind::Gcode.idx()]
        .get_canvas()
        .clone();
    c_gcode.add_event_listener_with_callback("contextmenu", closure.as_ref().unchecked_ref())?;
    closure.forget();

    Ok(())
}

pub(crate) fn on_gcode_mouse_move(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.dyn_into::<MouseEvent>() {
        let sys_mouse = SystemMouse::Move;
        let action = av.borrow_mut().update_canvas_inputs(mouse_event, sys_mouse);
        let _ = update(av.clone(), action);
    }
}

pub(crate) fn on_gcode_mouse_down(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.dyn_into::<MouseEvent>() {
        let sys_mouse = SystemMouse::Down(mouse_event.detail());
        let action = av.borrow_mut().update_canvas_inputs(mouse_event, sys_mouse);
        let _ = update(av.clone(), action);
    }
}

pub(crate) fn on_gcode_mouse_up(av: RefAV, event: Event) {
    if let Ok(mouse_event) = event.dyn_into::<MouseEvent>() {
        let sys_mouse = SystemMouse::Up(mouse_event.detail());
        let action = av.borrow_mut().update_canvas_inputs(mouse_event, sys_mouse);
        let _ = update(av.clone(), action);
    }
}

pub(crate) fn on_gcode_mouse_wheel(av: RefAV, event: Event) {
    if let Ok(wheel_event) = event.dyn_into::<WheelEvent>() {
        wheel_event.prevent_default();
        wheel_event.stop_propagation();
        let delta_y = wheel_event.delta_y();
        if let Ok(mouse_event) = wheel_event.clone().dyn_into::<MouseEvent>() {
            with_av_mut(&av, |avb| {
                let sys_mouse = SystemMouse::Move;
                let _ = avb.update_canvas_inputs(mouse_event, sys_mouse);
                let canvas = &mut avb.canvases[CanvasKind::Gcode.idx()];
                let world_pos = canvas.get_user_ui().draw_pos;
                let factor = if delta_y > 0.0 { 0.95 } else { 1.05 };
                zoom_canvas_at(canvas, world_pos, factor);
            });
            render_gcode_view(av.clone());
        }
    }
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
            fit_segments_canvas(canvas_gcode, &segs, |seg| (seg.x1, seg.y1, seg.x2, seg.y2));
        } else {
            center_segments_canvas(canvas_gcode, &segs, |seg| (seg.x1, seg.y1, seg.x2, seg.y2));
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

#[derive(Clone, Copy, Debug)]
pub struct Seg {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
    pub cut: bool,
} // cut=true => torche ON

pub fn gcode_to_segments(gcode: &str) -> Vec<Seg> {
    let mut segs = Vec::new();
    let mut x = 0.0;
    let mut y = 0.0;
    let mut torch_on = false;

    for raw in gcode.lines() {
        let line = raw.split(';').next().unwrap_or("").trim(); // retire commentaires ';'
        if line.is_empty() {
            continue;
        }

        let up = line.to_ascii_uppercase();

        if up.starts_with("M3") {
            torch_on = true;
            continue;
        }
        if up.starts_with("M5") {
            torch_on = false;
            continue;
        }

        let is_g0 = up.starts_with("G0") || up.starts_with("G00");
        let is_g1 = up.starts_with("G1") || up.starts_with("G01");
        let is_g2 = up.starts_with("G2") || up.starts_with("G02");
        let is_g3 = up.starts_with("G3") || up.starts_with("G03");
        if !is_g0 && !is_g1 && !is_g2 && !is_g3 {
            continue;
        }

        let mut nx = None::<f64>;
        let mut ny = None::<f64>;
        let mut ni = None::<f64>;
        let mut nj = None::<f64>;

        for tok in up.split_whitespace() {
            if let Some(v) = tok.strip_prefix('X') {
                if let Ok(f) = v.parse::<f64>() {
                    nx = Some(f);
                }
            } else if let Some(v) = tok.strip_prefix('Y') {
                if let Ok(f) = v.parse::<f64>() {
                    ny = Some(f);
                }
            } else if let Some(v) = tok.strip_prefix('I') {
                if let Ok(f) = v.parse::<f64>() {
                    ni = Some(f);
                }
            } else if let Some(v) = tok.strip_prefix('J') {
                if let Ok(f) = v.parse::<f64>() {
                    nj = Some(f);
                }
            }
        }

        let x2 = nx.unwrap_or(x);
        let y2 = ny.unwrap_or(y);

        if (is_g2 || is_g3) && (ni.is_some() || nj.is_some()) && (x2 != x || y2 != y) {
            let i = ni.unwrap_or(0.0);
            let j = nj.unwrap_or(0.0);
            let cx = x + i;
            let cy = y + j;
            let start_angle = (y - cy).atan2(x - cx);
            let end_angle = (y2 - cy).atan2(x2 - cx);
            let mut delta = if is_g3 {
                let mut d = end_angle - start_angle;
                if d <= 0.0 {
                    d += std::f64::consts::PI * 2.0;
                }
                d
            } else {
                let mut d = end_angle - start_angle;
                if d >= 0.0 {
                    d -= std::f64::consts::PI * 2.0;
                }
                d
            };
            if delta.abs() < 1e-6 {
                delta = 0.0;
            }
            let radius = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
            let arc_len = radius * delta.abs();
            let mut steps = (arc_len / 2.0).ceil() as usize;
            if steps < 8 {
                steps = 8;
            }
            let step = if steps > 0 { delta / steps as f64 } else { 0.0 };
            let mut px = x;
            let mut py = y;
            for k in 1..=steps {
                let ang = start_angle + step * k as f64;
                let nx = cx + radius * ang.cos();
                let ny = cy + radius * ang.sin();
                segs.push(Seg {
                    x1: px,
                    y1: py,
                    x2: nx,
                    y2: ny,
                    cut: torch_on,
                });
                px = nx;
                py = ny;
            }
            x = x2;
            y = y2;
        } else if x2 != x || y2 != y {
            segs.push(Seg {
                x1: x,
                y1: y,
                x2,
                y2,
                cut: torch_on && is_g1,
            });
            x = x2;
            y = y2;
        }
    }

    segs
}
