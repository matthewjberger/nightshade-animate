use std::collections::BTreeMap;

use crate::paint::{Paint, lerp_paint};
use crate::project::{
    AnimObject, Keyframe, Layer, PathPoint, PropertyKey, PropertyTracks, Shape, TweenType,
};

pub fn resolve_frame(layer: &Layer, frame: u32) -> Option<Vec<AnimObject>> {
    let prev_entry = layer.keyframes.range(..=frame).next_back();
    let (prev_frame, prev_keyframe) = prev_entry?;

    let mut objects = if *prev_frame == frame || prev_keyframe.tween == TweenType::None {
        prev_keyframe.objects.clone()
    } else {
        match layer.keyframes.range((frame + 1)..).next() {
            Some((next_frame, next_keyframe)) => {
                let raw_t = (frame - prev_frame) as f32 / (next_frame - prev_frame) as f32;
                let t = apply_easing(raw_t, prev_keyframe.tween);
                interpolate_objects(
                    &prev_keyframe.objects,
                    &next_keyframe.objects,
                    t,
                    prev_keyframe.shape_tween,
                )
            }
            None => prev_keyframe.objects.clone(),
        }
    };

    for object in &mut objects {
        if let Some(tracks) = layer.property_tracks.get(&object.id) {
            apply_property_tracks(object, tracks, frame);
        }
    }

    Some(objects)
}

pub fn ensure_keyframe_at(layer: &mut Layer, frame: u32) {
    if layer.keyframes.contains_key(&frame) {
        return;
    }
    let resolved = resolve_frame(layer, frame);
    let keyframe = match resolved {
        Some(objects) => Keyframe {
            objects,
            tween: TweenType::None,
            label: String::new(),
            comment: String::new(),
            shape_tween: false,
        },
        None => Keyframe::default(),
    };
    layer.keyframes.insert(frame, keyframe);
}

fn apply_easing(t: f32, tween: TweenType) -> f32 {
    match tween {
        TweenType::None => t,
        TweenType::Linear => t,
        TweenType::EaseIn => t * t,
        TweenType::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
        TweenType::EaseInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
            }
        }
        TweenType::CubicBezier { x1, y1, x2, y2 } => cubic_bezier_easing(t, x1, y1, x2, y2),
    }
}

fn cubic_bezier_easing(t: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
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

fn cubic_bezier_sample(t: f32, p1: f32, p2: f32) -> f32 {
    let omt = 1.0 - t;
    3.0 * omt * omt * t * p1 + 3.0 * omt * t * t * p2 + t * t * t
}

fn cubic_bezier_derivative(t: f32, p1: f32, p2: f32) -> f32 {
    let omt = 1.0 - t;
    3.0 * omt * omt * p1 + 6.0 * omt * t * (p2 - p1) + 3.0 * t * t * (1.0 - p2)
}

fn interpolate_objects(
    from: &[AnimObject],
    to: &[AnimObject],
    t: f32,
    shape_tween: bool,
) -> Vec<AnimObject> {
    let mut result = Vec::new();
    for from_obj in from {
        if let Some(to_obj) = to.iter().find(|object| object.id == from_obj.id) {
            result.push(interpolate_object(from_obj, to_obj, t, shape_tween));
        } else {
            result.push(from_obj.clone());
        }
    }
    result
}

fn interpolate_object(from: &AnimObject, to: &AnimObject, t: f32, shape_tween: bool) -> AnimObject {
    let shape = if shape_tween {
        interpolate_shape(&from.shape, &to.shape, t)
    } else {
        from.shape.clone()
    };

    AnimObject {
        id: from.id,
        shape,
        position: lerp_arr2(from.position, to.position, t),
        rotation: lerp_angle(from.rotation, to.rotation, t),
        scale: lerp_arr2(from.scale, to.scale, t),
        fill: lerp_paint(&from.fill, &to.fill, t),
        stroke: lerp_paint(&from.stroke, &to.stroke, t),
        stroke_width: lerp_f32(from.stroke_width, to.stroke_width, t),
    }
}

fn interpolate_shape(from: &Shape, to: &Shape, t: f32) -> Shape {
    match (from, to) {
        (
            Shape::Rectangle {
                width: from_w,
                height: from_h,
                corner_radius: from_r,
            },
            Shape::Rectangle {
                width: to_w,
                height: to_h,
                corner_radius: to_r,
            },
        ) => Shape::Rectangle {
            width: lerp_f32(*from_w, *to_w, t),
            height: lerp_f32(*from_h, *to_h, t),
            corner_radius: lerp_f32(*from_r, *to_r, t),
        },

        (
            Shape::Ellipse {
                radius_x: from_rx,
                radius_y: from_ry,
            },
            Shape::Ellipse {
                radius_x: to_rx,
                radius_y: to_ry,
            },
        ) => Shape::Ellipse {
            radius_x: lerp_f32(*from_rx, *to_rx, t),
            radius_y: lerp_f32(*from_ry, *to_ry, t),
        },

        (
            Shape::Line {
                end_x: from_ex,
                end_y: from_ey,
            },
            Shape::Line {
                end_x: to_ex,
                end_y: to_ey,
            },
        ) => Shape::Line {
            end_x: lerp_f32(*from_ex, *to_ex, t),
            end_y: lerp_f32(*from_ey, *to_ey, t),
        },

        (
            Shape::Path {
                points: from_points,
                closed: from_closed,
            },
            Shape::Path {
                points: to_points,
                closed: to_closed,
            },
        ) => {
            let (normalized_from, normalized_to) = normalize_path_points(from_points, to_points);
            let interpolated_points: Vec<PathPoint> = normalized_from
                .iter()
                .zip(normalized_to.iter())
                .map(|(from_point, to_point)| interpolate_path_point(from_point, to_point, t))
                .collect();
            let closed = if t < 0.5 { *from_closed } else { *to_closed };
            Shape::Path {
                points: interpolated_points,
                closed,
            }
        }

        _ => {
            let from_path = shape_to_path(from);
            let to_path = shape_to_path(to);
            if let (
                Shape::Path {
                    points: from_points,
                    closed: from_closed,
                },
                Shape::Path {
                    points: to_points,
                    closed: to_closed,
                },
            ) = (&from_path, &to_path)
            {
                let (normalized_from, normalized_to) =
                    normalize_path_points(from_points, to_points);
                let interpolated_points: Vec<PathPoint> = normalized_from
                    .iter()
                    .zip(normalized_to.iter())
                    .map(|(from_point, to_point)| interpolate_path_point(from_point, to_point, t))
                    .collect();
                let closed = if t < 0.5 { *from_closed } else { *to_closed };
                Shape::Path {
                    points: interpolated_points,
                    closed,
                }
            } else {
                from.clone()
            }
        }
    }
}

fn interpolate_path_point(from: &PathPoint, to: &PathPoint, t: f32) -> PathPoint {
    PathPoint {
        position: lerp_arr2(from.position, to.position, t),
        control_in: interpolate_optional_point(from.control_in, to.control_in, t),
        control_out: interpolate_optional_point(from.control_out, to.control_out, t),
        pressure: lerp_f32(from.pressure, to.pressure, t),
    }
}

fn interpolate_optional_point(
    from: Option<[f32; 2]>,
    to: Option<[f32; 2]>,
    t: f32,
) -> Option<[f32; 2]> {
    match (from, to) {
        (Some(a), Some(b)) => Some(lerp_arr2(a, b, t)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn normalize_path_points(from: &[PathPoint], to: &[PathPoint]) -> (Vec<PathPoint>, Vec<PathPoint>) {
    if from.len() == to.len() {
        return (from.to_vec(), to.to_vec());
    }

    let (shorter, longer, from_is_shorter) = if from.len() < to.len() {
        (from, to, true)
    } else {
        (to, from, false)
    };

    let subdivided = subdivide_path_to_count(shorter, longer.len());

    if from_is_shorter {
        (subdivided, longer.to_vec())
    } else {
        (longer.to_vec(), subdivided)
    }
}

fn subdivide_path_to_count(points: &[PathPoint], target_count: usize) -> Vec<PathPoint> {
    if points.len() >= target_count || points.len() < 2 {
        return points.to_vec();
    }

    let mut result = points.to_vec();

    while result.len() < target_count {
        let mut longest_segment_index = 0;
        let mut longest_distance = 0.0_f32;

        for index in 0..result.len() - 1 {
            let dx = result[index + 1].position[0] - result[index].position[0];
            let dy = result[index + 1].position[1] - result[index].position[1];
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > longest_distance {
                longest_distance = distance;
                longest_segment_index = index;
            }
        }

        let midpoint = interpolate_path_point(
            &result[longest_segment_index],
            &result[longest_segment_index + 1],
            0.5,
        );
        result.insert(longest_segment_index + 1, midpoint);
    }

    result
}

fn shape_to_path(shape: &Shape) -> Shape {
    match shape {
        Shape::Rectangle {
            width,
            height,
            corner_radius: _,
        } => {
            let half_w = width / 2.0;
            let half_h = height / 2.0;
            Shape::Path {
                points: vec![
                    PathPoint {
                        position: [-half_w, -half_h],
                        control_in: None,
                        control_out: None,
                        pressure: 1.0,
                    },
                    PathPoint {
                        position: [half_w, -half_h],
                        control_in: None,
                        control_out: None,
                        pressure: 1.0,
                    },
                    PathPoint {
                        position: [half_w, half_h],
                        control_in: None,
                        control_out: None,
                        pressure: 1.0,
                    },
                    PathPoint {
                        position: [-half_w, half_h],
                        control_in: None,
                        control_out: None,
                        pressure: 1.0,
                    },
                ],
                closed: true,
            }
        }
        Shape::Ellipse { radius_x, radius_y } => {
            let segment_count = 16;
            let points: Vec<PathPoint> = (0..segment_count)
                .map(|segment_index| {
                    let angle =
                        2.0 * std::f32::consts::PI * segment_index as f32 / segment_count as f32;
                    PathPoint {
                        position: [angle.cos() * radius_x, angle.sin() * radius_y],
                        control_in: None,
                        control_out: None,
                        pressure: 1.0,
                    }
                })
                .collect();
            Shape::Path {
                points,
                closed: true,
            }
        }
        Shape::Line { end_x, end_y } => Shape::Path {
            points: vec![
                PathPoint {
                    position: [0.0, 0.0],
                    control_in: None,
                    control_out: None,
                    pressure: 1.0,
                },
                PathPoint {
                    position: [*end_x, *end_y],
                    control_in: None,
                    control_out: None,
                    pressure: 1.0,
                },
            ],
            closed: false,
        },
        Shape::Path { .. } => shape.clone(),
        _ => shape.clone(),
    }
}

fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp_arr2(a: [f32; 2], b: [f32; 2], t: f32) -> [f32; 2] {
    [lerp_f32(a[0], b[0], t), lerp_f32(a[1], b[1], t)]
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

fn apply_property_tracks(object: &mut AnimObject, tracks: &PropertyTracks, frame: u32) {
    if let Some(value) = resolve_track_arr2(&tracks.position, frame) {
        object.position = value;
    }
    if let Some(value) = resolve_track_angle(&tracks.rotation, frame) {
        object.rotation = value;
    }
    if let Some(value) = resolve_track_arr2(&tracks.scale, frame) {
        object.scale = value;
    }
    if let Some(value) = resolve_track_paint(&tracks.fill, frame) {
        object.fill = value;
    }
    if let Some(value) = resolve_track_paint(&tracks.stroke, frame) {
        object.stroke = value;
    }
    if let Some(value) = resolve_track_f32(&tracks.stroke_width, frame) {
        object.stroke_width = value;
    }
}

fn resolve_track_f32(track: &BTreeMap<u32, PropertyKey<f32>>, frame: u32) -> Option<f32> {
    if track.is_empty() {
        return None;
    }
    let (prev_frame, prev_key) = track.range(..=frame).next_back()?;
    if *prev_frame == frame || prev_key.tween == TweenType::None {
        return Some(prev_key.value);
    }
    let (next_frame, next_key) = track.range((frame + 1)..).next()?;
    let raw_t = (frame - prev_frame) as f32 / (next_frame - prev_frame) as f32;
    let t = apply_easing(raw_t, prev_key.tween);
    Some(lerp_f32(prev_key.value, next_key.value, t))
}

fn resolve_track_angle(track: &BTreeMap<u32, PropertyKey<f32>>, frame: u32) -> Option<f32> {
    if track.is_empty() {
        return None;
    }
    let (prev_frame, prev_key) = track.range(..=frame).next_back()?;
    if *prev_frame == frame || prev_key.tween == TweenType::None {
        return Some(prev_key.value);
    }
    let (next_frame, next_key) = track.range((frame + 1)..).next()?;
    let raw_t = (frame - prev_frame) as f32 / (next_frame - prev_frame) as f32;
    let t = apply_easing(raw_t, prev_key.tween);
    Some(lerp_angle(prev_key.value, next_key.value, t))
}

fn resolve_track_arr2(
    track: &BTreeMap<u32, PropertyKey<[f32; 2]>>,
    frame: u32,
) -> Option<[f32; 2]> {
    if track.is_empty() {
        return None;
    }
    let (prev_frame, prev_key) = track.range(..=frame).next_back()?;
    if *prev_frame == frame || prev_key.tween == TweenType::None {
        return Some(prev_key.value);
    }
    let (next_frame, next_key) = track.range((frame + 1)..).next()?;
    let raw_t = (frame - prev_frame) as f32 / (next_frame - prev_frame) as f32;
    let t = apply_easing(raw_t, prev_key.tween);
    Some(lerp_arr2(prev_key.value, next_key.value, t))
}

fn resolve_track_paint(track: &BTreeMap<u32, PropertyKey<Paint>>, frame: u32) -> Option<Paint> {
    if track.is_empty() {
        return None;
    }
    let (prev_frame, prev_key) = track.range(..=frame).next_back()?;
    if *prev_frame == frame || prev_key.tween == TweenType::None {
        return Some(prev_key.value.clone());
    }
    let (next_frame, next_key) = track.range((frame + 1)..).next()?;
    let raw_t = (frame - prev_frame) as f32 / (next_frame - prev_frame) as f32;
    let t = apply_easing(raw_t, prev_key.tween);
    Some(lerp_paint(&prev_key.value, &next_key.value, t))
}
