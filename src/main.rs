use nightshade::prelude::*;

mod app;
mod canvas;
#[cfg(not(target_arch = "wasm32"))]
mod export;
mod history;
#[cfg(not(target_arch = "wasm32"))]
mod io;
mod menu;
mod onion;
mod playback;
mod project;
mod properties;
mod selection;
mod test_animation;
mod timeline;
mod toolbar;
mod tools;
mod tween;

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

        menu::handle_global_shortcuts(&mut self.app, ui_context);
        toolbar::handle_tool_shortcuts(&mut self.app, ui_context);
        timeline::handle_timeline_shortcuts(&mut self.app, ui_context);
        playback::handle_playback_shortcuts(&mut self.app, ui_context);
    }
}
