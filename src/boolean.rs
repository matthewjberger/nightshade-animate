use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;

use crate::app::AnimateApp;
use crate::project::{AnimObject, PathPoint, Shape};
use crate::tween;

#[derive(Clone, Copy)]
pub enum BooleanOp {
    Union,
    Subtract,
    Intersect,
    Exclude,
}

pub fn apply_boolean_operation(app: &mut AnimateApp, operation: BooleanOp) {
    if app.selection.selected_objects.len() != 2 {
        return;
    }

    let id_a = app.selection.selected_objects[0];
    let id_b = app.selection.selected_objects[1];

    let mut object_a = None;
    let mut object_b = None;

    for layer in &app.project.layers {
        if let Some(objects) = tween::resolve_frame(layer, app.current_frame) {
            for object in &objects {
                if object.id == id_a {
                    object_a = Some(object.clone());
                }
                if object.id == id_b {
                    object_b = Some(object.clone());
                }
            }
        }
    }

    let Some(object_a) = object_a else { return };
    let Some(object_b) = object_b else { return };

    let polygon_a = object_to_polygon(&object_a);
    let polygon_b = object_to_polygon(&object_b);

    if polygon_a.len() < 3 || polygon_b.len() < 3 {
        return;
    }

    let rule = match operation {
        BooleanOp::Union => OverlayRule::Union,
        BooleanOp::Subtract => OverlayRule::Difference,
        BooleanOp::Intersect => OverlayRule::Intersect,
        BooleanOp::Exclude => OverlayRule::Xor,
    };

    let result = polygon_a.overlay(&polygon_b, rule, FillRule::EvenOdd);

    if result.is_empty() {
        return;
    }

    let outer_contour = &result[0];
    if outer_contour.is_empty() {
        return;
    }

    let result_points: Vec<PathPoint> = outer_contour[0]
        .iter()
        .map(|point| PathPoint {
            position: [point[0] as f32, point[1] as f32],
            control_in: None,
            control_out: None,
            pressure: 1.0,
        })
        .collect();

    if result_points.len() < 3 {
        return;
    }

    app.history.push(app.project.clone());

    let result_object = AnimObject::new(
        Shape::Path {
            points: result_points,
            closed: true,
        },
        [0.0, 0.0],
        object_a.fill.clone(),
        object_a.stroke.clone(),
        object_a.stroke_width,
    );

    let result_id = result_object.id;

    for layer in &mut app.project.layers {
        let has_selected = tween::resolve_frame(layer, app.current_frame)
            .map(|objects| {
                objects
                    .iter()
                    .any(|object| object.id == id_a || object.id == id_b)
            })
            .unwrap_or(false);

        if has_selected {
            tween::ensure_keyframe_at(layer, app.current_frame);
        }

        if let Some(keyframe) = layer.keyframes.get_mut(&app.current_frame) {
            keyframe
                .objects
                .retain(|object| object.id != id_a && object.id != id_b);

            if has_selected {
                keyframe.objects.push(result_object.clone());
            }
        }
    }

    app.selection.selected_objects.clear();
    app.selection.selected_objects.push(result_id);
}

fn object_to_polygon(object: &AnimObject) -> Vec<[f64; 2]> {
    match &object.shape {
        Shape::Rectangle { width, height, .. } => {
            let half_w = (width * object.scale[0] / 2.0) as f64;
            let half_h = (height * object.scale[1] / 2.0) as f64;
            let cx = object.position[0] as f64;
            let cy = object.position[1] as f64;

            if object.rotation.abs() < 0.001 {
                vec![
                    [cx - half_w, cy - half_h],
                    [cx + half_w, cy - half_h],
                    [cx + half_w, cy + half_h],
                    [cx - half_w, cy + half_h],
                ]
            } else {
                let cos_r = (object.rotation as f64).cos();
                let sin_r = (object.rotation as f64).sin();
                let corners = [
                    [-half_w, -half_h],
                    [half_w, -half_h],
                    [half_w, half_h],
                    [-half_w, half_h],
                ];
                corners
                    .iter()
                    .map(|[local_x, local_y]| {
                        [
                            cx + local_x * cos_r - local_y * sin_r,
                            cy + local_x * sin_r + local_y * cos_r,
                        ]
                    })
                    .collect()
            }
        }
        Shape::Ellipse { radius_x, radius_y } => {
            let scaled_rx = (radius_x * object.scale[0]) as f64;
            let scaled_ry = (radius_y * object.scale[1]) as f64;
            let cx = object.position[0] as f64;
            let cy = object.position[1] as f64;
            let segments = 32;

            (0..segments)
                .map(|index| {
                    let angle = 2.0 * std::f64::consts::PI * index as f64 / segments as f64;
                    let local_x = angle.cos() * scaled_rx;
                    let local_y = angle.sin() * scaled_ry;

                    if object.rotation.abs() < 0.001 {
                        [cx + local_x, cy + local_y]
                    } else {
                        let cos_r = (object.rotation as f64).cos();
                        let sin_r = (object.rotation as f64).sin();
                        [
                            cx + local_x * cos_r - local_y * sin_r,
                            cy + local_x * sin_r + local_y * cos_r,
                        ]
                    }
                })
                .collect()
        }
        Shape::Path { points, .. } => {
            let mut polygon = Vec::new();
            for point_index in 0..points.len() {
                let point = &points[point_index];
                let world_x = (object.position[0] + point.position[0] * object.scale[0]) as f64;
                let world_y = (object.position[1] + point.position[1] * object.scale[1]) as f64;

                if point_index > 0 {
                    let prev = &points[point_index - 1];
                    if prev.control_out.is_some() || point.control_in.is_some() {
                        let ctrl_out = prev.control_out.unwrap_or(prev.position);
                        let ctrl_in = point.control_in.unwrap_or(point.position);
                        for step in 1..=8 {
                            let t = step as f32 / 8.0;
                            let omt = 1.0 - t;
                            let bx = omt * omt * omt * prev.position[0]
                                + 3.0 * omt * omt * t * ctrl_out[0]
                                + 3.0 * omt * t * t * ctrl_in[0]
                                + t * t * t * point.position[0];
                            let by = omt * omt * omt * prev.position[1]
                                + 3.0 * omt * omt * t * ctrl_out[1]
                                + 3.0 * omt * t * t * ctrl_in[1]
                                + t * t * t * point.position[1];
                            polygon.push([
                                (object.position[0] + bx * object.scale[0]) as f64,
                                (object.position[1] + by * object.scale[1]) as f64,
                            ]);
                        }
                        continue;
                    }
                }
                polygon.push([world_x, world_y]);
            }
            polygon
        }
        _ => Vec::new(),
    }
}
