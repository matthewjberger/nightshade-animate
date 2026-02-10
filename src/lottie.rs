use nightshade::prelude::*;

use crate::project::{LayerType, Project, Shape, TweenType};

pub fn export_lottie(project: &Project, path: &std::path::Path) {
    let composition = build_lottie_composition(project);
    let json = serde_json::to_string_pretty(&composition).unwrap_or_default();
    let _ = std::fs::write(path, json);
}

fn build_lottie_composition(project: &Project) -> serde_json::Value {
    let mut layers = Vec::new();

    for (layer_index, layer) in project.layers.iter().enumerate().rev() {
        if !layer.visible {
            continue;
        }
        if layer.layer_type == LayerType::Guide || layer.layer_type == LayerType::Folder {
            continue;
        }

        let lottie_layer = build_lottie_layer(project, layer_index);
        if let Some(lottie_layer) = lottie_layer {
            layers.push(lottie_layer);
        }
    }

    serde_json::json!({
        "v": "5.7.4",
        "fr": project.frame_rate,
        "ip": 0,
        "op": project.total_frames,
        "w": project.canvas_width,
        "h": project.canvas_height,
        "nm": project.name,
        "ddd": 0,
        "assets": [],
        "layers": layers
    })
}

fn build_lottie_layer(project: &Project, layer_index: usize) -> Option<serde_json::Value> {
    let layer = &project.layers[layer_index];

    let keyframe_entries: Vec<(u32, &crate::project::Keyframe)> = layer
        .keyframes
        .iter()
        .map(|(frame, keyframe)| (*frame, keyframe))
        .collect();

    if keyframe_entries.is_empty() {
        return None;
    }

    let all_object_ids = collect_unique_object_ids(layer);
    if all_object_ids.is_empty() {
        return None;
    }

    let mut shape_items = Vec::new();
    for object_id in &all_object_ids {
        let object_shapes = build_animated_object_shapes(project, layer, *object_id);
        shape_items.extend(object_shapes);
    }

    Some(serde_json::json!({
        "ddd": 0,
        "ind": layer_index,
        "ty": 4,
        "nm": layer.name,
        "sr": 1,
        "ks": {
            "o": static_value(vec![(layer.opacity * 100.0) as f64]),
            "r": static_value(vec![0.0]),
            "p": static_value(vec![0.0, 0.0, 0.0]),
            "a": static_value(vec![0.0, 0.0, 0.0]),
            "s": static_value(vec![100.0, 100.0, 100.0])
        },
        "ao": 0,
        "shapes": shape_items,
        "ip": 0,
        "op": project.total_frames,
        "st": 0,
        "bm": 0
    }))
}

fn collect_unique_object_ids(layer: &crate::project::Layer) -> Vec<uuid::Uuid> {
    let mut ids = Vec::new();
    for keyframe in layer.keyframes.values() {
        for object in &keyframe.objects {
            if !ids.contains(&object.id) {
                ids.push(object.id);
            }
        }
    }
    ids
}

fn build_animated_object_shapes(
    project: &Project,
    layer: &crate::project::Layer,
    object_id: uuid::Uuid,
) -> Vec<serde_json::Value> {
    let keyframe_frames: Vec<u32> = layer.keyframes.keys().copied().collect();

    let first_object = find_object_in_layer(layer, object_id);
    let Some(first_object) = first_object else {
        return Vec::new();
    };

    let mut group_items = Vec::new();

    let shape_item = build_shape_for_object(&first_object.shape);
    if let Some(shape_item) = shape_item {
        group_items.push(shape_item);
    }

    let fill_item = build_animated_fill(layer, object_id, &keyframe_frames, project.total_frames);
    group_items.push(fill_item);

    let stroke_color = first_object.stroke.as_solid();
    if stroke_color[3] > 0.001 && first_object.stroke_width > 0.0 {
        let stroke_item =
            build_animated_stroke(layer, object_id, &keyframe_frames, project.total_frames);
        group_items.push(stroke_item);
    }

    let transform_item =
        build_animated_transform(layer, object_id, &keyframe_frames, project.total_frames);
    group_items.push(transform_item);

    vec![serde_json::json!({
        "ty": "gr",
        "it": group_items,
        "nm": format!("Object"),
        "np": group_items.len(),
        "cix": 2,
        "bm": 0,
        "ix": 1,
        "mn": "ADBE Vector Group",
        "hd": false
    })]
}

fn find_object_in_layer(
    layer: &crate::project::Layer,
    object_id: uuid::Uuid,
) -> Option<crate::project::AnimObject> {
    for keyframe in layer.keyframes.values() {
        for object in &keyframe.objects {
            if object.id == object_id {
                return Some(object.clone());
            }
        }
    }
    None
}

fn build_shape_for_object(shape: &Shape) -> Option<serde_json::Value> {
    match shape {
        Shape::Rectangle {
            width,
            height,
            corner_radius,
        } => Some(serde_json::json!({
            "ty": "rc",
            "d": 1,
            "s": static_value(vec![*width as f64, *height as f64]),
            "p": static_value(vec![0.0, 0.0]),
            "r": static_value(vec![*corner_radius as f64]),
            "nm": "Rectangle",
            "mn": "ADBE Vector Shape - Rect",
            "hd": false
        })),
        Shape::Ellipse { radius_x, radius_y } => Some(serde_json::json!({
            "ty": "el",
            "d": 1,
            "s": static_value(vec![(*radius_x * 2.0) as f64, (*radius_y * 2.0) as f64]),
            "p": static_value(vec![0.0, 0.0]),
            "nm": "Ellipse",
            "mn": "ADBE Vector Shape - Ellipse",
            "hd": false
        })),
        Shape::Path { points, closed } => {
            let mut vertices = Vec::new();
            let mut in_tangents = Vec::new();
            let mut out_tangents = Vec::new();

            for point in points {
                vertices.push(vec![point.position[0], point.position[1]]);

                if let Some(control_in) = point.control_in {
                    in_tangents.push(vec![
                        control_in[0] - point.position[0],
                        control_in[1] - point.position[1],
                    ]);
                } else {
                    in_tangents.push(vec![0.0, 0.0]);
                }

                if let Some(control_out) = point.control_out {
                    out_tangents.push(vec![
                        control_out[0] - point.position[0],
                        control_out[1] - point.position[1],
                    ]);
                } else {
                    out_tangents.push(vec![0.0, 0.0]);
                }
            }

            Some(serde_json::json!({
                "ty": "sh",
                "d": 1,
                "ks": {
                    "a": 0,
                    "k": {
                        "i": in_tangents,
                        "o": out_tangents,
                        "v": vertices,
                        "c": closed
                    },
                    "ix": 2
                },
                "nm": "Path",
                "mn": "ADBE Vector Shape - Group",
                "hd": false
            }))
        }
        Shape::Line { end_x, end_y } => {
            let vertices = vec![vec![0.0_f32, 0.0], vec![*end_x, *end_y]];
            let in_tangents = vec![vec![0.0, 0.0], vec![0.0, 0.0]];
            let out_tangents = vec![vec![0.0, 0.0], vec![0.0, 0.0]];

            Some(serde_json::json!({
                "ty": "sh",
                "d": 1,
                "ks": {
                    "a": 0,
                    "k": {
                        "i": in_tangents,
                        "o": out_tangents,
                        "v": vertices,
                        "c": false
                    },
                    "ix": 2
                },
                "nm": "Line",
                "mn": "ADBE Vector Shape - Group",
                "hd": false
            }))
        }
        Shape::Text { .. } | Shape::RasterImage { .. } | Shape::SymbolInstance { .. } => None,
    }
}

fn build_animated_fill(
    layer: &crate::project::Layer,
    object_id: uuid::Uuid,
    keyframe_frames: &[u32],
    total_frames: u32,
) -> serde_json::Value {
    let mut color_keyframes = Vec::new();
    let mut opacity_keyframes = Vec::new();

    for (index, frame) in keyframe_frames.iter().enumerate() {
        let keyframe = &layer.keyframes[frame];
        let object = keyframe
            .objects
            .iter()
            .find(|object| object.id == object_id);
        let color = object
            .map(|object| object.fill.as_solid())
            .unwrap_or([0.0, 0.0, 0.0, 1.0]);

        let next_frame = keyframe_frames
            .get(index + 1)
            .copied()
            .unwrap_or(total_frames);
        let easing = tween_to_lottie_easing(keyframe.tween);

        color_keyframes.push(serde_json::json!({
            "t": *frame,
            "s": [color[0], color[1], color[2], 1.0],
            "i": easing.0,
            "o": easing.1
        }));

        opacity_keyframes.push(serde_json::json!({
            "t": *frame,
            "s": [color[3] * 100.0],
            "i": easing.0,
            "o": easing.1
        }));

        if index == keyframe_frames.len() - 1 {
            color_keyframes.push(serde_json::json!({
                "t": next_frame,
                "s": [color[0], color[1], color[2], 1.0]
            }));
            opacity_keyframes.push(serde_json::json!({
                "t": next_frame,
                "s": [color[3] * 100.0]
            }));
        }
    }

    let has_animation = keyframe_frames.len() > 1;

    if has_animation {
        serde_json::json!({
            "ty": "fl",
            "c": {
                "a": 1,
                "k": color_keyframes,
                "ix": 4
            },
            "o": {
                "a": 1,
                "k": opacity_keyframes,
                "ix": 5
            },
            "r": 1,
            "bm": 0,
            "nm": "Fill",
            "mn": "ADBE Vector Graphic - Fill",
            "hd": false
        })
    } else {
        let color = keyframe_frames
            .first()
            .and_then(|frame| layer.keyframes.get(frame))
            .and_then(|keyframe| {
                keyframe
                    .objects
                    .iter()
                    .find(|object| object.id == object_id)
            })
            .map(|object| object.fill.as_solid())
            .unwrap_or([0.0, 0.0, 0.0, 1.0]);

        serde_json::json!({
            "ty": "fl",
            "c": static_value(vec![color[0] as f64, color[1] as f64, color[2] as f64, 1.0]),
            "o": static_value(vec![color[3] as f64 * 100.0]),
            "r": 1,
            "bm": 0,
            "nm": "Fill",
            "mn": "ADBE Vector Graphic - Fill",
            "hd": false
        })
    }
}

fn build_animated_stroke(
    layer: &crate::project::Layer,
    object_id: uuid::Uuid,
    keyframe_frames: &[u32],
    total_frames: u32,
) -> serde_json::Value {
    let mut color_keyframes = Vec::new();
    let mut width_keyframes = Vec::new();

    for (index, frame) in keyframe_frames.iter().enumerate() {
        let keyframe = &layer.keyframes[frame];
        let object = keyframe
            .objects
            .iter()
            .find(|object| object.id == object_id);
        let color = object
            .map(|object| object.stroke.as_solid())
            .unwrap_or([0.0, 0.0, 0.0, 1.0]);
        let stroke_width = object.map(|object| object.stroke_width).unwrap_or(0.0);

        let next_frame = keyframe_frames
            .get(index + 1)
            .copied()
            .unwrap_or(total_frames);
        let easing = tween_to_lottie_easing(keyframe.tween);

        color_keyframes.push(serde_json::json!({
            "t": *frame,
            "s": [color[0], color[1], color[2], 1.0],
            "i": easing.0,
            "o": easing.1
        }));

        width_keyframes.push(serde_json::json!({
            "t": *frame,
            "s": [stroke_width],
            "i": easing.0,
            "o": easing.1
        }));

        if index == keyframe_frames.len() - 1 {
            color_keyframes.push(serde_json::json!({
                "t": next_frame,
                "s": [color[0], color[1], color[2], 1.0]
            }));
            width_keyframes.push(serde_json::json!({
                "t": next_frame,
                "s": [stroke_width]
            }));
        }
    }

    let has_animation = keyframe_frames.len() > 1;

    if has_animation {
        serde_json::json!({
            "ty": "st",
            "c": {
                "a": 1,
                "k": color_keyframes,
                "ix": 3
            },
            "o": static_value(vec![100.0]),
            "w": {
                "a": 1,
                "k": width_keyframes,
                "ix": 5
            },
            "lc": 2,
            "lj": 1,
            "ml": 4,
            "bm": 0,
            "nm": "Stroke",
            "mn": "ADBE Vector Graphic - Stroke",
            "hd": false
        })
    } else {
        let color = keyframe_frames
            .first()
            .and_then(|frame| layer.keyframes.get(frame))
            .and_then(|keyframe| {
                keyframe
                    .objects
                    .iter()
                    .find(|object| object.id == object_id)
            })
            .map(|object| object.stroke.as_solid())
            .unwrap_or([0.0, 0.0, 0.0, 1.0]);
        let stroke_width = keyframe_frames
            .first()
            .and_then(|frame| layer.keyframes.get(frame))
            .and_then(|keyframe| {
                keyframe
                    .objects
                    .iter()
                    .find(|object| object.id == object_id)
            })
            .map(|object| object.stroke_width)
            .unwrap_or(0.0);

        serde_json::json!({
            "ty": "st",
            "c": static_value(vec![color[0] as f64, color[1] as f64, color[2] as f64, 1.0]),
            "o": static_value(vec![100.0]),
            "w": static_value(vec![stroke_width as f64]),
            "lc": 2,
            "lj": 1,
            "ml": 4,
            "bm": 0,
            "nm": "Stroke",
            "mn": "ADBE Vector Graphic - Stroke",
            "hd": false
        })
    }
}

fn build_animated_transform(
    layer: &crate::project::Layer,
    object_id: uuid::Uuid,
    keyframe_frames: &[u32],
    total_frames: u32,
) -> serde_json::Value {
    let has_animation = keyframe_frames.len() > 1;

    if !has_animation {
        let object = find_object_in_layer(layer, object_id);
        let (position, rotation, scale) = match object {
            Some(object) => (
                object.position,
                object.rotation.to_degrees(),
                [object.scale[0] * 100.0, object.scale[1] * 100.0],
            ),
            None => ([0.0, 0.0], 0.0, [100.0, 100.0]),
        };

        return serde_json::json!({
            "ty": "tr",
            "p": static_value(vec![position[0] as f64, position[1] as f64, 0.0]),
            "a": static_value(vec![0.0, 0.0, 0.0]),
            "s": static_value(vec![scale[0] as f64, scale[1] as f64, 100.0]),
            "r": static_value(vec![rotation as f64]),
            "o": static_value(vec![100.0]),
            "sk": static_value(vec![0.0]),
            "sa": static_value(vec![0.0]),
            "nm": "Transform"
        });
    }

    let mut position_keyframes = Vec::new();
    let mut rotation_keyframes = Vec::new();
    let mut scale_keyframes = Vec::new();

    for (index, frame) in keyframe_frames.iter().enumerate() {
        let keyframe = &layer.keyframes[frame];
        let object = keyframe
            .objects
            .iter()
            .find(|object| object.id == object_id);
        let position = object.map(|object| object.position).unwrap_or([0.0, 0.0]);
        let rotation = object
            .map(|object| object.rotation.to_degrees())
            .unwrap_or(0.0);
        let scale = object
            .map(|object| [object.scale[0] * 100.0, object.scale[1] * 100.0])
            .unwrap_or([100.0, 100.0]);

        let next_frame = keyframe_frames
            .get(index + 1)
            .copied()
            .unwrap_or(total_frames);
        let easing = tween_to_lottie_easing(keyframe.tween);

        position_keyframes.push(serde_json::json!({
            "t": *frame,
            "s": [position[0], position[1], 0.0],
            "i": easing.0,
            "o": easing.1
        }));

        rotation_keyframes.push(serde_json::json!({
            "t": *frame,
            "s": [rotation],
            "i": easing.0,
            "o": easing.1
        }));

        scale_keyframes.push(serde_json::json!({
            "t": *frame,
            "s": [scale[0], scale[1], 100.0],
            "i": easing.0,
            "o": easing.1
        }));

        if index == keyframe_frames.len() - 1 {
            position_keyframes.push(serde_json::json!({
                "t": next_frame,
                "s": [position[0], position[1], 0.0]
            }));
            rotation_keyframes.push(serde_json::json!({
                "t": next_frame,
                "s": [rotation]
            }));
            scale_keyframes.push(serde_json::json!({
                "t": next_frame,
                "s": [scale[0], scale[1], 100.0]
            }));
        }
    }

    serde_json::json!({
        "ty": "tr",
        "p": {
            "a": 1,
            "k": position_keyframes,
            "ix": 2
        },
        "a": static_value(vec![0.0, 0.0, 0.0]),
        "s": {
            "a": 1,
            "k": scale_keyframes,
            "ix": 6
        },
        "r": {
            "a": 1,
            "k": rotation_keyframes,
            "ix": 10
        },
        "o": static_value(vec![100.0]),
        "sk": static_value(vec![0.0]),
        "sa": static_value(vec![0.0]),
        "nm": "Transform"
    })
}

fn static_value(value: Vec<f64>) -> serde_json::Value {
    serde_json::json!({
        "a": 0,
        "k": value,
        "ix": 1
    })
}

fn tween_to_lottie_easing(tween: TweenType) -> (serde_json::Value, serde_json::Value) {
    match tween {
        TweenType::None | TweenType::Linear => (
            serde_json::json!({"x": [0.167], "y": [0.167]}),
            serde_json::json!({"x": [0.833], "y": [0.833]}),
        ),
        TweenType::EaseIn => (
            serde_json::json!({"x": [0.42], "y": [0.0]}),
            serde_json::json!({"x": [1.0], "y": [1.0]}),
        ),
        TweenType::EaseOut => (
            serde_json::json!({"x": [0.0], "y": [0.0]}),
            serde_json::json!({"x": [0.58], "y": [1.0]}),
        ),
        TweenType::EaseInOut => (
            serde_json::json!({"x": [0.42], "y": [0.0]}),
            serde_json::json!({"x": [0.58], "y": [1.0]}),
        ),
        TweenType::CubicBezier { x1, y1, x2, y2 } => (
            serde_json::json!({"x": [x2], "y": [y2]}),
            serde_json::json!({"x": [x1], "y": [y1]}),
        ),
    }
}
