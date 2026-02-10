use std::collections::BTreeMap;

use nightshade::prelude::*;

use crate::app::{AnimateApp, PropertiesTab};
use crate::armature;
use crate::camera;
use crate::library;
use crate::paint::Paint;
use crate::paint_editor;
use crate::project::{BlendMode, LayerType, PropertyKey, Shape, TweenType};
use crate::tween;

enum ShapeEdit {
    Rectangle {
        width: f32,
        height: f32,
        corner_radius: f32,
    },
    Ellipse {
        radius_x: f32,
        radius_y: f32,
    },
    PathClosed {
        closed: bool,
    },
    Text {
        content: String,
        font_size: f32,
        font_family: crate::project::FontFamily,
    },
}

fn apply_shape_edit(shape: &mut Shape, edit: &ShapeEdit) {
    match edit {
        ShapeEdit::Rectangle {
            width,
            height,
            corner_radius,
        } => {
            if let Shape::Rectangle {
                width: w,
                height: h,
                corner_radius: r,
            } = shape
            {
                *w = *width;
                *h = *height;
                *r = *corner_radius;
            }
        }
        ShapeEdit::Ellipse { radius_x, radius_y } => {
            if let Shape::Ellipse {
                radius_x: rx,
                radius_y: ry,
            } = shape
            {
                *rx = *radius_x;
                *ry = *radius_y;
            }
        }
        ShapeEdit::PathClosed { closed } => {
            if let Shape::Path { closed: c, .. } = shape {
                *c = *closed;
            }
        }
        ShapeEdit::Text {
            content,
            font_size,
            font_family,
        } => {
            if let Shape::Text {
                content: c,
                font_size: s,
                font_family: f,
            } = shape
            {
                *c = content.clone();
                *s = *font_size;
                *f = *font_family;
            }
        }
    }
}

pub fn draw_properties(app: &mut AnimateApp, ui_context: &egui::Context) {
    egui::SidePanel::right("properties")
        .resizable(true)
        .default_width(250.0)
        .min_width(150.0)
        .show(ui_context, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut app.properties_tab,
                    PropertiesTab::Properties,
                    "Properties",
                );
                ui.selectable_value(&mut app.properties_tab, PropertiesTab::Library, "Library");
            });
            ui.separator();

            match app.properties_tab {
                PropertiesTab::Properties => {
                    if app.editing_symbol.is_some() {
                        library::draw_symbol_name_editor(app, ui);
                    } else if app.selection.selected_objects.is_empty() {
                        draw_canvas_properties(app, ui);
                        ui.separator();
                        draw_layer_properties(app, ui);
                        camera::draw_camera_properties(app, ui);
                        armature::draw_bone_properties(app, ui);
                    } else {
                        draw_object_properties(app, ui);
                    }
                }
                PropertiesTab::Library => {
                    library::draw_library_panel(app, ui);
                }
            }
        });
}

fn draw_canvas_properties(app: &mut AnimateApp, ui: &mut egui::Ui) {
    ui.label("Canvas");
    ui.separator();

    egui::Grid::new("canvas_props")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("Width:");
            let mut width = app.project.canvas_width as f32;
            if ui
                .add(
                    egui::DragValue::new(&mut width)
                        .range(1.0..=7680.0)
                        .speed(1.0),
                )
                .changed()
            {
                app.project.canvas_width = width as u32;
            }
            ui.end_row();

            ui.label("Height:");
            let mut height = app.project.canvas_height as f32;
            if ui
                .add(
                    egui::DragValue::new(&mut height)
                        .range(1.0..=4320.0)
                        .speed(1.0),
                )
                .changed()
            {
                app.project.canvas_height = height as u32;
            }
            ui.end_row();

            ui.label("FPS:");
            let mut fps = app.project.frame_rate as f32;
            if ui
                .add(egui::DragValue::new(&mut fps).range(1.0..=120.0).speed(1.0))
                .changed()
            {
                app.project.frame_rate = fps as u32;
            }
            ui.end_row();

            ui.label("Frames:");
            let mut frames = app.project.total_frames as f32;
            if ui
                .add(
                    egui::DragValue::new(&mut frames)
                        .range(1.0..=10000.0)
                        .speed(1.0),
                )
                .changed()
            {
                app.project.total_frames = frames as u32;
            }
            ui.end_row();

            ui.label("Background:");
            let mut bg = app.project.background_color;
            if ui.color_edit_button_rgba_unmultiplied(&mut bg).changed() {
                app.project.background_color = bg;
            }
            ui.end_row();
        });
}

fn draw_layer_properties(app: &mut AnimateApp, ui: &mut egui::Ui) {
    if app.active_layer >= app.project.layers.len() {
        return;
    }

    ui.label("Layer");
    ui.separator();

    egui::Grid::new("layer_props")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("Name:");
            let mut name = app.project.layers[app.active_layer].name.clone();
            if ui.text_edit_singleline(&mut name).changed() {
                app.project.layers[app.active_layer].name = name;
            }
            ui.end_row();

            ui.label("Type:");
            let current_type = app.project.layers[app.active_layer].layer_type;
            let type_label = match current_type {
                LayerType::Normal => "Normal",
                LayerType::Guide => "Guide",
                LayerType::Mask => "Mask",
                LayerType::Folder => "Folder",
            };
            egui::ComboBox::from_id_salt("layer_type")
                .selected_text(type_label)
                .show_ui(ui, |ui| {
                    for (layer_type, label) in [
                        (LayerType::Normal, "Normal"),
                        (LayerType::Guide, "Guide"),
                        (LayerType::Mask, "Mask"),
                        (LayerType::Folder, "Folder"),
                    ] {
                        if ui
                            .selectable_value(
                                &mut app.project.layers[app.active_layer].layer_type,
                                layer_type,
                                label,
                            )
                            .clicked()
                        {}
                    }
                });
            ui.end_row();

            ui.label("Blend:");
            let current_blend = app.project.layers[app.active_layer].blend_mode;
            let blend_label = match current_blend {
                BlendMode::Normal => "Normal",
                BlendMode::Multiply => "Multiply",
                BlendMode::Screen => "Screen",
                BlendMode::Overlay => "Overlay",
                BlendMode::Darken => "Darken",
                BlendMode::Lighten => "Lighten",
                BlendMode::ColorDodge => "Color Dodge",
                BlendMode::ColorBurn => "Color Burn",
                BlendMode::Difference => "Difference",
                BlendMode::Exclusion => "Exclusion",
            };
            egui::ComboBox::from_id_salt("blend_mode")
                .selected_text(blend_label)
                .show_ui(ui, |ui| {
                    for (blend_mode, label) in [
                        (BlendMode::Normal, "Normal"),
                        (BlendMode::Multiply, "Multiply"),
                        (BlendMode::Screen, "Screen"),
                        (BlendMode::Overlay, "Overlay"),
                        (BlendMode::Darken, "Darken"),
                        (BlendMode::Lighten, "Lighten"),
                        (BlendMode::ColorDodge, "Color Dodge"),
                        (BlendMode::ColorBurn, "Color Burn"),
                        (BlendMode::Difference, "Difference"),
                        (BlendMode::Exclusion, "Exclusion"),
                    ] {
                        if ui
                            .selectable_value(
                                &mut app.project.layers[app.active_layer].blend_mode,
                                blend_mode,
                                label,
                            )
                            .clicked()
                        {}
                    }
                });
            ui.end_row();

            ui.label("Opacity:");
            ui.add(
                egui::DragValue::new(&mut app.project.layers[app.active_layer].opacity)
                    .speed(0.01)
                    .range(0.0..=1.0),
            );
            ui.end_row();
        });
}

fn draw_object_properties(app: &mut AnimateApp, ui: &mut egui::Ui) {
    let selected_ids = app.selection.selected_objects.clone();

    ui.label(format!("{} object(s) selected", selected_ids.len()));
    ui.separator();

    let first_id = selected_ids[0];
    let mut found_object = None;
    let mut object_layer_index = None;

    for (layer_index, layer) in app.project.layers.iter().enumerate() {
        if let Some(objects) = tween::resolve_frame(layer, app.current_frame) {
            for object in &objects {
                if object.id == first_id {
                    found_object = Some(object.clone());
                    object_layer_index = Some(layer_index);
                    break;
                }
            }
        }
        if found_object.is_some() {
            break;
        }
    }

    let source_object = match found_object {
        Some(object) => object,
        None => {
            ui.label("(Object not visible on current frame)");
            return;
        }
    };

    let single_object = selected_ids.len() == 1;
    let current_frame = app.current_frame;

    let tracks = if single_object {
        object_layer_index.and_then(|idx| {
            app.project.layers[idx]
                .property_tracks
                .get(&first_id)
                .cloned()
        })
    } else {
        None
    };

    let pos_tracked = tracks.as_ref().is_some_and(|t| !t.position.is_empty());
    let rot_tracked = tracks.as_ref().is_some_and(|t| !t.rotation.is_empty());
    let scale_tracked = tracks.as_ref().is_some_and(|t| !t.scale.is_empty());
    let fill_tracked = tracks.as_ref().is_some_and(|t| !t.fill.is_empty());
    let stroke_tracked = tracks.as_ref().is_some_and(|t| !t.stroke.is_empty());
    let sw_tracked = tracks.as_ref().is_some_and(|t| !t.stroke_width.is_empty());

    let mut position = source_object.position;
    let mut rotation_deg = source_object.rotation.to_degrees();
    let mut scale = source_object.scale;
    let mut fill_paint = source_object.fill.clone();
    let mut stroke_paint = source_object.stroke.clone();
    let mut stroke_width = source_object.stroke_width;

    let mut position_changed = false;
    let mut rotation_changed = false;
    let mut scale_changed = false;
    let mut stroke_width_changed = false;

    egui::Grid::new("object_props")
        .num_columns(2)
        .spacing([8.0, 4.0])
        .show(ui, |ui| {
            ui.label("X:");
            position_changed |= ui
                .add(egui::DragValue::new(&mut position[0]).speed(1.0))
                .changed();
            ui.end_row();

            ui.label("Y:");
            position_changed |= ui
                .add(egui::DragValue::new(&mut position[1]).speed(1.0))
                .changed();
            ui.end_row();

            ui.label("Rotation:");
            rotation_changed |= ui
                .add(
                    egui::DragValue::new(&mut rotation_deg)
                        .speed(1.0)
                        .suffix("°"),
                )
                .changed();
            ui.end_row();

            ui.label("Scale X:");
            scale_changed |= ui
                .add(
                    egui::DragValue::new(&mut scale[0])
                        .speed(0.01)
                        .range(0.01..=100.0),
                )
                .changed();
            ui.end_row();

            ui.label("Scale Y:");
            scale_changed |= ui
                .add(
                    egui::DragValue::new(&mut scale[1])
                        .speed(0.01)
                        .range(0.01..=100.0),
                )
                .changed();
            ui.end_row();

            ui.label("Stroke W:");
            stroke_width_changed |= ui
                .add(
                    egui::DragValue::new(&mut stroke_width)
                        .speed(0.1)
                        .range(0.0..=100.0),
                )
                .changed();
            ui.end_row();
        });

    ui.separator();
    ui.label("Fill");
    let fill_changed = paint_editor::paint_editor_ui(ui, "obj_fill", &mut fill_paint);

    ui.separator();
    ui.label("Stroke");
    let stroke_changed = paint_editor::paint_editor_ui(ui, "obj_stroke", &mut stroke_paint);

    if let Some(layer_idx) = object_layer_index {
        let any_track_change = (pos_tracked && position_changed)
            || (rot_tracked && rotation_changed)
            || (scale_tracked && scale_changed)
            || (fill_tracked && fill_changed)
            || (stroke_tracked && stroke_changed)
            || (sw_tracked && stroke_width_changed);

        if any_track_change {
            let layer = &mut app.project.layers[layer_idx];
            let obj_tracks = layer.property_tracks.entry(first_id).or_default();
            if pos_tracked && position_changed {
                obj_tracks.position.insert(
                    current_frame,
                    PropertyKey {
                        value: position,
                        tween: TweenType::Linear,
                    },
                );
            }
            if rot_tracked && rotation_changed {
                obj_tracks.rotation.insert(
                    current_frame,
                    PropertyKey {
                        value: rotation_deg.to_radians(),
                        tween: TweenType::Linear,
                    },
                );
            }
            if scale_tracked && scale_changed {
                obj_tracks.scale.insert(
                    current_frame,
                    PropertyKey {
                        value: scale,
                        tween: TweenType::Linear,
                    },
                );
            }
            if fill_tracked && fill_changed {
                obj_tracks.fill.insert(
                    current_frame,
                    PropertyKey {
                        value: fill_paint.clone(),
                        tween: TweenType::Linear,
                    },
                );
            }
            if stroke_tracked && stroke_changed {
                obj_tracks.stroke.insert(
                    current_frame,
                    PropertyKey {
                        value: stroke_paint.clone(),
                        tween: TweenType::Linear,
                    },
                );
            }
            if sw_tracked && stroke_width_changed {
                obj_tracks.stroke_width.insert(
                    current_frame,
                    PropertyKey {
                        value: stroke_width,
                        tween: TweenType::Linear,
                    },
                );
            }
        }
    }

    let base_changed = (!pos_tracked && position_changed)
        || (!rot_tracked && rotation_changed)
        || (!scale_tracked && scale_changed)
        || (!fill_tracked && fill_changed)
        || (!stroke_tracked && stroke_changed)
        || (!sw_tracked && stroke_width_changed);

    if base_changed {
        let rotation_rad = rotation_deg.to_radians();
        for layer in &mut app.project.layers {
            let has_selected = tween::resolve_frame(layer, app.current_frame)
                .map(|objects| {
                    objects
                        .iter()
                        .any(|object| selected_ids.contains(&object.id))
                })
                .unwrap_or(false);

            if has_selected {
                tween::ensure_keyframe_at(layer, app.current_frame);
            }

            if let Some(keyframe) = layer.keyframes.get_mut(&app.current_frame) {
                for object in &mut keyframe.objects {
                    if selected_ids.contains(&object.id) {
                        if !pos_tracked && position_changed {
                            object.position = position;
                        }
                        if !rot_tracked && rotation_changed {
                            object.rotation = rotation_rad;
                        }
                        if !scale_tracked && scale_changed {
                            object.scale = scale;
                        }
                        if !fill_tracked && fill_changed {
                            object.fill = fill_paint.clone();
                        }
                        if !stroke_tracked && stroke_changed {
                            object.stroke = stroke_paint.clone();
                        }
                        if !sw_tracked && stroke_width_changed {
                            object.stroke_width = stroke_width;
                        }
                    }
                }
            }
        }
    }

    if let Some(layer_idx) = object_layer_index.filter(|_| single_object) {
        ui.separator();
        draw_property_keyframe_buttons(
            app,
            ui,
            PropertyKeyframeContext {
                object_id: first_id,
                layer_index: layer_idx,
                frame: current_frame,
                position,
                rotation: rotation_deg.to_radians(),
                scale,
                fill: fill_paint.clone(),
                stroke: stroke_paint.clone(),
                stroke_width,
            },
        );
    }

    ui.separator();

    let mut shape_changed = false;
    let mut new_shape_data: Option<ShapeEdit> = None;

    match &source_object.shape {
        Shape::Rectangle {
            width,
            height,
            corner_radius,
        } => {
            ui.label("Rectangle");
            let mut rect_w = *width;
            let mut rect_h = *height;
            let mut radius = *corner_radius;
            egui::Grid::new("rect_shape_props")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Width:");
                    shape_changed |= ui
                        .add(
                            egui::DragValue::new(&mut rect_w)
                                .speed(1.0)
                                .range(1.0..=10000.0),
                        )
                        .changed();
                    ui.end_row();
                    ui.label("Height:");
                    shape_changed |= ui
                        .add(
                            egui::DragValue::new(&mut rect_h)
                                .speed(1.0)
                                .range(1.0..=10000.0),
                        )
                        .changed();
                    ui.end_row();
                    ui.label("Radius:");
                    shape_changed |= ui
                        .add(
                            egui::DragValue::new(&mut radius)
                                .speed(0.5)
                                .range(0.0..=1000.0),
                        )
                        .changed();
                    ui.end_row();
                });
            if shape_changed {
                new_shape_data = Some(ShapeEdit::Rectangle {
                    width: rect_w,
                    height: rect_h,
                    corner_radius: radius,
                });
            }
        }
        Shape::Ellipse { radius_x, radius_y } => {
            ui.label("Ellipse");
            let mut rx = *radius_x;
            let mut ry = *radius_y;
            egui::Grid::new("ellipse_shape_props")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Radius X:");
                    shape_changed |= ui
                        .add(egui::DragValue::new(&mut rx).speed(0.5).range(0.1..=5000.0))
                        .changed();
                    ui.end_row();
                    ui.label("Radius Y:");
                    shape_changed |= ui
                        .add(egui::DragValue::new(&mut ry).speed(0.5).range(0.1..=5000.0))
                        .changed();
                    ui.end_row();
                });
            if shape_changed {
                new_shape_data = Some(ShapeEdit::Ellipse {
                    radius_x: rx,
                    radius_y: ry,
                });
            }
        }
        Shape::Line { end_x, end_y } => {
            let length = (end_x * end_x + end_y * end_y).sqrt();
            ui.label(format!("Line: length {:.1}", length));
        }
        Shape::Path { points, closed } => {
            ui.label(format!(
                "Path: {} points, {}",
                points.len(),
                if *closed { "closed" } else { "open" }
            ));
            let mut is_closed = *closed;
            if ui.checkbox(&mut is_closed, "Closed").changed() {
                new_shape_data = Some(ShapeEdit::PathClosed { closed: is_closed });
            }
        }
        Shape::Text {
            content,
            font_size,
            font_family,
        } => {
            ui.label("Text");
            let mut text_content = content.clone();
            let mut text_size = *font_size;
            let mut text_family = *font_family;
            egui::Grid::new("text_shape_props")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Content:");
                    shape_changed |= ui.text_edit_singleline(&mut text_content).changed();
                    ui.end_row();
                    ui.label("Size:");
                    shape_changed |= ui
                        .add(
                            egui::DragValue::new(&mut text_size)
                                .speed(0.5)
                                .range(1.0..=500.0),
                        )
                        .changed();
                    ui.end_row();
                    ui.label("Font:");
                    let family_label = match text_family {
                        crate::project::FontFamily::SansSerif => "Sans Serif",
                        crate::project::FontFamily::Serif => "Serif",
                        crate::project::FontFamily::Monospace => "Monospace",
                    };
                    egui::ComboBox::from_id_salt("font_family")
                        .selected_text(family_label)
                        .show_ui(ui, |ui| {
                            for (family, label) in [
                                (crate::project::FontFamily::SansSerif, "Sans Serif"),
                                (crate::project::FontFamily::Serif, "Serif"),
                                (crate::project::FontFamily::Monospace, "Monospace"),
                            ] {
                                if ui
                                    .selectable_value(&mut text_family, family, label)
                                    .changed()
                                {
                                    shape_changed = true;
                                }
                            }
                        });
                    ui.end_row();
                });
            if shape_changed {
                new_shape_data = Some(ShapeEdit::Text {
                    content: text_content,
                    font_size: text_size,
                    font_family: text_family,
                });
            }
        }
        Shape::RasterImage {
            source_width,
            source_height,
            ..
        } => {
            ui.label(format!("Image: {}x{}", source_width, source_height));
        }
        Shape::SymbolInstance { symbol_id } => {
            ui.label(format!("Symbol: {}", symbol_id));
        }
    }

    if let Some(edit) = new_shape_data {
        for layer in &mut app.project.layers {
            let has_selected = tween::resolve_frame(layer, app.current_frame)
                .map(|objects| {
                    objects
                        .iter()
                        .any(|object| selected_ids.contains(&object.id))
                })
                .unwrap_or(false);

            if has_selected {
                tween::ensure_keyframe_at(layer, app.current_frame);
            }

            if let Some(keyframe) = layer.keyframes.get_mut(&app.current_frame) {
                for object in &mut keyframe.objects {
                    if selected_ids.contains(&object.id) {
                        apply_shape_edit(&mut object.shape, &edit);
                    }
                }
            }
        }
    }

    ui.separator();

    if ui.button("Delete Selected").clicked() {
        app.history.push(app.project.clone());
        for layer in &mut app.project.layers {
            let has_selected = tween::resolve_frame(layer, app.current_frame)
                .map(|objects| {
                    objects
                        .iter()
                        .any(|object| selected_ids.contains(&object.id))
                })
                .unwrap_or(false);

            if has_selected {
                tween::ensure_keyframe_at(layer, app.current_frame);
            }

            if let Some(keyframe) = layer.keyframes.get_mut(&app.current_frame) {
                keyframe
                    .objects
                    .retain(|object| !selected_ids.contains(&object.id));
            }
        }
        app.selection.selected_objects.clear();
    }
}

struct PropertyKeyframeContext {
    object_id: uuid::Uuid,
    layer_index: usize,
    frame: u32,
    position: [f32; 2],
    rotation: f32,
    scale: [f32; 2],
    fill: Paint,
    stroke: Paint,
    stroke_width: f32,
}

fn draw_property_keyframe_buttons(
    app: &mut AnimateApp,
    ui: &mut egui::Ui,
    ctx: PropertyKeyframeContext,
) {
    let PropertyKeyframeContext {
        object_id,
        layer_index,
        frame,
        position,
        rotation,
        scale,
        fill,
        stroke,
        stroke_width,
    } = ctx;
    ui.label("Property Keyframes");
    ui.add_space(2.0);

    let tracks = app.project.layers[layer_index]
        .property_tracks
        .get(&object_id)
        .cloned();

    let props: [(&str, bool, bool); 6] = [
        (
            "Pos",
            tracks.as_ref().is_some_and(|t| !t.position.is_empty()),
            tracks
                .as_ref()
                .is_some_and(|t| t.position.contains_key(&frame)),
        ),
        (
            "Rot",
            tracks.as_ref().is_some_and(|t| !t.rotation.is_empty()),
            tracks
                .as_ref()
                .is_some_and(|t| t.rotation.contains_key(&frame)),
        ),
        (
            "Scale",
            tracks.as_ref().is_some_and(|t| !t.scale.is_empty()),
            tracks
                .as_ref()
                .is_some_and(|t| t.scale.contains_key(&frame)),
        ),
        (
            "Fill",
            tracks.as_ref().is_some_and(|t| !t.fill.is_empty()),
            tracks.as_ref().is_some_and(|t| t.fill.contains_key(&frame)),
        ),
        (
            "Stroke",
            tracks.as_ref().is_some_and(|t| !t.stroke.is_empty()),
            tracks
                .as_ref()
                .is_some_and(|t| t.stroke.contains_key(&frame)),
        ),
        (
            "SW",
            tracks.as_ref().is_some_and(|t| !t.stroke_width.is_empty()),
            tracks
                .as_ref()
                .is_some_and(|t| t.stroke_width.contains_key(&frame)),
        ),
    ];

    let mut toggle_index: Option<usize> = None;

    ui.horizontal_wrapped(|ui| {
        for (index, (name, has_track, has_key)) in props.iter().enumerate() {
            let (symbol, color) = if *has_key {
                ("\u{25C6}", egui::Color32::from_rgb(255, 200, 50))
            } else if *has_track {
                ("\u{25C7}", egui::Color32::from_rgb(200, 180, 80))
            } else {
                ("\u{25C7}", egui::Color32::from_rgb(100, 100, 100))
            };
            let text = format!("{} {}", symbol, name);
            let button = egui::Button::new(egui::RichText::new(text).size(10.0).color(color))
                .min_size(egui::vec2(0.0, 18.0));
            if ui.add(button).clicked() {
                toggle_index = Some(index);
            }
        }
    });

    if let Some(index) = toggle_index {
        app.history.push(app.project.clone());
        let layer = &mut app.project.layers[layer_index];
        let obj_tracks = layer.property_tracks.entry(object_id).or_default();

        match index {
            0 => toggle_track_key(&mut obj_tracks.position, frame, position),
            1 => toggle_track_key(&mut obj_tracks.rotation, frame, rotation),
            2 => toggle_track_key(&mut obj_tracks.scale, frame, scale),
            3 => toggle_track_key_clone(&mut obj_tracks.fill, frame, fill),
            4 => toggle_track_key_clone(&mut obj_tracks.stroke, frame, stroke),
            5 => toggle_track_key(&mut obj_tracks.stroke_width, frame, stroke_width),
            _ => {}
        }

        if layer
            .property_tracks
            .get(&object_id)
            .is_some_and(|t| t.is_empty())
        {
            layer.property_tracks.remove(&object_id);
        }
    }
}

fn toggle_track_key<T: Clone + Copy>(
    track: &mut BTreeMap<u32, PropertyKey<T>>,
    frame: u32,
    value: T,
) {
    if let std::collections::btree_map::Entry::Vacant(entry) = track.entry(frame) {
        entry.insert(PropertyKey {
            value,
            tween: TweenType::Linear,
        });
    } else {
        track.remove(&frame);
    }
}

fn toggle_track_key_clone<T: Clone>(
    track: &mut BTreeMap<u32, PropertyKey<T>>,
    frame: u32,
    value: T,
) {
    if let std::collections::btree_map::Entry::Vacant(entry) = track.entry(frame) {
        entry.insert(PropertyKey {
            value,
            tween: TweenType::Linear,
        });
    } else {
        track.remove(&frame);
    }
}
