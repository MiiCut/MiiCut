use crate::app::RefAV;
use crate::canvas::{CanvasKind, NotesData};
use crate::render::center_paths_canvas;
use crate::shape::{GeneralShape, Operation, ShapeType, SvgData, SvgFillRule};
use crate::shapes::DataSet;
use crate::status::update_status_bar;
use crate::type_scalar::Scalar;
use crate::type_vertex::Vertex;
use crate::types::{EUId, Properties, Property, PropertyValue};
use js_sys::{Array, Date, JSON};
use kurbo::{BezPath, PathEl, Shape, Size, Vec2};
use std::collections::HashMap;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
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

pub(crate) fn build_json_from_dataset(
    dataset: &DataSet,
    notes: &NotesData,
    meta: &ExportInfo,
) -> Option<String> {
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

    let mut export_shapes = Vec::new();
    export_shapes.extend(dataset.shapes.iter().map(|(eid, shape)| (*eid, shape)));
    export_shapes.extend(
        dataset
            .grouped_shapes
            .iter()
            .map(|(eid, shape)| (*eid, shape)),
    );

    let mut first = true;
    for (eid, elem) in export_shapes {
        if !first {
            out.push_str(",\n");
        }
        first = false;
        let shape_type = icon_to_name(elem.get_shape_type());
        let op_name = operation_to_name(elem.get_operation());
        out.push_str("    {\n");
        out.push_str(&format!("      \"id\": {},\n", eid));
        out.push_str(&format!("      \"type\": \"{shape_type}\",\n"));
        if let Some(name) = elem.get_name() {
            out.push_str(&format!("      \"name\": \"{}\",\n", json_escape(name)));
        }
        out.push_str(&format!("      \"operation\": \"{op_name}\",\n"));
        out.push_str(&format!("      \"order\": {},\n", elem.get_order()));
        out.push_str(&format!(
            "      \"rotation\": {:.6},\n",
            elem.get_rotation()
        ));
        if elem.is_group() {
            if let Some(children) = elem.get_group_children() {
                out.push_str("      \"children\": [");
                for (idx, child) in children.iter().enumerate() {
                    if idx > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&child.to_string());
                }
                out.push_str("],\n");
            }
        }
        if let Some(count) = elem.get_magnets_number() {
            out.push_str(&format!("      \"constr_vertices\": {count},\n"));
        }
        let vertices = elem.get_vertices();
        out.push_str("      \"vertices\": [\n");
        for (idx, (_, v)) in vertices.iter().enumerate() {
            let separator = if idx + 1 == vertices.len() {
                "\n"
            } else {
                ",\n"
            };
            let rounded = v
                .get_radius()
                .map(|val| val.to_string())
                .unwrap_or("null".to_string());
            out.push_str(&format!(
                "        {{\"x\": {:.6}, \"y\": {:.6}, \"rounded\": {rounded}}}{separator}",
                v.curr().x,
                v.curr().y
            ));
        }
        out.push_str("      ]");

        if let (Some(svg), Some(svg_key)) = (
            elem.get_svg(),
            match elem.get_shape_type() {
                ShapeType::Svg => Some("svg"),
                ShapeType::Voronoi => Some("voronoi"),
                _ => None,
            },
        ) {
            out.push_str(&format!(",\n      \"{svg_key}\": {{\n"));
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

    out.push_str("\n  ],\n");
    out.push_str("  \"notes\": [\n");
    for (idx, note) in notes.notes.iter().enumerate() {
        let separator = if idx + 1 == notes.notes.len() {
            "\n"
        } else {
            ",\n"
        };
        out.push_str(&format!(
            "    {{\"id\": {}, \"pos\": [{:.6}, {:.6}], \"size\": [{:.6}, {:.6}], \"text\": \"{}\"}}{separator}",
            note.id,
            note.pos.x,
            note.pos.y,
            note.size.x,
            note.size.y,
            json_escape(&note.text)
        ));
    }
    out.push_str("  ]\n}\n");
    Some(out)
}

pub(crate) fn get_prop(value: &JsValue, name: &str) -> Option<JsValue> {
    let prop = js_sys::Reflect::get(value, &JsValue::from_str(name)).ok()?;
    if prop.is_undefined() || prop.is_null() {
        return None;
    }
    Some(prop)
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
        "voronoi" => Some(ShapeType::Voronoi),
        "group" => Some(ShapeType::Group),
        "constr_line" => Some(ShapeType::ConstrLine),
        "constr_circle" => Some(ShapeType::ConstrCircle),
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
                        if let Some(window) = web_sys::window() {
                            let body = body.clone();
                            let link = link.clone();
                            let url = url.clone();
                            let callback = Closure::once(move || {
                                let _ = body.remove_child(&link);
                                let _ = Url::revoke_object_url(&url);
                            });
                            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                                callback.as_ref().unchecked_ref(),
                                0,
                            );
                            callback.forget();
                        } else {
                            let _ = body.remove_child(&link);
                            let _ = Url::revoke_object_url(&url);
                        }
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
    format!("{year:04}{month:02}{day:02}-{hours:02}{minutes:02}{seconds:02}")
}

pub(crate) fn icon_to_name(icon: ShapeType) -> &'static str {
    match icon {
        ShapeType::Disc => "disc",
        ShapeType::Square => "square",
        ShapeType::Oblong => "oblong",
        ShapeType::Poly => "poly",
        ShapeType::Text => "text",
        ShapeType::Svg => "svg",
        ShapeType::Voronoi => "voronoi",
        ShapeType::Group => "group",
        ShapeType::ConstrLine => "constr_line",
        ShapeType::ConstrCircle { .. } => "constr_circle",
        _ => "unknown",
    }
}

pub(crate) fn operation_to_name(operation: Operation) -> &'static str {
    match operation {
        Operation::Union => "union",
        Operation::Difference => "difference",
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
    canvas.dataset.grouped_shapes.clear();
    canvas.dataset.shapes_selected.clear();
    canvas.dataset.shapes_highlighted.clear();
    canvas.dataset.vertex_selected = None;
    canvas.dataset.vertex_highlighted = None;
    canvas.dataset.shapes_selector = crate::shapes::ShapeSelector::new();
    canvas.notes.clear();

    #[derive(Clone)]
    struct LoadedVertex {
        pos: Vec2,
        rounded: Option<u32>,
    }

    struct LoadedShape {
        saved_id: Option<usize>,
        icon: ShapeType,
        operation: Operation,
        name: Option<String>,
        order: i32,
        rotation: f64,
        children: Vec<usize>,
        constr_vertices: Option<usize>,
        vertices: Vec<LoadedVertex>,
        svg_data: Option<SvgData>,
        voronoi_data: Option<SvgData>,
    }

    fn f64_to_usize(value: f64) -> Option<usize> {
        if value.is_finite() && value >= 0.0 {
            Some(value.round() as usize)
        } else {
            None
        }
    }

    fn build_bb_properties(vertices: &[Vec2]) -> Option<Properties> {
        if vertices.len() < 4 {
            return None;
        }
        use PropertyValue::*;
        let mut properties = Properties::new();
        properties.add(
            Property::BottomLeft,
            BottomLeft {
                idx: 0,
                value: vertices[0],
                radius: None,
            },
        );
        properties.add(
            Property::TopLeft,
            TopLeft {
                idx: 1,
                value: vertices[1],
                radius: None,
            },
        );
        properties.add(
            Property::TopRight,
            TopRight {
                idx: 2,
                value: vertices[2],
                radius: None,
            },
        );
        properties.add(
            Property::BottomRight,
            BottomRight {
                idx: 3,
                value: vertices[3],
                radius: None,
            },
        );
        properties.add(
            Property::Scale,
            Scale {
                value: Scalar::new(1.0, 0.01, 1000.0, 0.1),
            },
        );
        Some(properties)
    }

    fn min_max_from_positions(positions: &[Vec2]) -> Option<(Vec2, Vec2)> {
        let mut min = Vec2::new(f64::INFINITY, f64::INFINITY);
        let mut max = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for pos in positions {
            min.x = min.x.min(pos.x);
            min.y = min.y.min(pos.y);
            max.x = max.x.max(pos.x);
            max.y = max.y.max(pos.y);
        }
        if !min.x.is_finite() || !min.y.is_finite() || !max.x.is_finite() || !max.y.is_finite() {
            return None;
        }
        Some((min, max))
    }

    let mut loaded_shapes = Vec::new();
    let mut fallback_order: i32 = 0;
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
        let name = get_string(&shape_value, "name")
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let mut order = get_f64(&shape_value, "order").and_then(|val| {
            if val.is_finite() {
                Some(val.round() as i32)
            } else {
                None
            }
        });
        if order.is_none() {
            order = Some(fallback_order);
        }
        if let Some(order_val) = order {
            fallback_order = fallback_order.max(order_val.saturating_add(1));
        }
        let rotation = get_f64(&shape_value, "rotation").unwrap_or(0.0);
        let saved_id = get_f64(&shape_value, "id").and_then(f64_to_usize);
        let mut children = Vec::new();
        if let Some(children_value) = get_prop(&shape_value, "children") {
            let children_array = Array::from(&children_value);
            for child_value in children_array.iter() {
                let Some(child_id) = child_value.as_f64().and_then(f64_to_usize) else {
                    continue;
                };
                children.push(child_id);
            }
        }
        let constr_vertices = get_f64(&shape_value, "constr_vertices").and_then(f64_to_usize);

        let vertices_value = get_prop(&shape_value, "vertices");
        let Some(vertices_value) = vertices_value else {
            continue;
        };
        let vertices_array = Array::from(&vertices_value);
        let mut vertices = Vec::new();
        for vertex_value in vertices_array.iter() {
            let x = match get_f64(&vertex_value, "x") {
                Some(value) => value,
                None => continue,
            };
            let y = match get_f64(&vertex_value, "y") {
                Some(value) => value,
                None => continue,
            };
            let rounded = get_prop(&vertex_value, "rounded")
                .and_then(|val| val.as_f64())
                .and_then(f64_to_usize)
                .map(|value| value as u32);
            vertices.push(LoadedVertex {
                pos: Vec2::new(x, y),
                rounded,
            });
        }

        let svg_data = if matches!(icon, ShapeType::Svg | ShapeType::Voronoi) {
            let svg_value = if icon == ShapeType::Voronoi {
                get_prop(&shape_value, "voronoi").or_else(|| get_prop(&shape_value, "svg"))
            } else {
                get_prop(&shape_value, "svg")
            };
            svg_value.map(|svg_value| {
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
        let (svg_data, voronoi_data) = if icon == ShapeType::Voronoi {
            (None, svg_data)
        } else {
            (svg_data, None)
        };
        loaded_shapes.push(LoadedShape {
            saved_id,
            icon,
            operation,
            name,
            order: order.unwrap_or(0),
            rotation,
            children,
            constr_vertices,
            vertices,
            svg_data,
            voronoi_data,
        });
    }

    let mut shapes_with_ids = Vec::new();
    let mut id_map: HashMap<usize, EUId> = HashMap::new();
    for shape in loaded_shapes {
        let new_eid = EUId::new();
        if let Some(saved_id) = shape.saved_id {
            id_map.insert(saved_id, new_eid);
        }
        shapes_with_ids.push((new_eid, shape));
    }

    for (new_eid, shape) in shapes_with_ids {
        let positions: Vec<Vec2> = shape.vertices.iter().map(|vertex| vertex.pos).collect();
        let elem = match shape.icon {
            ShapeType::Disc => {
                let Some(v0) = positions.get(0) else {
                    continue;
                };
                let Some(v1) = positions.get(1) else {
                    continue;
                };
                GeneralShape::new_shape_disc(*v0, *v1, shape.order)
            }
            ShapeType::Square => {
                let Some((min, max)) = min_max_from_positions(&positions) else {
                    continue;
                };
                GeneralShape::new_shape_rectangle(min, max, shape.order)
            }
            ShapeType::Oblong => {
                let Some(v0) = positions.get(0) else {
                    continue;
                };
                let Some(v1) = positions.get(1) else {
                    continue;
                };
                GeneralShape::new_shape_oblong(*v0, *v1, shape.order)
            }
            ShapeType::Text => {
                let Some((min, max)) = min_max_from_positions(&positions) else {
                    continue;
                };
                GeneralShape::new_shape_text(min, max, shape.order)
            }
            ShapeType::Poly => GeneralShape::new_shape_poly(positions.clone(), shape.order),
            ShapeType::ConstrLine => {
                let Some(v0) = positions.get(0) else {
                    continue;
                };
                let Some(v1) = positions.get(1) else {
                    continue;
                };
                GeneralShape::new_shape_constr_line(*v0, *v1, shape.order)
            }
            ShapeType::ConstrCircle => {
                let Some(v0) = positions.get(0) else {
                    continue;
                };
                let Some(v1) = positions.get(1) else {
                    continue;
                };
                let Some(mut elem) = GeneralShape::new_shape_constr_circle(*v0, *v1, shape.order)
                else {
                    continue;
                };
                let count = shape
                    .constr_vertices
                    .or_else(|| positions.len().checked_sub(2));
                if let Some(count) = count {
                    elem.set_magnets_number(count);
                }
                Some(elem)
            }
            ShapeType::Svg => {
                let Some(properties) = build_bb_properties(&positions) else {
                    continue;
                };
                let vertices: Vec<Vertex> = positions.iter().copied().map(Vertex::new).collect();
                GeneralShape::new(
                    ShapeType::Svg,
                    vertices,
                    properties,
                    shape.order,
                    None,
                    shape.svg_data.clone(),
                    None,
                    None,
                    Operation::Union,
                    None,
                )
            }
            ShapeType::Voronoi => {
                let Some(mut properties) = build_bb_properties(&positions) else {
                    continue;
                };
                properties.add(
                    Property::Seeds,
                    PropertyValue::Seeds {
                        value: Scalar::new(40_u64, 10_u64, 100_u64, 1_u64),
                    },
                );
                let vertices: Vec<Vertex> = positions.iter().copied().map(Vertex::new).collect();
                GeneralShape::new(
                    ShapeType::Voronoi,
                    vertices,
                    properties,
                    shape.order,
                    None,
                    None,
                    shape.voronoi_data.clone(),
                    None,
                    Operation::Union,
                    None,
                )
            }
            ShapeType::Group => {
                let Some((min, max)) = min_max_from_positions(&positions) else {
                    continue;
                };
                let children: Vec<EUId> = shape
                    .children
                    .iter()
                    .filter_map(|child_id| id_map.get(child_id).copied())
                    .collect();
                GeneralShape::new_shape_group(min, max, shape.order, children)
            }
            _ => None,
        };

        let Some(mut elem) = elem else {
            continue;
        };

        let max_vertices = elem.get_vertices().len().min(shape.vertices.len());
        for idx in 0..max_vertices {
            let loaded_vertex = &shape.vertices[idx];
            let vertex = elem.get_vertices_mut().val_mut(idx as i64);
            vertex.set_curr(loaded_vertex.pos);
            vertex.set_saved(loaded_vertex.pos);
            vertex.set_radius_value(loaded_vertex.rounded);
        }
        elem.update_properties();
        elem.set_bezpath();

        match shape.operation {
            Operation::Union => elem.op_union(),
            Operation::Difference => elem.op_difference(),
        }
        elem.set_name(shape.name.clone());
        elem.set_order(shape.order);
        if shape.rotation.abs() > f64::EPSILON {
            elem.set_rotation(shape.rotation);
        }
        canvas.dataset.shapes.insert(new_eid, elem);
    }

    let group_ids: Vec<EUId> = canvas
        .dataset
        .shapes
        .iter()
        .filter_map(|(eid, shape)| shape.is_group().then_some(*eid))
        .collect();
    for group_id in group_ids {
        let Some(children) = canvas
            .dataset
            .shapes
            .get(&group_id)
            .and_then(|shape| shape.get_group_children())
        else {
            continue;
        };
        let children = children.to_vec();
        for child_id in children {
            if let Some(child) = canvas.dataset.shapes.remove(&child_id) {
                canvas.dataset.grouped_shapes.insert(child_id, child);
            }
        }
    }
    canvas.dataset.sync_next_order();

    if let Some(notes_value) = get_prop(&value, "notes") {
        let notes_array = Array::from(&notes_value);
        for note_value in notes_array.iter() {
            let Some(id) = get_f64(&note_value, "id").and_then(f64_to_usize) else {
                continue;
            };
            let Some(pos) = get_vec2_array(&note_value, "pos") else {
                continue;
            };
            let Some(size) = get_vec2_array(&note_value, "size") else {
                continue;
            };
            let text = get_string(&note_value, "text").unwrap_or_default();
            canvas.notes.add_with_id(id, pos, size, text);
        }
    }

    canvas.dataset.refresh_svg_cache();
    canvas.dataset.mark_final_polygon_dirty();
    canvas.dataset.calc_final_polygon();
    {
        let mut paths = canvas.dataset.get_final_paths().clone();
        if paths.is_empty() {
            paths = canvas
                .dataset
                .shapes
                .values()
                .map(|shape| shape.get_bezpath().clone())
                .collect();
        }
        center_paths_canvas(canvas, &paths);
    }
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
