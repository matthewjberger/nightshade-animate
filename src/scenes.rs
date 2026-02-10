use nightshade::prelude::*;

use crate::app::AnimateApp;
use crate::project::{Keyframe, Layer, Scene};

pub fn draw_scene_tabs(app: &mut AnimateApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("Scenes:");

        if app.project.scenes.is_empty() {
            if ui.small_button("Enable Scenes").clicked() {
                let main_scene = Scene {
                    id: uuid::Uuid::new_v4(),
                    name: "Scene 1".to_string(),
                    layers: Vec::new(),
                    total_frames: app.project.total_frames,
                };
                app.project.scenes.push(main_scene);
                app.project.active_scene = 0;
            }
            return;
        }

        let mut switch_to: Option<usize> = None;

        for scene_index in 0..app.project.scenes.len() {
            let is_active = scene_index == app.project.active_scene;
            let label = app.project.scenes[scene_index].name.clone();
            if ui.selectable_label(is_active, &label).clicked() && !is_active {
                switch_to = Some(scene_index);
            }
        }

        if ui.small_button("+").on_hover_text("Add scene").clicked() {
            add_scene(app);
        }

        if app.project.scenes.len() > 1
            && ui
                .small_button("-")
                .on_hover_text("Delete current scene")
                .clicked()
        {
            delete_current_scene(app);
        }

        if ui
            .small_button("D")
            .on_hover_text("Duplicate scene")
            .clicked()
        {
            duplicate_current_scene(app);
        }

        if let Some(target) = switch_to {
            switch_scene(app, target);
        }
    });
}

fn switch_scene(app: &mut AnimateApp, target_index: usize) {
    let current = app.project.active_scene;

    app.project.scenes[current].layers = app.project.layers.clone();
    app.project.scenes[current].total_frames = app.project.total_frames;

    app.project.layers = app.project.scenes[target_index].layers.clone();
    app.project.total_frames = app.project.scenes[target_index].total_frames;
    app.project.active_scene = target_index;

    app.current_frame = 0;
    app.active_layer = 0;
    app.selection.selected_objects.clear();
}

fn add_scene(app: &mut AnimateApp) {
    app.history.push(app.project.clone());

    app.project.scenes[app.project.active_scene].layers = app.project.layers.clone();
    app.project.scenes[app.project.active_scene].total_frames = app.project.total_frames;

    let name = format!("Scene {}", app.project.scenes.len() + 1);
    let mut default_layer = Layer::new("Layer 1".to_string());
    default_layer.keyframes.insert(0, Keyframe::default());

    let new_scene = Scene {
        id: uuid::Uuid::new_v4(),
        name,
        layers: vec![default_layer.clone()],
        total_frames: 120,
    };

    let new_index = app.project.scenes.len();
    app.project.scenes.push(new_scene);

    app.project.layers = vec![default_layer];
    app.project.total_frames = 120;
    app.project.active_scene = new_index;

    app.current_frame = 0;
    app.active_layer = 0;
    app.selection.selected_objects.clear();
}

fn delete_current_scene(app: &mut AnimateApp) {
    if app.project.scenes.len() <= 1 {
        return;
    }

    app.history.push(app.project.clone());

    app.project.scenes.remove(app.project.active_scene);

    let new_active = if app.project.active_scene >= app.project.scenes.len() {
        app.project.scenes.len() - 1
    } else {
        app.project.active_scene
    };

    app.project.active_scene = new_active;
    app.project.layers = app.project.scenes[new_active].layers.clone();
    app.project.total_frames = app.project.scenes[new_active].total_frames;

    app.current_frame = 0;
    app.active_layer = 0;
    app.selection.selected_objects.clear();
}

fn duplicate_current_scene(app: &mut AnimateApp) {
    app.history.push(app.project.clone());

    app.project.scenes[app.project.active_scene].layers = app.project.layers.clone();
    app.project.scenes[app.project.active_scene].total_frames = app.project.total_frames;

    let source = &app.project.scenes[app.project.active_scene];
    let mut duplicated = source.clone();
    duplicated.id = uuid::Uuid::new_v4();
    duplicated.name = format!("{} (copy)", source.name);

    let new_index = app.project.scenes.len();
    app.project.scenes.push(duplicated.clone());

    app.project.layers = duplicated.layers;
    app.project.total_frames = duplicated.total_frames;
    app.project.active_scene = new_index;

    app.current_frame = 0;
    app.active_layer = 0;
    app.selection.selected_objects.clear();
}
