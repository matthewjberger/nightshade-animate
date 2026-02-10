use nightshade::prelude::*;

use crate::app::AnimateApp;
use crate::project::TweenType;

const EDITOR_SIZE: f32 = 200.0;
const HANDLE_RADIUS: f32 = 6.0;
const MARGIN: f32 = 20.0;

pub fn draw_easing_editor(app: &mut AnimateApp, ui_context: &egui::Context) {
    let Some(ref mut editor) = app.easing_editor else {
        return;
    };

    let layer_index = editor.layer_index;
    let frame = editor.frame;
    let mut x1 = editor.x1;
    let mut y1 = editor.y1;
    let mut x2 = editor.x2;
    let mut y2 = editor.y2;

    let mut open = true;
    let mut applied = false;

    egui::Window::new("Custom Easing")
        .open(&mut open)
        .resizable(false)
        .default_width(EDITOR_SIZE + MARGIN * 2.0 + 80.0)
        .show(ui_context, |ui| {
            ui.horizontal(|ui| {
                let plot_size = egui::vec2(EDITOR_SIZE + MARGIN * 2.0, EDITOR_SIZE + MARGIN * 2.0);
                let (rect, response) =
                    ui.allocate_exact_size(plot_size, egui::Sense::click_and_drag());

                let painter = ui.painter_at(rect);

                let plot_rect = egui::Rect::from_min_size(
                    rect.min + egui::vec2(MARGIN, MARGIN),
                    egui::vec2(EDITOR_SIZE, EDITOR_SIZE),
                );

                painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(30, 30, 30));
                painter.rect_stroke(
                    plot_rect,
                    0.0,
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(80, 80, 80)),
                    egui::StrokeKind::Inside,
                );

                painter.line_segment(
                    [
                        egui::pos2(plot_rect.min.x, plot_rect.max.y),
                        egui::pos2(plot_rect.max.x, plot_rect.min.y),
                    ],
                    egui::Stroke::new(0.5, egui::Color32::from_rgb(60, 60, 60)),
                );

                let curve_points: Vec<egui::Pos2> = (0..=64)
                    .map(|step| {
                        let t = step as f32 / 64.0;
                        let output_y = cubic_bezier_value(t, y1, y2);
                        egui::pos2(
                            plot_rect.min.x + t * EDITOR_SIZE,
                            plot_rect.max.y - output_y * EDITOR_SIZE,
                        )
                    })
                    .collect();

                for window in curve_points.windows(2) {
                    painter.line_segment(
                        [window[0], window[1]],
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 180, 255)),
                    );
                }

                let p1_screen = egui::pos2(
                    plot_rect.min.x + x1 * EDITOR_SIZE,
                    plot_rect.max.y - y1 * EDITOR_SIZE,
                );
                let p2_screen = egui::pos2(
                    plot_rect.min.x + x2 * EDITOR_SIZE,
                    plot_rect.max.y - y2 * EDITOR_SIZE,
                );

                let origin = egui::pos2(plot_rect.min.x, plot_rect.max.y);
                let end_point = egui::pos2(plot_rect.max.x, plot_rect.min.y);

                painter.line_segment(
                    [origin, p1_screen],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(255, 100, 100)),
                );
                painter.line_segment(
                    [end_point, p2_screen],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 255, 100)),
                );

                painter.circle_filled(
                    p1_screen,
                    HANDLE_RADIUS,
                    egui::Color32::from_rgb(255, 100, 100),
                );
                painter.circle_filled(
                    p2_screen,
                    HANDLE_RADIUS,
                    egui::Color32::from_rgb(100, 255, 100),
                );

                if response.dragged()
                    && let Some(pos) = ui_context.input(|input| input.pointer.latest_pos())
                {
                    let norm_x = ((pos.x - plot_rect.min.x) / EDITOR_SIZE).clamp(0.0, 1.0);
                    let norm_y = ((plot_rect.max.y - pos.y) / EDITOR_SIZE).clamp(-0.5, 1.5);

                    let dist_p1 = (pos - p1_screen).length();
                    let dist_p2 = (pos - p2_screen).length();

                    if dist_p1 <= dist_p2 {
                        x1 = norm_x;
                        y1 = norm_y;
                    } else {
                        x2 = norm_x;
                        y2 = norm_y;
                    }
                }

                ui.vertical(|ui| {
                    ui.set_min_width(70.0);
                    egui::Grid::new("easing_params")
                        .num_columns(2)
                        .spacing([4.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("x1:");
                            ui.add(egui::DragValue::new(&mut x1).speed(0.01).range(0.0..=1.0));
                            ui.end_row();
                            ui.label("y1:");
                            ui.add(egui::DragValue::new(&mut y1).speed(0.01).range(-0.5..=1.5));
                            ui.end_row();
                            ui.label("x2:");
                            ui.add(egui::DragValue::new(&mut x2).speed(0.01).range(0.0..=1.0));
                            ui.end_row();
                            ui.label("y2:");
                            ui.add(egui::DragValue::new(&mut y2).speed(0.01).range(-0.5..=1.5));
                            ui.end_row();
                        });

                    ui.add_space(8.0);

                    ui.label("Presets:");
                    for (name, preset_x1, preset_y1, preset_x2, preset_y2) in [
                        ("Ease", 0.25, 0.1, 0.25, 1.0),
                        ("Ease In", 0.42, 0.0, 1.0, 1.0),
                        ("Ease Out", 0.0, 0.0, 0.58, 1.0),
                        ("Ease In-Out", 0.42, 0.0, 0.58, 1.0),
                        ("Bounce", 0.34, 1.56, 0.64, 1.0),
                        ("Back", 0.36, 0.0, 0.66, -0.56),
                    ] {
                        if ui.small_button(name).clicked() {
                            x1 = preset_x1;
                            y1 = preset_y1;
                            x2 = preset_x2;
                            y2 = preset_y2;
                        }
                    }

                    ui.add_space(8.0);

                    if ui.button("Apply").clicked() {
                        applied = true;
                    }
                });
            });
        });

    if let Some(ref mut editor) = app.easing_editor {
        editor.x1 = x1;
        editor.y1 = y1;
        editor.x2 = x2;
        editor.y2 = y2;
    }

    if applied {
        app.history.push(app.project.clone());
        if layer_index < app.project.layers.len()
            && let Some(keyframe) = app.project.layers[layer_index].keyframes.get_mut(&frame)
        {
            keyframe.tween = TweenType::CubicBezier { x1, y1, x2, y2 };
        }
        app.easing_editor = None;
    }

    if !open {
        app.easing_editor = None;
    }
}

fn cubic_bezier_value(t: f32, p1: f32, p2: f32) -> f32 {
    let omt = 1.0 - t;
    3.0 * omt * omt * t * p1 + 3.0 * omt * t * t * p2 + t * t * t
}
