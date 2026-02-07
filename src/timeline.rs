use nightshade::prelude::*;

use crate::app::AnimateApp;
use crate::project::{Keyframe, Layer, TweenType};

const LAYER_PANEL_WIDTH: f32 = 180.0;
const FRAME_CELL_WIDTH: f32 = 16.0;
const FRAME_CELL_HEIGHT: f32 = 24.0;
const HEADER_HEIGHT: f32 = 20.0;

pub fn draw_timeline(app: &mut AnimateApp, ui_context: &egui::Context) {
    egui::TopBottomPanel::bottom("timeline")
        .resizable(true)
        .default_height(250.0)
        .min_height(100.0)
        .show(ui_context, |ui| {
            ui.horizontal(|ui| {
                draw_layer_panel(app, ui);
                ui.separator();
                draw_frame_grid(app, ui);
            });
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

        ui.add_space(HEADER_HEIGHT - ui.spacing().item_spacing.y);

        egui::ScrollArea::vertical()
            .id_salt("layer_scroll")
            .show(ui, |ui| {
                let mut layer_action = None;
                let mut visibility_toggle = None;
                let mut lock_toggle = None;

                for layer_index in 0..app.project.layers.len() {
                    let is_active = layer_index == app.active_layer;
                    let bg_color = if is_active {
                        egui::Color32::from_rgb(60, 80, 120)
                    } else {
                        egui::Color32::TRANSPARENT
                    };

                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(LAYER_PANEL_WIDTH - 10.0, FRAME_CELL_HEIGHT),
                        egui::Sense::click(),
                    );

                    let eye_rect = egui::Rect::from_min_size(
                        rect.min + egui::vec2(2.0, 2.0),
                        egui::vec2(20.0, 20.0),
                    );
                    let lock_rect = egui::Rect::from_min_size(
                        rect.min + egui::vec2(24.0, 2.0),
                        egui::vec2(20.0, 20.0),
                    );

                    if response.clicked()
                        && let Some(pos) = response.interact_pointer_pos()
                    {
                        if eye_rect.contains(pos) {
                            visibility_toggle = Some(layer_index);
                        } else if lock_rect.contains(pos) {
                            lock_toggle = Some(layer_index);
                        } else {
                            layer_action = Some(layer_index);
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

                    let name_pos = rect.min + egui::vec2(48.0, FRAME_CELL_HEIGHT / 2.0);
                    ui.painter().text(
                        name_pos,
                        egui::Align2::LEFT_CENTER,
                        &layer.name,
                        egui::FontId::proportional(12.0),
                        egui::Color32::WHITE,
                    );
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
            });
    });
}

fn draw_frame_grid(app: &mut AnimateApp, ui: &mut egui::Ui) {
    egui::ScrollArea::horizontal()
        .id_salt("frame_scroll")
        .show(ui, |ui| {
            let total_width = app.project.total_frames as f32 * FRAME_CELL_WIDTH;
            let total_height = HEADER_HEIGHT + app.project.layers.len() as f32 * FRAME_CELL_HEIGHT;

            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(total_width, total_height),
                egui::Sense::click_and_drag(),
            );

            let painter = ui.painter_at(rect);

            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(40, 40, 40));

            for frame in 0..app.project.total_frames {
                let x = rect.min.x + frame as f32 * FRAME_CELL_WIDTH;

                if frame % 5 == 0 {
                    painter.line_segment(
                        [
                            egui::pos2(x, rect.min.y),
                            egui::pos2(x, rect.min.y + HEADER_HEIGHT),
                        ],
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)),
                    );

                    if frame % 10 == 0 {
                        painter.text(
                            egui::pos2(x + 2.0, rect.min.y + 2.0),
                            egui::Align2::LEFT_TOP,
                            format!("{}", frame + 1),
                            egui::FontId::monospace(9.0),
                            egui::Color32::from_rgb(150, 150, 150),
                        );
                    }
                }

                painter.line_segment(
                    [
                        egui::pos2(x, rect.min.y + HEADER_HEIGHT),
                        egui::pos2(x, rect.max.y),
                    ],
                    egui::Stroke::new(0.5, egui::Color32::from_rgb(55, 55, 55)),
                );
            }

            for layer_index in 0..app.project.layers.len() {
                let y = rect.min.y + HEADER_HEIGHT + layer_index as f32 * FRAME_CELL_HEIGHT;

                painter.line_segment(
                    [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                    egui::Stroke::new(0.5, egui::Color32::from_rgb(55, 55, 55)),
                );

                let layer = &app.project.layers[layer_index];

                for (&frame, keyframe) in &layer.keyframes {
                    let x = rect.min.x + frame as f32 * FRAME_CELL_WIDTH + FRAME_CELL_WIDTH / 2.0;
                    let center_y = y + FRAME_CELL_HEIGHT / 2.0;

                    let color = match keyframe.tween {
                        TweenType::None => egui::Color32::from_rgb(100, 100, 100),
                        TweenType::Linear => egui::Color32::from_rgb(100, 180, 255),
                        TweenType::EaseIn => egui::Color32::from_rgb(100, 255, 100),
                        TweenType::EaseOut => egui::Color32::from_rgb(255, 200, 100),
                        TweenType::EaseInOut => egui::Color32::from_rgb(255, 100, 255),
                    };

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
                        let start_x = rect.min.x + (frame + 1) as f32 * FRAME_CELL_WIDTH;
                        let end_x = rect.min.x + next_frame as f32 * FRAME_CELL_WIDTH;
                        let tween_rect = egui::Rect::from_min_max(
                            egui::pos2(start_x, y + 1.0),
                            egui::pos2(end_x, y + FRAME_CELL_HEIGHT - 1.0),
                        );
                        let tween_color = color.gamma_multiply(0.2);
                        painter.rect_filled(tween_rect, 0.0, tween_color);
                    }
                }
            }

            let playhead_x =
                rect.min.x + app.current_frame as f32 * FRAME_CELL_WIDTH + FRAME_CELL_WIDTH / 2.0;
            painter.line_segment(
                [
                    egui::pos2(playhead_x, rect.min.y),
                    egui::pos2(playhead_x, rect.max.y),
                ],
                egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 50, 50)),
            );

            let playhead_handle = egui::Rect::from_center_size(
                egui::pos2(playhead_x, rect.min.y + HEADER_HEIGHT / 2.0),
                egui::vec2(10.0, HEADER_HEIGHT),
            );
            painter.rect_filled(playhead_handle, 2.0, egui::Color32::from_rgb(255, 50, 50));

            if (response.clicked() || response.dragged())
                && let Some(pos) = response.interact_pointer_pos()
            {
                let relative_x = pos.x - rect.min.x;
                let frame = (relative_x / FRAME_CELL_WIDTH).floor() as i32;
                let frame = frame.clamp(0, app.project.total_frames as i32 - 1) as u32;

                let relative_y = pos.y - rect.min.y - HEADER_HEIGHT;
                if relative_y >= 0.0 {
                    let layer_index = (relative_y / FRAME_CELL_HEIGHT) as usize;
                    if layer_index < app.project.layers.len() {
                        app.active_layer = layer_index;
                    }
                }

                app.current_frame = frame;
            }

            response.context_menu(|ui| {
                if let Some(pos) = ui.ctx().input(|input| input.pointer.latest_pos()) {
                    let relative_y = pos.y - rect.min.y - HEADER_HEIGHT;
                    let layer_index = if relative_y >= 0.0 {
                        (relative_y / FRAME_CELL_HEIGHT) as usize
                    } else {
                        0
                    };

                    if layer_index < app.project.layers.len() {
                        let current_tween = app.project.layers[layer_index]
                            .keyframes
                            .get(&app.current_frame)
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
                                    if let Some(keyframe) = app.project.layers[layer_index]
                                        .keyframes
                                        .get_mut(&app.current_frame)
                                    {
                                        keyframe.tween = tween;
                                    }
                                    ui.close();
                                }
                            }
                        }

                        if app.project.layers[layer_index]
                            .keyframes
                            .contains_key(&app.current_frame)
                            && app.project.layers[layer_index].keyframes.len() > 1
                            && ui.button("Delete Keyframe").clicked()
                        {
                            app.history.push(app.project.clone());
                            app.project.layers[layer_index]
                                .keyframes
                                .remove(&app.current_frame);
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
    });
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
