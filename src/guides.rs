use nightshade::prelude::*;

use crate::app::AnimateApp;
use crate::canvas::CanvasView;
use crate::project::{Guide, GuideOrientation};

const RULER_SIZE: f32 = 20.0;

pub fn draw_rulers_and_guides(app: &AnimateApp, view: &CanvasView, painter: &egui::Painter) {
    draw_guides(app, view, painter);
    draw_rulers(view, painter);
}

fn draw_guides(app: &AnimateApp, view: &CanvasView, painter: &egui::Painter) {
    let guide_color = egui::Color32::from_rgba_unmultiplied(0, 200, 255, 150);
    let guide_stroke = egui::Stroke::new(1.0, guide_color);

    for guide in &app.project.guides {
        match guide.orientation {
            GuideOrientation::Horizontal => {
                let left = view.canvas_to_screen(egui::pos2(0.0, guide.position));
                let right = view
                    .canvas_to_screen(egui::pos2(app.project.canvas_width as f32, guide.position));
                draw_dashed_line(painter, left, right, guide_stroke, 6.0, 4.0);
            }
            GuideOrientation::Vertical => {
                let top = view.canvas_to_screen(egui::pos2(guide.position, 0.0));
                let bottom = view
                    .canvas_to_screen(egui::pos2(guide.position, app.project.canvas_height as f32));
                draw_dashed_line(painter, top, bottom, guide_stroke, 6.0, 4.0);
            }
        }
    }
}

fn draw_rulers(view: &CanvasView, painter: &egui::Painter) {
    let panel_rect = view.panel_rect;
    let ruler_bg = egui::Color32::from_rgb(60, 60, 60);
    let ruler_text_color = egui::Color32::from_rgb(180, 180, 180);
    let tick_color = egui::Color32::from_rgb(100, 100, 100);

    let top_ruler = egui::Rect::from_min_max(
        egui::pos2(panel_rect.min.x + RULER_SIZE, panel_rect.min.y),
        egui::pos2(panel_rect.max.x, panel_rect.min.y + RULER_SIZE),
    );
    painter.rect_filled(top_ruler, 0.0, ruler_bg);

    let left_ruler = egui::Rect::from_min_max(
        egui::pos2(panel_rect.min.x, panel_rect.min.y + RULER_SIZE),
        egui::pos2(panel_rect.min.x + RULER_SIZE, panel_rect.max.y),
    );
    painter.rect_filled(left_ruler, 0.0, ruler_bg);

    let corner = egui::Rect::from_min_max(
        panel_rect.min,
        egui::pos2(panel_rect.min.x + RULER_SIZE, panel_rect.min.y + RULER_SIZE),
    );
    painter.rect_filled(corner, 0.0, ruler_bg);

    let tick_spacing = compute_tick_spacing(view.zoom);

    let canvas_left = view
        .screen_to_canvas(egui::pos2(panel_rect.min.x + RULER_SIZE, 0.0))
        .x;
    let canvas_right = view.screen_to_canvas(egui::pos2(panel_rect.max.x, 0.0)).x;
    let start_tick = (canvas_left / tick_spacing).floor() as i32;
    let end_tick = (canvas_right / tick_spacing).ceil() as i32;

    for tick in start_tick..=end_tick {
        let canvas_x = tick as f32 * tick_spacing;
        let screen_x = view.canvas_to_screen(egui::pos2(canvas_x, 0.0)).x;
        if screen_x < panel_rect.min.x + RULER_SIZE || screen_x > panel_rect.max.x {
            continue;
        }

        let is_major = (tick % 5) == 0;
        let tick_height = if is_major {
            RULER_SIZE * 0.6
        } else {
            RULER_SIZE * 0.3
        };
        painter.line_segment(
            [
                egui::pos2(screen_x, panel_rect.min.y + RULER_SIZE - tick_height),
                egui::pos2(screen_x, panel_rect.min.y + RULER_SIZE),
            ],
            egui::Stroke::new(0.5, tick_color),
        );

        if is_major {
            painter.text(
                egui::pos2(screen_x + 2.0, panel_rect.min.y + 2.0),
                egui::Align2::LEFT_TOP,
                format!("{}", canvas_x as i32),
                egui::FontId::monospace(8.0),
                ruler_text_color,
            );
        }
    }

    let canvas_top = view
        .screen_to_canvas(egui::pos2(0.0, panel_rect.min.y + RULER_SIZE))
        .y;
    let canvas_bottom = view.screen_to_canvas(egui::pos2(0.0, panel_rect.max.y)).y;
    let start_tick_y = (canvas_top / tick_spacing).floor() as i32;
    let end_tick_y = (canvas_bottom / tick_spacing).ceil() as i32;

    for tick in start_tick_y..=end_tick_y {
        let canvas_y = tick as f32 * tick_spacing;
        let screen_y = view.canvas_to_screen(egui::pos2(0.0, canvas_y)).y;
        if screen_y < panel_rect.min.y + RULER_SIZE || screen_y > panel_rect.max.y {
            continue;
        }

        let is_major = (tick % 5) == 0;
        let tick_width = if is_major {
            RULER_SIZE * 0.6
        } else {
            RULER_SIZE * 0.3
        };
        painter.line_segment(
            [
                egui::pos2(panel_rect.min.x + RULER_SIZE - tick_width, screen_y),
                egui::pos2(panel_rect.min.x + RULER_SIZE, screen_y),
            ],
            egui::Stroke::new(0.5, tick_color),
        );

        if is_major {
            painter.text(
                egui::pos2(panel_rect.min.x + 2.0, screen_y + 2.0),
                egui::Align2::LEFT_TOP,
                format!("{}", canvas_y as i32),
                egui::FontId::monospace(8.0),
                ruler_text_color,
            );
        }
    }
}

fn compute_tick_spacing(zoom: f32) -> f32 {
    let base_spacing = 10.0;
    let screen_spacing = base_spacing * zoom;

    if screen_spacing > 20.0 {
        base_spacing
    } else if screen_spacing > 4.0 {
        50.0
    } else {
        100.0
    }
}

pub fn handle_ruler_interaction(app: &mut AnimateApp, ui_context: &egui::Context) {
    let panel_rect = app.canvas_view.panel_rect;
    let pointer_pos = ui_context.input(|input| input.pointer.latest_pos());
    let primary_down = ui_context.input(|input| input.pointer.primary_down());
    let primary_released = ui_context.input(|input| input.pointer.primary_released());

    if let Some(pos) = pointer_pos {
        if primary_down {
            let in_top_ruler = pos.y >= panel_rect.min.y
                && pos.y <= panel_rect.min.y + RULER_SIZE
                && pos.x > panel_rect.min.x + RULER_SIZE;
            let in_left_ruler = pos.x >= panel_rect.min.x
                && pos.x <= panel_rect.min.x + RULER_SIZE
                && pos.y > panel_rect.min.y + RULER_SIZE;

            if in_top_ruler && app.selection.guide_dragging.is_none() {
                let canvas_pos = app.canvas_view.screen_to_canvas(pos);
                let guide = Guide {
                    id: uuid::Uuid::new_v4(),
                    orientation: GuideOrientation::Horizontal,
                    position: canvas_pos.y,
                };
                app.project.guides.push(guide);
                let guide_index = app.project.guides.len() - 1;
                app.selection.guide_dragging = Some(guide_index);
            }

            if in_left_ruler && app.selection.guide_dragging.is_none() {
                let canvas_pos = app.canvas_view.screen_to_canvas(pos);
                let guide = Guide {
                    id: uuid::Uuid::new_v4(),
                    orientation: GuideOrientation::Vertical,
                    position: canvas_pos.x,
                };
                app.project.guides.push(guide);
                let guide_index = app.project.guides.len() - 1;
                app.selection.guide_dragging = Some(guide_index);
            }
        }

        if let Some(guide_index) = app.selection.guide_dragging
            && guide_index < app.project.guides.len()
        {
            let canvas_pos = app.canvas_view.screen_to_canvas(pos);
            match app.project.guides[guide_index].orientation {
                GuideOrientation::Horizontal => {
                    app.project.guides[guide_index].position = canvas_pos.y;
                }
                GuideOrientation::Vertical => {
                    app.project.guides[guide_index].position = canvas_pos.x;
                }
            }
        }

        if primary_released {
            app.selection.guide_dragging = None;
        }
    }
}

fn draw_dashed_line(
    painter: &egui::Painter,
    start: egui::Pos2,
    end: egui::Pos2,
    stroke: egui::Stroke,
    dash_length: f32,
    gap_length: f32,
) {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let total_length = (dx * dx + dy * dy).sqrt();
    if total_length < 0.001 {
        return;
    }

    let dir_x = dx / total_length;
    let dir_y = dy / total_length;
    let pattern_length = dash_length + gap_length;
    let mut distance = 0.0;

    while distance < total_length {
        let dash_end = (distance + dash_length).min(total_length);
        let from = egui::pos2(start.x + dir_x * distance, start.y + dir_y * distance);
        let to = egui::pos2(start.x + dir_x * dash_end, start.y + dir_y * dash_end);
        painter.line_segment([from, to], stroke);
        distance += pattern_length;
    }
}
