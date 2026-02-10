use crate::app::AnimateApp;
use crate::selection;
use crate::tween;

fn get_selected_bounds(app: &AnimateApp) -> Vec<(uuid::Uuid, [f32; 4])> {
    let selected = &app.selection.selected_objects;
    let mut bounds = Vec::new();

    for layer in &app.project.layers {
        if let Some(objects) = tween::resolve_frame(layer, app.current_frame) {
            for object in &objects {
                if selected.contains(&object.id) {
                    let (half_w, half_h, center_offset) =
                        selection::get_object_bounds_public(object);
                    let cx = object.position[0] + center_offset[0];
                    let cy = object.position[1] + center_offset[1];
                    bounds.push((
                        object.id,
                        [cx - half_w, cy - half_h, cx + half_w, cy + half_h],
                    ));
                }
            }
        }
    }
    bounds
}

fn apply_position_deltas(app: &mut AnimateApp, deltas: &[(uuid::Uuid, [f32; 2])]) {
    if deltas.is_empty() {
        return;
    }

    app.history.push(app.project.clone());

    for layer in &mut app.project.layers {
        let has_match = tween::resolve_frame(layer, app.current_frame)
            .map(|objects| {
                objects
                    .iter()
                    .any(|object| deltas.iter().any(|(id, _)| *id == object.id))
            })
            .unwrap_or(false);

        if has_match {
            tween::ensure_keyframe_at(layer, app.current_frame);
        }

        if let Some(keyframe) = layer.keyframes.get_mut(&app.current_frame) {
            for object in &mut keyframe.objects {
                if let Some((_, delta)) = deltas.iter().find(|(id, _)| *id == object.id) {
                    object.position[0] += delta[0];
                    object.position[1] += delta[1];
                }
            }
        }
    }
}

pub fn align_left(app: &mut AnimateApp) {
    let bounds = get_selected_bounds(app);
    if bounds.len() < 2 {
        return;
    }

    let min_left = bounds
        .iter()
        .map(|(_, b)| b[0])
        .fold(f32::INFINITY, f32::min);
    let deltas: Vec<_> = bounds
        .iter()
        .map(|(id, b)| (*id, [min_left - b[0], 0.0]))
        .filter(|(_, d)| d[0].abs() > 0.001)
        .collect();

    apply_position_deltas(app, &deltas);
}

pub fn align_right(app: &mut AnimateApp) {
    let bounds = get_selected_bounds(app);
    if bounds.len() < 2 {
        return;
    }

    let max_right = bounds
        .iter()
        .map(|(_, b)| b[2])
        .fold(f32::NEG_INFINITY, f32::max);
    let deltas: Vec<_> = bounds
        .iter()
        .map(|(id, b)| (*id, [max_right - b[2], 0.0]))
        .filter(|(_, d)| d[0].abs() > 0.001)
        .collect();

    apply_position_deltas(app, &deltas);
}

pub fn align_top(app: &mut AnimateApp) {
    let bounds = get_selected_bounds(app);
    if bounds.len() < 2 {
        return;
    }

    let min_top = bounds
        .iter()
        .map(|(_, b)| b[1])
        .fold(f32::INFINITY, f32::min);
    let deltas: Vec<_> = bounds
        .iter()
        .map(|(id, b)| (*id, [0.0, min_top - b[1]]))
        .filter(|(_, d)| d[1].abs() > 0.001)
        .collect();

    apply_position_deltas(app, &deltas);
}

pub fn align_bottom(app: &mut AnimateApp) {
    let bounds = get_selected_bounds(app);
    if bounds.len() < 2 {
        return;
    }

    let max_bottom = bounds
        .iter()
        .map(|(_, b)| b[3])
        .fold(f32::NEG_INFINITY, f32::max);
    let deltas: Vec<_> = bounds
        .iter()
        .map(|(id, b)| (*id, [0.0, max_bottom - b[3]]))
        .filter(|(_, d)| d[1].abs() > 0.001)
        .collect();

    apply_position_deltas(app, &deltas);
}

pub fn align_center_horizontal(app: &mut AnimateApp) {
    let bounds = get_selected_bounds(app);
    if bounds.len() < 2 {
        return;
    }

    let min_left = bounds
        .iter()
        .map(|(_, b)| b[0])
        .fold(f32::INFINITY, f32::min);
    let max_right = bounds
        .iter()
        .map(|(_, b)| b[2])
        .fold(f32::NEG_INFINITY, f32::max);
    let center = (min_left + max_right) / 2.0;

    let deltas: Vec<_> = bounds
        .iter()
        .map(|(id, b)| {
            let obj_center = (b[0] + b[2]) / 2.0;
            (*id, [center - obj_center, 0.0])
        })
        .filter(|(_, d)| d[0].abs() > 0.001)
        .collect();

    apply_position_deltas(app, &deltas);
}

pub fn align_center_vertical(app: &mut AnimateApp) {
    let bounds = get_selected_bounds(app);
    if bounds.len() < 2 {
        return;
    }

    let min_top = bounds
        .iter()
        .map(|(_, b)| b[1])
        .fold(f32::INFINITY, f32::min);
    let max_bottom = bounds
        .iter()
        .map(|(_, b)| b[3])
        .fold(f32::NEG_INFINITY, f32::max);
    let center = (min_top + max_bottom) / 2.0;

    let deltas: Vec<_> = bounds
        .iter()
        .map(|(id, b)| {
            let obj_center = (b[1] + b[3]) / 2.0;
            (*id, [0.0, center - obj_center])
        })
        .filter(|(_, d)| d[1].abs() > 0.001)
        .collect();

    apply_position_deltas(app, &deltas);
}

pub fn distribute_horizontal(app: &mut AnimateApp) {
    let mut bounds = get_selected_bounds(app);
    if bounds.len() < 3 {
        return;
    }

    bounds.sort_by(|a, b| {
        let center_a = (a.1[0] + a.1[2]) / 2.0;
        let center_b = (b.1[0] + b.1[2]) / 2.0;
        center_a
            .partial_cmp(&center_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let first_center = (bounds[0].1[0] + bounds[0].1[2]) / 2.0;
    let last_center = (bounds.last().unwrap().1[0] + bounds.last().unwrap().1[2]) / 2.0;
    let total_span = last_center - first_center;
    let spacing = total_span / (bounds.len() - 1) as f32;

    let deltas: Vec<_> = bounds
        .iter()
        .enumerate()
        .map(|(index, (id, b))| {
            let obj_center = (b[0] + b[2]) / 2.0;
            let target = first_center + index as f32 * spacing;
            (*id, [target - obj_center, 0.0])
        })
        .filter(|(_, d)| d[0].abs() > 0.001)
        .collect();

    apply_position_deltas(app, &deltas);
}

pub fn distribute_vertical(app: &mut AnimateApp) {
    let mut bounds = get_selected_bounds(app);
    if bounds.len() < 3 {
        return;
    }

    bounds.sort_by(|a, b| {
        let center_a = (a.1[1] + a.1[3]) / 2.0;
        let center_b = (b.1[1] + b.1[3]) / 2.0;
        center_a
            .partial_cmp(&center_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let first_center = (bounds[0].1[1] + bounds[0].1[3]) / 2.0;
    let last_center = (bounds.last().unwrap().1[1] + bounds.last().unwrap().1[3]) / 2.0;
    let total_span = last_center - first_center;
    let spacing = total_span / (bounds.len() - 1) as f32;

    let deltas: Vec<_> = bounds
        .iter()
        .enumerate()
        .map(|(index, (id, b))| {
            let obj_center = (b[1] + b[3]) / 2.0;
            let target = first_center + index as f32 * spacing;
            (*id, [0.0, target - obj_center])
        })
        .filter(|(_, d)| d[1].abs() > 0.001)
        .collect();

    apply_position_deltas(app, &deltas);
}
