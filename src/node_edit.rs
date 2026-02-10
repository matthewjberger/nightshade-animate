use nightshade::prelude::*;

use crate::app::AnimateApp;
use crate::canvas::CanvasView;
use crate::project::{PathPoint, Shape};
use crate::tween;

#[derive(Clone, Default)]
pub struct NodeEditState {
    pub object_id: Option<uuid::Uuid>,
    pub selected_nodes: Vec<usize>,
    pub dragging_node: Option<DragTarget>,
    pub drag_start: Option<egui::Pos2>,
}

#[derive(Clone)]
pub enum DragTarget {
    AnchorPoint(usize),
    ControlIn(usize),
    ControlOut(usize),
}

pub fn handle_node_edit_tool(
    app: &mut AnimateApp,
    response: &egui::Response,
    ui_context: &egui::Context,
) {
    if app.selection.selected_objects.len() != 1 {
        app.selection.node_edit = NodeEditState::default();
        return;
    }

    let object_id = app.selection.selected_objects[0];
    if app.selection.node_edit.object_id != Some(object_id) {
        app.selection.node_edit = NodeEditState {
            object_id: Some(object_id),
            selected_nodes: Vec::new(),
            dragging_node: None,
            drag_start: None,
        };
    }

    let view = app.canvas_view.clone();

    let (path_points, object_position, object_scale) = {
        let mut result = None;
        for layer in &app.project.layers {
            if let Some(objects) = tween::resolve_frame(layer, app.current_frame) {
                for object in &objects {
                    if object.id == object_id {
                        if let Shape::Path { ref points, .. } = object.shape {
                            result = Some((points.clone(), object.position, object.scale));
                        }
                        break;
                    }
                }
            }
            if result.is_some() {
                break;
            }
        }
        match result {
            Some(data) => data,
            None => return,
        }
    };

    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(pos) = response.interact_pointer_pos()
    {
        let canvas_pos = view.screen_to_canvas(pos);
        let local_x = canvas_pos.x - object_position[0];
        let local_y = canvas_pos.y - object_position[1];

        let mut hit_target = None;
        let threshold = 8.0 / view.zoom;

        for (index, point) in path_points.iter().enumerate() {
            if let Some(control_in) = point.control_in {
                let cx = control_in[0] * object_scale[0];
                let cy = control_in[1] * object_scale[1];
                if (local_x - cx).abs() < threshold && (local_y - cy).abs() < threshold {
                    hit_target = Some(DragTarget::ControlIn(index));
                    break;
                }
            }
            if let Some(control_out) = point.control_out {
                let cx = control_out[0] * object_scale[0];
                let cy = control_out[1] * object_scale[1];
                if (local_x - cx).abs() < threshold && (local_y - cy).abs() < threshold {
                    hit_target = Some(DragTarget::ControlOut(index));
                    break;
                }
            }
        }

        if hit_target.is_none() {
            for (index, point) in path_points.iter().enumerate() {
                let px = point.position[0] * object_scale[0];
                let py = point.position[1] * object_scale[1];
                if (local_x - px).abs() < threshold && (local_y - py).abs() < threshold {
                    hit_target = Some(DragTarget::AnchorPoint(index));
                    break;
                }
            }
        }

        if let Some(target) = hit_target {
            app.history.push(app.project.clone());
            ensure_keyframe_for_object(app, object_id);

            if let DragTarget::AnchorPoint(index) = &target {
                let ctrl = ui_context.input(|input| input.modifiers.ctrl);
                if ctrl {
                    if app.selection.node_edit.selected_nodes.contains(index) {
                        app.selection
                            .node_edit
                            .selected_nodes
                            .retain(|node| *node != *index);
                    } else {
                        app.selection.node_edit.selected_nodes.push(*index);
                    }
                } else {
                    app.selection.node_edit.selected_nodes = vec![*index];
                }
            }

            app.selection.node_edit.dragging_node = Some(target);
            app.selection.node_edit.drag_start = Some(canvas_pos);
        } else {
            app.selection.node_edit.selected_nodes.clear();
        }
    }

    if response.dragged_by(egui::PointerButton::Primary)
        && let Some(ref target) = app.selection.node_edit.dragging_node.clone()
        && let Some(pos) = ui_context.input(|input| input.pointer.latest_pos())
    {
        let canvas_pos = view.screen_to_canvas(pos);
        let local_x = (canvas_pos.x - object_position[0]) / object_scale[0];
        let local_y = (canvas_pos.y - object_position[1]) / object_scale[1];

        for layer in &mut app.project.layers {
            if let Some(keyframe) = layer.keyframes.get_mut(&app.current_frame) {
                for object in &mut keyframe.objects {
                    if object.id == object_id
                        && let Shape::Path { ref mut points, .. } = object.shape
                    {
                        match target {
                            DragTarget::AnchorPoint(index) => {
                                if let Some(point) = points.get_mut(*index) {
                                    point.position = [local_x, local_y];
                                }
                            }
                            DragTarget::ControlIn(index) => {
                                if let Some(point) = points.get_mut(*index) {
                                    point.control_in = Some([local_x, local_y]);
                                }
                            }
                            DragTarget::ControlOut(index) => {
                                if let Some(point) = points.get_mut(*index) {
                                    point.control_out = Some([local_x, local_y]);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if response.drag_stopped() {
        app.selection.node_edit.dragging_node = None;
        app.selection.node_edit.drag_start = None;
    }

    if response.secondary_clicked()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let canvas_pos = view.screen_to_canvas(pos);
        let local_x = canvas_pos.x - object_position[0];
        let local_y = canvas_pos.y - object_position[1];
        let threshold = 8.0 / view.zoom;

        for (index, point) in path_points.iter().enumerate() {
            let px = point.position[0] * object_scale[0];
            let py = point.position[1] * object_scale[1];
            if (local_x - px).abs() < threshold && (local_y - py).abs() < threshold {
                app.history.push(app.project.clone());
                ensure_keyframe_for_object(app, object_id);
                delete_node(app, object_id, index);
                app.selection
                    .node_edit
                    .selected_nodes
                    .retain(|node| *node != index);
                break;
            }
        }
    }

    if response.double_clicked()
        && let Some(pos) = response.interact_pointer_pos()
    {
        let canvas_pos = view.screen_to_canvas(pos);
        let local_x = (canvas_pos.x - object_position[0]) / object_scale[0];
        let local_y = (canvas_pos.y - object_position[1]) / object_scale[1];

        let mut best_index = None;
        let mut best_dist = f32::MAX;
        let threshold = 10.0 / view.zoom;

        for index in 0..path_points.len().saturating_sub(1) {
            let a = &path_points[index];
            let b = &path_points[index + 1];
            let dist = point_to_segment_dist(
                local_x,
                local_y,
                a.position[0],
                a.position[1],
                b.position[0],
                b.position[1],
            );
            if dist < threshold && dist < best_dist {
                best_dist = dist;
                best_index = Some(index + 1);
            }
        }

        if let Some(insert_index) = best_index {
            app.history.push(app.project.clone());
            ensure_keyframe_for_object(app, object_id);
            insert_node(app, object_id, insert_index, [local_x, local_y]);
        }
    }
}

pub fn draw_node_edit_overlay(app: &AnimateApp, view: &CanvasView, painter: &egui::Painter) {
    let object_id = match app.selection.node_edit.object_id {
        Some(id) => id,
        None => return,
    };

    let mut found = None;
    for layer in &app.project.layers {
        if let Some(objects) = tween::resolve_frame(layer, app.current_frame) {
            for object in &objects {
                if object.id == object_id {
                    if let Shape::Path { ref points, .. } = object.shape {
                        found = Some((points.clone(), object.position, object.scale));
                    }
                    break;
                }
            }
        }
        if found.is_some() {
            break;
        }
    }

    let (points, position, scale) = match found {
        Some(data) => data,
        None => return,
    };

    let anchor_color = egui::Color32::from_rgb(0, 150, 255);
    let selected_color = egui::Color32::from_rgb(255, 200, 50);
    let handle_color = egui::Color32::WHITE;
    let line_color = egui::Color32::from_rgba_unmultiplied(150, 150, 150, 180);

    for (index, point) in points.iter().enumerate() {
        let screen_pt = view.canvas_to_screen(egui::pos2(
            position[0] + point.position[0] * scale[0],
            position[1] + point.position[1] * scale[1],
        ));

        let is_selected = app.selection.node_edit.selected_nodes.contains(&index);
        let color = if is_selected {
            selected_color
        } else {
            anchor_color
        };
        painter.circle_filled(screen_pt, 4.0, color);
        painter.circle_stroke(screen_pt, 4.0, egui::Stroke::new(1.0, egui::Color32::BLACK));

        if let Some(control_in) = point.control_in {
            let screen_control = view.canvas_to_screen(egui::pos2(
                position[0] + control_in[0] * scale[0],
                position[1] + control_in[1] * scale[1],
            ));
            painter.line_segment(
                [screen_pt, screen_control],
                egui::Stroke::new(1.0, line_color),
            );
            painter.rect_filled(
                egui::Rect::from_center_size(screen_control, egui::vec2(6.0, 6.0)),
                0.0,
                handle_color,
            );
        }

        if let Some(control_out) = point.control_out {
            let screen_control = view.canvas_to_screen(egui::pos2(
                position[0] + control_out[0] * scale[0],
                position[1] + control_out[1] * scale[1],
            ));
            painter.line_segment(
                [screen_pt, screen_control],
                egui::Stroke::new(1.0, line_color),
            );
            painter.rect_filled(
                egui::Rect::from_center_size(screen_control, egui::vec2(6.0, 6.0)),
                0.0,
                handle_color,
            );
        }
    }
}

fn ensure_keyframe_for_object(app: &mut AnimateApp, object_id: uuid::Uuid) {
    for layer in &mut app.project.layers {
        let has_object = tween::resolve_frame(layer, app.current_frame)
            .map(|objects| objects.iter().any(|object| object.id == object_id))
            .unwrap_or(false);

        if has_object {
            tween::ensure_keyframe_at(layer, app.current_frame);
        }
    }
}

fn delete_node(app: &mut AnimateApp, object_id: uuid::Uuid, node_index: usize) {
    for layer in &mut app.project.layers {
        if let Some(keyframe) = layer.keyframes.get_mut(&app.current_frame) {
            for object in &mut keyframe.objects {
                if object.id == object_id
                    && let Shape::Path { ref mut points, .. } = object.shape
                    && node_index < points.len()
                    && points.len() > 2
                {
                    points.remove(node_index);
                }
            }
        }
    }
}

fn insert_node(app: &mut AnimateApp, object_id: uuid::Uuid, at_index: usize, position: [f32; 2]) {
    for layer in &mut app.project.layers {
        if let Some(keyframe) = layer.keyframes.get_mut(&app.current_frame) {
            for object in &mut keyframe.objects {
                if object.id == object_id
                    && let Shape::Path { ref mut points, .. } = object.shape
                    && at_index <= points.len()
                {
                    let new_point = PathPoint {
                        position,
                        control_in: None,
                        control_out: None,
                        pressure: 1.0,
                    };
                    points.insert(at_index, new_point);
                }
            }
        }
    }
}

fn point_to_segment_dist(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = bx - ax;
    let dy = by - ay;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 0.001 {
        return ((px - ax).powi(2) + (py - ay).powi(2)).sqrt();
    }
    let t = ((px - ax) * dx + (py - ay) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let cx = ax + t * dx;
    let cy = ay + t * dy;
    ((px - cx).powi(2) + (py - cy).powi(2)).sqrt()
}
