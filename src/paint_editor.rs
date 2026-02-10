use nightshade::prelude::*;

use crate::paint::{GradientStop, Paint};

pub fn paint_editor_ui(ui: &mut egui::Ui, id_source: &str, paint: &mut Paint) -> bool {
    let mut changed = false;

    let mode_index = match paint {
        Paint::Solid(_) => 0,
        Paint::LinearGradient { .. } => 1,
        Paint::RadialGradient { .. } => 2,
    };

    let mode_label = match mode_index {
        0 => "Solid",
        1 => "Linear",
        _ => "Radial",
    };

    let mut new_mode = mode_index;
    egui::ComboBox::from_id_salt(format!("{}_mode", id_source))
        .selected_text(mode_label)
        .width(60.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut new_mode, 0, "Solid");
            ui.selectable_value(&mut new_mode, 1, "Linear");
            ui.selectable_value(&mut new_mode, 2, "Radial");
        });

    if new_mode != mode_index {
        changed = true;
        let base_color = paint.as_solid();
        match new_mode {
            1 => {
                *paint = Paint::LinearGradient {
                    start: [0.0, 0.5],
                    end: [1.0, 0.5],
                    stops: vec![
                        GradientStop {
                            offset: 0.0,
                            color: base_color,
                        },
                        GradientStop {
                            offset: 1.0,
                            color: [base_color[0], base_color[1], base_color[2], 0.0],
                        },
                    ],
                };
            }
            2 => {
                *paint = Paint::RadialGradient {
                    center: [0.5, 0.5],
                    radius: 0.5,
                    stops: vec![
                        GradientStop {
                            offset: 0.0,
                            color: base_color,
                        },
                        GradientStop {
                            offset: 1.0,
                            color: [base_color[0], base_color[1], base_color[2], 0.0],
                        },
                    ],
                };
            }
            _ => {
                let solid_color = paint.as_solid();
                *paint = Paint::Solid(solid_color);
            }
        }
    }

    match paint {
        Paint::Solid(color) => {
            if ui.color_edit_button_rgba_unmultiplied(color).changed() {
                changed = true;
            }
        }
        Paint::LinearGradient { start, end, stops } => {
            changed |= draw_gradient_bar(ui, id_source, stops);
            changed |= draw_gradient_stops_editor(ui, id_source, stops);

            egui::Grid::new(format!("{}_linear_params", id_source))
                .num_columns(2)
                .spacing([4.0, 2.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Start").size(9.0));
                    ui.horizontal(|ui| {
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut start[0])
                                    .speed(0.01)
                                    .range(0.0..=1.0)
                                    .prefix("x:"),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut start[1])
                                    .speed(0.01)
                                    .range(0.0..=1.0)
                                    .prefix("y:"),
                            )
                            .changed();
                    });
                    ui.end_row();

                    ui.label(egui::RichText::new("End").size(9.0));
                    ui.horizontal(|ui| {
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut end[0])
                                    .speed(0.01)
                                    .range(0.0..=1.0)
                                    .prefix("x:"),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut end[1])
                                    .speed(0.01)
                                    .range(0.0..=1.0)
                                    .prefix("y:"),
                            )
                            .changed();
                    });
                    ui.end_row();
                });
        }
        Paint::RadialGradient {
            center,
            radius,
            stops,
        } => {
            changed |= draw_gradient_bar(ui, id_source, stops);
            changed |= draw_gradient_stops_editor(ui, id_source, stops);

            egui::Grid::new(format!("{}_radial_params", id_source))
                .num_columns(2)
                .spacing([4.0, 2.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Center").size(9.0));
                    ui.horizontal(|ui| {
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut center[0])
                                    .speed(0.01)
                                    .range(0.0..=1.0)
                                    .prefix("x:"),
                            )
                            .changed();
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut center[1])
                                    .speed(0.01)
                                    .range(0.0..=1.0)
                                    .prefix("y:"),
                            )
                            .changed();
                    });
                    ui.end_row();

                    ui.label(egui::RichText::new("Radius").size(9.0));
                    changed |= ui
                        .add(egui::DragValue::new(radius).speed(0.01).range(0.01..=2.0))
                        .changed();
                    ui.end_row();
                });
        }
    }

    changed
}

fn draw_gradient_bar(ui: &mut egui::Ui, id_source: &str, stops: &[GradientStop]) -> bool {
    let bar_width = ui.available_width().min(200.0);
    let bar_height = 16.0;
    let (rect, _response) =
        ui.allocate_exact_size(egui::vec2(bar_width, bar_height), egui::Sense::hover());

    let painter = ui.painter_at(rect);

    let sample_count = (bar_width as usize).max(2);
    for sample_index in 0..sample_count {
        let t = sample_index as f32 / (sample_count - 1) as f32;
        let color = sample_stops_at(stops, t);
        let x_start = rect.left() + t * bar_width;
        let x_end = rect.left() + (sample_index + 1) as f32 / sample_count as f32 * bar_width;
        let segment_rect = egui::Rect::from_min_max(
            egui::pos2(x_start, rect.top()),
            egui::pos2(x_end, rect.bottom()),
        );
        let egui_color = egui::Color32::from_rgba_unmultiplied(
            (color[0] * 255.0) as u8,
            (color[1] * 255.0) as u8,
            (color[2] * 255.0) as u8,
            (color[3] * 255.0) as u8,
        );
        painter.rect_filled(segment_rect, 0.0, egui_color);
    }

    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, egui::Color32::GRAY),
        egui::StrokeKind::Outside,
    );

    for stop in stops {
        let marker_x = rect.left() + stop.offset * bar_width;
        let marker_top = rect.top() - 2.0;
        let marker_bottom = rect.bottom() + 2.0;
        painter.line_segment(
            [
                egui::pos2(marker_x, marker_top),
                egui::pos2(marker_x, marker_bottom),
            ],
            egui::Stroke::new(2.0, egui::Color32::WHITE),
        );
    }

    let _ = id_source;
    false
}

fn draw_gradient_stops_editor(
    ui: &mut egui::Ui,
    id_source: &str,
    stops: &mut Vec<GradientStop>,
) -> bool {
    let mut changed = false;
    let mut remove_index: Option<usize> = None;
    let stop_count = stops.len();

    for (stop_index, stop) in stops.iter_mut().enumerate() {
        let stop_number = stop_index + 1;
        let can_remove = stop_count > 2;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("#{}", stop_number)).size(9.0));
            if ui
                .color_edit_button_rgba_unmultiplied(&mut stop.color)
                .changed()
            {
                changed = true;
            }
            if ui
                .add(
                    egui::DragValue::new(&mut stop.offset)
                        .speed(0.01)
                        .range(0.0..=1.0),
                )
                .changed()
            {
                changed = true;
            }
            if can_remove && ui.small_button("x").on_hover_text("Remove stop").clicked() {
                remove_index = Some(stop_index);
            }
        });
    }

    if let Some(index) = remove_index {
        stops.remove(index);
        changed = true;
    }

    if ui
        .small_button(format!("+ Add Stop##{}", id_source))
        .clicked()
    {
        let new_offset = if stops.len() >= 2 {
            let last = stops[stops.len() - 1].offset;
            let second_last = stops[stops.len() - 2].offset;
            ((last + second_last) / 2.0).clamp(0.0, 1.0)
        } else {
            0.5
        };
        let new_color = sample_stops_at(stops, new_offset);
        stops.push(GradientStop {
            offset: new_offset,
            color: new_color,
        });
        stops.sort_by(|a, b| a.offset.partial_cmp(&b.offset).unwrap());
        changed = true;
    }

    changed
}

fn sample_stops_at(stops: &[GradientStop], position: f32) -> [f32; 4] {
    if stops.is_empty() {
        return [0.0, 0.0, 0.0, 1.0];
    }
    if stops.len() == 1 || position <= stops[0].offset {
        return stops[0].color;
    }
    if position >= stops[stops.len() - 1].offset {
        return stops[stops.len() - 1].color;
    }

    for index in 1..stops.len() {
        if position <= stops[index].offset {
            let prev = &stops[index - 1];
            let next = &stops[index];
            let range = next.offset - prev.offset;
            if range < f32::EPSILON {
                return next.color;
            }
            let t = (position - prev.offset) / range;
            return [
                prev.color[0] + (next.color[0] - prev.color[0]) * t,
                prev.color[1] + (next.color[1] - prev.color[1]) * t,
                prev.color[2] + (next.color[2] - prev.color[2]) * t,
                prev.color[3] + (next.color[3] - prev.color[3]) * t,
            ];
        }
    }
    stops.last().unwrap().color
}
