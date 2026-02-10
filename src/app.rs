#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
use std::collections::HashSet;

use nightshade::prelude::*;

use crate::canvas::CanvasView;
use crate::history::History;
use crate::onion::OnionSkinning;
use crate::paint::Paint;
use crate::playback::PlaybackState;
use crate::project::Project;
use crate::selection::Selection;
use crate::tools::{Tool, ToolState};

#[derive(Default)]
pub struct Clipboard {
    pub objects: Vec<crate::project::AnimObject>,
}

#[derive(Default)]
pub struct TimelineSelection {
    pub selected_frames: Vec<(usize, u32)>,
    pub range_start: Option<(usize, u32)>,
}

pub struct TimelineDragState {
    pub dragging_layer: Option<usize>,
    pub drop_target: Option<usize>,
    pub dragging_keyframe: Option<KeyframeDragState>,
}

pub struct KeyframeDragState {
    pub layer_index: usize,
    pub source_frame: u32,
    pub current_hover_frame: u32,
}

#[derive(Default, Clone, Copy, PartialEq)]
pub enum PropertiesTab {
    #[default]
    Properties,
    Library,
}

pub struct EasingEditorState {
    pub layer_index: usize,
    pub frame: u32,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

pub struct AnimateApp {
    pub project: Project,
    pub current_frame: u32,
    pub active_layer: usize,
    pub tool: Tool,
    pub tool_state: ToolState,
    pub canvas_view: CanvasView,
    pub selection: Selection,
    pub history: History,
    pub playback: PlaybackState,
    pub onion: OnionSkinning,
    pub fill_paint: Paint,
    pub stroke_paint: Paint,
    pub stroke_width: f32,
    pub save_path: Option<std::path::PathBuf>,
    pub clipboard: Clipboard,
    pub timeline_selection: TimelineSelection,
    pub timeline_drag: Option<TimelineDragState>,
    pub timeline_zoom: f32,
    pub snap_to_grid: bool,
    pub snap_to_objects: bool,
    pub snap_to_guides: bool,
    pub grid_size: f32,
    pub easing_editor: Option<EasingEditorState>,
    pub editing_symbol: Option<uuid::Uuid>,
    pub properties_tab: PropertiesTab,
    pub ik_drag_bone: Option<(usize, usize)>,
    pub expanded_property_layers: HashSet<uuid::Uuid>,
    #[cfg(not(target_arch = "wasm32"))]
    pub image_textures: HashMap<uuid::Uuid, egui::TextureHandle>,
    #[cfg(target_arch = "wasm32")]
    pub pending_project_load: std::rc::Rc<std::cell::RefCell<Option<Vec<u8>>>>,
}

impl Default for AnimateApp {
    fn default() -> Self {
        Self {
            project: Project::default(),
            current_frame: 0,
            active_layer: 0,
            tool: Tool::Select,
            tool_state: ToolState::Idle,
            canvas_view: CanvasView::default(),
            selection: Selection::default(),
            history: History::new(),
            playback: PlaybackState::default(),
            onion: OnionSkinning::default(),
            fill_paint: Paint::Solid([0.2, 0.5, 0.8, 1.0]),
            stroke_paint: Paint::Solid([0.0, 0.0, 0.0, 1.0]),
            stroke_width: 2.0,
            save_path: None,
            clipboard: Clipboard::default(),
            timeline_selection: TimelineSelection::default(),
            timeline_drag: None,
            timeline_zoom: 1.0,
            snap_to_grid: false,
            snap_to_objects: false,
            snap_to_guides: false,
            grid_size: 10.0,
            easing_editor: None,
            editing_symbol: None,
            properties_tab: PropertiesTab::default(),
            ik_drag_bone: None,
            expanded_property_layers: HashSet::new(),
            #[cfg(not(target_arch = "wasm32"))]
            image_textures: HashMap::new(),
            #[cfg(target_arch = "wasm32")]
            pending_project_load: std::rc::Rc::new(std::cell::RefCell::new(None)),
        }
    }
}
