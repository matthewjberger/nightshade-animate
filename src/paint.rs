use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GradientStop {
    pub offset: f32,
    pub color: [f32; 4],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Paint {
    Solid([f32; 4]),
    LinearGradient {
        start: [f32; 2],
        end: [f32; 2],
        stops: Vec<GradientStop>,
    },
    RadialGradient {
        center: [f32; 2],
        radius: f32,
        stops: Vec<GradientStop>,
    },
}

impl Default for Paint {
    fn default() -> Self {
        Paint::Solid([0.0, 0.0, 0.0, 1.0])
    }
}

impl Paint {
    pub fn as_solid(&self) -> [f32; 4] {
        match self {
            Paint::Solid(color) => *color,
            Paint::LinearGradient { stops, .. } | Paint::RadialGradient { stops, .. } => stops
                .first()
                .map(|stop| stop.color)
                .unwrap_or([0.0, 0.0, 0.0, 1.0]),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn sample_at(&self, normalized_position: f32) -> [f32; 4] {
        match self {
            Paint::Solid(color) => *color,
            Paint::LinearGradient { stops, .. } | Paint::RadialGradient { stops, .. } => {
                sample_gradient(stops, normalized_position)
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn sample_gradient(stops: &[GradientStop], position: f32) -> [f32; 4] {
    if stops.is_empty() {
        return [0.0, 0.0, 0.0, 1.0];
    }
    if stops.len() == 1 || position <= stops[0].offset {
        return stops[0].color;
    }
    if position >= stops[stops.len() - 1].offset {
        return stops[stops.len() - 1].color;
    }

    for index in 1..stops.len() {
        if position <= stops[index].offset {
            let prev = &stops[index - 1];
            let next = &stops[index];
            let range = next.offset - prev.offset;
            if range < f32::EPSILON {
                return next.color;
            }
            let t = (position - prev.offset) / range;
            return [
                prev.color[0] + (next.color[0] - prev.color[0]) * t,
                prev.color[1] + (next.color[1] - prev.color[1]) * t,
                prev.color[2] + (next.color[2] - prev.color[2]) * t,
                prev.color[3] + (next.color[3] - prev.color[3]) * t,
            ];
        }
    }
    stops.last().unwrap().color
}

pub fn lerp_paint(from: &Paint, to: &Paint, t: f32) -> Paint {
    match (from, to) {
        (Paint::Solid(a), Paint::Solid(b)) => Paint::Solid([
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
            a[3] + (b[3] - a[3]) * t,
        ]),
        _ => {
            let a = from.as_solid();
            let b = to.as_solid();
            Paint::Solid([
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
                a[3] + (b[3] - a[3]) * t,
            ])
        }
    }
}
