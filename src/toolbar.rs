use nightshade::prelude::*;

use crate::app::AnimateApp;
use crate::paint_editor;
use crate::tools::Tool;

pub fn draw_toolbar(app: &mut AnimateApp, ui_context: &egui::Context) {
    egui::SidePanel::left("toolbar")
        .resizable(false)
        .exact_width(48.0)
        .show(ui_context, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(4.0);

                let tools = [
                    (Tool::Select, "Sel", "Select (V)"),
                    (Tool::NodeEdit, "Nod", "Node Edit (A)"),
                    (Tool::Rectangle, "Rect", "Rectangle (R)"),
                    (Tool::Ellipse, "Ell", "Ellipse (E)"),
                    (Tool::Line, "Line", "Line (L)"),
                    (Tool::Pen, "Pen", "Pen (P)"),
                    (Tool::Pencil, "Pcl", "Pencil (B)"),
                    (Tool::Eraser, "Ers", "Eraser (X)"),
                    (Tool::PaintBucket, "Bkt", "Paint Bucket (K)"),
                    (Tool::Text, "Txt", "Text (T)"),
                    (Tool::Brush, "Brsh", "Brush (Shift+B)"),
                    (Tool::Bone, "Bone", "Bone (J)"),
                ];

                for (tool, label, tooltip) in tools {
                    let is_active = app.tool == tool;
                    let button = egui::Button::new(egui::RichText::new(label).size(11.0))
                        .min_size(egui::vec2(40.0, 32.0))
                        .selected(is_active);

                    if ui.add(button).on_hover_text(tooltip).clicked() {
                        app.tool = tool;
                    }
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(4.0);

                ui.label(egui::RichText::new("Fill").size(9.0));
                let fill_swatch_color = paint_swatch_color(&app.fill_paint);
                let fill_response = ui.add(
                    egui::Button::new("")
                        .min_size(egui::vec2(32.0, 20.0))
                        .fill(fill_swatch_color),
                );
                egui::Popup::from_toggle_button_response(&fill_response)
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                    .show(|ui| {
                        ui.set_min_width(220.0);
                        ui.label("Fill Paint");
                        ui.separator();
                        paint_editor::paint_editor_ui(ui, "toolbar_fill", &mut app.fill_paint);
                    });

                ui.add_space(4.0);

                ui.label(egui::RichText::new("Strk").size(9.0));
                let stroke_swatch_color = paint_swatch_color(&app.stroke_paint);
                let stroke_response = ui.add(
                    egui::Button::new("")
                        .min_size(egui::vec2(32.0, 20.0))
                        .fill(stroke_swatch_color),
                );
                egui::Popup::from_toggle_button_response(&stroke_response)
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                    .show(|ui| {
                        ui.set_min_width(220.0);
                        ui.label("Stroke Paint");
                        ui.separator();
                        paint_editor::paint_editor_ui(ui, "toolbar_stroke", &mut app.stroke_paint);
                    });

                ui.add_space(4.0);

                ui.label(egui::RichText::new("SW").size(9.0));
                ui.add(
                    egui::DragValue::new(&mut app.stroke_width)
                        .range(0.0..=50.0)
                        .speed(0.1),
                );
            });
        });
}

fn paint_swatch_color(paint: &crate::paint::Paint) -> egui::Color32 {
    let color = paint.as_solid();
    egui::Color32::from_rgba_unmultiplied(
        (color[0] * 255.0) as u8,
        (color[1] * 255.0) as u8,
        (color[2] * 255.0) as u8,
        (color[3] * 255.0) as u8,
    )
}

pub fn handle_tool_shortcuts(app: &mut AnimateApp, ui_context: &egui::Context) {
    if ui_context.wants_keyboard_input() {
        return;
    }
    ui_context.input(|input| {
        if input.key_pressed(egui::Key::V) {
            app.tool = Tool::Select;
        }
        if input.key_pressed(egui::Key::A) && !input.modifiers.ctrl {
            app.tool = Tool::NodeEdit;
        }
        if input.key_pressed(egui::Key::R) {
            app.tool = Tool::Rectangle;
        }
        if input.key_pressed(egui::Key::E) {
            app.tool = Tool::Ellipse;
        }
        if input.key_pressed(egui::Key::L) {
            app.tool = Tool::Line;
        }
        if input.key_pressed(egui::Key::P) {
            app.tool = Tool::Pen;
        }
        if input.key_pressed(egui::Key::B) && !input.modifiers.shift {
            app.tool = Tool::Pencil;
        }
        if input.key_pressed(egui::Key::X) {
            app.tool = Tool::Eraser;
        }
        if input.key_pressed(egui::Key::K) {
            app.tool = Tool::PaintBucket;
        }
        if input.key_pressed(egui::Key::T) {
            app.tool = Tool::Text;
        }
        if input.key_pressed(egui::Key::B) && input.modifiers.shift {
            app.tool = Tool::Brush;
        }
        if input.key_pressed(egui::Key::J) {
            app.tool = Tool::Bone;
        }
    });
}
