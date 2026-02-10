use nightshade::prelude::*;

use crate::app::{AnimateApp, KeyframeDragState};
use crate::project::{Keyframe, Layer, LayerType, TweenType};
use crate::scenes;

const LAYER_PANEL_WIDTH: f32 = 180.0;
const BASE_FRAME_CELL_WIDTH: f32 = 16.0;
const FRAME_CELL_HEIGHT: f32 = 24.0;
const PROPERTY_TRACK_HEIGHT: f32 = 18.0;
const HEADER_HEIGHT: f32 = 20.0;
const LABEL_HEIGHT: f32 = 14.0;

fn scaled_cell_width(app: &AnimateApp) -> f32 {
    BASE_FRAME_CELL_WIDTH * app.timeline_zoom
}

pub fn draw_timeline(app: &mut AnimateApp, ui_context: &egui::Context) {
    egui::TopBottomPanel::bottom("timeline")
        .resizable(true)
        .default_height(250.0)
        .min_height(100.0)
        .show(ui_context, |ui| {
            if !app.project.scenes.is_empty() {
                scenes::draw_scene_tabs(app, ui);
                ui.separator();
            }
            ui.horizontal(|ui| {
                draw_layer_panel(app, ui);
                ui.separator();
                draw_frame_grid(app, ui);
            });
            #[cfg(not(target_arch = "wasm32"))]
            crate::audio::draw_audio_tracks(app, ui);
        });
}

fn draw_layer_panel(app: &mut AnimateApp, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        ui.set_min_width(LAYER_PANEL_WIDTH);
        ui.set_max_width(LAYER_PANEL_WIDTH);

        ui.horizontal(|ui| {
            if ui.small_button("+").clicked() {
                let name = format!("Layer {}", app.project.layers.len() + 1);
                let mut layer = Layer::new(name);
                layer.keyframes.insert(0, Keyframe::default());
                app.project.layers.insert(0, layer);
            }
            if ui.small_button("-").clicked()
                && app.project.layers.len() > 1
                && app.active_layer < app.project.layers.len()
            {
                app.history.push(app.project.clone());
                app.project.layers.remove(app.active_layer);
                if app.active_layer >= app.project.layers.len() {
                    app.active_layer = app.project.layers.len() - 1;
                }
            }
            ui.label("Layers");
        });

        ui.add_space(HEADER_HEIGHT + LABEL_HEIGHT - ui.spacing().item_spacing.y);

        egui::ScrollArea::vertical()
            .id_salt("layer_scroll")
            .show(ui, |ui| {
                let mut layer_action = None;
                let mut visibility_toggle = None;
                let mut lock_toggle = None;
                let mut collapse_toggle = None;
                let mut property_expand_toggle = None;
                let mut drag_source: Option<usize> = None;
                let mut drag_target: Option<usize> = None;

                let layer_depths: Vec<usize> = (0..app.project.layers.len())
                    .map(|index| layer_depth(&app.project.layers, index))
                    .collect();

                for (layer_index, &depth) in layer_depths.iter().enumerate() {
                    let parent_collapsed = is_parent_collapsed(&app.project.layers, layer_index);
                    if parent_collapsed {
                        continue;
                    }

                    let is_active = layer_index == app.active_layer;
                    let bg_color = if is_active {
                        egui::Color32::from_rgb(60, 80, 120)
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    let indent = depth as f32 * 12.0;

                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(LAYER_PANEL_WIDTH - 10.0, FRAME_CELL_HEIGHT),
                        egui::Sense::click_and_drag(),
                    );

                    if response.drag_started() {
                        drag_source = Some(layer_index);
                    }
                    if response.hovered() && ui.ctx().input(|input| input.pointer.primary_down()) {
                        drag_target = Some(layer_index);
                    }

                    let eye_rect = egui::Rect::from_min_size(
                        rect.min + egui::vec2(2.0 + indent, 2.0),
                        egui::vec2(16.0, 20.0),
                    );
                    let lock_rect = egui::Rect::from_min_size(
                        rect.min + egui::vec2(20.0 + indent, 2.0),
                        egui::vec2(16.0, 20.0),
                    );

                    if response.clicked()
                        && let Some(pos) = response.interact_pointer_pos()
                    {
                        let prop_expand_rect = egui::Rect::from_min_size(
                            egui::pos2(rect.max.x - 16.0, rect.min.y + 2.0),
                            egui::vec2(14.0, 20.0),
                        );
                        if eye_rect.contains(pos) {
                            visibility_toggle = Some(layer_index);
                        } else if lock_rect.contains(pos) {
                            lock_toggle = Some(layer_index);
                        } else if prop_expand_rect.contains(pos)
                            && !app.project.layers[layer_index].property_tracks.is_empty()
                        {
                            property_expand_toggle = Some(layer_index);
                        } else {
                            let layer = &app.project.layers[layer_index];
                            if layer.layer_type == LayerType::Folder {
                                let collapse_rect = egui::Rect::from_min_size(
                                    rect.min + egui::vec2(38.0 + indent, 2.0),
                                    egui::vec2(16.0, 20.0),
                                );
                                if collapse_rect.contains(pos) {
                                    collapse_toggle = Some(layer_index);
                                } else {
                                    layer_action = Some(layer_index);
                                }
                            } else {
                                layer_action = Some(layer_index);
                            }
                        }
                    }

                    ui.painter().rect_filled(rect, 2.0, bg_color);

                    let layer = &app.project.layers[layer_index];
                    let pointer_pos = ui.ctx().input(|input| input.pointer.hover_pos());

                    let eye_hovered = pointer_pos.is_some_and(|pos| eye_rect.contains(pos));
                    let eye_text = if layer.visible { "V" } else { "-" };
                    let eye_color = if eye_hovered {
                        egui::Color32::from_rgb(255, 220, 100)
                    } else if layer.visible {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgb(80, 80, 80)
                    };
                    ui.painter().text(
                        eye_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        eye_text,
                        egui::FontId::monospace(10.0),
                        eye_color,
                    );

                    let lock_hovered = pointer_pos.is_some_and(|pos| lock_rect.contains(pos));
                    let lock_text = if layer.locked { "L" } else { "-" };
                    let lock_color = if lock_hovered {
                        egui::Color32::from_rgb(255, 220, 100)
                    } else if layer.locked {
                        egui::Color32::WHITE
                    } else {
                        egui::Color32::from_rgb(80, 80, 80)
                    };
                    ui.painter().text(
                        lock_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        lock_text,
                        egui::FontId::monospace(10.0),
                        lock_color,
                    );

                    let type_icon = match layer.layer_type {
                        LayerType::Normal => "N",
                        LayerType::Guide => "G",
                        LayerType::Mask => "M",
                        LayerType::Folder => {
                            if layer.collapsed {
                                "+"
                            } else {
                                "-"
                            }
                        }
                    };
                    let type_color = match layer.layer_type {
                        LayerType::Normal => egui::Color32::from_rgb(150, 150, 150),
                        LayerType::Guide => egui::Color32::from_rgb(100, 200, 100),
                        LayerType::Mask => egui::Color32::from_rgb(200, 150, 100),
                        LayerType::Folder => egui::Color32::from_rgb(200, 200, 100),
                    };
                    ui.painter().text(
                        egui::pos2(
                            rect.min.x + 38.0 + indent,
                            rect.min.y + FRAME_CELL_HEIGHT / 2.0,
                        ),
                        egui::Align2::LEFT_CENTER,
                        type_icon,
                        egui::FontId::monospace(10.0),
                        type_color,
                    );

                    let name_pos = rect.min + egui::vec2(52.0 + indent, FRAME_CELL_HEIGHT / 2.0);
                    ui.painter().text(
                        name_pos,
                        egui::Align2::LEFT_CENTER,
                        &layer.name,
                        egui::FontId::proportional(12.0),
                        egui::Color32::WHITE,
                    );

                    let has_any_property_tracks = !layer.property_tracks.is_empty();
                    if has_any_property_tracks {
                        let prop_expand_rect = egui::Rect::from_min_size(
                            egui::pos2(rect.max.x - 16.0, rect.min.y + 2.0),
                            egui::vec2(14.0, 20.0),
                        );
                        let is_expanded = app.expanded_property_layers.contains(&layer.id);
                        let expand_text = if is_expanded { "P-" } else { "P+" };
                        let prop_hovered =
                            pointer_pos.is_some_and(|pos| prop_expand_rect.contains(pos));
                        let prop_color = if prop_hovered {
                            egui::Color32::from_rgb(255, 200, 50)
                        } else {
                            egui::Color32::from_rgb(180, 160, 80)
                        };
                        ui.painter().text(
                            prop_expand_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            expand_text,
                            egui::FontId::monospace(8.0),
                            prop_color,
                        );
                    }

                    if let Some(ref drag) = app.timeline_drag
                        && drag.dragging_layer.is_some()
                        && drag.drop_target == Some(layer_index)
                    {
                        painter_drop_indicator(ui, rect);
                    }

                    if app.expanded_property_layers.contains(&layer.id) {
                        for tracks in layer.property_tracks.values() {
                            for prop_name in tracks.active_property_names() {
                                let (sub_rect, _sub_response) = ui.allocate_exact_size(
                                    egui::vec2(LAYER_PANEL_WIDTH - 10.0, PROPERTY_TRACK_HEIGHT),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(
                                    sub_rect,
                                    0.0,
                                    egui::Color32::from_rgb(35, 38, 50),
                                );
                                ui.painter().text(
                                    sub_rect.min
                                        + egui::vec2(24.0 + indent, PROPERTY_TRACK_HEIGHT / 2.0),
                                    egui::Align2::LEFT_CENTER,
                                    prop_name,
                                    egui::FontId::proportional(10.0),
                                    egui::Color32::from_rgb(180, 160, 80),
                                );
                            }
                        }
                    }
                }

                if let Some(index) = layer_action {
                    app.active_layer = index;
                }
                if let Some(index) = visibility_toggle {
                    app.history.push(app.project.clone());
                    app.project.layers[index].visible = !app.project.layers[index].visible;
                }
                if let Some(index) = lock_toggle {
                    app.history.push(app.project.clone());
                    app.project.layers[index].locked = !app.project.layers[index].locked;
                }
                if let Some(index) = collapse_toggle {
                    app.project.layers[index].collapsed = !app.project.layers[index].collapsed;
                }
                if let Some(index) = property_expand_toggle {
                    let layer_id = app.project.layers[index].id;
                    if app.expanded_property_layers.contains(&layer_id) {
                        app.expanded_property_layers.remove(&layer_id);
                    } else {
                        app.expanded_property_layers.insert(layer_id);
                    }
                }
                if let Some(source) = drag_source {
                    app.timeline_drag = Some(crate::app::TimelineDragState {
                        dragging_layer: Some(source),
                        drop_target: None,
                        dragging_keyframe: None,
                    });
                }
                if let Some(target) = drag_target
                    && let Some(ref mut drag) = app.timeline_drag
                    && drag.dragging_layer.is_some()
                {
                    drag.drop_target = Some(target);
                }

                let primary_released = ui.ctx().input(|input| input.pointer.primary_released());
                if primary_released {
                    if let Some(ref drag) = app.timeline_drag
                        && let (Some(source), Some(target)) =
                            (drag.dragging_layer, drag.drop_target)
                        && source != target
                        && source < app.project.layers.len()
                        && target < app.project.layers.len()
                    {
                        app.history.push(app.project.clone());
                        let layer = app.project.layers.remove(source);
                        let insert_at = if target > source { target } else { target + 1 };
                        let insert_at = insert_at.min(app.project.layers.len());
                        app.project.layers.insert(insert_at, layer);
                        if app.active_layer == source {
                            app.active_layer = insert_at.min(app.project.layers.len() - 1);
                        }
                    }
                    app.timeline_drag = None;
                }
            });
    });
}

fn draw_frame_grid(app: &mut AnimateApp, ui: &mut egui::Ui) {
    let cell_width = scaled_cell_width(app);

    let scroll_id = egui::Id::new("frame_scroll");
    let ctrl_scroll = ui.ctx().input(|input| {
        if input.modifiers.ctrl {
            input.smooth_scroll_delta.y
        } else {
            0.0
        }
    });

    if ctrl_scroll != 0.0 {
        let zoom_factor = 1.0 + ctrl_scroll * 0.002;
        app.timeline_zoom = (app.timeline_zoom * zoom_factor).clamp(0.3, 5.0);
    }

    egui::ScrollArea::horizontal()
        .id_salt(scroll_id)
        .show(ui, |ui| {
            let total_width = app.project.total_frames as f32 * cell_width;
            let layers_height = total_layers_height(app);
            let total_height = HEADER_HEIGHT + LABEL_HEIGHT + layers_height;

            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(total_width, total_height),
                egui::Sense::click_and_drag(),
            );

            let painter = ui.painter_at(rect);

            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(40, 40, 40));

            let label_area_y = rect.min.y;
            let header_y = rect.min.y + LABEL_HEIGHT;
            let grid_y = header_y + HEADER_HEIGHT;

            draw_loop_region(app, &painter, rect, cell_width, header_y);

            for frame in 0..app.project.total_frames {
                let x = rect.min.x + frame as f32 * cell_width;

                if frame % 5 == 0 {
                    painter.line_segment(
                        [
                            egui::pos2(x, header_y),
                            egui::pos2(x, header_y + HEADER_HEIGHT),
                        ],
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)),
                    );

                    if frame % 10 == 0 {
                        painter.text(
                            egui::pos2(x + 2.0, header_y + 2.0),
                            egui::Align2::LEFT_TOP,
                            format!("{}", frame + 1),
                            egui::FontId::monospace(9.0),
                            egui::Color32::from_rgb(150, 150, 150),
                        );
                    }
                }

                painter.line_segment(
                    [egui::pos2(x, grid_y), egui::pos2(x, rect.max.y)],
                    egui::Stroke::new(0.5, egui::Color32::from_rgb(55, 55, 55)),
                );
            }

            draw_frame_labels(app, &painter, rect, cell_width, label_area_y);

            let y_offsets = compute_layer_y_offsets(app);

            for (layer_index, &y_offset) in
                y_offsets.iter().enumerate().take(app.project.layers.len())
            {
                let y = grid_y + y_offset;

                painter.line_segment(
                    [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                    egui::Stroke::new(0.5, egui::Color32::from_rgb(55, 55, 55)),
                );

                draw_selected_frame_highlight(app, &painter, rect, cell_width, layer_index, y);

                let layer = &app.project.layers[layer_index];

                for (&frame, keyframe) in &layer.keyframes {
                    let x = rect.min.x + frame as f32 * cell_width + cell_width / 2.0;
                    let center_y = y + FRAME_CELL_HEIGHT / 2.0;

                    let color = tween_color(keyframe.tween);

                    if keyframe.objects.is_empty() {
                        painter.circle_stroke(
                            egui::pos2(x, center_y),
                            3.0,
                            egui::Stroke::new(1.5, color),
                        );
                    } else {
                        painter.circle_filled(egui::pos2(x, center_y), 4.0, color);
                    }

                    if keyframe.tween != TweenType::None
                        && let Some((&next_frame, _)) = layer.keyframes.range((frame + 1)..).next()
                    {
                        let start_x = rect.min.x + (frame + 1) as f32 * cell_width;
                        let end_x = rect.min.x + next_frame as f32 * cell_width;
                        let tween_rect = egui::Rect::from_min_max(
                            egui::pos2(start_x, y + 1.0),
                            egui::pos2(end_x, y + FRAME_CELL_HEIGHT - 1.0),
                        );
                        let tween_color = color.gamma_multiply(0.2);
                        painter.rect_filled(tween_rect, 0.0, tween_color);
                    }
                }

                if app.expanded_property_layers.contains(&layer.id) {
                    let mut sub_y = y + FRAME_CELL_HEIGHT;
                    let prop_key_color = egui::Color32::from_rgb(200, 180, 50);
                    for tracks in layer.property_tracks.values() {
                        for prop_name in tracks.active_property_names() {
                            painter.line_segment(
                                [egui::pos2(rect.min.x, sub_y), egui::pos2(rect.max.x, sub_y)],
                                egui::Stroke::new(0.5, egui::Color32::from_rgb(50, 50, 60)),
                            );

                            let frames = tracks.keyframe_frames_for(prop_name);
                            for frame in frames {
                                let x = rect.min.x + frame as f32 * cell_width + cell_width / 2.0;
                                let center_y = sub_y + PROPERTY_TRACK_HEIGHT / 2.0;
                                painter.rect_filled(
                                    egui::Rect::from_center_size(
                                        egui::pos2(x, center_y),
                                        egui::vec2(5.0, 5.0),
                                    ),
                                    0.0,
                                    prop_key_color,
                                );
                            }

                            sub_y += PROPERTY_TRACK_HEIGHT;
                        }
                    }
                }
            }

            draw_keyframe_drag_ghost(app, &painter, rect, cell_width, grid_y);

            #[cfg(not(target_arch = "wasm32"))]
            crate::audio::draw_audio_waveform(app, &painter, rect, cell_width, 0.0);

            let playhead_x = rect.min.x + app.current_frame as f32 * cell_width + cell_width / 2.0;
            painter.line_segment(
                [
                    egui::pos2(playhead_x, header_y),
                    egui::pos2(playhead_x, rect.max.y),
                ],
                egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 50, 50)),
            );

            let playhead_handle = egui::Rect::from_center_size(
                egui::pos2(playhead_x, header_y + HEADER_HEIGHT / 2.0),
                egui::vec2(10.0, HEADER_HEIGHT),
            );
            painter.rect_filled(playhead_handle, 2.0, egui::Color32::from_rgb(255, 50, 50));

            handle_grid_interaction(app, &response, ui, rect, cell_width, header_y, grid_y);

            draw_context_menu(app, &response, rect, cell_width, grid_y);
        });
}

fn tween_color(tween: TweenType) -> egui::Color32 {
    match tween {
        TweenType::None => egui::Color32::from_rgb(100, 100, 100),
        TweenType::Linear => egui::Color32::from_rgb(100, 180, 255),
        TweenType::EaseIn => egui::Color32::from_rgb(100, 255, 100),
        TweenType::EaseOut => egui::Color32::from_rgb(255, 200, 100),
        TweenType::EaseInOut => egui::Color32::from_rgb(255, 100, 255),
        TweenType::CubicBezier { .. } => egui::Color32::from_rgb(255, 180, 50),
    }
}

fn draw_loop_region(
    app: &AnimateApp,
    painter: &egui::Painter,
    rect: egui::Rect,
    cell_width: f32,
    header_y: f32,
) {
    if let (Some(loop_start), Some(loop_end)) = (app.project.loop_start, app.project.loop_end) {
        let start_x = rect.min.x + loop_start as f32 * cell_width;
        let end_x = rect.min.x + (loop_end + 1) as f32 * cell_width;
        let loop_rect = egui::Rect::from_min_max(
            egui::pos2(start_x, header_y),
            egui::pos2(end_x, header_y + HEADER_HEIGHT),
        );
        painter.rect_filled(
            loop_rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(80, 200, 80, 40),
        );

        let handle_size = egui::vec2(6.0, HEADER_HEIGHT);
        let start_handle = egui::Rect::from_center_size(
            egui::pos2(start_x, header_y + HEADER_HEIGHT / 2.0),
            handle_size,
        );
        let end_handle = egui::Rect::from_center_size(
            egui::pos2(end_x, header_y + HEADER_HEIGHT / 2.0),
            handle_size,
        );
        painter.rect_filled(start_handle, 1.0, egui::Color32::from_rgb(80, 200, 80));
        painter.rect_filled(end_handle, 1.0, egui::Color32::from_rgb(80, 200, 80));
    }
}

fn draw_frame_labels(
    app: &AnimateApp,
    painter: &egui::Painter,
    rect: egui::Rect,
    cell_width: f32,
    label_y: f32,
) {
    for layer in &app.project.layers {
        for (&frame, keyframe) in &layer.keyframes {
            if !keyframe.label.is_empty() {
                let x = rect.min.x + frame as f32 * cell_width + 2.0;
                painter.text(
                    egui::pos2(x, label_y + 1.0),
                    egui::Align2::LEFT_TOP,
                    &keyframe.label,
                    egui::FontId::proportional(9.0),
                    egui::Color32::from_rgb(255, 220, 100),
                );
            }
        }
    }
}

fn draw_selected_frame_highlight(
    app: &AnimateApp,
    painter: &egui::Painter,
    rect: egui::Rect,
    cell_width: f32,
    layer_index: usize,
    y: f32,
) {
    for &(selected_layer, selected_frame) in &app.timeline_selection.selected_frames {
        if selected_layer == layer_index {
            let x = rect.min.x + selected_frame as f32 * cell_width;
            let highlight_rect = egui::Rect::from_min_size(
                egui::pos2(x, y),
                egui::vec2(cell_width, FRAME_CELL_HEIGHT),
            );
            painter.rect_filled(
                highlight_rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(100, 150, 255, 40),
            );
        }
    }
}

fn draw_keyframe_drag_ghost(
    app: &AnimateApp,
    painter: &egui::Painter,
    rect: egui::Rect,
    cell_width: f32,
    grid_y: f32,
) {
    if let Some(ref drag) = app.timeline_drag
        && let Some(ref keyframe_drag) = drag.dragging_keyframe
    {
        let x =
            rect.min.x + keyframe_drag.current_hover_frame as f32 * cell_width + cell_width / 2.0;
        let y =
            grid_y + keyframe_drag.layer_index as f32 * FRAME_CELL_HEIGHT + FRAME_CELL_HEIGHT / 2.0;
        painter.circle_filled(
            egui::pos2(x, y),
            4.0,
            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 120),
        );
    }
}

fn layer_index_from_y(app: &AnimateApp, relative_y: f32) -> Option<usize> {
    let offsets = compute_layer_y_offsets(app);
    let mut result = None;
    for (layer_index, &offset) in offsets.iter().enumerate() {
        if relative_y >= offset && relative_y < offset + FRAME_CELL_HEIGHT {
            result = Some(layer_index);
            break;
        }
    }
    result
}

fn handle_grid_interaction(
    app: &mut AnimateApp,
    response: &egui::Response,
    ui: &egui::Ui,
    rect: egui::Rect,
    cell_width: f32,
    header_y: f32,
    grid_y: f32,
) {
    let modifiers = ui.ctx().input(|input| input.modifiers);

    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(pos) = response.interact_pointer_pos()
    {
        let relative_x = pos.x - rect.min.x;
        let frame = (relative_x / cell_width).floor() as i32;
        let frame = frame.clamp(0, app.project.total_frames as i32 - 1) as u32;

        if pos.y >= grid_y {
            let relative_y = pos.y - grid_y;
            let layer_index_opt = layer_index_from_y(app, relative_y);
            let layer_index = layer_index_opt.unwrap_or(0);

            if layer_index < app.project.layers.len()
                && app.project.layers[layer_index]
                    .keyframes
                    .contains_key(&frame)
            {
                app.history.push(app.project.clone());
                app.timeline_drag = Some(crate::app::TimelineDragState {
                    dragging_layer: None,
                    drop_target: None,
                    dragging_keyframe: Some(KeyframeDragState {
                        layer_index,
                        source_frame: frame,
                        current_hover_frame: frame,
                    }),
                });
                app.active_layer = layer_index;
                app.current_frame = frame;
                return;
            }
        }

        if pos.y < grid_y {
            app.current_frame = frame;
        }
    }

    if response.dragged_by(egui::PointerButton::Primary)
        && let Some(pos) = ui.ctx().input(|input| input.pointer.latest_pos())
    {
        let relative_x = pos.x - rect.min.x;
        let frame = (relative_x / cell_width).floor() as i32;
        let frame = frame.clamp(0, app.project.total_frames as i32 - 1) as u32;

        if let Some(ref mut drag) = app.timeline_drag
            && let Some(ref mut keyframe_drag) = drag.dragging_keyframe
        {
            keyframe_drag.current_hover_frame = frame;
            return;
        }

        app.current_frame = frame;
    }

    if response.drag_stopped() {
        if let Some(ref drag) = app.timeline_drag
            && let Some(ref keyframe_drag) = drag.dragging_keyframe
        {
            let source = keyframe_drag.source_frame;
            let target = keyframe_drag.current_hover_frame;
            let layer_index = keyframe_drag.layer_index;

            if source != target && layer_index < app.project.layers.len() {
                let layer = &mut app.project.layers[layer_index];
                if let Some(keyframe_data) = layer.keyframes.remove(&source) {
                    layer.keyframes.insert(target, keyframe_data);
                    app.current_frame = target;
                }
            }
        }
        app.timeline_drag = None;
    }

    if response.clicked_by(egui::PointerButton::Primary)
        && app.timeline_drag.is_none()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let relative_x = pos.x - rect.min.x;
        let frame = (relative_x / cell_width).floor() as i32;
        let frame = frame.clamp(0, app.project.total_frames as i32 - 1) as u32;

        if pos.y >= grid_y {
            let relative_y = pos.y - grid_y;
            let layer_index_opt = layer_index_from_y(app, relative_y);

            if let Some(layer_index) = layer_index_opt {
                app.active_layer = layer_index;

                if modifiers.shift {
                    if let Some((range_layer, range_start)) = app.timeline_selection.range_start
                        && range_layer == layer_index
                    {
                        let min_frame = range_start.min(frame);
                        let max_frame = range_start.max(frame);
                        for range_frame in min_frame..=max_frame {
                            let entry = (layer_index, range_frame);
                            if !app.timeline_selection.selected_frames.contains(&entry) {
                                app.timeline_selection.selected_frames.push(entry);
                            }
                        }
                    }
                } else if modifiers.ctrl {
                    let entry = (layer_index, frame);
                    if let Some(position) = app
                        .timeline_selection
                        .selected_frames
                        .iter()
                        .position(|selected| *selected == entry)
                    {
                        app.timeline_selection.selected_frames.remove(position);
                    } else {
                        app.timeline_selection.selected_frames.push(entry);
                    }
                    app.timeline_selection.range_start = Some((layer_index, frame));
                } else {
                    app.timeline_selection.selected_frames.clear();
                    app.timeline_selection
                        .selected_frames
                        .push((layer_index, frame));
                    app.timeline_selection.range_start = Some((layer_index, frame));
                }
            }

            app.current_frame = frame;
        } else if pos.y >= header_y {
            app.current_frame = frame;
        }
    }
}

fn draw_context_menu(
    app: &mut AnimateApp,
    response: &egui::Response,
    rect: egui::Rect,
    cell_width: f32,
    grid_y: f32,
) {
    response.context_menu(|ui| {
        if let Some(pos) = ui.ctx().input(|input| input.pointer.latest_pos()) {
            let relative_x = pos.x - rect.min.x;
            let frame = (relative_x / cell_width).floor() as i32;
            let frame = frame.clamp(0, app.project.total_frames as i32 - 1) as u32;

            let relative_y = pos.y - grid_y;
            let layer_index = if relative_y >= 0.0 {
                layer_index_from_y(app, relative_y).unwrap_or(0)
            } else {
                0
            };

            if layer_index < app.project.layers.len() {
                let current_tween = app.project.layers[layer_index]
                    .keyframes
                    .get(&frame)
                    .map(|keyframe| keyframe.tween);
                let is_visible = app.project.layers[layer_index].visible;
                let is_locked = app.project.layers[layer_index].locked;

                if let Some(current_tween) = current_tween {
                    ui.label("Tween Type:");
                    for (tween, name) in [
                        (TweenType::None, "None"),
                        (TweenType::Linear, "Linear"),
                        (TweenType::EaseIn, "Ease In"),
                        (TweenType::EaseOut, "Ease Out"),
                        (TweenType::EaseInOut, "Ease In-Out"),
                    ] {
                        let label = if current_tween == tween {
                            format!("* {}", name)
                        } else {
                            name.to_string()
                        };
                        if ui.button(label).clicked() {
                            app.history.push(app.project.clone());
                            if let Some(keyframe) =
                                app.project.layers[layer_index].keyframes.get_mut(&frame)
                            {
                                keyframe.tween = tween;
                            }
                            ui.close();
                        }
                    }

                    let is_custom = matches!(current_tween, TweenType::CubicBezier { .. });
                    let custom_label = if is_custom {
                        "* Custom Easing..."
                    } else {
                        "Custom Easing..."
                    };
                    if ui.button(custom_label).clicked() {
                        let (x1, y1, x2, y2) = match current_tween {
                            TweenType::CubicBezier { x1, y1, x2, y2 } => (x1, y1, x2, y2),
                            _ => (0.25, 0.1, 0.25, 1.0),
                        };
                        app.easing_editor = Some(crate::app::EasingEditorState {
                            layer_index,
                            frame,
                            x1,
                            y1,
                            x2,
                            y2,
                        });
                        ui.close();
                    }

                    ui.separator();

                    let current_shape_tween = app.project.layers[layer_index]
                        .keyframes
                        .get(&frame)
                        .map(|keyframe| keyframe.shape_tween)
                        .unwrap_or(false);
                    let mut shape_tween = current_shape_tween;
                    if ui.checkbox(&mut shape_tween, "Shape Tween").changed() {
                        app.history.push(app.project.clone());
                        if let Some(keyframe) =
                            app.project.layers[layer_index].keyframes.get_mut(&frame)
                        {
                            keyframe.shape_tween = shape_tween;
                        }
                    }

                    ui.separator();

                    let current_label = app.project.layers[layer_index]
                        .keyframes
                        .get(&frame)
                        .map(|keyframe| keyframe.label.clone())
                        .unwrap_or_default();
                    let mut label_text = current_label;
                    ui.horizontal(|ui| {
                        ui.label("Label:");
                        if ui.text_edit_singleline(&mut label_text).changed()
                            && let Some(keyframe) =
                                app.project.layers[layer_index].keyframes.get_mut(&frame)
                        {
                            keyframe.label = label_text.clone();
                        }
                    });

                    let current_comment = app.project.layers[layer_index]
                        .keyframes
                        .get(&frame)
                        .map(|keyframe| keyframe.comment.clone())
                        .unwrap_or_default();
                    let mut comment_text = current_comment;
                    ui.horizontal(|ui| {
                        ui.label("Comment:");
                        if ui.text_edit_singleline(&mut comment_text).changed()
                            && let Some(keyframe) =
                                app.project.layers[layer_index].keyframes.get_mut(&frame)
                        {
                            keyframe.comment = comment_text.clone();
                        }
                    });
                }

                ui.separator();

                if app.project.layers[layer_index]
                    .keyframes
                    .contains_key(&frame)
                    && app.project.layers[layer_index].keyframes.len() > 1
                    && ui.button("Delete Keyframe").clicked()
                {
                    app.history.push(app.project.clone());
                    app.project.layers[layer_index].keyframes.remove(&frame);
                    ui.close();
                }

                ui.separator();

                let has_loop = app.project.loop_start.is_some() && app.project.loop_end.is_some();
                if ui.button("Set Loop Start Here").clicked() {
                    app.project.loop_start = Some(frame);
                    if app.project.loop_end.is_none()
                        || app.project.loop_end.is_some_and(|end| end < frame)
                    {
                        app.project.loop_end = Some(app.project.total_frames - 1);
                    }
                    ui.close();
                }
                if ui.button("Set Loop End Here").clicked() {
                    app.project.loop_end = Some(frame);
                    if app.project.loop_start.is_none()
                        || app.project.loop_start.is_some_and(|start| start > frame)
                    {
                        app.project.loop_start = Some(0);
                    }
                    ui.close();
                }
                if has_loop && ui.button("Clear Loop Region").clicked() {
                    app.project.loop_start = None;
                    app.project.loop_end = None;
                    ui.close();
                }

                ui.separator();

                let mut toggle_visible = is_visible;
                if ui.checkbox(&mut toggle_visible, "Visible").changed() {
                    app.history.push(app.project.clone());
                    app.project.layers[layer_index].visible = toggle_visible;
                }

                let mut toggle_locked = is_locked;
                if ui.checkbox(&mut toggle_locked, "Locked").changed() {
                    app.history.push(app.project.clone());
                    app.project.layers[layer_index].locked = toggle_locked;
                }
            }
        }
    });
}

pub fn handle_timeline_shortcuts(app: &mut AnimateApp, ui_context: &egui::Context) {
    if ui_context.wants_keyboard_input() {
        return;
    }
    ui_context.input(|input| {
        if !input.modifiers.shift && input.key_pressed(egui::Key::F6) {
            insert_keyframe(app);
        }
        if input.modifiers.shift && input.key_pressed(egui::Key::F6) {
            delete_keyframe(app);
        }
        if input.key_pressed(egui::Key::F7) {
            insert_blank_keyframe(app);
        }
        if input.key_pressed(egui::Key::ArrowLeft) && app.current_frame > 0 {
            app.current_frame -= 1;
        }
        if input.key_pressed(egui::Key::ArrowRight)
            && app.current_frame < app.project.total_frames - 1
        {
            app.current_frame += 1;
        }
        if input.key_pressed(egui::Key::Delete)
            && !app.timeline_selection.selected_frames.is_empty()
        {
            delete_selected_frames(app);
        }
    });
}

fn delete_selected_frames(app: &mut AnimateApp) {
    app.history.push(app.project.clone());
    let selections = app.timeline_selection.selected_frames.clone();
    for (layer_index, frame) in selections {
        if layer_index < app.project.layers.len()
            && app.project.layers[layer_index].keyframes.len() > 1
        {
            app.project.layers[layer_index].keyframes.remove(&frame);
        }
    }
    app.timeline_selection.selected_frames.clear();
}

fn layer_depth(layers: &[Layer], index: usize) -> usize {
    let mut depth = 0;
    let mut current_parent = layers[index].parent_id;
    while let Some(parent_id) = current_parent {
        depth += 1;
        current_parent = layers
            .iter()
            .find(|layer| layer.id == parent_id)
            .and_then(|layer| layer.parent_id);
    }
    depth
}

fn is_parent_collapsed(layers: &[Layer], index: usize) -> bool {
    let mut current_parent = layers[index].parent_id;
    while let Some(parent_id) = current_parent {
        if let Some(parent) = layers.iter().find(|layer| layer.id == parent_id) {
            if parent.collapsed {
                return true;
            }
            current_parent = parent.parent_id;
        } else {
            break;
        }
    }
    false
}

fn compute_layer_y_offsets(app: &AnimateApp) -> Vec<f32> {
    let mut offsets = Vec::new();
    let mut y = 0.0;
    for layer in &app.project.layers {
        offsets.push(y);
        y += FRAME_CELL_HEIGHT;
        if app.expanded_property_layers.contains(&layer.id) {
            let sub_track_count = count_property_sub_tracks(layer);
            y += sub_track_count as f32 * PROPERTY_TRACK_HEIGHT;
        }
    }
    offsets
}

fn total_layers_height(app: &AnimateApp) -> f32 {
    let offsets = compute_layer_y_offsets(app);
    if let Some(&last_offset) = offsets.last() {
        let last_layer = &app.project.layers[offsets.len() - 1];
        let mut height = last_offset + FRAME_CELL_HEIGHT;
        if app.expanded_property_layers.contains(&last_layer.id) {
            height += count_property_sub_tracks(last_layer) as f32 * PROPERTY_TRACK_HEIGHT;
        }
        height
    } else {
        0.0
    }
}

fn count_property_sub_tracks(layer: &Layer) -> usize {
    let mut count = 0;
    for tracks in layer.property_tracks.values() {
        if !tracks.position.is_empty() {
            count += 1;
        }
        if !tracks.rotation.is_empty() {
            count += 1;
        }
        if !tracks.scale.is_empty() {
            count += 1;
        }
        if !tracks.fill.is_empty() {
            count += 1;
        }
        if !tracks.stroke.is_empty() {
            count += 1;
        }
        if !tracks.stroke_width.is_empty() {
            count += 1;
        }
    }
    count
}

fn painter_drop_indicator(ui: &egui::Ui, rect: egui::Rect) {
    let indicator_rect = egui::Rect::from_min_max(
        egui::pos2(rect.min.x, rect.max.y - 2.0),
        egui::pos2(rect.max.x, rect.max.y),
    );
    ui.painter()
        .rect_filled(indicator_rect, 0.0, egui::Color32::from_rgb(100, 200, 255));
}

pub fn insert_keyframe(app: &mut AnimateApp) {
    if app.active_layer >= app.project.layers.len() {
        return;
    }

    app.history.push(app.project.clone());

    let layer = &mut app.project.layers[app.active_layer];
    if !layer.keyframes.contains_key(&app.current_frame) {
        let prev_keyframe = layer
            .keyframes
            .range(..=app.current_frame)
            .next_back()
            .map(|(_, keyframe)| keyframe.clone())
            .unwrap_or_default();
        layer.keyframes.insert(app.current_frame, prev_keyframe);
    }
}

pub fn insert_blank_keyframe(app: &mut AnimateApp) {
    if app.active_layer >= app.project.layers.len() {
        return;
    }

    app.history.push(app.project.clone());

    let layer = &mut app.project.layers[app.active_layer];
    layer
        .keyframes
        .insert(app.current_frame, Keyframe::default());
}

pub fn delete_keyframe(app: &mut AnimateApp) {
    if app.active_layer >= app.project.layers.len() {
        return;
    }

    let layer = &app.project.layers[app.active_layer];
    if !layer.keyframes.contains_key(&app.current_frame) || layer.keyframes.len() <= 1 {
        return;
    }

    app.history.push(app.project.clone());
    app.project.layers[app.active_layer]
        .keyframes
        .remove(&app.current_frame);
}
