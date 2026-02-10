use nightshade::prelude::*;

use crate::app::AnimateApp;
use crate::canvas::CanvasView;
use crate::selection;
use crate::tween;

#[derive(Clone, Copy, PartialEq)]
pub enum TransformHandle {
    TopLeft,
    TopCenter,
    TopRight,
    MiddleLeft,
    MiddleRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    Rotation,
}

#[derive(Clone)]
pub struct TransformState {
    pub active_handle: Option<TransformHandle>,
    pub initial_mouse: egui::Pos2,
    pub initial_positions: Vec<(uuid::Uuid, [f32; 2])>,
    pub initial_scales: Vec<(uuid::Uuid, [f32; 2])>,
    pub initial_rotations: Vec<(uuid::Uuid, f32)>,
    pub bbox_center: [f32; 2],
    pub bbox_half: [f32; 2],
}

impl Default for TransformState {
    fn default() -> Self {
        Self {
            active_handle: None,
            initial_mouse: egui::Pos2::ZERO,
            initial_positions: Vec::new(),
            initial_scales: Vec::new(),
            initial_rotations: Vec::new(),
            bbox_center: [0.0, 0.0],
            bbox_half: [0.0, 0.0],
        }
    }
}

const HANDLE_SIZE: f32 = 5.0;
const ROTATION_DISTANCE: f32 = 15.0;

pub fn draw_transform_handles(app: &AnimateApp, view: &CanvasView, painter: &egui::Painter) {
    if app.selection.selected_objects.is_empty() {
        return;
    }

    let (center, half_w, half_h) = compute_selection_bbox(app);
    if half_w < 1.0 && half_h < 1.0 {
        return;
    }

    let screen_center = view.canvas_to_screen(egui::pos2(center[0], center[1]));
    let screen_half_w = half_w * view.zoom;
    let screen_half_h = half_h * view.zoom;

    let handle_positions =
        compute_handle_screen_positions(screen_center, screen_half_w, screen_half_h);

    for (_, pos) in &handle_positions {
        let rect =
            egui::Rect::from_center_size(*pos, egui::vec2(HANDLE_SIZE * 2.0, HANDLE_SIZE * 2.0));
        painter.rect_filled(rect, 1.0, egui::Color32::WHITE);
        painter.rect(
            rect,
            1.0,
            egui::Color32::TRANSPARENT,
            egui::Stroke::new(1.0, egui::Color32::from_rgb(0, 150, 255)),
            egui::StrokeKind::Outside,
        );
    }
}

pub fn handle_transform_interaction(
    app: &mut AnimateApp,
    response: &egui::Response,
    ui_context: &egui::Context,
) -> bool {
    if app.selection.selected_objects.is_empty() {
        return false;
    }

    let (center, half_w, half_h) = compute_selection_bbox(app);
    if half_w < 1.0 && half_h < 1.0 {
        return false;
    }

    let view = app.canvas_view.clone();
    let screen_center = view.canvas_to_screen(egui::pos2(center[0], center[1]));
    let screen_half_w = half_w * view.zoom;
    let screen_half_h = half_h * view.zoom;

    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(pos) = response.interact_pointer_pos()
    {
        let handle_positions =
            compute_handle_screen_positions(screen_center, screen_half_w, screen_half_h);
        let hit = find_hit_handle(&handle_positions, pos);

        if let Some(handle) = hit {
            let selected = app.selection.selected_objects.clone();
            let mut initial_positions = Vec::new();
            let mut initial_scales = Vec::new();
            let mut initial_rotations = Vec::new();

            for layer in &app.project.layers {
                if let Some(objects) = tween::resolve_frame(layer, app.current_frame) {
                    for object in &objects {
                        if selected.contains(&object.id) {
                            initial_positions.push((object.id, object.position));
                            initial_scales.push((object.id, object.scale));
                            initial_rotations.push((object.id, object.rotation));
                        }
                    }
                }
            }

            app.history.push(app.project.clone());
            ensure_keyframes_for_selected(app);

            app.selection.transform_state = TransformState {
                active_handle: Some(handle),
                initial_mouse: pos,
                initial_positions,
                initial_scales,
                initial_rotations,
                bbox_center: center,
                bbox_half: [half_w, half_h],
            };

            return true;
        }
    }

    if app.selection.transform_state.active_handle.is_some()
        && response.dragged_by(egui::PointerButton::Primary)
        && let Some(pos) = ui_context.input(|input| input.pointer.latest_pos())
    {
        let shift_held = ui_context.input(|input| input.modifiers.shift);
        apply_transform(app, pos, shift_held);
        return true;
    }

    if response.drag_stopped() && app.selection.transform_state.active_handle.is_some() {
        app.selection.transform_state = TransformState::default();
        return true;
    }

    false
}

fn compute_selection_bbox(app: &AnimateApp) -> ([f32; 2], f32, f32) {
    let selected = &app.selection.selected_objects;
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for layer in &app.project.layers {
        if let Some(objects) = tween::resolve_frame(layer, app.current_frame) {
            for object in &objects {
                if selected.contains(&object.id) {
                    let (half_w, half_h, offset) = selection::get_object_bounds_public(object);
                    let cx = object.position[0] + offset[0];
                    let cy = object.position[1] + offset[1];
                    min_x = min_x.min(cx - half_w);
                    min_y = min_y.min(cy - half_h);
                    max_x = max_x.max(cx + half_w);
                    max_y = max_y.max(cy + half_h);
                }
            }
        }
    }

    let center = [(min_x + max_x) / 2.0, (min_y + max_y) / 2.0];
    let half_w = (max_x - min_x) / 2.0;
    let half_h = (max_y - min_y) / 2.0;
    (center, half_w, half_h)
}

fn compute_handle_screen_positions(
    screen_center: egui::Pos2,
    half_w: f32,
    half_h: f32,
) -> [(TransformHandle, egui::Pos2); 8] {
    [
        (
            TransformHandle::TopLeft,
            egui::pos2(screen_center.x - half_w, screen_center.y - half_h),
        ),
        (
            TransformHandle::TopCenter,
            egui::pos2(screen_center.x, screen_center.y - half_h),
        ),
        (
            TransformHandle::TopRight,
            egui::pos2(screen_center.x + half_w, screen_center.y - half_h),
        ),
        (
            TransformHandle::MiddleLeft,
            egui::pos2(screen_center.x - half_w, screen_center.y),
        ),
        (
            TransformHandle::MiddleRight,
            egui::pos2(screen_center.x + half_w, screen_center.y),
        ),
        (
            TransformHandle::BottomLeft,
            egui::pos2(screen_center.x - half_w, screen_center.y + half_h),
        ),
        (
            TransformHandle::BottomCenter,
            egui::pos2(screen_center.x, screen_center.y + half_h),
        ),
        (
            TransformHandle::BottomRight,
            egui::pos2(screen_center.x + half_w, screen_center.y + half_h),
        ),
    ]
}

fn find_hit_handle(
    positions: &[(TransformHandle, egui::Pos2); 8],
    mouse: egui::Pos2,
) -> Option<TransformHandle> {
    let hit_distance = HANDLE_SIZE + 4.0;

    for (handle, pos) in positions {
        let dx = mouse.x - pos.x;
        let dy = mouse.y - pos.y;
        if dx * dx + dy * dy <= hit_distance * hit_distance {
            return Some(*handle);
        }
    }

    let center = egui::pos2(
        (positions[0].1.x + positions[7].1.x) / 2.0,
        (positions[0].1.y + positions[7].1.y) / 2.0,
    );
    let dist_from_center = ((mouse.x - center.x).powi(2) + (mouse.y - center.y).powi(2)).sqrt();
    let half_diag =
        ((positions[7].1.x - center.x).powi(2) + (positions[7].1.y - center.y).powi(2)).sqrt();
    if dist_from_center > half_diag && dist_from_center < half_diag + ROTATION_DISTANCE {
        return Some(TransformHandle::Rotation);
    }

    None
}

fn ensure_keyframes_for_selected(app: &mut AnimateApp) {
    let selected = app.selection.selected_objects.clone();
    for layer in &mut app.project.layers {
        let has_match = tween::resolve_frame(layer, app.current_frame)
            .map(|objects| objects.iter().any(|object| selected.contains(&object.id)))
            .unwrap_or(false);

        if has_match {
            tween::ensure_keyframe_at(layer, app.current_frame);
        }
    }
}

fn apply_transform(app: &mut AnimateApp, current_mouse: egui::Pos2, shift_held: bool) {
    let transform = app.selection.transform_state.clone();
    let handle = match transform.active_handle {
        Some(handle) => handle,
        None => return,
    };

    let view = app.canvas_view.clone();

    match handle {
        TransformHandle::Rotation => {
            let center_screen = view.canvas_to_screen(egui::pos2(
                transform.bbox_center[0],
                transform.bbox_center[1],
            ));
            let initial_angle = (transform.initial_mouse.y - center_screen.y)
                .atan2(transform.initial_mouse.x - center_screen.x);
            let current_angle =
                (current_mouse.y - center_screen.y).atan2(current_mouse.x - center_screen.x);
            let delta_angle = current_angle - initial_angle;

            let snapped_angle = if shift_held {
                let snap = std::f32::consts::PI / 12.0;
                (delta_angle / snap).round() * snap
            } else {
                delta_angle
            };

            for layer in &mut app.project.layers {
                if let Some(keyframe) = layer.keyframes.get_mut(&app.current_frame) {
                    for object in &mut keyframe.objects {
                        if let Some((_, initial_rotation)) = transform
                            .initial_rotations
                            .iter()
                            .find(|(id, _)| *id == object.id)
                        {
                            object.rotation = initial_rotation + snapped_angle;

                            if let Some((_, initial_pos)) = transform
                                .initial_positions
                                .iter()
                                .find(|(id, _)| *id == object.id)
                            {
                                let dx = initial_pos[0] - transform.bbox_center[0];
                                let dy = initial_pos[1] - transform.bbox_center[1];
                                let cos_a = snapped_angle.cos();
                                let sin_a = snapped_angle.sin();
                                object.position[0] =
                                    transform.bbox_center[0] + dx * cos_a - dy * sin_a;
                                object.position[1] =
                                    transform.bbox_center[1] + dx * sin_a + dy * cos_a;
                            }
                        }
                    }
                }
            }
        }
        _ => {
            let initial_canvas = view.screen_to_canvas(transform.initial_mouse);
            let current_canvas = view.screen_to_canvas(current_mouse);

            let delta_x = current_canvas.x - initial_canvas.x;
            let delta_y = current_canvas.y - initial_canvas.y;

            let (scale_x, scale_y) = compute_scale_factors(
                handle,
                delta_x,
                delta_y,
                transform.bbox_half[0],
                transform.bbox_half[1],
                shift_held,
            );

            for layer in &mut app.project.layers {
                if let Some(keyframe) = layer.keyframes.get_mut(&app.current_frame) {
                    for object in &mut keyframe.objects {
                        if let Some((_, initial_scale)) = transform
                            .initial_scales
                            .iter()
                            .find(|(id, _)| *id == object.id)
                        {
                            object.scale[0] = (initial_scale[0] * scale_x).max(0.01);
                            object.scale[1] = (initial_scale[1] * scale_y).max(0.01);

                            if let Some((_, initial_pos)) = transform
                                .initial_positions
                                .iter()
                                .find(|(id, _)| *id == object.id)
                            {
                                let dx = initial_pos[0] - transform.bbox_center[0];
                                let dy = initial_pos[1] - transform.bbox_center[1];
                                object.position[0] = transform.bbox_center[0] + dx * scale_x;
                                object.position[1] = transform.bbox_center[1] + dy * scale_y;
                            }
                        }
                    }
                }
            }
        }
    }
}

fn compute_scale_factors(
    handle: TransformHandle,
    delta_x: f32,
    delta_y: f32,
    half_w: f32,
    half_h: f32,
    proportional: bool,
) -> (f32, f32) {
    let safe_half_w = half_w.max(1.0);
    let safe_half_h = half_h.max(1.0);

    let (raw_sx, raw_sy) = match handle {
        TransformHandle::TopLeft => (1.0 - delta_x / safe_half_w, 1.0 - delta_y / safe_half_h),
        TransformHandle::TopCenter => (1.0, 1.0 - delta_y / safe_half_h),
        TransformHandle::TopRight => (1.0 + delta_x / safe_half_w, 1.0 - delta_y / safe_half_h),
        TransformHandle::MiddleLeft => (1.0 - delta_x / safe_half_w, 1.0),
        TransformHandle::MiddleRight => (1.0 + delta_x / safe_half_w, 1.0),
        TransformHandle::BottomLeft => (1.0 - delta_x / safe_half_w, 1.0 + delta_y / safe_half_h),
        TransformHandle::BottomCenter => (1.0, 1.0 + delta_y / safe_half_h),
        TransformHandle::BottomRight => (1.0 + delta_x / safe_half_w, 1.0 + delta_y / safe_half_h),
        TransformHandle::Rotation => (1.0, 1.0),
    };

    if proportional {
        match handle {
            TransformHandle::TopLeft
            | TransformHandle::TopRight
            | TransformHandle::BottomLeft
            | TransformHandle::BottomRight => {
                let uniform = (raw_sx + raw_sy) / 2.0;
                (uniform, uniform)
            }
            _ => (raw_sx, raw_sy),
        }
    } else {
        (raw_sx, raw_sy)
    }
}
