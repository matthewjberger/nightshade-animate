use std::sync::OnceLock;

use nightshade::prelude::*;

use crate::camera;
use crate::paint::Paint;
use crate::project::{AnimObject, BlendMode, LayerType, PathPoint, Project, Shape};
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

pub enum VideoFormat {
    Mp4,
    WebM,
}

pub fn export_video(
    project: &Project,
    path: &std::path::Path,
    format: VideoFormat,
) -> Result<(), String> {
    let temp_dir = std::env::temp_dir().join(format!("framekey_export_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir)
        .map_err(|error| format!("Failed to create temp directory: {}", error))?;

    for frame in 0..project.total_frames {
        let image = rasterize_frame(project, frame);
        let filename = format!("frame_{:05}.png", frame);
        let frame_path = temp_dir.join(&filename);
        image
            .save(&frame_path)
            .map_err(|error| format!("Failed to save frame {}: {}", frame, error))?;
    }

    let input_pattern = temp_dir.join("frame_%05d.png");
    let input_pattern_str = input_pattern.to_string_lossy().to_string();
    let output_str = path.to_string_lossy().to_string();

    let mut command = std::process::Command::new("ffmpeg");
    command
        .arg("-y")
        .arg("-framerate")
        .arg(project.frame_rate.to_string())
        .arg("-i")
        .arg(&input_pattern_str);

    match format {
        VideoFormat::Mp4 => {
            command
                .arg("-c:v")
                .arg("libx264")
                .arg("-pix_fmt")
                .arg("yuv420p")
                .arg("-crf")
                .arg("18")
                .arg("-preset")
                .arg("medium");
        }
        VideoFormat::WebM => {
            command
                .arg("-c:v")
                .arg("libvpx-vp9")
                .arg("-crf")
                .arg("30")
                .arg("-b:v")
                .arg("0")
                .arg("-pix_fmt")
                .arg("yuva420p");
        }
    }

    command.arg(&output_str);

    let output = command
        .output()
        .map_err(|error| format!("Failed to run ffmpeg (is it installed?): {}", error))?;

    let _ = std::fs::remove_dir_all(&temp_dir);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffmpeg failed: {}", stderr));
    }

    Ok(())
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

fn apply_camera_to_object(
    object: &AnimObject,
    cam: &camera::ResolvedCamera,
    canvas_width: f32,
    canvas_height: f32,
) -> AnimObject {
    let mut transformed = object.clone();
    let new_pos = camera::transform_point(object.position, cam, canvas_width, canvas_height);
    transformed.position = new_pos;
    transformed.rotation += cam.rotation;
    transformed.scale[0] *= cam.zoom;
    transformed.scale[1] *= cam.zoom;
    transformed
}

fn rasterize_frame(project: &Project, frame: u32) -> image::RgbaImage {
    let width = project.canvas_width;
    let height = project.canvas_height;

    let cam = camera::resolve_camera(project, frame);
    let has_camera = !project.camera_keyframes.is_empty();

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
        if layer.layer_type == LayerType::Guide || layer.layer_type == LayerType::Folder {
            continue;
        }
        if layer.layer_type == LayerType::Mask {
            continue;
        }

        let is_masked = layer_index > 0 && {
            let above = &project.layers[layer_index - 1];
            above.layer_type == LayerType::Mask && above.visible
        };

        if is_masked {
            let mask_layer = &project.layers[layer_index - 1];

            let mut layer_buffer: image::RgbaImage =
                image::ImageBuffer::from_pixel(width, height, image::Rgba([0, 0, 0, 0]));
            if let Some(objects) = tween::resolve_frame(layer, frame) {
                for object in &objects {
                    let render_obj = if has_camera {
                        apply_camera_to_object(object, &cam, width as f32, height as f32)
                    } else {
                        object.clone()
                    };
                    rasterize_object_with_assets(
                        &mut layer_buffer,
                        &render_obj,
                        layer.opacity,
                        &project.image_assets,
                    );
                }
            }

            let mut mask_buffer: image::RgbaImage =
                image::ImageBuffer::from_pixel(width, height, image::Rgba([0, 0, 0, 0]));
            if let Some(objects) = tween::resolve_frame(mask_layer, frame) {
                for object in &objects {
                    let render_obj = if has_camera {
                        apply_camera_to_object(object, &cam, width as f32, height as f32)
                    } else {
                        object.clone()
                    };
                    rasterize_object_with_assets(
                        &mut mask_buffer,
                        &render_obj,
                        mask_layer.opacity,
                        &project.image_assets,
                    );
                }
            }

            for y in 0..height {
                for x in 0..width {
                    let layer_pixel = layer_buffer.get_pixel(x, y);
                    let mask_pixel = mask_buffer.get_pixel(x, y);
                    let mask_alpha = mask_pixel[3] as f32 / 255.0;
                    let result = image::Rgba([
                        layer_pixel[0],
                        layer_pixel[1],
                        layer_pixel[2],
                        (layer_pixel[3] as f32 * mask_alpha) as u8,
                    ]);
                    layer_buffer.put_pixel(x, y, result);
                }
            }

            composite_layer(&mut image_buffer, &layer_buffer, layer.blend_mode);
        } else if layer.blend_mode != BlendMode::Normal {
            let mut layer_buffer: image::RgbaImage =
                image::ImageBuffer::from_pixel(width, height, image::Rgba([0, 0, 0, 0]));
            if let Some(objects) = tween::resolve_frame(layer, frame) {
                for object in &objects {
                    let render_obj = if has_camera {
                        apply_camera_to_object(object, &cam, width as f32, height as f32)
                    } else {
                        object.clone()
                    };
                    rasterize_object_with_assets(
                        &mut layer_buffer,
                        &render_obj,
                        layer.opacity,
                        &project.image_assets,
                    );
                }
            }
            composite_layer(&mut image_buffer, &layer_buffer, layer.blend_mode);
        } else if let Some(objects) = tween::resolve_frame(layer, frame) {
            for object in &objects {
                let render_obj = if has_camera {
                    apply_camera_to_object(object, &cam, width as f32, height as f32)
                } else {
                    object.clone()
                };
                rasterize_object_with_assets(
                    &mut image_buffer,
                    &render_obj,
                    layer.opacity,
                    &project.image_assets,
                );
            }
        }
    }

    image_buffer
}

fn composite_layer(dst: &mut image::RgbaImage, src: &image::RgbaImage, blend_mode: BlendMode) {
    let (width, height) = dst.dimensions();
    for y in 0..height {
        for x in 0..width {
            let src_pixel = src.get_pixel(x, y);
            if src_pixel[3] == 0 {
                continue;
            }
            let dst_pixel = *dst.get_pixel(x, y);
            let blended = blend_pixel_with_mode(&dst_pixel, src_pixel, blend_mode);
            dst.put_pixel(x, y, blended);
        }
    }
}

fn blend_pixel_with_mode(
    dst: &image::Rgba<u8>,
    src: &image::Rgba<u8>,
    blend_mode: BlendMode,
) -> image::Rgba<u8> {
    let src_a = src[3] as f32 / 255.0;
    let dst_a = dst[3] as f32 / 255.0;

    if src_a < 0.001 {
        return *dst;
    }

    let src_r = src[0] as f32 / 255.0;
    let src_g = src[1] as f32 / 255.0;
    let src_b = src[2] as f32 / 255.0;
    let dst_r = dst[0] as f32 / 255.0;
    let dst_g = dst[1] as f32 / 255.0;
    let dst_b = dst[2] as f32 / 255.0;

    let (blended_r, blended_g, blended_b) = match blend_mode {
        BlendMode::Normal => (src_r, src_g, src_b),
        BlendMode::Multiply => (src_r * dst_r, src_g * dst_g, src_b * dst_b),
        BlendMode::Screen => (
            1.0 - (1.0 - src_r) * (1.0 - dst_r),
            1.0 - (1.0 - src_g) * (1.0 - dst_g),
            1.0 - (1.0 - src_b) * (1.0 - dst_b),
        ),
        BlendMode::Overlay => (
            overlay_channel(dst_r, src_r),
            overlay_channel(dst_g, src_g),
            overlay_channel(dst_b, src_b),
        ),
        BlendMode::Darken => (src_r.min(dst_r), src_g.min(dst_g), src_b.min(dst_b)),
        BlendMode::Lighten => (src_r.max(dst_r), src_g.max(dst_g), src_b.max(dst_b)),
        BlendMode::ColorDodge => (
            color_dodge_channel(dst_r, src_r),
            color_dodge_channel(dst_g, src_g),
            color_dodge_channel(dst_b, src_b),
        ),
        BlendMode::ColorBurn => (
            color_burn_channel(dst_r, src_r),
            color_burn_channel(dst_g, src_g),
            color_burn_channel(dst_b, src_b),
        ),
        BlendMode::Difference => (
            (src_r - dst_r).abs(),
            (src_g - dst_g).abs(),
            (src_b - dst_b).abs(),
        ),
        BlendMode::Exclusion => (
            src_r + dst_r - 2.0 * src_r * dst_r,
            src_g + dst_g - 2.0 * src_g * dst_g,
            src_b + dst_b - 2.0 * src_b * dst_b,
        ),
    };

    let result_r = blended_r * src_a + dst_r * (1.0 - src_a);
    let result_g = blended_g * src_a + dst_g * (1.0 - src_a);
    let result_b = blended_b * src_a + dst_b * (1.0 - src_a);
    let result_a = src_a + dst_a * (1.0 - src_a);

    image::Rgba([
        (result_r * 255.0).clamp(0.0, 255.0) as u8,
        (result_g * 255.0).clamp(0.0, 255.0) as u8,
        (result_b * 255.0).clamp(0.0, 255.0) as u8,
        (result_a * 255.0).clamp(0.0, 255.0) as u8,
    ])
}

fn overlay_channel(dst: f32, src: f32) -> f32 {
    if dst < 0.5 {
        2.0 * src * dst
    } else {
        1.0 - 2.0 * (1.0 - src) * (1.0 - dst)
    }
}

fn color_dodge_channel(dst: f32, src: f32) -> f32 {
    if src >= 1.0 {
        1.0
    } else {
        (dst / (1.0 - src)).min(1.0)
    }
}

fn color_burn_channel(dst: f32, src: f32) -> f32 {
    if src <= 0.0 {
        0.0
    } else {
        1.0 - ((1.0 - dst) / src).min(1.0)
    }
}

fn rasterize_object_with_assets(
    image_buffer: &mut image::RgbaImage,
    object: &AnimObject,
    layer_opacity: f32,
    image_assets: &[crate::project::ImageAsset],
) {
    let (width, height) = image_buffer.dimensions();
    let fill = sample_paint_solid(&object.fill);
    let stroke = sample_paint_solid(&object.stroke);

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
                            sample_paint_at_local(
                                &object.stroke,
                                unrotated_x,
                                unrotated_y,
                                half_w,
                                half_h,
                            )
                        } else {
                            sample_paint_at_local(
                                &object.fill,
                                unrotated_x,
                                unrotated_y,
                                half_w,
                                half_h,
                            )
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
            let bound_x = if object.rotation.abs() > 0.001 {
                max_radius
            } else {
                scaled_rx
            };
            let bound_y = if object.rotation.abs() > 0.001 {
                max_radius
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
                                sample_paint_at_local(
                                    &object.stroke,
                                    unrotated_x,
                                    unrotated_y,
                                    scaled_rx,
                                    scaled_ry,
                                )
                            } else {
                                sample_paint_at_local(
                                    &object.fill,
                                    unrotated_x,
                                    unrotated_y,
                                    scaled_rx,
                                    scaled_ry,
                                )
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
                        blend_pixel(image_buffer, x, y, stroke, layer_opacity);
                    }
                }
            }
        }
        Shape::Path { points, closed } => {
            if points.len() < 2 {
                return;
            }

            let has_variable_pressure = points
                .iter()
                .any(|point| (point.pressure - 1.0).abs() > 0.01);

            if has_variable_pressure && !*closed {
                rasterize_variable_width_path(image_buffer, object, points, layer_opacity);
                return;
            }

            let canvas_points = build_path_points(object, points, *closed);

            if *closed && canvas_points.len() >= 3 {
                rasterize_closed_path(
                    image_buffer,
                    &canvas_points,
                    fill,
                    stroke,
                    object.stroke_width,
                    layer_opacity,
                );
            } else {
                rasterize_open_path(
                    image_buffer,
                    &canvas_points,
                    stroke,
                    object.stroke_width,
                    layer_opacity,
                );
            }
        }
        Shape::Text {
            content, font_size, ..
        } => {
            rasterize_text(image_buffer, object, content, *font_size, layer_opacity);
        }
        Shape::RasterImage {
            image_id,
            display_width,
            display_height,
            ..
        } => {
            let Some(asset) = image_assets.iter().find(|asset| asset.id == *image_id) else {
                return;
            };
            let Ok(source_image) = image::load_from_memory(&asset.data) else {
                return;
            };
            let scaled = image::imageops::resize(
                &source_image.to_rgba8(),
                (display_width * object.scale[0]) as u32,
                (display_height * object.scale[1]) as u32,
                image::imageops::FilterType::Lanczos3,
            );
            let (scaled_w, scaled_h) = scaled.dimensions();
            let half_w = scaled_w as f32 / 2.0;
            let half_h = scaled_h as f32 / 2.0;
            let origin_x = object.position[0] - half_w;
            let origin_y = object.position[1] - half_h;

            for src_y in 0..scaled_h {
                for src_x in 0..scaled_w {
                    let dst_x = (origin_x + src_x as f32) as i32;
                    let dst_y = (origin_y + src_y as f32) as i32;
                    if dst_x >= 0 && dst_y >= 0 && (dst_x as u32) < width && (dst_y as u32) < height
                    {
                        let src_pixel = scaled.get_pixel(src_x, src_y);
                        let color = [
                            src_pixel[0] as f32 / 255.0,
                            src_pixel[1] as f32 / 255.0,
                            src_pixel[2] as f32 / 255.0,
                            src_pixel[3] as f32 / 255.0,
                        ];
                        blend_pixel(
                            image_buffer,
                            dst_x as u32,
                            dst_y as u32,
                            color,
                            layer_opacity,
                        );
                    }
                }
            }
        }
        Shape::SymbolInstance { .. } => {}
    }
}

fn sample_paint_solid(paint: &Paint) -> [f32; 4] {
    paint.as_solid()
}

fn sample_paint_at_local(
    paint: &Paint,
    local_x: f32,
    local_y: f32,
    half_w: f32,
    half_h: f32,
) -> [f32; 4] {
    match paint {
        Paint::Solid(color) => *color,
        Paint::LinearGradient { start, end, .. } => {
            let dx = end[0] - start[0];
            let dy = end[1] - start[1];
            let len_sq = dx * dx + dy * dy;
            if len_sq < 0.001 {
                return paint.as_solid();
            }
            let norm_x = if half_w > 0.001 {
                (local_x / half_w + 1.0) / 2.0
            } else {
                0.5
            };
            let norm_y = if half_h > 0.001 {
                (local_y / half_h + 1.0) / 2.0
            } else {
                0.5
            };
            let px = norm_x - start[0];
            let py = norm_y - start[1];
            let t = ((px * dx + py * dy) / len_sq).clamp(0.0, 1.0);
            paint.sample_at(t)
        }
        Paint::RadialGradient { center, radius, .. } => {
            let norm_x = if half_w > 0.001 {
                (local_x / half_w + 1.0) / 2.0
            } else {
                0.5
            };
            let norm_y = if half_h > 0.001 {
                (local_y / half_h + 1.0) / 2.0
            } else {
                0.5
            };
            let dx = norm_x - center[0];
            let dy = norm_y - center[1];
            let dist = (dx * dx + dy * dy).sqrt();
            let t = if *radius > 0.001 {
                (dist / radius).clamp(0.0, 1.0)
            } else {
                0.0
            };
            paint.sample_at(t)
        }
    }
}

fn rasterize_variable_width_path(
    image_buffer: &mut image::RgbaImage,
    object: &AnimObject,
    points: &[PathPoint],
    layer_opacity: f32,
) {
    let stroke_color = object.stroke.as_solid();
    if stroke_color[3] < 0.001 {
        return;
    }

    let (img_w, img_h) = image_buffer.dimensions();

    for index in 1..points.len() {
        let prev = &points[index - 1];
        let curr = &points[index];

        let ax = object.position[0] + prev.position[0] * object.scale[0];
        let ay = object.position[1] + prev.position[1] * object.scale[1];
        let bx = object.position[0] + curr.position[0] * object.scale[0];
        let by = object.position[1] + curr.position[1] * object.scale[1];

        let avg_pressure = (prev.pressure + curr.pressure) / 2.0;
        let half_thick = (object.stroke_width * avg_pressure / 2.0).max(0.25);

        let seg_min_x = ((ax.min(bx) - half_thick).floor() as i32).max(0) as u32;
        let seg_min_y = ((ay.min(by) - half_thick).floor() as i32).max(0) as u32;
        let seg_max_x = ((ax.max(bx) + half_thick).ceil() as u32).min(img_w - 1);
        let seg_max_y = ((ay.max(by) + half_thick).ceil() as u32).min(img_h - 1);

        let dx = bx - ax;
        let dy = by - ay;
        let len_sq = dx * dx + dy * dy;
        if len_sq < 0.001 {
            continue;
        }

        for y in seg_min_y..=seg_max_y {
            for x in seg_min_x..=seg_max_x {
                let px = x as f32 + 0.5 - ax;
                let py = y as f32 + 0.5 - ay;
                let t = ((px * dx + py * dy) / len_sq).clamp(0.0, 1.0);
                let cx = t * dx;
                let cy = t * dy;
                let dist = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();

                let interp_pressure = prev.pressure + (curr.pressure - prev.pressure) * t;
                let local_half_thick = (object.stroke_width * interp_pressure / 2.0).max(0.25);

                if dist <= local_half_thick {
                    blend_pixel(image_buffer, x, y, stroke_color, layer_opacity);
                }
            }
        }
    }
}

fn rasterize_text(
    image_buffer: &mut image::RgbaImage,
    object: &AnimObject,
    content: &str,
    font_size: f32,
    layer_opacity: f32,
) {
    use ab_glyph::{Font, ScaleFont};

    let font = match get_system_font() {
        Some(font) => font,
        None => return,
    };

    let scale = ab_glyph::PxScale::from(font_size * object.scale[1]);
    let scaled_font = font.as_scaled(scale);

    let fill = object.fill.as_solid();
    let base_x = object.position[0];
    let mut cursor_x = base_x;
    let cursor_y = object.position[1] + scaled_font.ascent();

    for ch in content.chars() {
        let glyph_id = scaled_font.glyph_id(ch);
        let glyph = glyph_id.with_scale_and_position(scale, ab_glyph::point(cursor_x, cursor_y));

        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|px, py, coverage| {
                let abs_x = bounds.min.x as i32 + px as i32;
                let abs_y = bounds.min.y as i32 + py as i32;
                if abs_x >= 0
                    && abs_y >= 0
                    && (abs_x as u32) < image_buffer.width()
                    && (abs_y as u32) < image_buffer.height()
                {
                    let color = [fill[0], fill[1], fill[2], fill[3] * coverage];
                    blend_pixel(
                        image_buffer,
                        abs_x as u32,
                        abs_y as u32,
                        color,
                        layer_opacity,
                    );
                }
            });
        }

        cursor_x += scaled_font.h_advance(glyph_id);
    }
}

fn get_system_font() -> Option<&'static ab_glyph::FontArc> {
    static FONT: OnceLock<Option<ab_glyph::FontArc>> = OnceLock::new();
    FONT.get_or_init(|| {
        let font_paths = [
            "C:\\Windows\\Fonts\\arial.ttf",
            "C:\\Windows\\Fonts\\segoeui.ttf",
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/System/Library/Fonts/Helvetica.ttc",
            "/System/Library/Fonts/SFPro.ttf",
        ];

        for path in &font_paths {
            if let Ok(data) = std::fs::read(path)
                && let Ok(font) = ab_glyph::FontArc::try_from_vec(data)
            {
                return Some(font);
            }
        }

        None
    })
    .as_ref()
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
