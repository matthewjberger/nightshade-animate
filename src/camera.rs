use nightshade::prelude::*;

use crate::project::{CameraKeyframe, Project, TweenType};

pub struct ResolvedCamera {
    pub position: [f32; 2],
    pub zoom: f32,
    pub rotation: f32,
}

impl Default for ResolvedCamera {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0],
            zoom: 1.0,
            rotation: 0.0,
        }
    }
}

pub fn resolve_camera(project: &Project, frame: u32) -> ResolvedCamera {
    if project.camera_keyframes.is_empty() {
        return ResolvedCamera::default();
    }

    let prev_entry = project.camera_keyframes.range(..=frame).next_back();
    let Some((&prev_frame, prev_camera)) = prev_entry else {
        let first = project.camera_keyframes.values().next().unwrap();
        return ResolvedCamera {
            position: first.position,
            zoom: first.zoom,
            rotation: first.rotation,
        };
    };

    if prev_frame == frame {
        return ResolvedCamera {
            position: prev_camera.position,
            zoom: prev_camera.zoom,
            rotation: prev_camera.rotation,
        };
    }

    if project.camera_tween == TweenType::None {
        return ResolvedCamera {
            position: prev_camera.position,
            zoom: prev_camera.zoom,
            rotation: prev_camera.rotation,
        };
    }

    let next_entry = project.camera_keyframes.range((frame + 1)..).next();
    let Some((&next_frame, next_camera)) = next_entry else {
        return ResolvedCamera {
            position: prev_camera.position,
            zoom: prev_camera.zoom,
            rotation: prev_camera.rotation,
        };
    };

    let raw_t = (frame - prev_frame) as f32 / (next_frame - prev_frame) as f32;
    let t = apply_camera_easing(raw_t, project.camera_tween);

    ResolvedCamera {
        position: [
            lerp(prev_camera.position[0], next_camera.position[0], t),
            lerp(prev_camera.position[1], next_camera.position[1], t),
        ],
        zoom: lerp(prev_camera.zoom, next_camera.zoom, t),
        rotation: lerp_angle(prev_camera.rotation, next_camera.rotation, t),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn transform_point(
    point: [f32; 2],
    camera: &ResolvedCamera,
    canvas_width: f32,
    canvas_height: f32,
) -> [f32; 2] {
    let center_x = canvas_width / 2.0;
    let center_y = canvas_height / 2.0;

    let shifted_x = point[0] - camera.position[0];
    let shifted_y = point[1] - camera.position[1];

    let relative_x = shifted_x - center_x;
    let relative_y = shifted_y - center_y;

    let cos_r = camera.rotation.cos();
    let sin_r = camera.rotation.sin();
    let rotated_x = relative_x * cos_r - relative_y * sin_r;
    let rotated_y = relative_x * sin_r + relative_y * cos_r;

    let zoomed_x = rotated_x * camera.zoom;
    let zoomed_y = rotated_y * camera.zoom;

    [center_x + zoomed_x, center_y + zoomed_y]
}

pub fn draw_camera_properties(app: &mut crate::app::AnimateApp, ui: &mut egui::Ui) {
    ui.separator();
    ui.heading("Camera");

    let frame = app.current_frame;
    let has_keyframe = app.project.camera_keyframes.contains_key(&frame);
    let camera = resolve_camera(&app.project, frame);

    let mut position = camera.position;
    let mut zoom = camera.zoom;
    let mut rotation_degrees = camera.rotation.to_degrees();

    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label("X:");
        if ui
            .add(egui::DragValue::new(&mut position[0]).speed(1.0))
            .changed()
        {
            changed = true;
        }
        ui.label("Y:");
        if ui
            .add(egui::DragValue::new(&mut position[1]).speed(1.0))
            .changed()
        {
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Zoom:");
        if ui
            .add(
                egui::DragValue::new(&mut zoom)
                    .speed(0.01)
                    .range(0.1..=10.0),
            )
            .changed()
        {
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Rotation:");
        if ui
            .add(
                egui::DragValue::new(&mut rotation_degrees)
                    .speed(1.0)
                    .suffix("°"),
            )
            .changed()
        {
            changed = true;
        }
    });

    if changed {
        app.history.push(app.project.clone());
        app.project.camera_keyframes.insert(
            frame,
            CameraKeyframe {
                position,
                zoom,
                rotation: rotation_degrees.to_radians(),
            },
        );
    }

    ui.horizontal(|ui| {
        if has_keyframe {
            if ui.small_button("Remove Camera Keyframe").clicked() {
                app.history.push(app.project.clone());
                app.project.camera_keyframes.remove(&frame);
            }
        } else if ui.small_button("Add Camera Keyframe").clicked() {
            app.history.push(app.project.clone());
            app.project.camera_keyframes.insert(
                frame,
                CameraKeyframe {
                    position: camera.position,
                    zoom: camera.zoom,
                    rotation: camera.rotation,
                },
            );
        }
    });

    ui.horizontal(|ui| {
        ui.label("Tween:");
        let tween = &mut app.project.camera_tween;
        egui::ComboBox::from_id_salt("camera_tween")
            .selected_text(tween_name(*tween))
            .show_ui(ui, |ui| {
                ui.selectable_value(tween, TweenType::None, "None");
                ui.selectable_value(tween, TweenType::Linear, "Linear");
                ui.selectable_value(tween, TweenType::EaseIn, "Ease In");
                ui.selectable_value(tween, TweenType::EaseOut, "Ease Out");
                ui.selectable_value(tween, TweenType::EaseInOut, "Ease In/Out");
            });
    });

    if ui.small_button("Reset Camera").clicked() {
        app.history.push(app.project.clone());
        app.project.camera_keyframes.clear();
        app.project.camera_tween = TweenType::None;
    }
}

fn tween_name(tween: TweenType) -> &'static str {
    match tween {
        TweenType::None => "None",
        TweenType::Linear => "Linear",
        TweenType::EaseIn => "Ease In",
        TweenType::EaseOut => "Ease Out",
        TweenType::EaseInOut => "Ease In/Out",
        TweenType::CubicBezier { .. } => "Custom",
    }
}

fn apply_camera_easing(t: f32, tween: TweenType) -> f32 {
    match tween {
        TweenType::None | TweenType::Linear => t,
        TweenType::EaseIn => t * t,
        TweenType::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
        TweenType::EaseInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
            }
        }
        TweenType::CubicBezier { x1, y1, x2, y2 } => {
            let mut guess_t = t;
            for _ in 0..8 {
                let current_x = cubic_bezier_sample(guess_t, x1, x2);
                let dx = current_x - t;
                if dx.abs() < 1e-6 {
                    break;
                }
                let derivative = cubic_bezier_derivative(guess_t, x1, x2);
                if derivative.abs() < 1e-6 {
                    break;
                }
                guess_t -= dx / derivative;
                guess_t = guess_t.clamp(0.0, 1.0);
            }
            cubic_bezier_sample(guess_t, y1, y2)
        }
    }
}

fn cubic_bezier_sample(t: f32, p1: f32, p2: f32) -> f32 {
    let omt = 1.0 - t;
    3.0 * omt * omt * t * p1 + 3.0 * omt * t * t * p2 + t * t * t
}

fn cubic_bezier_derivative(t: f32, p1: f32, p2: f32) -> f32 {
    let omt = 1.0 - t;
    3.0 * omt * omt * p1 + 6.0 * omt * t * (p2 - p1) + 3.0 * t * t * (1.0 - p2)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let mut diff = b - a;
    while diff > std::f32::consts::PI {
        diff -= 2.0 * std::f32::consts::PI;
    }
    while diff < -std::f32::consts::PI {
        diff += 2.0 * std::f32::consts::PI;
    }
    a + diff * t
}
