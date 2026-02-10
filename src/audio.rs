use std::io::Cursor;
use std::sync::Mutex;

use nightshade::prelude::*;
use rodio::Source;

use crate::app::AnimateApp;
use crate::project::AudioTrack;

struct AudioPlaybackState {
    _stream: rodio::OutputStream,
    sink: rodio::Sink,
}

static AUDIO_STATE: Mutex<Option<AudioPlaybackState>> = Mutex::new(None);

pub fn import_audio(app: &mut AnimateApp) {
    let Some(path) = rfd::FileDialog::new()
        .add_filter("Audio Files", &["wav", "mp3", "ogg", "flac"])
        .set_title("Import Audio")
        .pick_file()
    else {
        return;
    };

    let Ok(data) = std::fs::read(&path) else {
        return;
    };

    let name = path
        .file_name()
        .map(|os_str| os_str.to_string_lossy().to_string())
        .unwrap_or_else(|| "audio".to_string());

    app.history.push(app.project.clone());

    app.project.audio_tracks.push(AudioTrack {
        id: uuid::Uuid::new_v4(),
        name,
        data,
        start_frame: 0,
        volume: 1.0,
    });
}

pub fn start_audio_playback(app: &AnimateApp) {
    stop_audio_playback();

    if app.project.audio_tracks.is_empty() {
        return;
    }

    let track = &app.project.audio_tracks[0];

    let Ok(stream) = rodio::OutputStreamBuilder::open_default_stream() else {
        return;
    };

    let sink = rodio::Sink::connect_new(stream.mixer());

    let cursor = Cursor::new(track.data.clone());
    let Ok(source) = rodio::Decoder::new(cursor) else {
        return;
    };

    let frame_offset = app.current_frame.saturating_sub(track.start_frame);
    let seconds_offset = frame_offset as f64 / app.project.frame_rate as f64;

    sink.set_volume(track.volume);
    sink.append(source);

    if seconds_offset > 0.0 {
        sink.try_seek(std::time::Duration::from_secs_f64(seconds_offset))
            .ok();
    }

    let state = AudioPlaybackState {
        _stream: stream,
        sink,
    };

    if let Ok(mut guard) = AUDIO_STATE.lock() {
        *guard = Some(state);
    }
}

pub fn stop_audio_playback() {
    if let Ok(mut guard) = AUDIO_STATE.lock()
        && let Some(state) = guard.take()
    {
        state.sink.stop();
    }
}

pub fn sync_playback_state(playing: bool, app: &AnimateApp) {
    if playing {
        let has_audio = AUDIO_STATE
            .lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false);
        if !has_audio {
            start_audio_playback(app);
        }
    } else {
        stop_audio_playback();
    }
}

pub fn draw_audio_tracks(app: &mut AnimateApp, ui: &mut egui::Ui) {
    if app.project.audio_tracks.is_empty() {
        return;
    }

    ui.separator();

    let mut remove_index = None;

    for track_index in 0..app.project.audio_tracks.len() {
        let track_name = app.project.audio_tracks[track_index].name.clone();
        let track_volume = app.project.audio_tracks[track_index].volume;
        let track_start = app.project.audio_tracks[track_index].start_frame;

        ui.horizontal(|ui| {
            ui.label(&track_name);

            let mut volume = track_volume;
            ui.label("Vol:");
            if ui
                .add(
                    egui::DragValue::new(&mut volume)
                        .speed(0.01)
                        .range(0.0..=2.0),
                )
                .changed()
            {
                app.project.audio_tracks[track_index].volume = volume;
            }

            let mut start = track_start as f32;
            ui.label("Start:");
            if ui
                .add(
                    egui::DragValue::new(&mut start)
                        .speed(1.0)
                        .range(0.0..=10000.0),
                )
                .changed()
            {
                app.project.audio_tracks[track_index].start_frame = start as u32;
            }

            if ui.small_button("X").on_hover_text("Remove track").clicked() {
                remove_index = Some(track_index);
            }
        });
    }

    if let Some(index) = remove_index {
        app.history.push(app.project.clone());
        app.project.audio_tracks.remove(index);
    }
}

pub fn draw_audio_waveform(
    app: &AnimateApp,
    painter: &egui::Painter,
    timeline_rect: egui::Rect,
    frame_cell_width: f32,
    scroll_offset: f32,
) {
    for track in &app.project.audio_tracks {
        let waveform_height = 20.0;
        let waveform_top = timeline_rect.max.y - waveform_height - 2.0;

        let samples = extract_waveform_samples(&track.data, 1024);
        if samples.is_empty() {
            continue;
        }

        let sample_rate = estimate_sample_rate(&track.data);
        let total_duration_seconds = samples.len() as f32 / sample_rate;
        let total_frames = total_duration_seconds * app.project.frame_rate as f32;

        let waveform_color = egui::Color32::from_rgba_unmultiplied(100, 150, 255, 120);

        for sample_index in 0..samples.len() {
            let frame_pos = track.start_frame as f32
                + (sample_index as f32 / samples.len() as f32) * total_frames;
            let x = timeline_rect.min.x + (frame_pos - scroll_offset) * frame_cell_width;

            if x < timeline_rect.min.x || x > timeline_rect.max.x {
                continue;
            }

            let amplitude = samples[sample_index].abs();
            let bar_height = amplitude * waveform_height;
            let bar_top = waveform_top + (waveform_height - bar_height) / 2.0;

            painter.line_segment(
                [egui::pos2(x, bar_top), egui::pos2(x, bar_top + bar_height)],
                egui::Stroke::new(1.0, waveform_color),
            );
        }
    }
}

fn extract_waveform_samples(data: &[u8], target_count: usize) -> Vec<f32> {
    let cursor = Cursor::new(data.to_vec());
    let Ok(source) = rodio::Decoder::new(cursor) else {
        return Vec::new();
    };

    let all_samples: Vec<f32> = source.map(|sample| sample / i16::MAX as f32).collect();

    if all_samples.is_empty() {
        return Vec::new();
    }

    let chunk_size = (all_samples.len() / target_count).max(1);
    all_samples
        .chunks(chunk_size)
        .map(|chunk| {
            chunk
                .iter()
                .fold(0.0_f32, |max_val, &sample| max_val.max(sample.abs()))
        })
        .collect()
}

fn estimate_sample_rate(data: &[u8]) -> f32 {
    let cursor = Cursor::new(data.to_vec());
    if let Ok(source) = rodio::Decoder::new(cursor) {
        source.sample_rate() as f32
    } else {
        44100.0
    }
}
