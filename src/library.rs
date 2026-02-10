use nightshade::prelude::*;

use crate::app::AnimateApp;
use crate::paint::Paint;
use crate::project::{AnimObject, Keyframe, Layer, Shape, Symbol, TweenType};
use crate::tween;

pub fn convert_selection_to_symbol(app: &mut AnimateApp) {
    if app.selection.selected_objects.is_empty() {
        return;
    }

    let selected_ids = app.selection.selected_objects.clone();
    let mut collected_objects = Vec::new();

    for layer in &app.project.layers {
        if let Some(objects) = tween::resolve_frame(layer, app.current_frame) {
            for object in &objects {
                if selected_ids.contains(&object.id) {
                    collected_objects.push(object.clone());
                }
            }
        }
    }

    if collected_objects.is_empty() {
        return;
    }

    app.history.push(app.project.clone());

    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for object in &collected_objects {
        min_x = min_x.min(object.position[0]);
        min_y = min_y.min(object.position[1]);
        max_x = max_x.max(object.position[0]);
        max_y = max_y.max(object.position[1]);
    }

    let center_x = (min_x + max_x) / 2.0;
    let center_y = (min_y + max_y) / 2.0;
    let symbol_width = (max_x - min_x).max(1.0);
    let symbol_height = (max_y - min_y).max(1.0);

    let mut symbol_objects = Vec::new();
    for mut object in collected_objects {
        object.position[0] -= center_x;
        object.position[1] -= center_y;
        object.id = uuid::Uuid::new_v4();
        symbol_objects.push(object);
    }

    let symbol_id = uuid::Uuid::new_v4();
    let symbol_name = format!("Symbol {}", app.project.library.symbols.len() + 1);

    let mut symbol_layer = Layer::new("Layer 1".to_string());
    symbol_layer.keyframes.insert(
        0,
        Keyframe {
            objects: symbol_objects,
            tween: TweenType::None,
            label: String::new(),
            comment: String::new(),
            shape_tween: false,
        },
    );

    let symbol = Symbol {
        id: symbol_id,
        name: symbol_name,
        layers: vec![symbol_layer],
        total_frames: 1,
        width: symbol_width,
        height: symbol_height,
    };

    app.project.library.symbols.push(symbol);

    for layer in &mut app.project.layers {
        tween::ensure_keyframe_at(layer, app.current_frame);
        if let Some(keyframe) = layer.keyframes.get_mut(&app.current_frame) {
            keyframe
                .objects
                .retain(|object| !selected_ids.contains(&object.id));
        }
    }

    let instance = AnimObject::new(
        Shape::SymbolInstance { symbol_id },
        [center_x, center_y],
        Paint::Solid([1.0, 1.0, 1.0, 1.0]),
        Paint::Solid([0.0, 0.0, 0.0, 0.0]),
        0.0,
    );

    let instance_id = instance.id;

    if let Some(keyframe) = app.project.layers[app.active_layer]
        .keyframes
        .get_mut(&app.current_frame)
    {
        keyframe.objects.push(instance);
    }

    app.selection.selected_objects.clear();
    app.selection.selected_objects.push(instance_id);
}

pub fn instantiate_symbol(app: &mut AnimateApp, symbol_id: uuid::Uuid) {
    app.history.push(app.project.clone());

    tween::ensure_keyframe_at(&mut app.project.layers[app.active_layer], app.current_frame);

    let instance = AnimObject::new(
        Shape::SymbolInstance { symbol_id },
        [
            app.project.canvas_width as f32 / 2.0,
            app.project.canvas_height as f32 / 2.0,
        ],
        Paint::Solid([1.0, 1.0, 1.0, 1.0]),
        Paint::Solid([0.0, 0.0, 0.0, 0.0]),
        0.0,
    );

    let instance_id = instance.id;

    if let Some(keyframe) = app.project.layers[app.active_layer]
        .keyframes
        .get_mut(&app.current_frame)
    {
        keyframe.objects.push(instance);
    }

    app.selection.selected_objects.clear();
    app.selection.selected_objects.push(instance_id);
}

pub fn draw_library_panel(app: &mut AnimateApp, ui: &mut egui::Ui) {
    ui.heading("Library");
    ui.separator();

    if !app.selection.selected_objects.is_empty() && ui.button("Convert to Symbol").clicked() {
        convert_selection_to_symbol(app);
    }

    ui.separator();

    if app.project.library.symbols.is_empty() {
        ui.label("No symbols yet.");
        ui.label("Select objects and click");
        ui.label("'Convert to Symbol'.");
        return;
    }

    let mut action: Option<LibraryAction> = None;

    egui::ScrollArea::vertical().show(ui, |ui| {
        for symbol_index in 0..app.project.library.symbols.len() {
            let symbol = &app.project.library.symbols[symbol_index];
            let symbol_id = symbol.id;

            ui.horizontal(|ui| {
                let is_editing = app.editing_symbol == Some(symbol_id);
                let label = if is_editing {
                    format!("[editing] {}", symbol.name)
                } else {
                    symbol.name.clone()
                };

                if ui.selectable_label(is_editing, &label).clicked() {
                    action = Some(LibraryAction::Instantiate(symbol_id));
                }

                if ui.small_button("E").on_hover_text("Edit symbol").clicked() {
                    if app.editing_symbol == Some(symbol_id) {
                        action = Some(LibraryAction::ExitEdit);
                    } else {
                        action = Some(LibraryAction::EnterEdit(symbol_id));
                    }
                }

                if ui
                    .small_button("X")
                    .on_hover_text("Delete symbol")
                    .clicked()
                {
                    action = Some(LibraryAction::Delete(symbol_index));
                }
            });
        }
    });

    if let Some(library_action) = action {
        match library_action {
            LibraryAction::Instantiate(symbol_id) => {
                instantiate_symbol(app, symbol_id);
            }
            LibraryAction::EnterEdit(symbol_id) => {
                app.editing_symbol = Some(symbol_id);
            }
            LibraryAction::ExitEdit => {
                app.editing_symbol = None;
            }
            LibraryAction::Delete(index) => {
                app.history.push(app.project.clone());
                let removed_id = app.project.library.symbols[index].id;
                app.project.library.symbols.remove(index);
                if app.editing_symbol == Some(removed_id) {
                    app.editing_symbol = None;
                }
            }
        }
    }
}

pub fn draw_symbol_name_editor(app: &mut AnimateApp, ui: &mut egui::Ui) {
    let Some(symbol_id) = app.editing_symbol else {
        return;
    };

    let Some(symbol) = app
        .project
        .library
        .symbols
        .iter_mut()
        .find(|symbol| symbol.id == symbol_id)
    else {
        return;
    };

    ui.separator();
    ui.label("Editing Symbol:");
    let mut name = symbol.name.clone();
    if ui.text_edit_singleline(&mut name).changed() {
        symbol.name = name;
    }

    let mut sym_width = symbol.width;
    let mut sym_height = symbol.height;
    let mut sym_frames = symbol.total_frames as f32;
    let mut changed = false;

    egui::Grid::new("symbol_edit_props")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("Width:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut sym_width)
                        .speed(1.0)
                        .range(1.0..=10000.0),
                )
                .changed();
            ui.end_row();
            ui.label("Height:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut sym_height)
                        .speed(1.0)
                        .range(1.0..=10000.0),
                )
                .changed();
            ui.end_row();
            ui.label("Frames:");
            changed |= ui
                .add(
                    egui::DragValue::new(&mut sym_frames)
                        .speed(1.0)
                        .range(1.0..=10000.0),
                )
                .changed();
            ui.end_row();
        });

    if changed {
        let symbol = app
            .project
            .library
            .symbols
            .iter_mut()
            .find(|symbol| symbol.id == symbol_id)
            .unwrap();
        symbol.width = sym_width;
        symbol.height = sym_height;
        symbol.total_frames = sym_frames as u32;
    }

    if ui.button("Exit Symbol Editing").clicked() {
        app.editing_symbol = None;
    }
}

enum LibraryAction {
    Instantiate(uuid::Uuid),
    EnterEdit(uuid::Uuid),
    ExitEdit,
    Delete(usize),
}
