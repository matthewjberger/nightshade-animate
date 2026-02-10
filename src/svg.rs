use crate::paint::{GradientStop, Paint};
use crate::project::{AnimObject, FontFamily, Keyframe, PathPoint, Project, Shape, TweenType};
use crate::tween;

pub fn export_svg(project: &Project, frame: u32, path: &std::path::Path) {
    let svg_content = render_frame_to_svg(project, frame);
    let _ = std::fs::write(path, svg_content);
}

fn render_frame_to_svg(project: &Project, frame: u32) -> String {
    let mut defs = String::new();
    let mut body = String::new();
    let mut gradient_counter = 0_usize;

    for layer_index in (0..project.layers.len()).rev() {
        let layer = &project.layers[layer_index];
        if !layer.visible {
            continue;
        }

        if let Some(objects) = tween::resolve_frame(layer, frame) {
            let opacity = layer.opacity;
            for object in &objects {
                gradient_counter += 1;
                let element = object_to_svg_element(object, opacity, gradient_counter, &mut defs);
                body.push_str(&element);
            }
        }
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">
<defs>
{}
</defs>
{}
</svg>"#,
        project.canvas_width,
        project.canvas_height,
        project.canvas_width,
        project.canvas_height,
        defs,
        body
    )
}

fn object_to_svg_element(
    object: &AnimObject,
    layer_opacity: f32,
    gradient_id: usize,
    defs: &mut String,
) -> String {
    let fill_attr = paint_to_svg_attr(&object.fill, &format!("fill_{}", gradient_id), defs);
    let stroke_attr = paint_to_svg_attr(&object.stroke, &format!("stroke_{}", gradient_id), defs);
    let stroke_width = object.stroke_width;
    let opacity = if (layer_opacity - 1.0).abs() > 0.001 {
        format!(r#" opacity="{}""#, layer_opacity)
    } else {
        String::new()
    };

    let transform = build_transform(object);

    match &object.shape {
        Shape::Rectangle {
            width,
            height,
            corner_radius,
        } => {
            let half_w = width / 2.0;
            let half_h = height / 2.0;
            let rx = if *corner_radius > 0.0 {
                format!(r#" rx="{}" ry="{}""#, corner_radius, corner_radius)
            } else {
                String::new()
            };
            format!(
                r#"<rect x="{}" y="{}" width="{}" height="{}"{} fill="{}" stroke="{}" stroke-width="{}"{}{}/>"#,
                -half_w,
                -half_h,
                width,
                height,
                rx,
                fill_attr,
                stroke_attr,
                stroke_width,
                opacity,
                transform,
            ) + "\n"
        }
        Shape::Ellipse { radius_x, radius_y } => {
            format!(
                r#"<ellipse cx="0" cy="0" rx="{}" ry="{}" fill="{}" stroke="{}" stroke-width="{}"{}{}/>"#,
                radius_x, radius_y, fill_attr, stroke_attr, stroke_width, opacity, transform,
            ) + "\n"
        }
        Shape::Line { end_x, end_y } => {
            format!(
                r#"<line x1="0" y1="0" x2="{}" y2="{}" stroke="{}" stroke-width="{}"{}{}/>"#,
                end_x, end_y, stroke_attr, stroke_width, opacity, transform,
            ) + "\n"
        }
        Shape::Path { points, closed } => {
            let d = path_points_to_svg_d(points, *closed);
            format!(
                r#"<path d="{}" fill="{}" stroke="{}" stroke-width="{}"{}{}/>"#,
                d,
                if *closed {
                    fill_attr
                } else {
                    "none".to_string()
                },
                stroke_attr,
                stroke_width,
                opacity,
                transform,
            ) + "\n"
        }
        Shape::Text {
            content,
            font_size,
            font_family,
        } => {
            let family = match font_family {
                FontFamily::SansSerif => "sans-serif",
                FontFamily::Serif => "serif",
                FontFamily::Monospace => "monospace",
            };
            format!(
                r#"<text x="0" y="0" font-size="{}" font-family="{}" fill="{}" stroke="{}" stroke-width="{}"{}{} dominant-baseline="hanging">{}</text>"#,
                font_size,
                family,
                fill_attr,
                stroke_attr,
                stroke_width,
                opacity,
                transform,
                escape_xml(content),
            ) + "\n"
        }
        Shape::RasterImage { .. } | Shape::SymbolInstance { .. } => String::new(),
    }
}

fn build_transform(object: &AnimObject) -> String {
    let mut parts = Vec::new();

    if object.position[0].abs() > 0.001 || object.position[1].abs() > 0.001 {
        parts.push(format!(
            "translate({}, {})",
            object.position[0], object.position[1]
        ));
    }
    if object.rotation.abs() > 0.001 {
        parts.push(format!("rotate({})", object.rotation.to_degrees()));
    }
    if (object.scale[0] - 1.0).abs() > 0.001 || (object.scale[1] - 1.0).abs() > 0.001 {
        parts.push(format!("scale({}, {})", object.scale[0], object.scale[1]));
    }

    if parts.is_empty() {
        String::new()
    } else {
        format!(r#" transform="{}""#, parts.join(" "))
    }
}

fn paint_to_svg_attr(paint: &Paint, grad_id: &str, defs: &mut String) -> String {
    match paint {
        Paint::Solid(color) => rgba_to_svg_color(color),
        Paint::LinearGradient { start, end, stops } => {
            defs.push_str(&format!(
                r#"<linearGradient id="{}" x1="{}%" y1="{}%" x2="{}%" y2="{}%">"#,
                grad_id,
                start[0] * 100.0,
                start[1] * 100.0,
                end[0] * 100.0,
                end[1] * 100.0,
            ));
            defs.push('\n');
            for stop in stops {
                defs.push_str(&format!(
                    r#"<stop offset="{}%" stop-color="{}" stop-opacity="{}"/>"#,
                    stop.offset * 100.0,
                    rgba_to_svg_color_no_alpha(&stop.color),
                    stop.color[3],
                ));
                defs.push('\n');
            }
            defs.push_str("</linearGradient>\n");
            format!("url(#{})", grad_id)
        }
        Paint::RadialGradient {
            center,
            radius,
            stops,
        } => {
            defs.push_str(&format!(
                r#"<radialGradient id="{}" cx="{}%" cy="{}%" r="{}%">"#,
                grad_id,
                center[0] * 100.0,
                center[1] * 100.0,
                radius * 100.0,
            ));
            defs.push('\n');
            for stop in stops {
                defs.push_str(&format!(
                    r#"<stop offset="{}%" stop-color="{}" stop-opacity="{}"/>"#,
                    stop.offset * 100.0,
                    rgba_to_svg_color_no_alpha(&stop.color),
                    stop.color[3],
                ));
                defs.push('\n');
            }
            defs.push_str("</radialGradient>\n");
            format!("url(#{})", grad_id)
        }
    }
}

fn rgba_to_svg_color(color: &[f32; 4]) -> String {
    if color[3] < 0.001 {
        return "none".to_string();
    }
    let red = (color[0] * 255.0) as u8;
    let green = (color[1] * 255.0) as u8;
    let blue = (color[2] * 255.0) as u8;
    if (color[3] - 1.0).abs() < 0.001 {
        format!("rgb({},{},{})", red, green, blue)
    } else {
        format!("rgba({},{},{},{})", red, green, blue, color[3])
    }
}

fn rgba_to_svg_color_no_alpha(color: &[f32; 4]) -> String {
    let red = (color[0] * 255.0) as u8;
    let green = (color[1] * 255.0) as u8;
    let blue = (color[2] * 255.0) as u8;
    format!("rgb({},{},{})", red, green, blue)
}

fn path_points_to_svg_d(points: &[PathPoint], closed: bool) -> String {
    if points.is_empty() {
        return String::new();
    }

    let mut d = format!("M{},{}", points[0].position[0], points[0].position[1]);

    for index in 1..points.len() {
        let prev = &points[index - 1];
        let curr = &points[index];

        if prev.control_out.is_some() || curr.control_in.is_some() {
            let control_out = prev.control_out.unwrap_or(prev.position);
            let control_in = curr.control_in.unwrap_or(curr.position);
            d.push_str(&format!(
                " C{},{} {},{} {},{}",
                control_out[0],
                control_out[1],
                control_in[0],
                control_in[1],
                curr.position[0],
                curr.position[1],
            ));
        } else {
            d.push_str(&format!(" L{},{}", curr.position[0], curr.position[1]));
        }
    }

    if closed {
        d.push_str(" Z");
    }

    d
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn import_svg(project: &mut Project, path: &std::path::Path, layer_index: usize, frame: u32) {
    let Ok(svg_data) = std::fs::read(path) else {
        return;
    };

    let options = usvg::Options::default();
    let Ok(tree) = usvg::Tree::from_data(&svg_data, &options) else {
        return;
    };

    let mut objects = Vec::new();

    import_group(tree.root(), &mut objects, usvg::Transform::identity());

    if layer_index < project.layers.len() {
        let layer = &mut project.layers[layer_index];
        if let Some(keyframe) = layer.keyframes.get_mut(&frame) {
            keyframe.objects.extend(objects);
        } else {
            layer.keyframes.insert(
                frame,
                Keyframe {
                    objects,
                    tween: TweenType::None,
                    label: String::new(),
                    comment: String::new(),
                    shape_tween: false,
                },
            );
        }
    }
}

fn import_group(
    group: &usvg::Group,
    objects: &mut Vec<AnimObject>,
    parent_transform: usvg::Transform,
) {
    for child in group.children() {
        match child {
            usvg::Node::Group(subgroup) => {
                let combined = combine_transforms(parent_transform, subgroup.transform());
                import_group(subgroup, objects, combined);
            }
            usvg::Node::Path(usvg_path) => {
                let combined = combine_transforms(parent_transform, usvg_path.abs_transform());
                if let Some(object) = convert_usvg_path(usvg_path, combined) {
                    objects.push(object);
                }
            }
            usvg::Node::Text(text_node) => {
                let combined = combine_transforms(parent_transform, text_node.abs_transform());
                if let Some(first_chunk) = text_node.chunks().first() {
                    let text_content = first_chunk.text().to_string();
                    let mut font_size = 16.0_f32;
                    let mut fill_color = Paint::Solid([0.0, 0.0, 0.0, 1.0]);
                    if let Some(first_span) = first_chunk.spans().first() {
                        font_size = first_span.font_size().get();
                        fill_color = extract_usvg_fill(first_span.fill());
                    }
                    let obj = AnimObject::new(
                        Shape::Text {
                            content: text_content,
                            font_size,
                            font_family: FontFamily::SansSerif,
                        },
                        [combined.tx, combined.ty],
                        fill_color,
                        Paint::Solid([0.0, 0.0, 0.0, 0.0]),
                        0.0,
                    );
                    objects.push(obj);
                }
            }
            usvg::Node::Image(_) => {}
        }
    }
}

fn combine_transforms(parent: usvg::Transform, child: usvg::Transform) -> usvg::Transform {
    parent.pre_concat(child)
}

fn convert_usvg_path(usvg_path: &usvg::Path, transform: usvg::Transform) -> Option<AnimObject> {
    let mut path_points = Vec::new();
    let mut closed = false;
    let mut current_pos = [0.0_f32; 2];

    for segment in usvg_path.data().segments() {
        match segment {
            usvg::tiny_skia_path::PathSegment::MoveTo(point) => {
                current_pos = [point.x, point.y];
                path_points.push(PathPoint {
                    position: current_pos,
                    control_in: None,
                    control_out: None,
                    pressure: 1.0,
                });
            }
            usvg::tiny_skia_path::PathSegment::LineTo(point) => {
                current_pos = [point.x, point.y];
                path_points.push(PathPoint {
                    position: current_pos,
                    control_in: None,
                    control_out: None,
                    pressure: 1.0,
                });
            }
            usvg::tiny_skia_path::PathSegment::CubicTo(point1, point2, point3) => {
                if let Some(last) = path_points.last_mut() {
                    last.control_out = Some([point1.x, point1.y]);
                }
                current_pos = [point3.x, point3.y];
                path_points.push(PathPoint {
                    position: current_pos,
                    control_in: Some([point2.x, point2.y]),
                    control_out: None,
                    pressure: 1.0,
                });
            }
            usvg::tiny_skia_path::PathSegment::QuadTo(control, end_point) => {
                let prev = current_pos;
                let ctrl1 = [
                    prev[0] + 2.0 / 3.0 * (control.x - prev[0]),
                    prev[1] + 2.0 / 3.0 * (control.y - prev[1]),
                ];
                let ctrl2 = [
                    end_point.x + 2.0 / 3.0 * (control.x - end_point.x),
                    end_point.y + 2.0 / 3.0 * (control.y - end_point.y),
                ];
                if let Some(last) = path_points.last_mut() {
                    last.control_out = Some(ctrl1);
                }
                current_pos = [end_point.x, end_point.y];
                path_points.push(PathPoint {
                    position: current_pos,
                    control_in: Some(ctrl2),
                    control_out: None,
                    pressure: 1.0,
                });
            }
            usvg::tiny_skia_path::PathSegment::Close => {
                closed = true;
            }
        }
    }

    if path_points.is_empty() {
        return None;
    }

    let fill_color = extract_usvg_fill(usvg_path.fill());
    let (stroke_color, stroke_width) = extract_usvg_stroke(usvg_path.stroke());

    let position = [transform.tx, transform.ty];
    let scale_x = (transform.sx * transform.sx + transform.kx * transform.kx).sqrt();
    let scale_y = (transform.sy * transform.sy + transform.ky * transform.ky).sqrt();
    let rotation = transform.kx.atan2(transform.sx);

    Some(AnimObject {
        id: uuid::Uuid::new_v4(),
        shape: Shape::Path {
            points: path_points,
            closed,
        },
        position,
        rotation,
        scale: [scale_x, scale_y],
        fill: fill_color,
        stroke: stroke_color,
        stroke_width,
    })
}

fn extract_usvg_fill(fill: Option<&usvg::Fill>) -> Paint {
    match fill {
        Some(fill) => match fill.paint() {
            usvg::Paint::Color(color) => Paint::Solid([
                color.red as f32 / 255.0,
                color.green as f32 / 255.0,
                color.blue as f32 / 255.0,
                fill.opacity().get(),
            ]),
            usvg::Paint::LinearGradient(gradient) => {
                let stops: Vec<GradientStop> = gradient
                    .stops()
                    .iter()
                    .map(|stop| GradientStop {
                        offset: stop.offset().get(),
                        color: [
                            stop.color().red as f32 / 255.0,
                            stop.color().green as f32 / 255.0,
                            stop.color().blue as f32 / 255.0,
                            stop.opacity().get(),
                        ],
                    })
                    .collect();
                Paint::LinearGradient {
                    start: [gradient.x1(), gradient.y1()],
                    end: [gradient.x2(), gradient.y2()],
                    stops,
                }
            }
            usvg::Paint::RadialGradient(gradient) => {
                let stops: Vec<GradientStop> = gradient
                    .stops()
                    .iter()
                    .map(|stop| GradientStop {
                        offset: stop.offset().get(),
                        color: [
                            stop.color().red as f32 / 255.0,
                            stop.color().green as f32 / 255.0,
                            stop.color().blue as f32 / 255.0,
                            stop.opacity().get(),
                        ],
                    })
                    .collect();
                Paint::RadialGradient {
                    center: [gradient.cx(), gradient.cy()],
                    radius: gradient.r().get(),
                    stops,
                }
            }
            usvg::Paint::Pattern(_) => Paint::Solid([0.5, 0.5, 0.5, 1.0]),
        },
        None => Paint::Solid([0.0, 0.0, 0.0, 0.0]),
    }
}

fn extract_usvg_stroke(stroke: Option<&usvg::Stroke>) -> (Paint, f32) {
    match stroke {
        Some(stroke) => {
            let paint = match stroke.paint() {
                usvg::Paint::Color(color) => Paint::Solid([
                    color.red as f32 / 255.0,
                    color.green as f32 / 255.0,
                    color.blue as f32 / 255.0,
                    stroke.opacity().get(),
                ]),
                _ => Paint::Solid([0.0, 0.0, 0.0, 1.0]),
            };
            (paint, stroke.width().get())
        }
        None => (Paint::Solid([0.0, 0.0, 0.0, 0.0]), 0.0),
    }
}
