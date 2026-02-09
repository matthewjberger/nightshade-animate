use nightshade::prelude::*;

use crate::project::{AnimObject, PathPoint, Project, Shape};
use crate::tween;

pub fn export_gif(project: &Project, path: &std::path::Path) {
    let width = project.canvas_width as u16;
    let height = project.canvas_height as u16;
    let delay = ((100 + project.frame_rate / 2) / project.frame_rate) as u16;

    let Ok(file) = std::fs::File::create(path) else {
        return;
    };

    let Ok(mut encoder) = gif::Encoder::new(file, width, height, &[]) else {
        return;
    };

    let _ = encoder.set_repeat(gif::Repeat::Infinite);

    for frame_index in 0..project.total_frames {
        let rgba_image = rasterize_frame(project, frame_index);
        let mut pixels = rgba_image.into_raw();

        let mut gif_frame = gif::Frame::from_rgba_speed(width, height, &mut pixels, 10);
        gif_frame.delay = delay;

        let _ = encoder.write_frame(&gif_frame);
    }
}

pub fn export_png_sequence(project: &Project, folder: &std::path::Path) {
    for frame in 0..project.total_frames {
        let image = rasterize_frame(project, frame);
        let filename = format!("frame_{:04}.png", frame + 1);
        let path = folder.join(filename);
        let _ = image.save(path);
    }
}

pub fn export_sprite_sheet(project: &Project, path: &std::path::Path) {
    let columns = (project.total_frames as f64).sqrt().ceil() as u32;
    let rows = project.total_frames.div_ceil(columns);

    let sheet_width = columns * project.canvas_width;
    let sheet_height = rows * project.canvas_height;

    let mut sheet: image::RgbaImage = image::ImageBuffer::new(sheet_width, sheet_height);

    for frame in 0..project.total_frames {
        let frame_image = rasterize_frame(project, frame);
        let column = frame % columns;
        let row = frame / columns;
        let offset_x = column * project.canvas_width;
        let offset_y = row * project.canvas_height;

        for y in 0..project.canvas_height {
            for x in 0..project.canvas_width {
                let pixel = frame_image.get_pixel(x, y);
                sheet.put_pixel(offset_x + x, offset_y + y, *pixel);
            }
        }
    }

    let _ = sheet.save(path);
}

fn rasterize_frame(project: &Project, frame: u32) -> image::RgbaImage {
    let width = project.canvas_width;
    let height = project.canvas_height;

    let bg = project.background_color;
    let bg_pixel = image::Rgba([
        (bg[0] * 255.0) as u8,
        (bg[1] * 255.0) as u8,
        (bg[2] * 255.0) as u8,
        (bg[3] * 255.0) as u8,
    ]);

    let mut image_buffer: image::RgbaImage =
        image::ImageBuffer::from_pixel(width, height, bg_pixel);

    for layer_index in (0..project.layers.len()).rev() {
        let layer = &project.layers[layer_index];
        if !layer.visible {
            continue;
        }

        if let Some(objects) = tween::resolve_frame(layer, frame) {
            for object in &objects {
                rasterize_object(&mut image_buffer, object, layer.opacity);
            }
        }
    }

    image_buffer
}

fn rasterize_object(image_buffer: &mut image::RgbaImage, object: &AnimObject, layer_opacity: f32) {
    let (width, height) = image_buffer.dimensions();

    match &object.shape {
        Shape::Rectangle {
            width: rect_width,
            height: rect_height,
            corner_radius,
        } => {
            let half_w = rect_width * object.scale[0] / 2.0;
            let half_h = rect_height * object.scale[1] / 2.0;
            let radius = corner_radius.min(half_w).min(half_h);
            let diagonal = (half_w * half_w + half_h * half_h).sqrt();
            let bound_w = if object.rotation.abs() > 0.001 {
                diagonal
            } else {
                half_w
            };
            let bound_h = if object.rotation.abs() > 0.001 {
                diagonal
            } else {
                half_h
            };
            let min_x = ((object.position[0] - bound_w).floor() as i32).max(0) as u32;
            let min_y = ((object.position[1] - bound_h).floor() as i32).max(0) as u32;
            let max_x = ((object.position[0] + bound_w).ceil() as u32).min(width - 1);
            let max_y = ((object.position[1] + bound_h).ceil() as u32).min(height - 1);

            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let local_x = x as f32 - object.position[0];
                    let local_y = y as f32 - object.position[1];
                    let cos_r = (-object.rotation).cos();
                    let sin_r = (-object.rotation).sin();
                    let unrotated_x = local_x * cos_r - local_y * sin_r;
                    let unrotated_y = local_x * sin_r + local_y * cos_r;

                    let dist = rounded_rect_dist(unrotated_x, unrotated_y, half_w, half_h, radius);
                    if dist <= 0.0 {
                        let inner_dist = if object.stroke_width > 0.0 {
                            rounded_rect_dist(
                                unrotated_x,
                                unrotated_y,
                                (half_w - object.stroke_width).max(0.0),
                                (half_h - object.stroke_width).max(0.0),
                                (radius - object.stroke_width).max(0.0),
                            )
                        } else {
                            -1.0
                        };

                        let color = if inner_dist > 0.0 {
                            object.stroke
                        } else {
                            object.fill
                        };

                        blend_pixel(image_buffer, x, y, color, layer_opacity);
                    }
                }
            }
        }
        Shape::Ellipse { radius_x, radius_y } => {
            let scaled_rx = radius_x * object.scale[0];
            let scaled_ry = radius_y * object.scale[1];
            let max_radius = scaled_rx.max(scaled_ry);
            let bound = if object.rotation.abs() > 0.001 {
                max_radius
            } else {
                0.0
            };
            let bound_x = if object.rotation.abs() > 0.001 {
                bound
            } else {
                scaled_rx
            };
            let bound_y = if object.rotation.abs() > 0.001 {
                bound
            } else {
                scaled_ry
            };
            let min_x =
                ((object.position[0] - bound_x - object.stroke_width).floor() as i32).max(0) as u32;
            let min_y =
                ((object.position[1] - bound_y - object.stroke_width).floor() as i32).max(0) as u32;
            let max_x =
                ((object.position[0] + bound_x + object.stroke_width).ceil() as u32).min(width - 1);
            let max_y = ((object.position[1] + bound_y + object.stroke_width).ceil() as u32)
                .min(height - 1);

            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let local_x = x as f32 - object.position[0];
                    let local_y = y as f32 - object.position[1];
                    let cos_r = (-object.rotation).cos();
                    let sin_r = (-object.rotation).sin();
                    let unrotated_x = local_x * cos_r - local_y * sin_r;
                    let unrotated_y = local_x * sin_r + local_y * cos_r;

                    if scaled_rx > 0.001 && scaled_ry > 0.001 {
                        let dist =
                            (unrotated_x / scaled_rx).powi(2) + (unrotated_y / scaled_ry).powi(2);
                        if dist <= 1.0 {
                            let inner_rx = (scaled_rx - object.stroke_width).max(0.0);
                            let inner_ry = (scaled_ry - object.stroke_width).max(0.0);
                            let inner_dist = if inner_rx > 0.0 && inner_ry > 0.0 {
                                (unrotated_x / inner_rx).powi(2) + (unrotated_y / inner_ry).powi(2)
                            } else {
                                2.0
                            };

                            let color = if inner_dist > 1.0 && object.stroke_width > 0.0 {
                                object.stroke
                            } else {
                                object.fill
                            };

                            blend_pixel(image_buffer, x, y, color, layer_opacity);
                        }
                    }
                }
            }
        }
        Shape::Line { end_x, end_y } => {
            let start_x = object.position[0];
            let start_y = object.position[1];
            let line_end_x = start_x + end_x * object.scale[0];
            let line_end_y = start_y + end_y * object.scale[1];

            let thickness = object.stroke_width.max(1.0);
            let min_px = ((start_x.min(line_end_x) - thickness).floor() as i32).max(0) as u32;
            let min_py = ((start_y.min(line_end_y) - thickness).floor() as i32).max(0) as u32;
            let max_px = ((start_x.max(line_end_x) + thickness).ceil() as u32).min(width - 1);
            let max_py = ((start_y.max(line_end_y) + thickness).ceil() as u32).min(height - 1);

            let dx = line_end_x - start_x;
            let dy = line_end_y - start_y;
            let line_len_sq = dx * dx + dy * dy;

            for y in min_py..=max_py {
                for x in min_px..=max_px {
                    let px = x as f32 - start_x;
                    let py = y as f32 - start_y;

                    if line_len_sq < 0.001 {
                        continue;
                    }

                    let t = ((px * dx + py * dy) / line_len_sq).clamp(0.0, 1.0);
                    let closest_x = t * dx;
                    let closest_y = t * dy;
                    let dist = ((px - closest_x).powi(2) + (py - closest_y).powi(2)).sqrt();

                    if dist <= thickness / 2.0 {
                        blend_pixel(image_buffer, x, y, object.stroke, layer_opacity);
                    }
                }
            }
        }
        Shape::Path { points, closed } => {
            if points.len() < 2 {
                return;
            }

            let canvas_points = build_path_points(object, points, *closed);

            if *closed && canvas_points.len() >= 3 {
                rasterize_closed_path(
                    image_buffer,
                    &canvas_points,
                    object.fill,
                    object.stroke,
                    object.stroke_width,
                    layer_opacity,
                );
            } else {
                rasterize_open_path(
                    image_buffer,
                    &canvas_points,
                    object.stroke,
                    object.stroke_width,
                    layer_opacity,
                );
            }
        }
    }
}

fn blend_pixel(
    image_buffer: &mut image::RgbaImage,
    x: u32,
    y: u32,
    color: [f32; 4],
    layer_opacity: f32,
) {
    let alpha = color[3] * layer_opacity;
    if alpha < 0.001 {
        return;
    }

    let existing = image_buffer.get_pixel(x, y);
    let src_r = (color[0] * 255.0) as u8;
    let src_g = (color[1] * 255.0) as u8;
    let src_b = (color[2] * 255.0) as u8;
    let src_a = (alpha * 255.0) as u8;

    let alpha_f = alpha;
    let inv_alpha = 1.0 - alpha_f;

    let result = image::Rgba([
        (src_r as f32 * alpha_f + existing[0] as f32 * inv_alpha) as u8,
        (src_g as f32 * alpha_f + existing[1] as f32 * inv_alpha) as u8,
        (src_b as f32 * alpha_f + existing[2] as f32 * inv_alpha) as u8,
        (src_a as f32 + existing[3] as f32 * inv_alpha).min(255.0) as u8,
    ]);

    image_buffer.put_pixel(x, y, result);
}

fn rounded_rect_dist(px: f32, py: f32, half_w: f32, half_h: f32, radius: f32) -> f32 {
    let r = radius.min(half_w).min(half_h);
    let dx = px.abs() - half_w + r;
    let dy = py.abs() - half_h + r;
    (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt() + dx.max(dy).min(0.0) - r
}

fn cubic_bezier(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], p3: [f32; 2], t: f32) -> [f32; 2] {
    let omt = 1.0 - t;
    let omt2 = omt * omt;
    let omt3 = omt2 * omt;
    let t2 = t * t;
    let t3 = t2 * t;
    [
        omt3 * p0[0] + 3.0 * omt2 * t * p1[0] + 3.0 * omt * t2 * p2[0] + t3 * p3[0],
        omt3 * p0[1] + 3.0 * omt2 * t * p1[1] + 3.0 * omt * t2 * p2[1] + t3 * p3[1],
    ]
}

fn build_path_points(object: &AnimObject, points: &[PathPoint], closed: bool) -> Vec<[f32; 2]> {
    let mut result = Vec::new();

    for index in 0..points.len() {
        let point = &points[index];
        let canvas_pt = [
            object.position[0] + point.position[0] * object.scale[0],
            object.position[1] + point.position[1] * object.scale[1],
        ];

        if index > 0 {
            let prev = &points[index - 1];
            if prev.control_out.is_some() || point.control_in.is_some() {
                let ctrl_out = prev.control_out.unwrap_or(prev.position);
                let ctrl_in = point.control_in.unwrap_or(point.position);
                for step in 1..=16 {
                    let t = step as f32 / 16.0;
                    let b = cubic_bezier(prev.position, ctrl_out, ctrl_in, point.position, t);
                    result.push([
                        object.position[0] + b[0] * object.scale[0],
                        object.position[1] + b[1] * object.scale[1],
                    ]);
                }
                continue;
            }
        }
        result.push(canvas_pt);
    }

    if closed && points.len() > 2 {
        let last = points.last().unwrap();
        let first = &points[0];
        if last.control_out.is_some() || first.control_in.is_some() {
            let ctrl_out = last.control_out.unwrap_or(last.position);
            let ctrl_in = first.control_in.unwrap_or(first.position);
            for step in 1..=16 {
                let t = step as f32 / 16.0;
                let b = cubic_bezier(last.position, ctrl_out, ctrl_in, first.position, t);
                result.push([
                    object.position[0] + b[0] * object.scale[0],
                    object.position[1] + b[1] * object.scale[1],
                ]);
            }
        }
    }

    result
}

fn point_in_polygon(px: f32, py: f32, polygon: &[[f32; 2]]) -> bool {
    let mut inside = false;
    let n = polygon.len();
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = (polygon[i][0], polygon[i][1]);
        let (xj, yj) = (polygon[j][0], polygon[j][1]);
        if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn dist_to_segment(px: f32, py: f32, a: [f32; 2], b: [f32; 2]) -> f32 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len_sq = dx * dx + dy * dy;
    if len_sq < 0.001 {
        return ((px - a[0]).powi(2) + (py - a[1]).powi(2)).sqrt();
    }
    let t = ((px - a[0]) * dx + (py - a[1]) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let cx = a[0] + t * dx;
    let cy = a[1] + t * dy;
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}

fn min_dist_to_edges(px: f32, py: f32, polygon: &[[f32; 2]]) -> f32 {
    let n = polygon.len();
    let mut min_d = f32::MAX;
    for i in 0..n {
        let j = (i + 1) % n;
        let d = dist_to_segment(px, py, polygon[i], polygon[j]);
        if d < min_d {
            min_d = d;
        }
    }
    min_d
}

fn rasterize_closed_path(
    image_buffer: &mut image::RgbaImage,
    points: &[[f32; 2]],
    fill: [f32; 4],
    stroke: [f32; 4],
    stroke_width: f32,
    layer_opacity: f32,
) {
    let (img_w, img_h) = image_buffer.dimensions();

    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for p in points {
        min_x = min_x.min(p[0]);
        min_y = min_y.min(p[1]);
        max_x = max_x.max(p[0]);
        max_y = max_y.max(p[1]);
    }

    let pad = stroke_width + 1.0;
    let px_min_x = ((min_x - pad).floor() as i32).max(0) as u32;
    let px_min_y = ((min_y - pad).floor() as i32).max(0) as u32;
    let px_max_x = ((max_x + pad).ceil() as u32).min(img_w - 1);
    let px_max_y = ((max_y + pad).ceil() as u32).min(img_h - 1);

    for y in px_min_y..=px_max_y {
        for x in px_min_x..=px_max_x {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;

            let inside = point_in_polygon(fx, fy, points);
            if !inside {
                continue;
            }

            let color = if stroke_width > 0.0 && stroke[3] > 0.001 {
                let edge_dist = min_dist_to_edges(fx, fy, points);
                if edge_dist <= stroke_width {
                    stroke
                } else {
                    fill
                }
            } else {
                fill
            };

            blend_pixel(image_buffer, x, y, color, layer_opacity);
        }
    }
}

fn rasterize_open_path(
    image_buffer: &mut image::RgbaImage,
    points: &[[f32; 2]],
    stroke: [f32; 4],
    stroke_width: f32,
    layer_opacity: f32,
) {
    if stroke[3] < 0.001 || stroke_width < 0.1 {
        return;
    }

    let (img_w, img_h) = image_buffer.dimensions();
    let half_thick = stroke_width / 2.0;

    for i in 0..points.len() - 1 {
        let a = points[i];
        let b = points[i + 1];

        let seg_min_x = ((a[0].min(b[0]) - half_thick).floor() as i32).max(0) as u32;
        let seg_min_y = ((a[1].min(b[1]) - half_thick).floor() as i32).max(0) as u32;
        let seg_max_x = ((a[0].max(b[0]) + half_thick).ceil() as u32).min(img_w - 1);
        let seg_max_y = ((a[1].max(b[1]) + half_thick).ceil() as u32).min(img_h - 1);

        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len_sq = dx * dx + dy * dy;
        if len_sq < 0.001 {
            continue;
        }

        for y in seg_min_y..=seg_max_y {
            for x in seg_min_x..=seg_max_x {
                let px = x as f32 + 0.5 - a[0];
                let py = y as f32 + 0.5 - a[1];
                let t = ((px * dx + py * dy) / len_sq).clamp(0.0, 1.0);
                let cx = t * dx;
                let cy = t * dy;
                let dist = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();

                if dist <= half_thick {
                    blend_pixel(image_buffer, x, y, stroke, layer_opacity);
                }
            }
        }
    }
}
