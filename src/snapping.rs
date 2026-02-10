use nightshade::prelude::*;

use crate::app::AnimateApp;
use crate::selection;
use crate::tween;

const SNAP_THRESHOLD: f32 = 8.0;

pub struct SnapResult {
    pub position: [f32; 2],
    pub snapped_x: bool,
    pub snapped_y: bool,
    pub snap_line_x: Option<f32>,
    pub snap_line_y: Option<f32>,
}

pub fn snap_point(app: &AnimateApp, position: [f32; 2], exclude_ids: &[uuid::Uuid]) -> SnapResult {
    let mut result = SnapResult {
        position,
        snapped_x: false,
        snapped_y: false,
        snap_line_x: None,
        snap_line_y: None,
    };

    let threshold = SNAP_THRESHOLD / app.canvas_view.zoom;

    if app.snap_to_grid {
        let grid = app.grid_size;
        let snapped_x = (position[0] / grid).round() * grid;
        let snapped_y = (position[1] / grid).round() * grid;

        if (position[0] - snapped_x).abs() < threshold {
            result.position[0] = snapped_x;
            result.snapped_x = true;
            result.snap_line_x = Some(snapped_x);
        }
        if (position[1] - snapped_y).abs() < threshold {
            result.position[1] = snapped_y;
            result.snapped_y = true;
            result.snap_line_y = Some(snapped_y);
        }
    }

    if app.snap_to_guides {
        for guide in &app.project.guides {
            match guide.orientation {
                crate::project::GuideOrientation::Vertical => {
                    if !result.snapped_x && (position[0] - guide.position).abs() < threshold {
                        result.position[0] = guide.position;
                        result.snapped_x = true;
                        result.snap_line_x = Some(guide.position);
                    }
                }
                crate::project::GuideOrientation::Horizontal => {
                    if !result.snapped_y && (position[1] - guide.position).abs() < threshold {
                        result.position[1] = guide.position;
                        result.snapped_y = true;
                        result.snap_line_y = Some(guide.position);
                    }
                }
            }
        }
    }

    if app.snap_to_objects {
        for layer in &app.project.layers {
            if !layer.visible || layer.locked {
                continue;
            }
            if let Some(objects) = tween::resolve_frame(layer, app.current_frame) {
                for object in &objects {
                    if exclude_ids.contains(&object.id) {
                        continue;
                    }
                    let (half_w, half_h, offset) = selection::get_object_bounds_public(object);
                    let cx = object.position[0] + offset[0];
                    let cy = object.position[1] + offset[1];

                    let snap_xs = [cx - half_w, cx, cx + half_w];
                    let snap_ys = [cy - half_h, cy, cy + half_h];

                    if !result.snapped_x {
                        for snap_x in &snap_xs {
                            if (position[0] - snap_x).abs() < threshold {
                                result.position[0] = *snap_x;
                                result.snapped_x = true;
                                result.snap_line_x = Some(*snap_x);
                                break;
                            }
                        }
                    }

                    if !result.snapped_y {
                        for snap_y in &snap_ys {
                            if (position[1] - snap_y).abs() < threshold {
                                result.position[1] = *snap_y;
                                result.snapped_y = true;
                                result.snap_line_y = Some(*snap_y);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    if !result.snapped_x && (position[0]).abs() < threshold {
        result.position[0] = 0.0;
        result.snapped_x = true;
        result.snap_line_x = Some(0.0);
    }
    if !result.snapped_y && (position[1]).abs() < threshold {
        result.position[1] = 0.0;
        result.snapped_y = true;
        result.snap_line_y = Some(0.0);
    }

    let canvas_w = app.project.canvas_width as f32;
    let canvas_h = app.project.canvas_height as f32;
    if !result.snapped_x && (position[0] - canvas_w).abs() < threshold {
        result.position[0] = canvas_w;
        result.snapped_x = true;
        result.snap_line_x = Some(canvas_w);
    }
    if !result.snapped_y && (position[1] - canvas_h).abs() < threshold {
        result.position[1] = canvas_h;
        result.snapped_y = true;
        result.snap_line_y = Some(canvas_h);
    }

    result
}
