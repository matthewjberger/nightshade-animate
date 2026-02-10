use nightshade::prelude::*;

use crate::app::AnimateApp;
use crate::canvas::{CanvasView, render_object};
use crate::paint::Paint;
use crate::tween;

pub struct OnionSkinning {
    pub enabled: bool,
    pub frames_before: u32,
    pub frames_after: u32,
}

impl Default for OnionSkinning {
    fn default() -> Self {
        Self {
            enabled: false,
            frames_before: 2,
            frames_after: 2,
        }
    }
}

pub fn draw_onion_skins(app: &AnimateApp, view: &CanvasView, painter: &egui::Painter) {
    if !app.onion.enabled {
        return;
    }

    for offset in 1..=app.onion.frames_before {
        if app.current_frame < offset {
            continue;
        }
        let frame = app.current_frame - offset;
        let alpha = 0.3 / offset as f32;
        draw_ghost_frame(app, view, painter, frame, [1.0, 0.3, 0.3, alpha]);
    }

    for offset in 1..=app.onion.frames_after {
        let frame = app.current_frame + offset;
        if frame >= app.project.total_frames {
            continue;
        }
        let alpha = 0.3 / offset as f32;
        draw_ghost_frame(app, view, painter, frame, [0.3, 1.0, 0.3, alpha]);
    }
}

fn draw_ghost_frame(
    app: &AnimateApp,
    view: &CanvasView,
    painter: &egui::Painter,
    frame: u32,
    tint: [f32; 4],
) {
    for layer_index in (0..app.project.layers.len()).rev() {
        let layer = &app.project.layers[layer_index];
        if !layer.visible {
            continue;
        }

        if let Some(objects) = tween::resolve_frame(layer, frame) {
            for object in &objects {
                let mut tinted = object.clone();
                let original_fill = object.fill.as_solid();
                let original_stroke = object.stroke.as_solid();
                tinted.fill = Paint::Solid([
                    tint[0] * original_fill[0],
                    tint[1] * original_fill[1],
                    tint[2] * original_fill[2],
                    tint[3],
                ]);
                tinted.stroke = Paint::Solid([
                    tint[0] * original_stroke[0],
                    tint[1] * original_stroke[1],
                    tint[2] * original_stroke[2],
                    tint[3],
                ]);
                render_object(&tinted, view, painter, layer.opacity, None);
            }
        }
    }
}
