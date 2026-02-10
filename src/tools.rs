use nightshade::prelude::*;

use crate::app::AnimateApp;
use crate::canvas::CanvasView;
use crate::project::{AnimObject, FontFamily, PathPoint, Shape};
use crate::selection;
use crate::tween;

#[derive(Clone, Copy, PartialEq)]
pub enum Tool {
    Select,
    NodeEdit,
    Rectangle,
    Ellipse,
    Line,
    Pen,
    Pencil,
    Eraser,
    PaintBucket,
    Text,
    Brush,
    Bone,
}

#[derive(Clone)]
pub enum ToolState {
    Idle,
    Drawing {
        start: egui::Pos2,
        current: egui::Pos2,
    },
    PenDrawing {
        points: Vec<PathPoint>,
        current_pos: egui::Pos2,
        dragging_handle: bool,
        last_control_out: Option<[f32; 2]>,
    },
    PencilDrawing {
        points: Vec<[f32; 2]>,
    },
    Erasing {
        points: Vec<[f32; 2]>,
    },
    BrushDrawing {
        points: Vec<PathPoint>,
    },
}

impl Default for ToolState {
    fn default() -> Self {
        Self::Idle
    }
}

pub fn handle_drawing_tool(
    app: &mut AnimateApp,
    response: &egui::Response,
    ui_context: &egui::Context,
) {
    match app.tool {
        Tool::Rectangle | Tool::Ellipse | Tool::Line => {
            handle_shape_tool(app, response, ui_context);
        }
        Tool::Pen => {
            handle_pen_tool(app, response, ui_context);
        }
        Tool::Pencil => {
            handle_pencil_tool(app, response, ui_context);
        }
        Tool::Eraser => {
            handle_eraser_tool(app, response, ui_context);
        }
        Tool::PaintBucket => {
            handle_paint_bucket_tool(app, response);
        }
        Tool::NodeEdit => {
            crate::node_edit::handle_node_edit_tool(app, response, ui_context);
        }
        Tool::Text => {
            handle_text_tool(app, response);
        }
        Tool::Brush => {
            handle_brush_tool(app, response, ui_context);
        }
        Tool::Bone => {
            crate::armature::handle_bone_tool(app, response, ui_context);
        }
        Tool::Select => {}
    }
}

fn handle_shape_tool(app: &mut AnimateApp, response: &egui::Response, ui_context: &egui::Context) {
    let shift_held = ui_context.input(|input| input.modifiers.shift);

    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(pos) = response.interact_pointer_pos()
    {
        let canvas_pos = app.canvas_view.screen_to_canvas(pos);
        app.tool_state = ToolState::Drawing {
            start: canvas_pos,
            current: canvas_pos,
        };
    }

    if let ToolState::Drawing {
        start,
        ref mut current,
    } = app.tool_state
    {
        if let Some(pos) = ui_context.input(|input| input.pointer.latest_pos()) {
            *current = app.canvas_view.screen_to_canvas(pos);
        }

        if response.drag_stopped() {
            let mut end = *current;

            if shift_held {
                let delta_x = end.x - start.x;
                let delta_y = end.y - start.y;
                let max_delta = delta_x.abs().max(delta_y.abs());
                end.x = start.x + max_delta * delta_x.signum();
                end.y = start.y + max_delta * delta_y.signum();
            }

            let min_x = start.x.min(end.x);
            let min_y = start.y.min(end.y);
            let max_x = start.x.max(end.x);
            let max_y = start.y.max(end.y);
            let width = max_x - min_x;
            let height = max_y - min_y;

            if width > 1.0 || height > 1.0 {
                let center_x = (min_x + max_x) / 2.0;
                let center_y = (min_y + max_y) / 2.0;

                let shape = match app.tool {
                    Tool::Rectangle => Shape::Rectangle {
                        width,
                        height,
                        corner_radius: 0.0,
                    },
                    Tool::Ellipse => Shape::Ellipse {
                        radius_x: width / 2.0,
                        radius_y: height / 2.0,
                    },
                    Tool::Line => Shape::Line {
                        end_x: end.x - start.x,
                        end_y: end.y - start.y,
                    },
                    _ => unreachable!(),
                };

                let position = if matches!(app.tool, Tool::Line) {
                    [start.x, start.y]
                } else {
                    [center_x, center_y]
                };

                let object = AnimObject::new(
                    shape,
                    position,
                    app.fill_paint.clone(),
                    app.stroke_paint.clone(),
                    app.stroke_width,
                );

                insert_object_at_current_frame(app, object);
            }

            app.tool_state = ToolState::Idle;
        }
    }
}

fn handle_pen_tool(app: &mut AnimateApp, response: &egui::Response, ui_context: &egui::Context) {
    if response.double_clicked() {
        if let ToolState::PenDrawing { ref points, .. } = app.tool_state {
            if points.len() >= 2 {
                let min_x = points
                    .iter()
                    .map(|point| point.position[0])
                    .fold(f32::INFINITY, f32::min);
                let min_y = points
                    .iter()
                    .map(|point| point.position[1])
                    .fold(f32::INFINITY, f32::min);

                let offset_points: Vec<PathPoint> = points
                    .iter()
                    .map(|point| PathPoint {
                        position: [point.position[0] - min_x, point.position[1] - min_y],
                        control_in: point
                            .control_in
                            .map(|control| [control[0] - min_x, control[1] - min_y]),
                        control_out: point
                            .control_out
                            .map(|control| [control[0] - min_x, control[1] - min_y]),
                        pressure: point.pressure,
                    })
                    .collect();

                let shape = Shape::Path {
                    points: offset_points,
                    closed: false,
                };

                let object = AnimObject::new(
                    shape,
                    [min_x, min_y],
                    app.fill_paint.clone(),
                    app.stroke_paint.clone(),
                    app.stroke_width,
                );

                insert_object_at_current_frame(app, object);
            }
            app.tool_state = ToolState::Idle;
        }
        return;
    }

    match app.tool_state.clone() {
        ToolState::Idle => {
            if response.clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                let canvas_pos = app.canvas_view.screen_to_canvas(pos);
                let point = PathPoint {
                    position: [canvas_pos.x, canvas_pos.y],
                    control_in: None,
                    control_out: None,
                    pressure: 1.0,
                };
                app.tool_state = ToolState::PenDrawing {
                    points: vec![point],
                    current_pos: canvas_pos,
                    dragging_handle: false,
                    last_control_out: None,
                };
            }
        }
        ToolState::PenDrawing {
            mut points,
            current_pos: _,
            dragging_handle,
            last_control_out,
        } => {
            let current = if let Some(pos) = ui_context.input(|input| input.pointer.latest_pos()) {
                app.canvas_view.screen_to_canvas(pos)
            } else {
                egui::Pos2::ZERO
            };

            if response.clicked_by(egui::PointerButton::Primary)
                && let Some(pos) = response.interact_pointer_pos()
            {
                let canvas_pos = app.canvas_view.screen_to_canvas(pos);
                let point = PathPoint {
                    position: [canvas_pos.x, canvas_pos.y],
                    control_in: last_control_out.map(|control| {
                        let last_pos = points.last().unwrap().position;
                        [
                            2.0 * last_pos[0] - control[0],
                            2.0 * last_pos[1] - control[1],
                        ]
                    }),
                    control_out: None,
                    pressure: 1.0,
                };
                points.push(point);
                app.tool_state = ToolState::PenDrawing {
                    points,
                    current_pos: canvas_pos,
                    dragging_handle: false,
                    last_control_out: None,
                };
                return;
            }

            if response.drag_started_by(egui::PointerButton::Primary)
                && let Some(pos) = response.interact_pointer_pos()
            {
                let canvas_pos = app.canvas_view.screen_to_canvas(pos);
                let point = PathPoint {
                    position: [canvas_pos.x, canvas_pos.y],
                    control_in: last_control_out.map(|control| {
                        let last_pos = points.last().unwrap().position;
                        [
                            2.0 * last_pos[0] - control[0],
                            2.0 * last_pos[1] - control[1],
                        ]
                    }),
                    control_out: None,
                    pressure: 1.0,
                };
                points.push(point);
                app.tool_state = ToolState::PenDrawing {
                    points,
                    current_pos: canvas_pos,
                    dragging_handle: true,
                    last_control_out: None,
                };
                return;
            }

            if dragging_handle && response.dragged_by(egui::PointerButton::Primary) {
                if let Some(last) = points.last_mut() {
                    last.control_out = Some([current.x, current.y]);
                    let mirrored = [
                        2.0 * last.position[0] - current.x,
                        2.0 * last.position[1] - current.y,
                    ];
                    last.control_in = Some(mirrored);
                }
                app.tool_state = ToolState::PenDrawing {
                    points,
                    current_pos: current,
                    dragging_handle: true,
                    last_control_out: None,
                };
                return;
            }

            if dragging_handle && response.drag_stopped() {
                let control_out = points.last().and_then(|point| point.control_out);
                app.tool_state = ToolState::PenDrawing {
                    points,
                    current_pos: current,
                    dragging_handle: false,
                    last_control_out: control_out,
                };
                return;
            }

            app.tool_state = ToolState::PenDrawing {
                points,
                current_pos: current,
                dragging_handle,
                last_control_out,
            };
        }
        ToolState::Drawing { .. }
        | ToolState::PencilDrawing { .. }
        | ToolState::Erasing { .. }
        | ToolState::BrushDrawing { .. } => {}
    }
}

fn handle_pencil_tool(app: &mut AnimateApp, response: &egui::Response, ui_context: &egui::Context) {
    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(pos) = response.interact_pointer_pos()
    {
        let canvas_pos = app.canvas_view.screen_to_canvas(pos);
        app.tool_state = ToolState::PencilDrawing {
            points: vec![[canvas_pos.x, canvas_pos.y]],
        };
    }

    if let ToolState::PencilDrawing { ref mut points } = app.tool_state {
        if response.dragged_by(egui::PointerButton::Primary)
            && let Some(pos) = ui_context.input(|input| input.pointer.latest_pos())
        {
            let canvas_pos = app.canvas_view.screen_to_canvas(pos);
            let new_point = [canvas_pos.x, canvas_pos.y];
            if let Some(last) = points.last() {
                let dx = new_point[0] - last[0];
                let dy = new_point[1] - last[1];
                if dx * dx + dy * dy > 4.0 {
                    points.push(new_point);
                }
            }
        }

        if response.drag_stopped() {
            if points.len() >= 2 {
                let simplified = douglas_peucker(points, 1.5);

                let min_x = simplified
                    .iter()
                    .map(|point| point[0])
                    .fold(f32::INFINITY, f32::min);
                let min_y = simplified
                    .iter()
                    .map(|point| point[1])
                    .fold(f32::INFINITY, f32::min);

                let path_points: Vec<PathPoint> = simplified
                    .iter()
                    .map(|point| PathPoint {
                        position: [point[0] - min_x, point[1] - min_y],
                        control_in: None,
                        control_out: None,
                        pressure: 1.0,
                    })
                    .collect();

                let shape = Shape::Path {
                    points: path_points,
                    closed: false,
                };

                let object = AnimObject::new(
                    shape,
                    [min_x, min_y],
                    app.fill_paint.clone(),
                    app.stroke_paint.clone(),
                    app.stroke_width,
                );

                insert_object_at_current_frame(app, object);
            }

            app.tool_state = ToolState::Idle;
        }
    }
}

fn handle_eraser_tool(app: &mut AnimateApp, response: &egui::Response, ui_context: &egui::Context) {
    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(pos) = response.interact_pointer_pos()
    {
        let canvas_pos = app.canvas_view.screen_to_canvas(pos);
        app.tool_state = ToolState::Erasing {
            points: vec![[canvas_pos.x, canvas_pos.y]],
        };
        app.history.push(app.project.clone());
    }

    if let ToolState::Erasing { ref mut points } = app.tool_state {
        if let Some(pos) = ui_context.input(|input| input.pointer.latest_pos()) {
            let canvas_pos = app.canvas_view.screen_to_canvas(pos);
            let new_point = [canvas_pos.x, canvas_pos.y];
            if let Some(last) = points.last() {
                let dx = new_point[0] - last[0];
                let dy = new_point[1] - last[1];
                if dx * dx + dy * dy > 4.0 {
                    points.push(new_point);
                }
            }

            let mut objects_to_delete = Vec::new();
            for layer in &app.project.layers {
                if !layer.visible || layer.locked {
                    continue;
                }
                if let Some(objects) = tween::resolve_frame(layer, app.current_frame) {
                    for object in &objects {
                        if selection::point_in_object_public(
                            egui::pos2(canvas_pos.x, canvas_pos.y),
                            object,
                        ) {
                            objects_to_delete.push(object.id);
                        }
                    }
                }
            }

            if !objects_to_delete.is_empty() {
                for layer in &mut app.project.layers {
                    let has_match = tween::resolve_frame(layer, app.current_frame)
                        .map(|objects| {
                            objects
                                .iter()
                                .any(|object| objects_to_delete.contains(&object.id))
                        })
                        .unwrap_or(false);

                    if has_match {
                        tween::ensure_keyframe_at(layer, app.current_frame);
                    }

                    if let Some(keyframe) = layer.keyframes.get_mut(&app.current_frame) {
                        keyframe
                            .objects
                            .retain(|object| !objects_to_delete.contains(&object.id));
                    }
                }
            }
        }

        if response.drag_stopped() {
            app.tool_state = ToolState::Idle;
        }
    }
}

fn handle_paint_bucket_tool(app: &mut AnimateApp, response: &egui::Response) {
    if response.clicked_by(egui::PointerButton::Primary)
        && let Some(pos) = response.interact_pointer_pos()
    {
        let canvas_pos = app.canvas_view.screen_to_canvas(pos);
        let hit_id = selection::hit_test_public(app, canvas_pos);

        if let Some(object_id) = hit_id {
            app.history.push(app.project.clone());
            let fill = app.fill_paint.clone();
            for layer in &mut app.project.layers {
                let has_match = tween::resolve_frame(layer, app.current_frame)
                    .map(|objects| objects.iter().any(|object| object.id == object_id))
                    .unwrap_or(false);

                if has_match {
                    tween::ensure_keyframe_at(layer, app.current_frame);
                }

                if let Some(keyframe) = layer.keyframes.get_mut(&app.current_frame) {
                    for object in &mut keyframe.objects {
                        if object.id == object_id {
                            object.fill = fill.clone();
                        }
                    }
                }
            }
        }
    }
}

fn handle_text_tool(app: &mut AnimateApp, response: &egui::Response) {
    if response.clicked_by(egui::PointerButton::Primary)
        && let Some(pos) = response.interact_pointer_pos()
    {
        let canvas_pos = app.canvas_view.screen_to_canvas(pos);

        let shape = Shape::Text {
            content: "Text".to_string(),
            font_size: 24.0,
            font_family: FontFamily::SansSerif,
        };

        let object = AnimObject::new(
            shape,
            [canvas_pos.x, canvas_pos.y],
            app.fill_paint.clone(),
            app.stroke_paint.clone(),
            0.0,
        );

        let object_id = object.id;
        insert_object_at_current_frame(app, object);
        app.selection.selected_objects = vec![object_id];
    }
}

fn handle_brush_tool(app: &mut AnimateApp, response: &egui::Response, ui_context: &egui::Context) {
    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(pos) = response.interact_pointer_pos()
    {
        let canvas_pos = app.canvas_view.screen_to_canvas(pos);
        app.tool_state = ToolState::BrushDrawing {
            points: vec![PathPoint {
                position: [canvas_pos.x, canvas_pos.y],
                control_in: None,
                control_out: None,
                pressure: 1.0,
            }],
        };
    }

    if let ToolState::BrushDrawing { ref mut points } = app.tool_state {
        if response.dragged_by(egui::PointerButton::Primary)
            && let Some(pos) = ui_context.input(|input| input.pointer.latest_pos())
        {
            let canvas_pos = app.canvas_view.screen_to_canvas(pos);
            let new_position = [canvas_pos.x, canvas_pos.y];
            if let Some(last) = points.last() {
                let dx = new_position[0] - last.position[0];
                let dy = new_position[1] - last.position[1];
                let dist = (dx * dx + dy * dy).sqrt();
                if dist > 2.0 {
                    let pressure = (1.0 / (1.0 + dist * 0.05)).clamp(0.1, 1.0);
                    points.push(PathPoint {
                        position: new_position,
                        control_in: None,
                        control_out: None,
                        pressure,
                    });
                }
            }
        }

        if response.drag_stopped() {
            if points.len() >= 2 {
                let min_x = points
                    .iter()
                    .map(|point| point.position[0])
                    .fold(f32::INFINITY, f32::min);
                let min_y = points
                    .iter()
                    .map(|point| point.position[1])
                    .fold(f32::INFINITY, f32::min);

                let path_points: Vec<PathPoint> = points
                    .iter()
                    .map(|point| PathPoint {
                        position: [point.position[0] - min_x, point.position[1] - min_y],
                        control_in: None,
                        control_out: None,
                        pressure: point.pressure,
                    })
                    .collect();

                let shape = Shape::Path {
                    points: path_points,
                    closed: false,
                };

                let object = AnimObject::new(
                    shape,
                    [min_x, min_y],
                    app.fill_paint.clone(),
                    app.stroke_paint.clone(),
                    app.stroke_width,
                );

                insert_object_at_current_frame(app, object);
            }

            app.tool_state = ToolState::Idle;
        }
    }
}

fn douglas_peucker(points: &[[f32; 2]], epsilon: f32) -> Vec<[f32; 2]> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let first = points[0];
    let last = points[points.len() - 1];

    let line_dx = last[0] - first[0];
    let line_dy = last[1] - first[1];
    let line_length_squared = line_dx * line_dx + line_dy * line_dy;

    let mut max_distance = 0.0f32;
    let mut max_index = 0;

    for (index, point) in points.iter().enumerate().take(points.len() - 1).skip(1) {
        let distance = if line_length_squared < f32::EPSILON {
            let dx = point[0] - first[0];
            let dy = point[1] - first[1];
            (dx * dx + dy * dy).sqrt()
        } else {
            let cross = (point[0] - first[0]) * line_dy - (point[1] - first[1]) * line_dx;
            cross.abs() / line_length_squared.sqrt()
        };

        if distance > max_distance {
            max_distance = distance;
            max_index = index;
        }
    }

    if max_distance > epsilon {
        let mut left = douglas_peucker(&points[..=max_index], epsilon);
        let right = douglas_peucker(&points[max_index..], epsilon);
        left.pop();
        left.extend(right);
        left
    } else {
        vec![first, last]
    }
}

pub fn draw_tool_preview(app: &AnimateApp, view: &CanvasView, painter: &egui::Painter) {
    let fill_color = app.fill_paint.as_solid();
    let stroke_color = app.stroke_paint.as_solid();
    let fill = egui::Color32::from_rgba_unmultiplied(
        (fill_color[0] * 255.0) as u8,
        (fill_color[1] * 255.0) as u8,
        (fill_color[2] * 255.0) as u8,
        100,
    );
    let stroke = egui::Stroke::new(
        app.stroke_width * view.zoom,
        egui::Color32::from_rgba_unmultiplied(
            (stroke_color[0] * 255.0) as u8,
            (stroke_color[1] * 255.0) as u8,
            (stroke_color[2] * 255.0) as u8,
            180,
        ),
    );

    match &app.tool_state {
        ToolState::Drawing { start, current } => {
            let screen_start = view.canvas_to_screen(*start);
            let screen_current = view.canvas_to_screen(*current);

            match app.tool {
                Tool::Rectangle => {
                    let rect = egui::Rect::from_two_pos(screen_start, screen_current);
                    painter.rect(rect, 0.0, fill, stroke, egui::StrokeKind::Outside);
                }
                Tool::Ellipse => {
                    let center = egui::pos2(
                        (screen_start.x + screen_current.x) / 2.0,
                        (screen_start.y + screen_current.y) / 2.0,
                    );
                    let radius_x = (screen_current.x - screen_start.x).abs() / 2.0;
                    let radius_y = (screen_current.y - screen_start.y).abs() / 2.0;

                    let segments = 64;
                    let points: Vec<egui::Pos2> = (0..segments)
                        .map(|segment_index| {
                            let angle =
                                2.0 * std::f32::consts::PI * segment_index as f32 / segments as f32;
                            egui::pos2(
                                center.x + angle.cos() * radius_x,
                                center.y + angle.sin() * radius_y,
                            )
                        })
                        .collect();
                    let shape = egui::epaint::PathShape::convex_polygon(points, fill, stroke);
                    painter.add(shape);
                }
                Tool::Line => {
                    painter.line_segment([screen_start, screen_current], stroke);
                }
                _ => {}
            }
        }
        ToolState::PenDrawing {
            points,
            current_pos,
            ..
        } => {
            if points.is_empty() {
                return;
            }

            let preview_stroke = egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 150, 255));

            for point_index in 0..points.len() {
                let point = &points[point_index];
                let screen_pt =
                    view.canvas_to_screen(egui::pos2(point.position[0], point.position[1]));

                painter.circle_filled(screen_pt, 4.0, egui::Color32::from_rgb(100, 150, 255));

                if point_index > 0 {
                    let prev = &points[point_index - 1];
                    draw_bezier_segment(prev, point, view, painter, preview_stroke);
                }

                if let Some(control_out) = point.control_out {
                    let screen_control =
                        view.canvas_to_screen(egui::pos2(control_out[0], control_out[1]));
                    painter.line_segment(
                        [screen_pt, screen_control],
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(150, 150, 150)),
                    );
                    painter.circle_filled(screen_control, 3.0, egui::Color32::WHITE);
                }
                if let Some(control_in) = point.control_in {
                    let screen_control =
                        view.canvas_to_screen(egui::pos2(control_in[0], control_in[1]));
                    painter.line_segment(
                        [screen_pt, screen_control],
                        egui::Stroke::new(1.0, egui::Color32::from_rgb(150, 150, 150)),
                    );
                    painter.circle_filled(screen_control, 3.0, egui::Color32::WHITE);
                }
            }

            if let Some(last) = points.last() {
                let screen_last =
                    view.canvas_to_screen(egui::pos2(last.position[0], last.position[1]));
                let screen_current = view.canvas_to_screen(*current_pos);
                painter.line_segment(
                    [screen_last, screen_current],
                    egui::Stroke::new(
                        1.0,
                        egui::Color32::from_rgba_unmultiplied(100, 150, 255, 100),
                    ),
                );
            }
        }
        ToolState::PencilDrawing { points } => {
            if points.len() >= 2 {
                let pencil_stroke_color = app.stroke_paint.as_solid();
                let pencil_stroke = egui::Stroke::new(
                    app.stroke_width * view.zoom,
                    egui::Color32::from_rgba_unmultiplied(
                        (pencil_stroke_color[0] * 255.0) as u8,
                        (pencil_stroke_color[1] * 255.0) as u8,
                        (pencil_stroke_color[2] * 255.0) as u8,
                        128,
                    ),
                );

                for segment_index in 1..points.len() {
                    let from = view.canvas_to_screen(egui::pos2(
                        points[segment_index - 1][0],
                        points[segment_index - 1][1],
                    ));
                    let to = view.canvas_to_screen(egui::pos2(
                        points[segment_index][0],
                        points[segment_index][1],
                    ));
                    painter.line_segment([from, to], pencil_stroke);
                }
            }
        }
        ToolState::Erasing { points } => {
            if points.len() >= 2 {
                let eraser_stroke = egui::Stroke::new(3.0, egui::Color32::from_rgb(255, 80, 80));
                for segment_index in 1..points.len() {
                    let from = view.canvas_to_screen(egui::pos2(
                        points[segment_index - 1][0],
                        points[segment_index - 1][1],
                    ));
                    let to = view.canvas_to_screen(egui::pos2(
                        points[segment_index][0],
                        points[segment_index][1],
                    ));
                    painter.line_segment([from, to], eraser_stroke);
                }
            }
        }
        ToolState::BrushDrawing { points } => {
            if points.len() >= 2 {
                let stroke_c = app.stroke_paint.as_solid();
                let base_width = app.stroke_width * view.zoom;
                for index in 1..points.len() {
                    let prev = &points[index - 1];
                    let curr = &points[index];
                    let from =
                        view.canvas_to_screen(egui::pos2(prev.position[0], prev.position[1]));
                    let to = view.canvas_to_screen(egui::pos2(curr.position[0], curr.position[1]));
                    let avg_pressure = (prev.pressure + curr.pressure) / 2.0;
                    let width = base_width * avg_pressure;
                    painter.line_segment(
                        [from, to],
                        egui::Stroke::new(
                            width,
                            egui::Color32::from_rgba_unmultiplied(
                                (stroke_c[0] * 255.0) as u8,
                                (stroke_c[1] * 255.0) as u8,
                                (stroke_c[2] * 255.0) as u8,
                                128,
                            ),
                        ),
                    );
                }
            }
        }
        ToolState::Idle => {}
    }
}

fn draw_bezier_segment(
    from: &PathPoint,
    to: &PathPoint,
    view: &CanvasView,
    painter: &egui::Painter,
    stroke: egui::Stroke,
) {
    if from.control_out.is_none() && to.control_in.is_none() {
        let screen_from = view.canvas_to_screen(egui::pos2(from.position[0], from.position[1]));
        let screen_to = view.canvas_to_screen(egui::pos2(to.position[0], to.position[1]));
        painter.line_segment([screen_from, screen_to], stroke);
        return;
    }

    let control_out = from.control_out.unwrap_or(from.position);
    let control_in = to.control_in.unwrap_or(to.position);

    let mut screen_points = Vec::with_capacity(17);
    for step in 0..=16 {
        let t = step as f32 / 16.0;
        let one_minus_t = 1.0 - t;
        let position = [
            one_minus_t.powi(3) * from.position[0]
                + 3.0 * one_minus_t.powi(2) * t * control_out[0]
                + 3.0 * one_minus_t * t.powi(2) * control_in[0]
                + t.powi(3) * to.position[0],
            one_minus_t.powi(3) * from.position[1]
                + 3.0 * one_minus_t.powi(2) * t * control_out[1]
                + 3.0 * one_minus_t * t.powi(2) * control_in[1]
                + t.powi(3) * to.position[1],
        ];
        screen_points.push(view.canvas_to_screen(egui::pos2(position[0], position[1])));
    }

    for segment_index in 1..screen_points.len() {
        painter.line_segment(
            [
                screen_points[segment_index - 1],
                screen_points[segment_index],
            ],
            stroke,
        );
    }
}

fn insert_object_at_current_frame(app: &mut AnimateApp, object: AnimObject) {
    if app.active_layer >= app.project.layers.len() {
        return;
    }

    if app.project.layers[app.active_layer].locked {
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

    if let Some(keyframe) = layer.keyframes.get_mut(&app.current_frame) {
        keyframe.objects.push(object);
    }
}
