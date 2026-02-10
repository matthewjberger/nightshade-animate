use std::collections::{BTreeMap, HashMap};

use crate::paint::Paint;
use crate::project::{
    AnimObject, BlendMode, Keyframe, Layer, LayerType, Library, PathPoint, Project, Shape,
    TweenType,
};

pub fn generate_bouncing_ball() -> Project {
    let ball_id = uuid::Uuid::new_v4();
    let shadow_id = uuid::Uuid::new_v4();

    let canvas_width = 1920u32;
    let canvas_height = 1080u32;
    let total_frames = 120u32;
    let frame_rate = 24u32;

    let ball_radius = 40.0f32;
    let ground_y = canvas_height as f32 - 200.0;
    let start_y = 150.0f32;
    let center_x = canvas_width as f32 / 2.0;
    let initial_height = ground_y - start_y;
    let restitution = 0.6f32;

    let shadow_y = ground_y + ball_radius + 10.0;

    let ball_fill = Paint::Solid([0.95, 0.3, 0.15, 1.0]);
    let ball_stroke = Paint::Solid([0.7, 0.15, 0.05, 1.0]);
    let ball_stroke_width = 2.0f32;

    let mut bounce_heights = Vec::new();
    let mut height = initial_height;
    for _ in 0..5 {
        height *= restitution;
        if height < 10.0 {
            break;
        }
        bounce_heights.push(height);
    }

    let mut time_segments: Vec<f32> = Vec::new();
    time_segments.push(initial_height.sqrt());
    for bounce_height in &bounce_heights {
        time_segments.push(bounce_height.sqrt());
        time_segments.push(bounce_height.sqrt());
    }

    let total_time: f32 = time_segments.iter().sum();
    let frames_available = (total_frames - 2) as f32;

    struct KeyframeData {
        frame: u32,
        ball_y: f32,
        ball_scale: [f32; 2],
        tween: TweenType,
    }

    let mut keyframe_data: Vec<KeyframeData> = Vec::new();

    keyframe_data.push(KeyframeData {
        frame: 0,
        ball_y: start_y,
        ball_scale: [0.92, 1.08],
        tween: TweenType::EaseIn,
    });

    let mut accumulated_frames = 0.0f32;
    let mut bounce_index = 0;
    let mut going_up = false;

    for segment_time in &time_segments {
        accumulated_frames += segment_time / total_time * frames_available;
        let frame = (accumulated_frames.round() as u32 + 1).min(total_frames - 1);

        if !going_up {
            let squash_amount = 0.3 * (1.0 - bounce_index as f32 * 0.12).max(0.1);
            keyframe_data.push(KeyframeData {
                frame,
                ball_y: ground_y,
                ball_scale: [1.0 + squash_amount, 1.0 - squash_amount],
                tween: TweenType::EaseOut,
            });
            going_up = true;
        } else {
            let peak_y = ground_y - bounce_heights[bounce_index];
            let stretch_amount = 0.08 * (1.0 - bounce_index as f32 * 0.15).max(0.02);
            keyframe_data.push(KeyframeData {
                frame,
                ball_y: peak_y,
                ball_scale: [1.0 - stretch_amount, 1.0 + stretch_amount],
                tween: TweenType::EaseIn,
            });
            bounce_index += 1;
            going_up = false;
        }
    }

    if keyframe_data
        .last()
        .is_none_or(|last| last.frame < total_frames - 1)
    {
        keyframe_data.push(KeyframeData {
            frame: total_frames - 1,
            ball_y: ground_y,
            ball_scale: [1.0, 1.0],
            tween: TweenType::None,
        });
    }

    let mut ball_keyframes: BTreeMap<u32, Keyframe> = BTreeMap::new();
    for data in &keyframe_data {
        let ball = AnimObject {
            id: ball_id,
            shape: Shape::Ellipse {
                radius_x: ball_radius,
                radius_y: ball_radius,
            },
            position: [center_x, data.ball_y],
            rotation: 0.0,
            scale: data.ball_scale,
            fill: ball_fill.clone(),
            stroke: ball_stroke.clone(),
            stroke_width: ball_stroke_width,
        };

        ball_keyframes.insert(
            data.frame,
            Keyframe {
                objects: vec![ball],
                tween: data.tween,
                label: String::new(),
                comment: String::new(),
                shape_tween: false,
            },
        );
    }

    let mut shadow_keyframes: BTreeMap<u32, Keyframe> = BTreeMap::new();
    for data in &keyframe_data {
        let height_ratio = (ground_y - data.ball_y) / initial_height;
        let shadow_scale_x = 1.3 - 0.7 * height_ratio;
        let shadow_opacity = 0.5 - 0.35 * height_ratio;

        let shadow = AnimObject {
            id: shadow_id,
            shape: Shape::Ellipse {
                radius_x: 35.0,
                radius_y: 6.0,
            },
            position: [center_x, shadow_y],
            rotation: 0.0,
            scale: [shadow_scale_x, 1.0],
            fill: Paint::Solid([0.0, 0.0, 0.0, shadow_opacity]),
            stroke: Paint::Solid([0.0, 0.0, 0.0, 0.0]),
            stroke_width: 0.0,
        };

        shadow_keyframes.insert(
            data.frame,
            Keyframe {
                objects: vec![shadow],
                tween: data.tween,
                label: String::new(),
                comment: String::new(),
                shape_tween: false,
            },
        );
    }

    let shadow_layer = Layer {
        id: uuid::Uuid::new_v4(),
        name: "Shadow".to_string(),
        visible: true,
        locked: false,
        opacity: 1.0,
        keyframes: shadow_keyframes,
        layer_type: LayerType::Normal,
        blend_mode: BlendMode::Normal,
        parent_id: None,
        collapsed: false,
        property_tracks: HashMap::new(),
    };

    let ball_layer = Layer {
        id: uuid::Uuid::new_v4(),
        name: "Ball".to_string(),
        visible: true,
        locked: false,
        opacity: 1.0,
        keyframes: ball_keyframes,
        layer_type: LayerType::Normal,
        blend_mode: BlendMode::Normal,
        parent_id: None,
        collapsed: false,
        property_tracks: HashMap::new(),
    };

    Project {
        name: "Bouncing Ball".to_string(),
        canvas_width,
        canvas_height,
        background_color: [0.95, 0.97, 1.0, 1.0],
        frame_rate,
        total_frames,
        layers: vec![ball_layer, shadow_layer],
        guides: Vec::new(),
        camera_keyframes: BTreeMap::new(),
        camera_tween: TweenType::None,
        scenes: Vec::new(),
        active_scene: 0,
        library: Library::default(),
        image_assets: Vec::new(),
        loop_start: None,
        loop_end: None,
        audio_tracks: Vec::new(),
        armatures: Vec::new(),
    }
}

pub fn generate_showcase_animation() -> Project {
    let canvas_width = 1920u32;
    let canvas_height = 1080u32;
    let total_frames = 240u32;
    let frame_rate = 24u32;

    let sky_id = uuid::Uuid::new_v4();
    let water_id = uuid::Uuid::new_v4();
    let sun_id = uuid::Uuid::new_v4();
    let reflection_id = uuid::Uuid::new_v4();
    let horizon_id = uuid::Uuid::new_v4();
    let lighthouse_tower_id = uuid::Uuid::new_v4();
    let lighthouse_beam_id = uuid::Uuid::new_v4();
    let lighthouse_base_id = uuid::Uuid::new_v4();
    let lighthouse_lantern_id = uuid::Uuid::new_v4();
    let lighthouse_light_id = uuid::Uuid::new_v4();
    let sailboat_id = uuid::Uuid::new_v4();
    let sailboat_hull_id = uuid::Uuid::new_v4();
    let sailboat_mast_id = uuid::Uuid::new_v4();
    let waves_id = uuid::Uuid::new_v4();
    let moon_id = uuid::Uuid::new_v4();
    let star_ids = [
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
    ];

    let sky_keyframes = build_full_object_layer(
        sky_id,
        Shape::Rectangle {
            width: 1920.0,
            height: 1080.0,
            corner_radius: 0.0,
        },
        &[
            FullFrameSpec {
                frame: 0,
                position: [960.0, 540.0],
                fill: [0.4, 0.6, 0.9, 1.0],
                stroke: [0.0; 4],
                stroke_width: 0.0,
                scale: [1.0, 1.0],
                rotation: 0.0,
                tween: TweenType::Linear,
            },
            FullFrameSpec {
                frame: 80,
                position: [960.0, 540.0],
                fill: [0.95, 0.55, 0.25, 1.0],
                stroke: [0.0; 4],
                stroke_width: 0.0,
                scale: [1.0, 1.0],
                rotation: 0.0,
                tween: TweenType::Linear,
            },
            FullFrameSpec {
                frame: 160,
                position: [960.0, 540.0],
                fill: [0.55, 0.15, 0.3, 1.0],
                stroke: [0.0; 4],
                stroke_width: 0.0,
                scale: [1.0, 1.0],
                rotation: 0.0,
                tween: TweenType::Linear,
            },
            FullFrameSpec {
                frame: 239,
                position: [960.0, 540.0],
                fill: [0.05, 0.02, 0.12, 1.0],
                stroke: [0.0; 4],
                stroke_width: 0.0,
                scale: [1.0, 1.0],
                rotation: 0.0,
                tween: TweenType::None,
            },
        ],
    );

    let water_keyframes = build_full_object_layer(
        water_id,
        Shape::Rectangle {
            width: 1920.0,
            height: 250.0,
            corner_radius: 20.0,
        },
        &[
            FullFrameSpec {
                frame: 0,
                position: [960.0, 955.0],
                fill: [0.2, 0.3, 0.5, 1.0],
                stroke: [0.0; 4],
                stroke_width: 0.0,
                scale: [1.0, 1.0],
                rotation: 0.0,
                tween: TweenType::Linear,
            },
            FullFrameSpec {
                frame: 80,
                position: [960.0, 955.0],
                fill: [0.5, 0.35, 0.15, 1.0],
                stroke: [0.0; 4],
                stroke_width: 0.0,
                scale: [1.0, 1.0],
                rotation: 0.0,
                tween: TweenType::Linear,
            },
            FullFrameSpec {
                frame: 160,
                position: [960.0, 955.0],
                fill: [0.2, 0.08, 0.15, 1.0],
                stroke: [0.0; 4],
                stroke_width: 0.0,
                scale: [1.0, 1.0],
                rotation: 0.0,
                tween: TweenType::Linear,
            },
            FullFrameSpec {
                frame: 239,
                position: [960.0, 955.0],
                fill: [0.02, 0.02, 0.08, 1.0],
                stroke: [0.0; 4],
                stroke_width: 0.0,
                scale: [1.0, 1.0],
                rotation: 0.0,
                tween: TweenType::None,
            },
        ],
    );

    let sun_keyframes = build_full_object_layer(
        sun_id,
        Shape::Ellipse {
            radius_x: 60.0,
            radius_y: 60.0,
        },
        &[
            FullFrameSpec {
                frame: 0,
                position: [1400.0, 250.0],
                fill: [1.0, 0.9, 0.4, 1.0],
                stroke: [1.0, 0.95, 0.6, 0.8],
                stroke_width: 4.0,
                scale: [1.0, 1.0],
                rotation: 0.0,
                tween: TweenType::EaseInOut,
            },
            FullFrameSpec {
                frame: 80,
                position: [960.0, 500.0],
                fill: [1.0, 0.55, 0.15, 1.0],
                stroke: [1.0, 0.4, 0.1, 0.9],
                stroke_width: 12.0,
                scale: [1.15, 1.0],
                rotation: 0.0,
                tween: TweenType::EaseIn,
            },
            FullFrameSpec {
                frame: 180,
                position: [960.0, 750.0],
                fill: [0.9, 0.2, 0.05, 1.0],
                stroke: [0.8, 0.15, 0.05, 0.6],
                stroke_width: 6.0,
                scale: [1.4, 0.65],
                rotation: 0.0,
                tween: TweenType::EaseIn,
            },
            FullFrameSpec {
                frame: 239,
                position: [960.0, 860.0],
                fill: [0.7, 0.1, 0.05, 0.4],
                stroke: [0.5, 0.05, 0.02, 0.2],
                stroke_width: 2.0,
                scale: [1.8, 0.3],
                rotation: 0.0,
                tween: TweenType::None,
            },
        ],
    );

    let reflection_keyframes = build_full_object_layer(
        reflection_id,
        Shape::Ellipse {
            radius_x: 80.0,
            radius_y: 15.0,
        },
        &[
            FullFrameSpec {
                frame: 0,
                position: [1400.0, 870.0],
                fill: [1.0, 0.85, 0.4, 0.5],
                stroke: [0.0; 4],
                stroke_width: 0.0,
                scale: [0.6, 0.4],
                rotation: 0.0,
                tween: TweenType::Linear,
            },
            FullFrameSpec {
                frame: 80,
                position: [960.0, 870.0],
                fill: [1.0, 0.5, 0.15, 0.7],
                stroke: [0.0; 4],
                stroke_width: 0.0,
                scale: [1.5, 1.0],
                rotation: 0.0,
                tween: TweenType::Linear,
            },
            FullFrameSpec {
                frame: 180,
                position: [960.0, 870.0],
                fill: [0.9, 0.2, 0.05, 0.5],
                stroke: [0.0; 4],
                stroke_width: 0.0,
                scale: [2.0, 1.3],
                rotation: 0.0,
                tween: TweenType::Linear,
            },
            FullFrameSpec {
                frame: 239,
                position: [960.0, 870.0],
                fill: [0.5, 0.1, 0.05, 0.15],
                stroke: [0.0; 4],
                stroke_width: 0.0,
                scale: [0.5, 0.3],
                rotation: 0.0,
                tween: TweenType::None,
            },
        ],
    );

    let mut horizon_keyframes = BTreeMap::new();
    for &(frame, stroke, stroke_width, tween) in &[
        (0u32, [0.3f32, 0.3, 0.5, 0.4], 1.0f32, TweenType::EaseInOut),
        (80, [1.0, 0.7, 0.3, 0.7], 3.0, TweenType::EaseInOut),
        (160, [0.6, 0.2, 0.3, 0.5], 2.0, TweenType::EaseInOut),
        (239, [0.1, 0.05, 0.1, 0.2], 1.0, TweenType::None),
    ] {
        horizon_keyframes.insert(
            frame,
            Keyframe {
                objects: vec![AnimObject {
                    id: horizon_id,
                    shape: Shape::Line {
                        end_x: 1920.0,
                        end_y: 0.0,
                    },
                    position: [0.0, 830.0],
                    rotation: 0.0,
                    scale: [1.0, 1.0],
                    fill: Paint::Solid([0.0; 4]),
                    stroke: Paint::Solid(stroke),
                    stroke_width,
                }],
                tween,
                label: String::new(),
                comment: String::new(),
                shape_tween: false,
            },
        );
    }

    let lighthouse_beam_shape = Shape::Path {
        points: vec![
            PathPoint {
                position: [0.0, 0.0],
                control_in: Some([-20.0, 5.0]),
                control_out: Some([20.0, -5.0]),
                pressure: 1.0,
            },
            PathPoint {
                position: [200.0, -50.0],
                control_in: Some([180.0, -30.0]),
                control_out: Some([220.0, -70.0]),
                pressure: 1.0,
            },
            PathPoint {
                position: [200.0, 50.0],
                control_in: Some([220.0, 70.0]),
                control_out: Some([180.0, 30.0]),
                pressure: 1.0,
            },
        ],
        closed: true,
    };

    let mut lighthouse_keyframes = BTreeMap::new();
    for &(frame, tower_rotation, light_alpha, tween) in &[
        (0u32, 0.0f32, 0.0f32, TweenType::EaseInOut),
        (60, 0.03, 0.0, TweenType::EaseInOut),
        (80, -0.02, 0.15, TweenType::EaseInOut),
        (120, 0.03, 0.3, TweenType::EaseInOut),
        (160, -0.02, 0.5, TweenType::EaseInOut),
        (200, 0.02, 0.6, TweenType::EaseInOut),
        (239, 0.0, 0.7, TweenType::None),
    ] {
        lighthouse_keyframes.insert(
            frame,
            Keyframe {
                objects: vec![
                    AnimObject {
                        id: lighthouse_base_id,
                        shape: Shape::Rectangle {
                            width: 50.0,
                            height: 20.0,
                            corner_radius: 4.0,
                        },
                        position: [300.0, 835.0],
                        rotation: 0.0,
                        scale: [1.0, 1.0],
                        fill: Paint::Solid([0.65, 0.6, 0.55, 1.0]),
                        stroke: Paint::Solid([0.4, 0.35, 0.3, 0.8]),
                        stroke_width: 1.5,
                    },
                    AnimObject {
                        id: lighthouse_tower_id,
                        shape: Shape::Rectangle {
                            width: 30.0,
                            height: 120.0,
                            corner_radius: 8.0,
                        },
                        position: [300.0, 770.0],
                        rotation: tower_rotation,
                        scale: [1.0, 1.0],
                        fill: Paint::Solid([0.85, 0.82, 0.75, 1.0]),
                        stroke: Paint::Solid([0.5, 0.2, 0.15, 0.8]),
                        stroke_width: 2.0,
                    },
                    AnimObject {
                        id: lighthouse_beam_id,
                        shape: lighthouse_beam_shape.clone(),
                        position: [312.0, 702.0],
                        rotation: 0.0,
                        scale: [1.0, 1.0],
                        fill: Paint::Solid([1.0, 0.95, 0.6, light_alpha * 0.85]),
                        stroke: Paint::Solid([0.0; 4]),
                        stroke_width: 0.0,
                    },
                    AnimObject {
                        id: lighthouse_lantern_id,
                        shape: Shape::Rectangle {
                            width: 24.0,
                            height: 18.0,
                            corner_radius: 3.0,
                        },
                        position: [300.0, 702.0],
                        rotation: tower_rotation,
                        scale: [1.0, 1.0],
                        fill: Paint::Solid([0.2, 0.18, 0.16, 1.0]),
                        stroke: Paint::Solid([0.12, 0.1, 0.08, 0.9]),
                        stroke_width: 1.5,
                    },
                    AnimObject {
                        id: lighthouse_light_id,
                        shape: Shape::Ellipse {
                            radius_x: 7.0,
                            radius_y: 7.0,
                        },
                        position: [300.0, 702.0],
                        rotation: 0.0,
                        scale: [1.0, 1.0],
                        fill: Paint::Solid([1.0, 0.95, 0.5, light_alpha]),
                        stroke: Paint::Solid([1.0, 0.9, 0.3, light_alpha * 0.6]),
                        stroke_width: 3.0,
                    },
                ],
                tween,
                label: String::new(),
                comment: String::new(),
                shape_tween: false,
            },
        );
    }

    let sailboat_sail = Shape::Path {
        points: vec![
            PathPoint {
                position: [0.0, 35.0],
                control_in: Some([15.0, 32.0]),
                control_out: Some([2.0, 15.0]),
                pressure: 1.0,
            },
            PathPoint {
                position: [0.0, -40.0],
                control_in: Some([-2.0, -20.0]),
                control_out: Some([25.0, -35.0]),
                pressure: 1.0,
            },
            PathPoint {
                position: [45.0, 10.0],
                control_in: Some([40.0, -15.0]),
                control_out: Some([35.0, 25.0]),
                pressure: 1.0,
            },
        ],
        closed: true,
    };

    let sailboat_hull = Shape::Path {
        points: vec![
            PathPoint {
                position: [-45.0, 0.0],
                control_in: Some([-40.0, -3.0]),
                control_out: Some([-30.0, 10.0]),
                pressure: 1.0,
            },
            PathPoint {
                position: [5.0, 15.0],
                control_in: Some([-15.0, 14.0]),
                control_out: Some([25.0, 14.0]),
                pressure: 1.0,
            },
            PathPoint {
                position: [45.0, 0.0],
                control_in: Some([40.0, 8.0]),
                control_out: Some([42.0, -3.0]),
                pressure: 1.0,
            },
        ],
        closed: true,
    };

    let mut sailboat_keyframes = BTreeMap::new();
    for &(frame, base_x, y_offset, rotation, brightness, tween) in &[
        (
            0u32,
            1500.0f32,
            0.0f32,
            0.0f32,
            1.0f32,
            TweenType::EaseInOut,
        ),
        (80, 1200.0, 0.0, 0.05, 0.8, TweenType::EaseInOut),
        (160, 900.0, 2.0, -0.03, 0.35, TweenType::EaseInOut),
        (239, 650.0, 3.0, 0.02, 0.1, TweenType::None),
    ] {
        let sail_fill = [0.95 * brightness, 0.95 * brightness, 0.9 * brightness, 0.9];
        let sail_stroke = [0.6 * brightness, 0.55 * brightness, 0.5 * brightness, 0.8];
        let hull_fill = [
            0.45 * brightness,
            0.28 * brightness,
            0.15 * brightness,
            0.95,
        ];
        let hull_stroke = [0.3 * brightness, 0.18 * brightness, 0.1 * brightness, 0.9];
        let mast_fill = [0.25 * brightness, 0.15 * brightness, 0.08 * brightness, 0.9];

        sailboat_keyframes.insert(
            frame,
            Keyframe {
                objects: vec![
                    AnimObject {
                        id: sailboat_hull_id,
                        shape: sailboat_hull.clone(),
                        position: [base_x, 828.0 + y_offset],
                        rotation,
                        scale: [1.0, 1.0],
                        fill: Paint::Solid(hull_fill),
                        stroke: Paint::Solid(hull_stroke),
                        stroke_width: 1.5,
                    },
                    AnimObject {
                        id: sailboat_mast_id,
                        shape: Shape::Rectangle {
                            width: 2.5,
                            height: 65.0,
                            corner_radius: 0.0,
                        },
                        position: [base_x, 793.0 + y_offset],
                        rotation,
                        scale: [1.0, 1.0],
                        fill: Paint::Solid(mast_fill),
                        stroke: Paint::Solid([0.0; 4]),
                        stroke_width: 0.0,
                    },
                    AnimObject {
                        id: sailboat_id,
                        shape: sailboat_sail.clone(),
                        position: [base_x + 5.0, 785.0 + y_offset],
                        rotation,
                        scale: [1.0, 1.0],
                        fill: Paint::Solid(sail_fill),
                        stroke: Paint::Solid(sail_stroke),
                        stroke_width: 1.5,
                    },
                ],
                tween,
                label: String::new(),
                comment: String::new(),
                shape_tween: false,
            },
        );
    }

    let waves_shape = Shape::Path {
        points: vec![
            PathPoint {
                position: [-200.0, 0.0],
                control_in: None,
                control_out: Some([-100.0, -18.0]),
                pressure: 1.0,
            },
            PathPoint {
                position: [40.0, 0.0],
                control_in: Some([-60.0, 18.0]),
                control_out: Some([140.0, -18.0]),
                pressure: 1.0,
            },
            PathPoint {
                position: [280.0, 0.0],
                control_in: Some([180.0, 18.0]),
                control_out: Some([380.0, -18.0]),
                pressure: 1.0,
            },
            PathPoint {
                position: [520.0, 0.0],
                control_in: Some([420.0, 18.0]),
                control_out: Some([620.0, -18.0]),
                pressure: 1.0,
            },
            PathPoint {
                position: [760.0, 0.0],
                control_in: Some([660.0, 18.0]),
                control_out: Some([860.0, -18.0]),
                pressure: 1.0,
            },
            PathPoint {
                position: [1000.0, 0.0],
                control_in: Some([900.0, 18.0]),
                control_out: Some([1100.0, -18.0]),
                pressure: 1.0,
            },
            PathPoint {
                position: [1240.0, 0.0],
                control_in: Some([1140.0, 18.0]),
                control_out: Some([1340.0, -18.0]),
                pressure: 1.0,
            },
            PathPoint {
                position: [1480.0, 0.0],
                control_in: Some([1380.0, 18.0]),
                control_out: Some([1580.0, -18.0]),
                pressure: 1.0,
            },
            PathPoint {
                position: [1720.0, 0.0],
                control_in: Some([1620.0, 18.0]),
                control_out: Some([1820.0, -18.0]),
                pressure: 1.0,
            },
            PathPoint {
                position: [2100.0, 0.0],
                control_in: Some([1960.0, 18.0]),
                control_out: None,
                pressure: 1.0,
            },
        ],
        closed: false,
    };

    let waves_keyframes = build_full_object_layer(
        waves_id,
        waves_shape,
        &[
            FullFrameSpec {
                frame: 0,
                position: [0.0, 850.0],
                fill: [0.0; 4],
                stroke: [0.3, 0.4, 0.6, 0.6],
                stroke_width: 2.0,
                scale: [1.0, 1.0],
                rotation: 0.0,
                tween: TweenType::Linear,
            },
            FullFrameSpec {
                frame: 80,
                position: [-40.0, 848.0],
                fill: [0.0; 4],
                stroke: [0.7, 0.5, 0.25, 0.7],
                stroke_width: 3.0,
                scale: [1.0, 1.2],
                rotation: 0.0,
                tween: TweenType::Linear,
            },
            FullFrameSpec {
                frame: 160,
                position: [-80.0, 850.0],
                fill: [0.0; 4],
                stroke: [0.4, 0.15, 0.25, 0.6],
                stroke_width: 3.5,
                scale: [1.0, 1.1],
                rotation: 0.0,
                tween: TweenType::Linear,
            },
            FullFrameSpec {
                frame: 239,
                position: [-120.0, 852.0],
                fill: [0.0; 4],
                stroke: [0.08, 0.06, 0.12, 0.4],
                stroke_width: 2.0,
                scale: [1.0, 0.8],
                rotation: 0.0,
                tween: TweenType::None,
            },
        ],
    );

    let moon_keyframes = build_full_object_layer(
        moon_id,
        Shape::Ellipse {
            radius_x: 35.0,
            radius_y: 30.0,
        },
        &[
            FullFrameSpec {
                frame: 0,
                position: [400.0, 900.0],
                fill: [0.9, 0.9, 0.85, 0.0],
                stroke: [0.8, 0.8, 0.95, 0.0],
                stroke_width: 0.0,
                scale: [0.3, 0.3],
                rotation: 0.0,
                tween: TweenType::None,
            },
            FullFrameSpec {
                frame: 140,
                position: [400.0, 900.0],
                fill: [0.9, 0.9, 0.85, 0.0],
                stroke: [0.8, 0.8, 0.95, 0.0],
                stroke_width: 0.0,
                scale: [0.3, 0.3],
                rotation: 0.0,
                tween: TweenType::EaseOut,
            },
            FullFrameSpec {
                frame: 200,
                position: [400.0, 180.0],
                fill: [0.95, 0.95, 0.9, 0.9],
                stroke: [0.8, 0.8, 0.95, 0.4],
                stroke_width: 4.0,
                scale: [0.9, 0.9],
                rotation: 0.15,
                tween: TweenType::EaseOut,
            },
            FullFrameSpec {
                frame: 239,
                position: [400.0, 150.0],
                fill: [0.95, 0.95, 0.9, 1.0],
                stroke: [0.7, 0.75, 0.95, 0.5],
                stroke_width: 6.0,
                scale: [1.0, 1.0],
                rotation: 0.2,
                tween: TweenType::None,
            },
        ],
    );

    let star_positions: [[f32; 2]; 3] = [[550.0, 80.0], [1500.0, 60.0], [900.0, 120.0]];
    let star_radii: [f32; 3] = [5.0, 4.0, 4.5];
    let make_stars =
        |fill: [f32; 4], stroke: [f32; 4], stroke_width: f32, scale: [f32; 2]| -> Vec<AnimObject> {
            (0..3)
                .map(|index| AnimObject {
                    id: star_ids[index],
                    shape: Shape::Ellipse {
                        radius_x: star_radii[index],
                        radius_y: star_radii[index],
                    },
                    position: star_positions[index],
                    rotation: 0.0,
                    scale,
                    fill: Paint::Solid(fill),
                    stroke: Paint::Solid(stroke),
                    stroke_width,
                })
                .collect()
        };

    let stars_keyframes = BTreeMap::from([
        (
            0,
            Keyframe {
                objects: make_stars([1.0, 1.0, 0.9, 0.0], [1.0, 1.0, 0.8, 0.0], 0.0, [0.1, 0.1]),
                tween: TweenType::None,
                label: String::new(),
                comment: String::new(),
                shape_tween: false,
            },
        ),
        (
            140,
            Keyframe {
                objects: make_stars([1.0, 1.0, 0.9, 0.0], [1.0, 1.0, 0.8, 0.0], 0.0, [0.1, 0.1]),
                tween: TweenType::EaseOut,
                label: String::new(),
                comment: String::new(),
                shape_tween: false,
            },
        ),
        (
            200,
            Keyframe {
                objects: make_stars([1.0, 1.0, 0.9, 0.9], [0.8, 0.85, 1.0, 0.4], 3.0, [1.0, 1.0]),
                tween: TweenType::None,
                label: String::new(),
                comment: String::new(),
                shape_tween: false,
            },
        ),
        (
            239,
            Keyframe {
                objects: make_stars([1.0, 1.0, 0.9, 1.0], [0.8, 0.85, 1.0, 0.5], 4.0, [1.0, 1.0]),
                tween: TweenType::None,
                label: String::new(),
                comment: String::new(),
                shape_tween: false,
            },
        ),
    ]);

    let make_layer = |name: &str, opacity: f32, keyframes: BTreeMap<u32, Keyframe>| -> Layer {
        Layer {
            id: uuid::Uuid::new_v4(),
            name: name.to_string(),
            visible: true,
            locked: false,
            opacity,
            keyframes,
            layer_type: LayerType::Normal,
            blend_mode: BlendMode::Normal,
            parent_id: None,
            collapsed: false,
            property_tracks: HashMap::new(),
        }
    };

    Project {
        name: "Sunset Showcase".to_string(),
        canvas_width,
        canvas_height,
        background_color: [0.0, 0.0, 0.0, 1.0],
        frame_rate,
        total_frames,
        layers: vec![
            make_layer("Stars", 1.0, stars_keyframes),
            make_layer("Moon", 1.0, moon_keyframes),
            make_layer("Waves", 0.7, waves_keyframes),
            make_layer("Sailboat", 1.0, sailboat_keyframes),
            make_layer("Lighthouse", 1.0, lighthouse_keyframes),
            make_layer("Horizon", 1.0, horizon_keyframes),
            make_layer("Sun Reflection", 0.6, reflection_keyframes),
            make_layer("Sun", 1.0, sun_keyframes),
            make_layer("Water", 0.85, water_keyframes),
            make_layer("Sky", 1.0, sky_keyframes),
        ],
        guides: Vec::new(),
        camera_keyframes: BTreeMap::new(),
        camera_tween: TweenType::None,
        scenes: Vec::new(),
        active_scene: 0,
        library: Library::default(),
        image_assets: Vec::new(),
        loop_start: None,
        loop_end: None,
        audio_tracks: Vec::new(),
        armatures: Vec::new(),
    }
}

struct FullFrameSpec {
    frame: u32,
    position: [f32; 2],
    fill: [f32; 4],
    stroke: [f32; 4],
    stroke_width: f32,
    scale: [f32; 2],
    rotation: f32,
    tween: TweenType,
}

fn build_full_object_layer(
    id: uuid::Uuid,
    shape: Shape,
    frames: &[FullFrameSpec],
) -> BTreeMap<u32, Keyframe> {
    let mut keyframes = BTreeMap::new();
    for spec in frames {
        keyframes.insert(
            spec.frame,
            Keyframe {
                objects: vec![AnimObject {
                    id,
                    shape: shape.clone(),
                    position: spec.position,
                    rotation: spec.rotation,
                    scale: spec.scale,
                    fill: Paint::Solid(spec.fill),
                    stroke: Paint::Solid(spec.stroke),
                    stroke_width: spec.stroke_width,
                }],
                tween: spec.tween,
                label: String::new(),
                comment: String::new(),
                shape_tween: false,
            },
        );
    }
    keyframes
}
