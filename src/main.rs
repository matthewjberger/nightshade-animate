use nightshade::prelude::*;

mod align;
mod app;
mod armature;
#[cfg(not(target_arch = "wasm32"))]
mod audio;
mod boolean;
mod camera;
mod canvas;
mod clipboard;
mod easing_editor;
#[cfg(not(target_arch = "wasm32"))]
mod export;
mod guides;
mod history;
#[cfg(not(target_arch = "wasm32"))]
mod io;
mod library;
#[cfg(not(target_arch = "wasm32"))]
mod lottie;
mod menu;
mod node_edit;
mod onion;
mod paint;
mod paint_editor;
mod playback;
mod project;
mod properties;
mod scenes;
mod selection;
mod snapping;
#[cfg(not(target_arch = "wasm32"))]
mod svg;
mod test_animation;
mod timeline;
mod toolbar;
mod tools;
mod transform;
mod tween;
mod z_order;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    launch(FrameKey::default())?;
    Ok(())
}

#[derive(Default)]
struct FrameKey {
    app: app::AnimateApp,
}

impl State for FrameKey {
    fn title(&self) -> &str {
        "FrameKey"
    }

    fn initialize(&mut self, world: &mut World) {
        world.resources.user_interface.enabled = true;
        world.resources.graphics.atmosphere = Atmosphere::None;
    }

    fn ui(&mut self, _world: &mut World, ui_context: &egui::Context) {
        playback::advance_playback(&mut self.app);

        #[cfg(target_arch = "wasm32")]
        menu::process_pending_load(&mut self.app);

        menu::draw_menu_bar(&mut self.app, ui_context);
        toolbar::draw_toolbar(&mut self.app, ui_context);
        properties::draw_properties(&mut self.app, ui_context);
        timeline::draw_timeline(&mut self.app, ui_context);
        canvas::draw_canvas(&mut self.app, ui_context);
        easing_editor::draw_easing_editor(&mut self.app, ui_context);

        menu::handle_global_shortcuts(&mut self.app, ui_context);
        toolbar::handle_tool_shortcuts(&mut self.app, ui_context);
        timeline::handle_timeline_shortcuts(&mut self.app, ui_context);
        playback::handle_playback_shortcuts(&mut self.app, ui_context);
    }
}
