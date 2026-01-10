use crate::app::RefAV;
use crate::canvas::CanvasKind;
use crate::dom::ShapeType;
use crate::shape::{ClosedShape, Operation, SvgData, SvgFillRule, TextData, TextFont};
use crate::shapes::DataSet;
use crate::status::update_status_bar;
use crate::types::EUId;
use js_sys::{Array, Date, JSON};
use kurbo::{flatten, BezPath, PathEl, Shape, Size, Vec2};
use svg::parser::{Event as SvgEvent, Parser as SvgParser};
use wasm_bindgen::JsValue;
use wasm_bindgen::JsCast;
use web_sys::{Blob, BlobPropertyBag, Document, HtmlAnchorElement, Url};

pub(crate) fn build_svg_from_dataset(dataset: &DataSet) -> Option<String> {
    let paths = geo_multipolygon_to_bez_paths(&dataset.final_polygon);
    if paths.is_empty() {
        return None;
    }

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for path in &paths {
        if path.is_empty() {
            continue;
        }
        let bbox = path.bounding_box();
        min_x = min_x.min(bbox.x0);
        min_y = min_y.min(bbox.y0);
        max_x = max_x.max(bbox.x1);
        max_y = max_y.max(bbox.y1);
    }

    if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
        return None;
    }

    let width = (max_x - min_x).max(1.0);
    let height = (max_y - min_y).max(1.0);

    let mut svg = String::new();
    svg.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{min_x:.3} {min_y:.3} {width:.3} {height:.3}\">\n"
    ));
    svg.push_str("  <g fill=\"none\" stroke=\"black\" stroke-width=\"1\">\n");

    for path in paths {
        let mut data = String::new();
        for el in path.elements() {
            match el {
                PathEl::MoveTo(p) => {
                    data.push_str(&format!("M{:.3} {:.3} ", p.x, p.y));
                }
                PathEl::LineTo(p) => {
                    data.push_str(&format!("L{:.3} {:.3} ", p.x, p.y));
                }
                PathEl::CurveTo(p1, p2, p3) => {
                    data.push_str(&format!(
                        "C{:.3} {:.3} {:.3} {:.3} {:.3} {:.3} ",
                        p1.x, p1.y, p2.x, p2.y, p3.x, p3.y
                    ));
                }
                PathEl::QuadTo(p1, p2) => {
                    data.push_str(&format!(
                        "Q{:.3} {:.3} {:.3} {:.3} ",
                        p1.x, p1.y, p2.x, p2.y
                    ));
                }
                PathEl::ClosePath => {
                    data.push_str("Z");
                }
            }
        }
        svg.push_str(&format!("    <path d=\"{data}\" />\n"));
    }

    svg.push_str("  </g>\n</svg>\n");
    Some(svg)
}

pub(crate) fn build_json_from_dataset(dataset: &DataSet, meta: &ExportInfo) -> Option<String> {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str("  \"version\": 1,\n");
    out.push_str("  \"meta\": {\n");
    out.push_str(&format!(
        "    \"title\": \"{}\",\n",
        json_escape(meta.title.as_deref().unwrap_or("MiiCut"))
    ));
    out.push_str(&format!("    \"timestamp\": \"{}\",\n", meta.timestamp));
    out.push_str("    \"canvas\": {\n");
    out.push_str(&format!(
        "      \"size\": [{:.6}, {:.6}],\n",
        meta.canvas_size.0, meta.canvas_size.1
    ));
    out.push_str(&format!("      \"scale\": {:.6},\n", meta.canvas_scale));
    out.push_str(&format!(
        "      \"offset\": [{:.6}, {:.6}]\n",
        meta.canvas_offset.0, meta.canvas_offset.1
    ));
    out.push_str("    }\n");
    out.push_str("  },\n");
    out.push_str("  \"shapes\": [\n");

    let mut first = true;
    for (eid, elem) in &dataset.shapes {
        if !first {
            out.push_str(",\n");
        }
        first = false;
        let shape_type = icon_to_name(elem.get_shape_type());
        let op_name = operation_to_name(elem.get_operation());
        out.push_str("    {\n");
        out.push_str(&format!("      \"id\": {},\n", eid));
        out.push_str(&format!("      \"type\": \"{shape_type}\",\n"));
        out.push_str(&format!("      \"operation\": \"{op_name}\",\n"));
        out.push_str(&format!("      \"rotation\": {:.6},\n", elem.get_rotation()));
        let vertices = elem.get_vertices();
        out.push_str("      \"vertices\": [\n");
        for (idx, (_, v)) in vertices.iter().enumerate() {
            let separator = if idx + 1 == vertices.len() {
                "\n"
            } else {
                ",\n"
            };
            let rounded = v.rounded.map(|val| val.to_string()).unwrap_or("null".to_string());
            let binds = if v.bind.is_empty() {
                "[]".to_string()
            } else {
                let mut binds_str = String::new();
                binds_str.push('[');
                for (b_idx, (bind_eid, _)) in v.bind.iter().enumerate() {
                    if b_idx > 0 {
                        binds_str.push_str(", ");
                    }
                    binds_str.push_str(&format!("{bind_eid}"));
                }
                binds_str.push(']');
                binds_str
            };
            out.push_str(&format!(
                "        {{\"x\": {:.6}, \"y\": {:.6}, \"rounded\": {rounded}, \"binds\": {binds}}}{separator}",
                v.curr.x, v.curr.y
            ));
        }
        out.push_str("      ]");

        if let Some(text) = elem.get_text() {
            out.push_str(",\n      \"text\": {\n");
            out.push_str(&format!(
                "        \"content\": \"{}\",\n",
                json_escape(&text.text)
            ));
            let scale = text
                .scale
                .map(|value| format!("{value:.6}"))
                .unwrap_or_else(|| "null".to_string());
            out.push_str(&format!("        \"scale\": {scale},\n"));
            out.push_str(&format!(
                "        \"auto_fit\": {},\n",
                if text.auto_fit { "true" } else { "false" }
            ));
            let font_name = text_font_to_name(&text.font);
            out.push_str(&format!("        \"font\": \"{font_name}\"\n"));
            out.push_str("      }");
        } else if let Some(svg) = elem.get_svg() {
            out.push_str(",\n      \"svg\": {\n");
            let fill_rule = match svg.fill_rule {
                SvgFillRule::NonZero => "nonzero",
                SvgFillRule::EvenOdd => "evenodd",
            };
            out.push_str(&format!("        \"fill_rule\": \"{fill_rule}\",\n"));
            out.push_str("        \"rings\": [\n");
            for (r_idx, ring) in svg.rings.iter().enumerate() {
                let sep = if r_idx + 1 == svg.rings.len() {
                    "\n"
                } else {
                    ",\n"
                };
                out.push_str("          [");
                for (p_idx, p) in ring.iter().enumerate() {
                    if p_idx > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&format!("[{:.6}, {:.6}]", p.x, p.y));
                }
                out.push_str(&format!("]{sep}"));
            }
            out.push_str("        ]\n");
            out.push_str("      }");
        }

        out.push_str("\n    }");
    }

    out.push_str("\n  ]\n}\n");
    Some(out)
}

pub(crate) fn get_prop(value: &JsValue, name: &str) -> Option<JsValue> {
    js_sys::Reflect::get(value, &JsValue::from_str(name)).ok()
}

pub(crate) fn get_string(value: &JsValue, name: &str) -> Option<String> {
    get_prop(value, name).and_then(|val| val.as_string())
}

pub(crate) fn get_f64(value: &JsValue, name: &str) -> Option<f64> {
    get_prop(value, name).and_then(|val| val.as_f64())
}

pub(crate) fn get_vec2_array(value: &JsValue, name: &str) -> Option<Vec2> {
    let arr = get_prop(value, name)?;
    let arr = Array::from(&arr);
    if arr.length() < 2 {
        return None;
    }
    let x = arr.get(0).as_f64()?;
    let y = arr.get(1).as_f64()?;
    Some(Vec2::new(x, y))
}

pub(crate) fn name_to_icon(name: &str) -> Option<ShapeType> {
    match name {
        "disc" => Some(ShapeType::Disc),
        "square" => Some(ShapeType::Square),
        "oblong" => Some(ShapeType::Oblong),
        "poly" => Some(ShapeType::Poly),
        "text" => Some(ShapeType::Text),
        "svg" => Some(ShapeType::Svg),
        _ => None,
    }
}

pub(crate) fn name_to_operation(name: String) -> Option<Operation> {
    match name.as_str() {
        "union" => Some(Operation::Union),
        "difference" => Some(Operation::Difference),
        _ => None,
    }
}

pub(crate) fn name_to_text_font(name: String) -> Option<TextFont> {
    match name.as_str() {
        "stencilia" => Some(TextFont::Stencilia),
        "urbanist" => Some(TextFont::Urbanist),
        _ => None,
    }
}

pub(crate) fn trigger_download(document: &Document, filename: &str, contents: &str, mime: &str) {
    if let Some(body) = document.body() {
        if let Ok(link) = document.create_element("a") {
            if let Ok(link) = link.dyn_into::<HtmlAnchorElement>() {
            let parts = Array::new();
            parts.push(&JsValue::from_str(contents));
            let options = BlobPropertyBag::new();
            options.set_type(mime);
            if let Ok(blob) = Blob::new_with_str_sequence_and_options(&parts, &options) {
                if let Ok(url) = Url::create_object_url_with_blob(&blob) {
                    link.set_href(&url);
                    link.set_download(filename);
                    let _ = body.append_child(&link);
                    link.click();
                    let _ = body.remove_child(&link);
                    let _ = Url::revoke_object_url(&url);
                }
            }
            }
        }
    }
}

pub(crate) struct ExportInfo {
    pub(crate) title: Option<String>,
    pub(crate) timestamp: String,
    pub(crate) canvas_size: (f64, f64),
    pub(crate) canvas_scale: f64,
    pub(crate) canvas_offset: (f64, f64),
}

pub(crate) fn make_export_info(document: &Document) -> ExportInfo {
    let title = document
        .get_element_by_id("project-name")
        .and_then(|el| el.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|el| el.value())
        .filter(|value| !value.trim().is_empty())
        .map(|value| sanitize_filename(&value));
    let timestamp = timestamp_string();
    ExportInfo {
        title,
        timestamp,
        canvas_size: (3000.0, 1500.0),
        canvas_scale: 1.0,
        canvas_offset: (0.0, 0.0),
    }
}

pub(crate) fn sanitize_filename(input: &str) -> String {
    input
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '-',
        })
        .collect()
}

pub(crate) fn timestamp_string() -> String {
    let now = Date::new_0();
    let year = now.get_full_year();
    let month = now.get_month() + 1;
    let day = now.get_date();
    let hours = now.get_hours();
    let minutes = now.get_minutes();
    let seconds = now.get_seconds();
    format!(
        "{year:04}{month:02}{day:02}-{hours:02}{minutes:02}{seconds:02}"
    )
}

pub(crate) fn icon_to_name(icon: ShapeType) -> &'static str {
    match icon {
        ShapeType::Disc => "disc",
        ShapeType::Square => "square",
        ShapeType::Oblong => "oblong",
        ShapeType::Poly => "poly",
        ShapeType::Text => "text",
        ShapeType::Svg => "svg",
        _ => "unknown",
    }
}

pub(crate) fn operation_to_name(operation: Operation) -> &'static str {
    match operation {
        Operation::Union => "union",
        Operation::Difference => "difference",
    }
}

pub(crate) fn text_font_to_name(font: &TextFont) -> &'static str {
    match font {
        TextFont::Stencilia => "stencilia",
        TextFont::Urbanist => "urbanist",
    }
}

pub(crate) fn json_escape(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

pub(crate) fn load_json_to_dataset(av: RefAV, json_data: String) {
    let Ok(value) = JSON::parse(&json_data) else {
        return;
    };
    let mut avb = av.borrow_mut();
    let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];

    if let Some(meta) = get_prop(&value, "meta") {
            if let Some(canvas_meta) = get_prop(&meta, "canvas") {
            if let Some(size) = get_vec2_array(&canvas_meta, "size") {
                canvas.set_area_size(Size::new(size.x, size.y));
            }
            if let Some(scale) = get_f64(&canvas_meta, "scale") {
                canvas.set_scale(scale);
            }
            if let Some(offset) = get_vec2_array(&canvas_meta, "offset") {
                canvas.set_offset(offset);
            }
        }
    }

    let Some(shapes_value) = get_prop(&value, "shapes") else {
        return;
    };
    let shapes_array = Array::from(&shapes_value);
    canvas.dataset.shapes.clear();
    canvas.dataset.shapes_selected.clear();
    canvas.dataset.shapes_highlighted.clear();
    canvas.dataset.vertices_selected.clear();
    canvas.dataset.vertices_highlighted.clear();
    canvas.dataset.shapes_selector = crate::shapes::ShapeSelector::new();

    for shape_value in shapes_array.iter() {
        let Some(type_name) = get_string(&shape_value, "type") else {
            continue;
        };
        let Some(icon) = name_to_icon(&type_name) else {
            continue;
        };
        let operation = get_string(&shape_value, "operation")
            .and_then(name_to_operation)
            .unwrap_or(Operation::Union);
        let rotation = get_f64(&shape_value, "rotation").unwrap_or(0.0);

        let vertices_value = get_prop(&shape_value, "vertices");
        let Some(vertices_value) = vertices_value else {
            continue;
        };
        let vertices_array = Array::from(&vertices_value);
        let mut vertices = Vec::new();
        let mut rounded = Vec::new();
        for vertex_value in vertices_array.iter() {
            let x = match get_f64(&vertex_value, "x") {
                Some(value) => value,
                None => continue,
            };
            let y = match get_f64(&vertex_value, "y") {
                Some(value) => value,
                None => continue,
            };
            let round_value = get_prop(&vertex_value, "rounded")
                .and_then(|val| val.as_f64())
                .map(|val| val as u32);
            vertices.push(Vec2::new(x, y));
            rounded.push(round_value);
        }

        let text_data = if icon == ShapeType::Text {
            get_prop(&shape_value, "text").and_then(|text_value| {
                let content = get_string(&text_value, "content")?;
                let scale = get_f64(&text_value, "scale")
                    .or_else(|| get_f64(&text_value, "size"))
                    .and_then(|value| if value > 0.0 { Some(value) } else { None });
                let auto_fit = get_prop(&text_value, "auto_fit")
                    .and_then(|val| val.as_bool())
                    .unwrap_or(false);
                let font = get_string(&text_value, "font")
                    .and_then(name_to_text_font)
                    .unwrap_or(TextFont::Stencilia);
                Some(TextData::new(content, font, scale, auto_fit))
            })
        } else {
            None
        };

        let svg_data = if icon == ShapeType::Svg {
            get_prop(&shape_value, "svg").map(|svg_value| {
                let fill_rule = get_string(&svg_value, "fill_rule")
                    .map(|rule| match rule.as_str() {
                        "evenodd" => SvgFillRule::EvenOdd,
                        _ => SvgFillRule::NonZero,
                    })
                    .unwrap_or(SvgFillRule::NonZero);
                let mut rings = Vec::new();
                if let Some(rings_value) = get_prop(&svg_value, "rings") {
                    let rings_array = Array::from(&rings_value);
                    for ring_value in rings_array.iter() {
                        let ring_points_array = Array::from(&ring_value);
                        let mut ring = Vec::new();
                        for point_value in ring_points_array.iter() {
                            let point_value = Array::from(&point_value);
                            if point_value.length() < 2 {
                                continue;
                            }
                            let x = point_value.get(0).as_f64().unwrap_or(0.0);
                            let y = point_value.get(1).as_f64().unwrap_or(0.0);
                            ring.push(Vec2::new(x, y));
                        }
                        if ring.len() >= 3 {
                            rings.push(ring);
                        }
                    }
                }
                SvgData::new(rings, fill_rule)
            })
        } else {
            None
        };

        let Some(mut elem) = ClosedShape::from_raw(
            icon,
            operation,
            vertices,
            &rounded,
            text_data,
            svg_data,
        ) else {
            continue;
        };
        if rotation.abs() > f64::EPSILON {
            elem.set_rotation(rotation);
        }

        canvas.dataset.shapes.insert(EUId::new(), elem);
    }

    canvas.dataset.refresh_svg_cache();
    canvas.dataset.mark_final_polygon_dirty();
    canvas.dataset.calc_final_polygon();
    drop(avb);
    update_status_bar(av);
}

pub(crate) fn parse_svg_length(value: &str) -> Option<(f64, String)> {
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

pub(crate) fn length_unit_to_mm(unit: &str) -> Option<f64> {
    match unit {
        "mm" => Some(1.0),
        "cm" => Some(10.0),
        "in" => Some(25.4),
        "px" => Some(1.0),
        _ => None,
    }
}

pub(crate) fn parse_view_box(value: &str) -> Option<(f64, f64, f64, f64)> {
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

pub(crate) fn parse_style_value(style: &str, key: &str) -> Option<String> {
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

pub(crate) fn fill_rule_from_attrs(
    attrs: &svg::node::Attributes,
    fallback: SvgFillRule,
) -> SvgFillRule {
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

pub(crate) fn bezpath_to_rings(path: &BezPath, tolerance: f64) -> Vec<Vec<Vec2>> {
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

pub(crate) fn normalize_ring(mut ring: Vec<Vec2>) -> Vec<Vec2> {
    if ring.len() > 1 {
        let first = ring[0];
        let last = ring[ring.len() - 1];
        if (first - last).hypot() < 1e-6 {
            ring.pop();
        }
    }
    ring
}

pub(crate) fn load_svg_to_dataset(av: RefAV, svg_data: String, combine_paths: bool) {
    let mut avb = av.borrow_mut();
    let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
    let mut paths = Vec::new();
    let mut view_box: Option<(f64, f64, f64, f64)> = None;
    let mut width: Option<(f64, String)> = None;
    let mut height: Option<(f64, String)> = None;

    let parser = SvgParser::new(svg_data.as_str());
    for event in parser {
        match event {
            SvgEvent::Tag(tag, _, attributes) => {
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
                }
                if tag == "path" {
                    if let Some(d) = attributes.get("d") {
                        if let Ok(path) = BezPath::from_svg(d.as_ref()) {
                            if !path.is_empty() {
                                let fill_rule =
                                    fill_rule_from_attrs(&attributes, SvgFillRule::NonZero);
                                paths.push((path, fill_rule));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if paths.is_empty() {
        return;
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

    let mut shape_rings: Vec<(Vec<Vec<Vec2>>, SvgFillRule)> = Vec::new();
    let mut all_rings = Vec::new();
    for (path, fill_rule) in paths {
        let rings = bezpath_to_rings(&path, 0.5);
        if rings.is_empty() {
            continue;
        }
        let mut normalized_rings: Vec<Vec<Vec2>> = rings
            .into_iter()
            .map(normalize_ring)
            .collect();
        if fill_rule == SvgFillRule::EvenOdd {
            for ring in normalized_rings.iter_mut().skip(1) {
                ring.reverse();
            }
        }

        if combine_paths {
            all_rings.extend(normalized_rings);
        } else {
            shape_rings.push((normalized_rings, fill_rule));
        }
    }

    if combine_paths && !all_rings.is_empty() {
        shape_rings.push((all_rings, SvgFillRule::NonZero));
    }

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (rings, _) in shape_rings.iter() {
        for ring in rings.iter() {
            for p in ring {
                min_x = min_x.min(p.x);
                min_y = min_y.min(p.y);
                max_x = max_x.max(p.x);
                max_y = max_y.max(p.y);
            }
        }
    }

    if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
        return;
    }

    let bbox_w = (max_x - min_x).max(1.0);
    let bbox_h = (max_y - min_y).max(1.0);
    let view_size = canvas.get_canvas_size();
    let view_world_w = (view_size.width / canvas.get_scale()).max(1.0);
    let view_world_h = (view_size.height / canvas.get_scale()).max(1.0);
    let scaled_w = bbox_w * scale;
    let scaled_h = bbox_h * scale;
    if scaled_w > view_world_w || scaled_h > view_world_h {
        let fit_scale = (view_world_w / scaled_w)
            .min(view_world_h / scaled_h)
            .min(1.0);
        scale *= fit_scale;
    }

    for (mut rings, fill_rule) in shape_rings.into_iter() {
        for ring in rings.iter_mut() {
            for p in ring.iter_mut() {
                p.x = (p.x - min_x) * scale;
                p.y = (p.y - min_y) * scale;
            }
        }
        if let Some(shape) = ClosedShape::new_svg(rings, fill_rule) {
            canvas.dataset.shapes.insert(EUId::new(), shape);
        }
    }

    canvas.dataset.refresh_svg_cache();
    canvas.dataset.mark_final_polygon_dirty();
    canvas.dataset.calc_final_polygon();
    drop(avb);
    update_status_bar(av);
}

fn geo_multipolygon_to_bez_paths(poly: &geo::MultiPolygon<f64>) -> Vec<BezPath> {
    let mut paths = Vec::new();
    for polygon in &poly.0 {
        if let Some(path) = polygon_to_bez_path(polygon) {
            paths.push(path);
        }
    }
    paths
}

fn polygon_to_bez_path(polygon: &geo::Polygon<f64>) -> Option<BezPath> {
    let mut path = BezPath::new();
    let exterior = polygon.exterior();
    if exterior.0.len() < 3 {
        return None;
    }
    let mut iter = exterior.0.iter();
    if let Some(start) = iter.next() {
        path.move_to((start.x, start.y));
        for coord in iter {
            path.line_to((coord.x, coord.y));
        }
        path.close_path();
    }
    for interior in polygon.interiors() {
        let mut iter = interior.0.iter();
        if let Some(start) = iter.next() {
            path.move_to((start.x, start.y));
            for coord in iter {
                path.line_to((coord.x, coord.y));
            }
            path.close_path();
        }
    }
    Some(path)
}
