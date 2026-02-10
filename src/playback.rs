use nightshade::prelude::*;

use crate::app::AnimateApp;

pub struct PlaybackState {
    pub playing: bool,
    pub accumulator: f64,
    pub last_instant: Option<Instant>,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            playing: false,
            accumulator: 0.0,
            last_instant: None,
        }
    }
}

pub fn advance_playback(app: &mut AnimateApp) {
    if !app.playback.playing {
        app.playback.last_instant = None;
        return;
    }

    let now = Instant::now();
    if let Some(last) = app.playback.last_instant {
        let delta = now.duration_since(last).as_secs_f64();
        app.playback.accumulator += delta;

        let frame_duration = 1.0 / app.project.frame_rate as f64;
        while app.playback.accumulator >= frame_duration {
            app.playback.accumulator -= frame_duration;
            app.current_frame += 1;

            if let (Some(loop_start), Some(loop_end)) =
                (app.project.loop_start, app.project.loop_end)
            {
                if app.current_frame > loop_end {
                    app.current_frame = loop_start;
                }
            } else if app.current_frame >= app.project.total_frames {
                app.current_frame = 0;
            }
        }
    }
    app.playback.last_instant = Some(now);
}

pub fn toggle_playback(app: &mut AnimateApp) {
    app.playback.playing = !app.playback.playing;
    if !app.playback.playing {
        app.playback.last_instant = None;
        app.playback.accumulator = 0.0;
    }
    #[cfg(not(target_arch = "wasm32"))]
    crate::audio::sync_playback_state(app.playback.playing, app);
}

pub fn handle_playback_shortcuts(app: &mut AnimateApp, ui_context: &egui::Context) {
    if ui_context.wants_keyboard_input() {
        return;
    }
    ui_context.input(|input| {
        if input.key_pressed(egui::Key::Space) {
            toggle_playback(app);
        }
    });
}
