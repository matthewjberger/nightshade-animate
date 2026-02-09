use nightshade::prelude::*;

use crate::app::AnimateApp;
#[cfg(not(target_arch = "wasm32"))]
use crate::io;
use crate::playback;
use crate::project::Project;
use crate::timeline;
use crate::tween;

pub fn draw_menu_bar(app: &mut AnimateApp, ui_context: &egui::Context) {
    egui::TopBottomPanel::top("menu_bar").show(ui_context, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New").clicked() {
                    app.history.push(app.project.clone());
                    app.project = Project::default();
                    app.current_frame = 0;
                    app.active_layer = 0;
                    app.selection.selected_objects.clear();
                    app.save_path = None;
                    ui.close();
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if ui.button("Open...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Animation", &["anim"])
                            .add_filter("All Files", &["*"])
                            .set_title("Open Project")
                            .pick_file()
                            && let Ok(project) = io::load_project(&path)
                        {
                            app.history.push(app.project.clone());
                            app.project = project;
                            app.current_frame = 0;
                            app.active_layer = 0;
                            app.selection.selected_objects.clear();
                            app.save_path = Some(path);
                        }
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Save").clicked() {
                        if let Some(path) = app.save_path.clone() {
                            let _ = io::save_project(&app.project, &path);
                        } else {
                            save_as(app);
                        }
                        ui.close();
                    }
                    if ui.button("Save As...").clicked() {
                        save_as(app);
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Export PNG Sequence...").clicked() {
                        if let Some(folder) = rfd::FileDialog::new()
                            .set_title("Export PNG Sequence")
                            .pick_folder()
                        {
                            crate::export::export_png_sequence(&app.project, &folder);
                        }
                        ui.close();
                    }
                    if ui.button("Export GIF...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("GIF Image", &["gif"])
                            .set_file_name("animation.gif")
                            .set_title("Export GIF")
                            .save_file()
                        {
                            crate::export::export_gif(&app.project, &path);
                        }
                        ui.close();
                    }
                    if ui.button("Export Sprite Sheet...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("PNG Image", &["png"])
                            .set_file_name("spritesheet.png")
                            .set_title("Export Sprite Sheet")
                            .save_file()
                        {
                            crate::export::export_sprite_sheet(&app.project, &path);
                        }
                        ui.close();
                    }
                }
                #[cfg(target_arch = "wasm32")]
                {
                    if ui.button("Open...").clicked() {
                        wasm_load_project(app);
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Save").clicked() {
                        wasm_save_project(app);
                        ui.close();
                    }
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.button("Undo (Ctrl+Z)").clicked() {
                    app.history.undo(&mut app.project);
                    ui.close();
                }
                if ui.button("Redo (Ctrl+Shift+Z)").clicked() {
                    app.history.redo(&mut app.project);
                    ui.close();
                }
                ui.separator();
                if ui.button("Select All (Ctrl+A)").clicked() {
                    select_all(app);
                    ui.close();
                }
                if ui.button("Delete (Del)").clicked() {
                    delete_selected(app);
                    ui.close();
                }
            });
            ui.menu_button("View", |ui| {
                let onion_label = if app.onion.enabled {
                    "Onion Skinning [ON]"
                } else {
                    "Onion Skinning [OFF]"
                };
                if ui.button(onion_label).clicked() {
                    app.onion.enabled = !app.onion.enabled;
                    ui.close();
                }
                if ui.button("Reset Zoom").clicked() {
                    app.canvas_view.zoom = 0.5;
                    app.canvas_view.pan = egui::Vec2::ZERO;
                    ui.close();
                }
            });
            ui.menu_button("Insert", |ui| {
                if ui.button("Keyframe (F6)").clicked() {
                    timeline::insert_keyframe(app);
                    ui.close();
                }
                if ui.button("Blank Keyframe (F7)").clicked() {
                    timeline::insert_blank_keyframe(app);
                    ui.close();
                }
                if ui.button("Delete Keyframe (Shift+F6)").clicked() {
                    timeline::delete_keyframe(app);
                    ui.close();
                }
                ui.separator();
                if ui.button("Generate Test Animation").clicked() {
                    app.history.push(app.project.clone());
                    app.project = crate::test_animation::generate_bouncing_ball();
                    app.current_frame = 0;
                    app.active_layer = 0;
                    app.selection.selected_objects.clear();
                    app.save_path = None;
                    ui.close();
                }
                if ui.button("Generate Showcase Animation").clicked() {
                    app.history.push(app.project.clone());
                    app.project = crate::test_animation::generate_showcase_animation();
                    app.current_frame = 0;
                    app.active_layer = 0;
                    app.selection.selected_objects.clear();
                    app.save_path = None;
                    ui.close();
                }
            });
            ui.menu_button("Playback", |ui| {
                let play_label = if app.playback.playing {
                    "Pause"
                } else {
                    "Play"
                };
                if ui.button(format!("{} (Space)", play_label)).clicked() {
                    playback::toggle_playback(app);
                    ui.close();
                }
                if ui.button("Go to Start").clicked() {
                    app.current_frame = 0;
                    ui.close();
                }
                if ui.button("Go to End").clicked() {
                    app.current_frame = app.project.total_frames.saturating_sub(1);
                    ui.close();
                }
            });
        });
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn save_as(app: &mut AnimateApp) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Animation", &["anim"])
        .set_file_name("project.anim")
        .set_title("Save Project As")
        .save_file()
    {
        let _ = io::save_project(&app.project, &path);
        app.save_path = Some(path);
    }
}

#[cfg(target_arch = "wasm32")]
fn wasm_save_project(app: &AnimateApp) {
    use wasm_bindgen::JsCast;

    let json = match serde_json::to_string_pretty(&app.project) {
        Ok(json) => json,
        Err(_) => return,
    };

    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };

    let array = js_sys::Array::new();
    array.push(&wasm_bindgen::JsValue::from_str(&json));

    let options = web_sys::BlobPropertyBag::new();
    options.set_type("application/json");

    let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&array, &options) else {
        return;
    };

    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
        return;
    };

    let Some(anchor) = document
        .create_element("a")
        .ok()
        .and_then(|element| element.dyn_into::<web_sys::HtmlAnchorElement>().ok())
    else {
        return;
    };

    anchor.set_href(&url);
    anchor.set_download("project.anim");
    anchor.click();
    let _ = web_sys::Url::revoke_object_url(&url);
}

#[cfg(target_arch = "wasm32")]
fn wasm_load_project(app: &AnimateApp) {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;

    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };

    let Some(input) = document
        .create_element("input")
        .ok()
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
    else {
        return;
    };

    input.set_type("file");
    input.set_accept(".anim");

    let pending = app.pending_project_load.clone();
    let input_for_closure = input.clone();
    let closure = Closure::once(Box::new(move |_event: web_sys::Event| {
        let input_clone = input_for_closure;
        let pending_clone = pending;

        wasm_bindgen_futures::spawn_local(async move {
            if let Some(files) = input_clone.files()
                && let Some(file) = files.get(0)
            {
                let array_buffer_promise = file.array_buffer();
                if let Ok(array_buffer) =
                    wasm_bindgen_futures::JsFuture::from(array_buffer_promise).await
                {
                    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
                    let bytes = uint8_array.to_vec();
                    *pending_clone.borrow_mut() = Some(bytes);
                }
            }
        });
    }) as Box<dyn FnOnce(_)>);

    input.set_onchange(Some(closure.as_ref().unchecked_ref()));
    closure.forget();
    input.click();
}

#[cfg(target_arch = "wasm32")]
pub fn process_pending_load(app: &mut AnimateApp) {
    let data = app.pending_project_load.borrow_mut().take();
    if let Some(bytes) = data
        && let Ok(json) = std::str::from_utf8(&bytes)
        && let Ok(project) = serde_json::from_str::<crate::project::Project>(json)
    {
        app.history.push(app.project.clone());
        app.project = project;
        app.current_frame = 0;
        app.active_layer = 0;
        app.selection.selected_objects.clear();
        app.save_path = None;
    }
}

pub fn select_all(app: &mut AnimateApp) {
    app.selection.selected_objects.clear();
    for layer in &app.project.layers {
        if !layer.visible || layer.locked {
            continue;
        }
        if let Some(objects) = tween::resolve_frame(layer, app.current_frame) {
            for object in &objects {
                app.selection.selected_objects.push(object.id);
            }
        }
    }
}

pub fn delete_selected(app: &mut AnimateApp) {
    if app.selection.selected_objects.is_empty() {
        return;
    }

    app.history.push(app.project.clone());
    let selected = app.selection.selected_objects.clone();
    for layer in &mut app.project.layers {
        let has_selected = tween::resolve_frame(layer, app.current_frame)
            .map(|objects| objects.iter().any(|object| selected.contains(&object.id)))
            .unwrap_or(false);

        if has_selected {
            tween::ensure_keyframe_at(layer, app.current_frame);
        }

        if let Some(keyframe) = layer.keyframes.get_mut(&app.current_frame) {
            keyframe
                .objects
                .retain(|object| !selected.contains(&object.id));
        }
    }
    app.selection.selected_objects.clear();
}

pub fn handle_global_shortcuts(app: &mut AnimateApp, ui_context: &egui::Context) {
    if ui_context.wants_keyboard_input() {
        return;
    }
    ui_context.input(|input| {
        if input.modifiers.ctrl && !input.modifiers.shift && input.key_pressed(egui::Key::Z) {
            app.history.undo(&mut app.project);
        }
        if input.modifiers.ctrl && input.modifiers.shift && input.key_pressed(egui::Key::Z) {
            app.history.redo(&mut app.project);
        }
        if input.modifiers.ctrl && input.key_pressed(egui::Key::A) {
            select_all(app);
        }
        if input.key_pressed(egui::Key::Delete) {
            delete_selected(app);
        }
        if input.key_pressed(egui::Key::O) && !input.modifiers.ctrl {
            app.onion.enabled = !app.onion.enabled;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if input.modifiers.ctrl && !input.modifiers.shift && input.key_pressed(egui::Key::S) {
                if let Some(path) = app.save_path.clone() {
                    let _ = io::save_project(&app.project, &path);
                } else {
                    save_as(app);
                }
            }
            if input.modifiers.ctrl && input.modifiers.shift && input.key_pressed(egui::Key::S) {
                save_as(app);
            }
            if input.modifiers.ctrl
                && input.key_pressed(egui::Key::O)
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("Animation", &["anim"])
                    .add_filter("All Files", &["*"])
                    .set_title("Open Project")
                    .pick_file()
                && let Ok(project) = io::load_project(&path)
            {
                app.history.push(app.project.clone());
                app.project = project;
                app.current_frame = 0;
                app.active_layer = 0;
                app.selection.selected_objects.clear();
                app.save_path = Some(path);
            }
        }
    });
}
