use std::collections::HashMap;

use nightshade::prelude::*;

use crate::app::AnimateApp;
use crate::armature;
use crate::clipboard;
use crate::guides;
use crate::library;
use crate::menu;
use crate::node_edit;
use crate::onion;
use crate::paint::Paint;
use crate::project::{AnimObject, LayerType, Shape};
use crate::selection;
use crate::tools;
use crate::transform;
use crate::tween;
use crate::z_order;

#[derive(Clone)]
pub struct CanvasView {
    pub pan: egui::Vec2,
    pub zoom: f32,
    pub panel_rect: egui::Rect,
}

impl Default for CanvasView {
    fn default() -> Self {
        Self {
            pan: egui::Vec2::ZERO,
            zoom: 0.5,
            panel_rect: egui::Rect::NOTHING,
        }
    }
}

impl CanvasView {
    pub fn canvas_to_screen(&self, canvas_pos: egui::Pos2) -> egui::Pos2 {
        let center = self.panel_rect.center();
        egui::pos2(
            center.x + (canvas_pos.x + self.pan.x) * self.zoom,
            center.y + (canvas_pos.y + self.pan.y) * self.zoom,
        )
    }

    pub fn screen_to_canvas(&self, screen_pos: egui::Pos2) -> egui::Pos2 {
        let center = self.panel_rect.center();
        egui::pos2(
            (screen_pos.x - center.x) / self.zoom - self.pan.x,
            (screen_pos.y - center.y) / self.zoom - self.pan.y,
        )
    }

    pub fn canvas_size_to_screen(&self, size: f32) -> f32 {
        size * self.zoom
    }
}

pub fn draw_canvas(app: &mut AnimateApp, ui_context: &egui::Context) {
    #[cfg(not(target_arch = "wasm32"))]
    ensure_image_textures(app, ui_context);

    egui::CentralPanel::default().show(ui_context, |ui| {
        let panel_rect = ui.available_rect_before_wrap();
        app.canvas_view.panel_rect = panel_rect;

        let (response, painter) = ui.allocate_painter(
            ui.available_size_before_wrap(),
            egui::Sense::click_and_drag(),
        );

        let clipped_painter = painter.with_clip_rect(panel_rect);

        draw_canvas_background(app, &clipped_painter);

        onion::draw_onion_skins(app, &app.canvas_view.clone(), &clipped_painter);

        draw_frame_objects(app, &clipped_painter);

        selection::draw_selection_indicators(app, &app.canvas_view.clone(), &clipped_painter);
        transform::draw_transform_handles(app, &app.canvas_view.clone(), &clipped_painter);
        node_edit::draw_node_edit_overlay(app, &app.canvas_view.clone(), &clipped_painter);

        armature::draw_bone_overlay(app, &app.canvas_view.clone(), &clipped_painter);

        tools::draw_tool_preview(app, &app.canvas_view.clone(), &clipped_painter);

        guides::draw_rulers_and_guides(app, &app.canvas_view.clone(), &clipped_painter);

        handle_pan_zoom(app, &response, ui_context);

        match app.tool {
            crate::tools::Tool::Select => {
                let handled = transform::handle_transform_interaction(app, &response, ui_context);
                if !handled {
                    selection::handle_select_tool(app, &response, ui_context);
                }
            }
            _ => {
                tools::handle_drawing_tool(app, &response, ui_context);
            }
        }

        guides::handle_ruler_interaction(app, ui_context);

        draw_context_menu(app, &response);

        ui.ctx().request_repaint();
    });
}

fn draw_context_menu(app: &mut AnimateApp, response: &egui::Response) {
    response.context_menu(|ui| {
        if !app.selection.selected_objects.is_empty() {
            if ui.button("Cut (Ctrl+X)").clicked() {
                clipboard::cut_selected(app);
                ui.close();
            }
            if ui.button("Copy (Ctrl+C)").clicked() {
                clipboard::copy_selected(app);
                ui.close();
            }
        }
        if !app.clipboard.objects.is_empty() && ui.button("Paste (Ctrl+V)").clicked() {
            clipboard::paste(app);
            ui.close();
        }
        if !app.selection.selected_objects.is_empty() {
            if ui.button("Duplicate (Ctrl+D)").clicked() {
                clipboard::duplicate_selected(app);
                ui.close();
            }
            ui.separator();
            ui.menu_button("Arrange", |ui| {
                if ui.button("Bring to Front").clicked() {
                    z_order::bring_to_front(app);
                    ui.close();
                }
                if ui.button("Bring Forward").clicked() {
                    z_order::bring_forward(app);
                    ui.close();
                }
                if ui.button("Send Backward").clicked() {
                    z_order::send_backward(app);
                    ui.close();
                }
                if ui.button("Send to Back").clicked() {
                    z_order::send_to_back(app);
                    ui.close();
                }
            });
            ui.separator();
            if ui.button("Convert to Symbol").clicked() {
                library::convert_selection_to_symbol(app);
                ui.close();
            }
            ui.separator();
            if ui.button("Delete").clicked() {
                menu::delete_selected(app);
                ui.close();
            }
        }
        if app.selection.selected_objects.is_empty() {
            if !app.clipboard.objects.is_empty() && ui.button("Paste (Ctrl+V)").clicked() {
                clipboard::paste(app);
                ui.close();
            }
            if ui.button("Select All (Ctrl+A)").clicked() {
                menu::select_all(app);
                ui.close();
            }
        }
    });
}

fn handle_pan_zoom(app: &mut AnimateApp, response: &egui::Response, ui_context: &egui::Context) {
    if response.dragged_by(egui::PointerButton::Middle) {
        let delta = response.drag_delta();
        app.canvas_view.pan.x += delta.x / app.canvas_view.zoom;
        app.canvas_view.pan.y += delta.y / app.canvas_view.zoom;
    }

    let scroll_delta = ui_context.input(|input| input.smooth_scroll_delta.y);
    if scroll_delta != 0.0 && response.hovered() {
        let zoom_factor = 1.0 + scroll_delta * 0.001;
        let old_zoom = app.canvas_view.zoom;
        app.canvas_view.zoom = (app.canvas_view.zoom * zoom_factor).clamp(0.05, 10.0);

        if let Some(pointer_pos) = ui_context.input(|input| input.pointer.hover_pos()) {
            let center = app.canvas_view.panel_rect.center();
            let pointer_offset = pointer_pos - center;
            let adjust = pointer_offset * (1.0 / old_zoom - 1.0 / app.canvas_view.zoom);
            app.canvas_view.pan += adjust;
        }
    }
}

fn draw_canvas_background(app: &AnimateApp, painter: &egui::Painter) {
    let panel_bg = egui::Color32::from_rgb(50, 50, 50);
    painter.rect_filled(app.canvas_view.panel_rect, 0.0, panel_bg);

    let top_left = app.canvas_view.canvas_to_screen(egui::pos2(0.0, 0.0));
    let bottom_right = app.canvas_view.canvas_to_screen(egui::pos2(
        app.project.canvas_width as f32,
        app.project.canvas_height as f32,
    ));
    let canvas_rect = egui::Rect::from_two_pos(top_left, bottom_right);

    let bg = app.project.background_color;
    let bg_color = egui::Color32::from_rgba_unmultiplied(
        (bg[0] * 255.0) as u8,
        (bg[1] * 255.0) as u8,
        (bg[2] * 255.0) as u8,
        (bg[3] * 255.0) as u8,
    );

    painter.rect(
        canvas_rect,
        0.0,
        bg_color,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 100, 100)),
        egui::StrokeKind::Outside,
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_image_textures(app: &mut AnimateApp, ui_context: &egui::Context) {
    for asset in &app.project.image_assets {
        if app.image_textures.contains_key(&asset.id) {
            continue;
        }
        let Ok(dynamic_image) = image::load_from_memory(&asset.data) else {
            continue;
        };
        let rgba = dynamic_image.to_rgba8();
        let (tex_width, tex_height) = rgba.dimensions();
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [tex_width as usize, tex_height as usize],
            rgba.as_raw(),
        );
        let handle = ui_context.load_texture(
            format!("img_{}", asset.id),
            color_image,
            egui::TextureOptions::LINEAR,
        );
        app.image_textures.insert(asset.id, handle);
    }
}

fn draw_frame_objects(app: &AnimateApp, painter: &egui::Painter) {
    #[cfg(not(target_arch = "wasm32"))]
    let textures = &app.image_textures;
    #[cfg(target_arch = "wasm32")]
    let textures: HashMap<uuid::Uuid, egui::TextureHandle> = HashMap::new();
    #[cfg(target_arch = "wasm32")]
    let textures = &textures;

    for layer_index in (0..app.project.layers.len()).rev() {
        let layer = &app.project.layers[layer_index];
        if !layer.visible {
            continue;
        }
        if layer.layer_type == LayerType::Folder {
            continue;
        }

        if let Some(objects) = tween::resolve_frame(layer, app.current_frame) {
            for object in &objects {
                if layer.layer_type == LayerType::Guide {
                    render_object(
                        object,
                        &app.canvas_view,
                        painter,
                        layer.opacity * 0.5,
                        Some(textures),
                    );
                    draw_guide_indicator(object, &app.canvas_view, painter);
                } else {
                    render_object(
                        object,
                        &app.canvas_view,
                        painter,
                        layer.opacity,
                        Some(textures),
                    );
                }
            }
        }
    }

    render_symbol_instances(app, painter, textures);
}

fn draw_guide_indicator(object: &AnimObject, view: &CanvasView, painter: &egui::Painter) {
    let (half_w, half_h, offset) = selection::get_object_bounds_public(object);
    let center_x = object.position[0] + offset[0];
    let center_y = object.position[1] + offset[1];

    let screen_min = view.canvas_to_screen(egui::pos2(center_x - half_w, center_y - half_h));
    let screen_max = view.canvas_to_screen(egui::pos2(center_x + half_w, center_y + half_h));
    let rect = egui::Rect::from_two_pos(screen_min, screen_max);

    let green_fill = egui::Color32::from_rgba_unmultiplied(0, 200, 100, 40);
    let green_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 200, 100));
    painter.rect(
        rect,
        0.0,
        green_fill,
        green_stroke,
        egui::StrokeKind::Outside,
    );
}

fn paint_to_color32(paint: &Paint, opacity: f32) -> egui::Color32 {
    let color = paint.as_solid();
    egui::Color32::from_rgba_unmultiplied(
        (color[0] * 255.0) as u8,
        (color[1] * 255.0) as u8,
        (color[2] * 255.0) as u8,
        (color[3] * opacity * 255.0) as u8,
    )
}

pub fn render_object(
    object: &AnimObject,
    view: &CanvasView,
    painter: &egui::Painter,
    layer_opacity: f32,
    image_textures: Option<&HashMap<uuid::Uuid, egui::TextureHandle>>,
) {
    let fill = paint_to_color32(&object.fill, layer_opacity);
    let stroke_color = paint_to_color32(&object.stroke, layer_opacity);
    let stroke = egui::Stroke::new(object.stroke_width * view.zoom, stroke_color);

    let pos = egui::pos2(object.position[0], object.position[1]);
    let screen_pos = view.canvas_to_screen(pos);

    match &object.shape {
        Shape::Rectangle {
            width,
            height,
            corner_radius,
        } => {
            let half_w = width * object.scale[0] / 2.0;
            let half_h = height * object.scale[1] / 2.0;

            if object.rotation.abs() < 0.001 {
                let screen_min = view.canvas_to_screen(egui::pos2(
                    object.position[0] - half_w,
                    object.position[1] - half_h,
                ));
                let screen_max = view.canvas_to_screen(egui::pos2(
                    object.position[0] + half_w,
                    object.position[1] + half_h,
                ));
                let rect = egui::Rect::from_two_pos(screen_min, screen_max);
                let screen_radius = corner_radius * view.zoom;
                painter.rect(rect, screen_radius, fill, stroke, egui::StrokeKind::Outside);
            } else {
                let corners = [
                    [-half_w, -half_h],
                    [half_w, -half_h],
                    [half_w, half_h],
                    [-half_w, half_h],
                ];
                let rotated: Vec<egui::Pos2> = corners
                    .iter()
                    .map(|[corner_x, corner_y]| {
                        let rotated_x =
                            corner_x * object.rotation.cos() - corner_y * object.rotation.sin();
                        let rotated_y =
                            corner_x * object.rotation.sin() + corner_y * object.rotation.cos();
                        view.canvas_to_screen(egui::pos2(
                            object.position[0] + rotated_x,
                            object.position[1] + rotated_y,
                        ))
                    })
                    .collect();

                let shape = egui::epaint::PathShape::convex_polygon(rotated, fill, stroke);
                painter.add(shape);
            }
        }
        Shape::Ellipse { radius_x, radius_y } => {
            let scaled_rx = radius_x * object.scale[0];
            let scaled_ry = radius_y * object.scale[1];
            let screen_rx = view.canvas_size_to_screen(scaled_rx);
            let screen_ry = view.canvas_size_to_screen(scaled_ry);

            if (screen_rx - screen_ry).abs() < 0.5 && object.rotation.abs() < 0.001 {
                painter.circle(screen_pos, screen_rx, fill, stroke);
            } else {
                let segments = 64;
                let points: Vec<egui::Pos2> = (0..segments)
                    .map(|segment_index| {
                        let angle =
                            2.0 * std::f32::consts::PI * segment_index as f32 / segments as f32;
                        let ellipse_x = angle.cos() * scaled_rx;
                        let ellipse_y = angle.sin() * scaled_ry;
                        let rotated_x =
                            ellipse_x * object.rotation.cos() - ellipse_y * object.rotation.sin();
                        let rotated_y =
                            ellipse_x * object.rotation.sin() + ellipse_y * object.rotation.cos();
                        view.canvas_to_screen(egui::pos2(
                            object.position[0] + rotated_x,
                            object.position[1] + rotated_y,
                        ))
                    })
                    .collect();

                let shape = egui::epaint::PathShape::convex_polygon(points, fill, stroke);
                painter.add(shape);
            }
        }
        Shape::Line { end_x, end_y } => {
            let end_canvas = egui::pos2(
                object.position[0] + end_x * object.scale[0],
                object.position[1] + end_y * object.scale[1],
            );
            let screen_end = view.canvas_to_screen(end_canvas);
            painter.line_segment([screen_pos, screen_end], stroke);
        }
        Shape::Path { points, closed } => {
            if points.len() < 2 {
                return;
            }

            let has_variable_pressure = points
                .iter()
                .any(|point| (point.pressure - 1.0).abs() > 0.01);

            if has_variable_pressure && !*closed {
                render_variable_width_path(object, points, view, painter, layer_opacity);
                return;
            }

            let mut screen_points = Vec::new();
            for path_point_index in 0..points.len() {
                let point = &points[path_point_index];
                let canvas_pt = egui::pos2(
                    object.position[0] + point.position[0] * object.scale[0],
                    object.position[1] + point.position[1] * object.scale[1],
                );

                if path_point_index > 0 {
                    let prev = &points[path_point_index - 1];
                    if prev.control_out.is_some() || point.control_in.is_some() {
                        let control_out = prev.control_out.unwrap_or(prev.position);
                        let control_in = point.control_in.unwrap_or(point.position);
                        for step in 1..=16 {
                            let t = step as f32 / 16.0;
                            let bezier = cubic_bezier(
                                prev.position,
                                control_out,
                                control_in,
                                point.position,
                                t,
                            );
                            let canvas_bezier = egui::pos2(
                                object.position[0] + bezier[0] * object.scale[0],
                                object.position[1] + bezier[1] * object.scale[1],
                            );
                            screen_points.push(view.canvas_to_screen(canvas_bezier));
                        }
                        continue;
                    }
                }
                screen_points.push(view.canvas_to_screen(canvas_pt));
            }

            if *closed && points.len() > 2 {
                let last = points.last().unwrap();
                let first = &points[0];
                if last.control_out.is_some() || first.control_in.is_some() {
                    let control_out = last.control_out.unwrap_or(last.position);
                    let control_in = first.control_in.unwrap_or(first.position);
                    for step in 1..=16 {
                        let t = step as f32 / 16.0;
                        let bezier =
                            cubic_bezier(last.position, control_out, control_in, first.position, t);
                        let canvas_bezier = egui::pos2(
                            object.position[0] + bezier[0] * object.scale[0],
                            object.position[1] + bezier[1] * object.scale[1],
                        );
                        screen_points.push(view.canvas_to_screen(canvas_bezier));
                    }
                }

                let shape = egui::epaint::PathShape::convex_polygon(screen_points, fill, stroke);
                painter.add(shape);
            } else {
                let path_shape = egui::epaint::PathShape::line(screen_points, stroke);
                painter.add(path_shape);
            }
        }
        Shape::Text {
            content, font_size, ..
        } => {
            let screen_font_size = font_size * view.zoom * object.scale[1];
            let text_color = paint_to_color32(&object.fill, layer_opacity);
            painter.text(
                screen_pos,
                egui::Align2::LEFT_TOP,
                content,
                egui::FontId::proportional(screen_font_size),
                text_color,
            );
        }
        Shape::RasterImage {
            image_id,
            display_width,
            display_height,
            ..
        } => {
            if let Some(textures) = image_textures
                && let Some(handle) = textures.get(image_id)
            {
                let half_w = display_width * object.scale[0] / 2.0;
                let half_h = display_height * object.scale[1] / 2.0;
                let screen_min = view.canvas_to_screen(egui::pos2(
                    object.position[0] - half_w,
                    object.position[1] - half_h,
                ));
                let screen_max = view.canvas_to_screen(egui::pos2(
                    object.position[0] + half_w,
                    object.position[1] + half_h,
                ));
                let rect = egui::Rect::from_two_pos(screen_min, screen_max);
                let tint = egui::Color32::from_rgba_unmultiplied(
                    255,
                    255,
                    255,
                    (layer_opacity * 255.0) as u8,
                );
                painter.image(
                    handle.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    tint,
                );
            }
        }
        Shape::SymbolInstance { .. } => {}
    }
}

pub fn render_symbol_instances(
    app: &AnimateApp,
    painter: &egui::Painter,
    image_textures: &HashMap<uuid::Uuid, egui::TextureHandle>,
) {
    for layer_index in (0..app.project.layers.len()).rev() {
        let layer = &app.project.layers[layer_index];
        if !layer.visible || layer.layer_type == LayerType::Folder {
            continue;
        }

        if let Some(objects) = tween::resolve_frame(layer, app.current_frame) {
            for object in &objects {
                if let Shape::SymbolInstance { symbol_id } = &object.shape
                    && let Some(symbol) = app
                        .project
                        .library
                        .symbols
                        .iter()
                        .find(|symbol| symbol.id == *symbol_id)
                {
                    for symbol_layer in symbol.layers.iter().rev() {
                        if !symbol_layer.visible {
                            continue;
                        }
                        if let Some(symbol_objects) = tween::resolve_frame(symbol_layer, 0) {
                            for symbol_object in &symbol_objects {
                                let mut transformed = symbol_object.clone();
                                let cos_r = object.rotation.cos();
                                let sin_r = object.rotation.sin();
                                let local_x = symbol_object.position[0] * object.scale[0];
                                let local_y = symbol_object.position[1] * object.scale[1];
                                transformed.position[0] =
                                    object.position[0] + local_x * cos_r - local_y * sin_r;
                                transformed.position[1] =
                                    object.position[1] + local_x * sin_r + local_y * cos_r;
                                transformed.rotation += object.rotation;
                                transformed.scale[0] *= object.scale[0];
                                transformed.scale[1] *= object.scale[1];
                                render_object(
                                    &transformed,
                                    &app.canvas_view,
                                    painter,
                                    layer.opacity,
                                    Some(image_textures),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

fn render_variable_width_path(
    object: &AnimObject,
    points: &[crate::project::PathPoint],
    view: &CanvasView,
    painter: &egui::Painter,
    layer_opacity: f32,
) {
    let stroke_color = paint_to_color32(&object.stroke, layer_opacity);
    let base_width = object.stroke_width * view.zoom;

    for index in 1..points.len() {
        let prev = &points[index - 1];
        let curr = &points[index];

        let prev_screen = view.canvas_to_screen(egui::pos2(
            object.position[0] + prev.position[0] * object.scale[0],
            object.position[1] + prev.position[1] * object.scale[1],
        ));
        let curr_screen = view.canvas_to_screen(egui::pos2(
            object.position[0] + curr.position[0] * object.scale[0],
            object.position[1] + curr.position[1] * object.scale[1],
        ));

        let avg_pressure = (prev.pressure + curr.pressure) / 2.0;
        let width = (base_width * avg_pressure).max(0.5);

        painter.line_segment(
            [prev_screen, curr_screen],
            egui::Stroke::new(width, stroke_color),
        );
    }
}

fn cubic_bezier(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], p3: [f32; 2], t: f32) -> [f32; 2] {
    let one_minus_t = 1.0 - t;
    let one_minus_t_sq = one_minus_t * one_minus_t;
    let one_minus_t_cu = one_minus_t_sq * one_minus_t;
    let t_sq = t * t;
    let t_cu = t_sq * t;

    [
        one_minus_t_cu * p0[0]
            + 3.0 * one_minus_t_sq * t * p1[0]
            + 3.0 * one_minus_t * t_sq * p2[0]
            + t_cu * p3[0],
        one_minus_t_cu * p0[1]
            + 3.0 * one_minus_t_sq * t * p1[1]
            + 3.0 * one_minus_t * t_sq * p2[1]
            + t_cu * p3[1],
    ]
}
