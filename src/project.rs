use std::collections::{BTreeMap, HashMap};

use crate::paint::Paint;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Project {
    pub name: String,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub background_color: [f32; 4],
    pub frame_rate: u32,
    pub total_frames: u32,
    pub layers: Vec<Layer>,
    pub guides: Vec<Guide>,
    pub camera_keyframes: BTreeMap<u32, CameraKeyframe>,
    pub camera_tween: TweenType,
    pub scenes: Vec<Scene>,
    pub active_scene: usize,
    pub library: Library,
    pub image_assets: Vec<ImageAsset>,
    pub loop_start: Option<u32>,
    pub loop_end: Option<u32>,
    pub audio_tracks: Vec<AudioTrack>,
    pub armatures: Vec<Armature>,
}

impl Default for Project {
    fn default() -> Self {
        let mut layer = Layer::new("Layer 1".to_string());
        layer.keyframes.insert(0, Keyframe::default());
        Self {
            name: "Untitled".to_string(),
            canvas_width: 1920,
            canvas_height: 1080,
            background_color: [1.0, 1.0, 1.0, 1.0],
            frame_rate: 24,
            total_frames: 120,
            layers: vec![layer],
            guides: Vec::new(),
            camera_keyframes: BTreeMap::new(),
            camera_tween: TweenType::None,
            scenes: Vec::new(),
            active_scene: 0,
            library: Library::default(),
            image_assets: Vec::new(),
            loop_start: None,
            loop_end: None,
            audio_tracks: Vec::new(),
            armatures: Vec::new(),
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Layer {
    pub id: uuid::Uuid,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32,
    pub keyframes: BTreeMap<u32, Keyframe>,
    pub layer_type: LayerType,
    pub blend_mode: BlendMode,
    pub parent_id: Option<uuid::Uuid>,
    pub collapsed: bool,
    #[serde(default)]
    pub property_tracks: HashMap<uuid::Uuid, PropertyTracks>,
}

impl Layer {
    pub fn new(name: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name,
            visible: true,
            locked: false,
            opacity: 1.0,
            keyframes: BTreeMap::new(),
            layer_type: LayerType::Normal,
            blend_mode: BlendMode::Normal,
            parent_id: None,
            collapsed: false,
            property_tracks: HashMap::new(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum LayerType {
    Normal,
    Guide,
    Mask,
    Folder,
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    Difference,
    Exclusion,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Keyframe {
    pub objects: Vec<AnimObject>,
    pub tween: TweenType,
    pub label: String,
    pub comment: String,
    #[serde(default)]
    pub shape_tween: bool,
}

impl Default for Keyframe {
    fn default() -> Self {
        Self {
            objects: Vec::new(),
            tween: TweenType::None,
            label: String::new(),
            comment: String::new(),
            shape_tween: false,
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct AnimObject {
    pub id: uuid::Uuid,
    pub shape: Shape,
    pub position: [f32; 2],
    pub rotation: f32,
    pub scale: [f32; 2],
    pub fill: Paint,
    pub stroke: Paint,
    pub stroke_width: f32,
}

impl AnimObject {
    pub fn new(
        shape: Shape,
        position: [f32; 2],
        fill: Paint,
        stroke: Paint,
        stroke_width: f32,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            shape,
            position,
            rotation: 0.0,
            scale: [1.0, 1.0],
            fill,
            stroke,
            stroke_width,
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Shape {
    Rectangle {
        width: f32,
        height: f32,
        corner_radius: f32,
    },
    Ellipse {
        radius_x: f32,
        radius_y: f32,
    },
    Line {
        end_x: f32,
        end_y: f32,
    },
    Path {
        points: Vec<PathPoint>,
        closed: bool,
    },
    Text {
        content: String,
        font_size: f32,
        font_family: FontFamily,
    },
    RasterImage {
        image_id: uuid::Uuid,
        source_width: u32,
        source_height: u32,
        display_width: f32,
        display_height: f32,
    },
    SymbolInstance {
        symbol_id: uuid::Uuid,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FontFamily {
    SansSerif,
    Serif,
    Monospace,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PathPoint {
    pub position: [f32; 2],
    pub control_in: Option<[f32; 2]>,
    pub control_out: Option<[f32; 2]>,
    pub pressure: f32,
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TweenType {
    None,
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    CubicBezier { x1: f32, y1: f32, x2: f32, y2: f32 },
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Guide {
    pub id: uuid::Uuid,
    pub orientation: GuideOrientation,
    pub position: f32,
}

#[derive(Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum GuideOrientation {
    Horizontal,
    Vertical,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct CameraKeyframe {
    pub position: [f32; 2],
    pub zoom: f32,
    pub rotation: f32,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Scene {
    pub id: uuid::Uuid,
    pub name: String,
    pub layers: Vec<Layer>,
    pub total_frames: u32,
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Library {
    pub symbols: Vec<Symbol>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Symbol {
    pub id: uuid::Uuid,
    pub name: String,
    pub layers: Vec<Layer>,
    pub total_frames: u32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageAsset {
    pub id: uuid::Uuid,
    pub name: String,
    #[serde(
        serialize_with = "serialize_bytes_as_base64",
        deserialize_with = "deserialize_bytes_from_base64"
    )]
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

fn serialize_bytes_as_base64<S: serde::Serializer>(
    data: &Vec<u8>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(data);
    serializer.serialize_str(&encoded)
}

fn deserialize_bytes_from_base64<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<u8>, D::Error> {
    use base64::Engine;
    let encoded: String = serde::Deserialize::deserialize(deserializer)?;
    base64::engine::general_purpose::STANDARD
        .decode(&encoded)
        .map_err(serde::de::Error::custom)
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Armature {
    pub id: uuid::Uuid,
    pub name: String,
    pub bones: Vec<Bone>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Bone {
    pub id: uuid::Uuid,
    pub name: String,
    pub parent_bone_id: Option<uuid::Uuid>,
    pub position: [f32; 2],
    pub length: f32,
    pub rotation: f32,
    pub bound_object_ids: Vec<uuid::Uuid>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioTrack {
    pub id: uuid::Uuid,
    pub name: String,
    pub data: Vec<u8>,
    pub start_frame: u32,
    pub volume: f32,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PropertyKey<T> {
    pub value: T,
    pub tween: TweenType,
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PropertyTracks {
    pub position: BTreeMap<u32, PropertyKey<[f32; 2]>>,
    pub rotation: BTreeMap<u32, PropertyKey<f32>>,
    pub scale: BTreeMap<u32, PropertyKey<[f32; 2]>>,
    pub fill: BTreeMap<u32, PropertyKey<Paint>>,
    pub stroke: BTreeMap<u32, PropertyKey<Paint>>,
    pub stroke_width: BTreeMap<u32, PropertyKey<f32>>,
}

impl PropertyTracks {
    pub fn is_empty(&self) -> bool {
        self.position.is_empty()
            && self.rotation.is_empty()
            && self.scale.is_empty()
            && self.fill.is_empty()
            && self.stroke.is_empty()
            && self.stroke_width.is_empty()
    }

    pub fn active_property_names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        if !self.position.is_empty() {
            names.push("Position");
        }
        if !self.rotation.is_empty() {
            names.push("Rotation");
        }
        if !self.scale.is_empty() {
            names.push("Scale");
        }
        if !self.fill.is_empty() {
            names.push("Fill");
        }
        if !self.stroke.is_empty() {
            names.push("Stroke");
        }
        if !self.stroke_width.is_empty() {
            names.push("Stroke W");
        }
        names
    }

    pub fn keyframe_frames_for(&self, property_name: &str) -> Vec<u32> {
        match property_name {
            "Position" => self.position.keys().copied().collect(),
            "Rotation" => self.rotation.keys().copied().collect(),
            "Scale" => self.scale.keys().copied().collect(),
            "Fill" => self.fill.keys().copied().collect(),
            "Stroke" => self.stroke.keys().copied().collect(),
            "Stroke W" => self.stroke_width.keys().copied().collect(),
            _ => Vec::new(),
        }
    }
}
