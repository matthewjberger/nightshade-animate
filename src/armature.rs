use nightshade::prelude::*;

use crate::app::AnimateApp;
use crate::canvas::CanvasView;
use crate::project::{Armature, Bone};

pub fn draw_bone_overlay(app: &AnimateApp, view: &CanvasView, painter: &egui::Painter) {
    for armature in &app.project.armatures {
        for bone in &armature.bones {
            let resolved_rotation = resolve_bone_rotation(app, bone);
            let (start, end) = bone_endpoints(bone, armature, resolved_rotation);
            let screen_start = view.canvas_to_screen(egui::pos2(start[0], start[1]));
            let screen_end = view.canvas_to_screen(egui::pos2(end[0], end[1]));

            draw_bone_shape(painter, screen_start, screen_end, view.zoom);
        }
    }
}

fn draw_bone_shape(painter: &egui::Painter, start: egui::Pos2, end: egui::Pos2, zoom: f32) {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length = (dx * dx + dy * dy).sqrt();
    if length < 1.0 {
        return;
    }

    let nx = dx / length;
    let ny = dy / length;
    let px = -ny;
    let py = nx;

    let bone_width = (6.0 * zoom).max(2.0);
    let mid_factor = 0.2;
    let mid = egui::pos2(start.x + dx * mid_factor, start.y + dy * mid_factor);

    let points = vec![
        start,
        egui::pos2(mid.x + px * bone_width, mid.y + py * bone_width),
        end,
        egui::pos2(mid.x - px * bone_width, mid.y - py * bone_width),
    ];

    let fill = egui::Color32::from_rgba_unmultiplied(200, 200, 220, 100);
    let stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(100, 120, 200));

    let shape = egui::epaint::PathShape::convex_polygon(points, fill, stroke);
    painter.add(shape);

    let joint_radius = (3.0 * zoom).max(2.0);
    painter.circle_filled(start, joint_radius, egui::Color32::from_rgb(80, 100, 180));
    painter.circle_filled(end, joint_radius, egui::Color32::from_rgb(80, 100, 180));
}

fn bone_endpoints(
    bone: &Bone,
    armature: &Armature,
    resolved_rotation: f32,
) -> ([f32; 2], [f32; 2]) {
    let start = compute_bone_world_position(bone, armature);
    let end = [
        start[0] + bone.length * resolved_rotation.cos(),
        start[1] + bone.length * resolved_rotation.sin(),
    ];
    (start, end)
}

fn compute_bone_world_position(bone: &Bone, armature: &Armature) -> [f32; 2] {
    if let Some(parent_id) = bone.parent_bone_id {
        if let Some(parent) = armature.bones.iter().find(|b| b.id == parent_id) {
            let parent_pos = compute_bone_world_position(parent, armature);
            let parent_rot = parent.rotation;
            [
                parent_pos[0] + parent.length * parent_rot.cos(),
                parent_pos[1] + parent.length * parent_rot.sin(),
            ]
        } else {
            bone.position
        }
    } else {
        bone.position
    }
}

fn resolve_bone_rotation(_app: &AnimateApp, bone: &Bone) -> f32 {
    bone.rotation
}

pub fn handle_bone_tool(
    app: &mut AnimateApp,
    response: &egui::Response,
    _ui_context: &egui::Context,
) {
    if response.clicked()
        && let Some(pointer_pos) = response.interact_pointer_pos()
    {
        let canvas_pos = app.canvas_view.screen_to_canvas(pointer_pos);
        place_bone(app, canvas_pos);
    }

    if response.drag_started_by(egui::PointerButton::Primary)
        && let Some(pointer_pos) = response.interact_pointer_pos()
    {
        let canvas_pos = app.canvas_view.screen_to_canvas(pointer_pos);
        try_start_ik_drag(app, canvas_pos);
    }

    if response.dragged_by(egui::PointerButton::Primary)
        && let Some(pointer_pos) = response
            .hover_pos()
            .or(response.ctx.input(|input| input.pointer.latest_pos()))
    {
        let canvas_pos = app.canvas_view.screen_to_canvas(pointer_pos);
        update_ik_drag(app, canvas_pos);
    }

    if response.drag_stopped() {
        app.ik_drag_bone = None;
    }
}

fn place_bone(app: &mut AnimateApp, canvas_pos: egui::Pos2) {
    let click_pos = [canvas_pos.x, canvas_pos.y];

    if let Some((armature_index, bone_index, is_end)) = find_nearest_joint(app, click_pos) {
        let armature = &app.project.armatures[armature_index];
        let parent_bone = &armature.bones[bone_index];

        if is_end {
            app.history.push(app.project.clone());
            let parent_id = parent_bone.id;
            let parent_end = bone_end_position(parent_bone, armature);
            let dx = click_pos[0] - parent_end[0];
            let dy = click_pos[1] - parent_end[1];
            let length = (dx * dx + dy * dy).sqrt().max(10.0);
            let rotation = dy.atan2(dx);

            let new_bone = Bone {
                id: uuid::Uuid::new_v4(),
                name: format!("Bone {}", armature.bones.len() + 1),
                parent_bone_id: Some(parent_id),
                position: parent_end,
                length,
                rotation,
                bound_object_ids: Vec::new(),
            };
            app.project.armatures[armature_index].bones.push(new_bone);
            return;
        }
    }

    app.history.push(app.project.clone());

    let new_bone = Bone {
        id: uuid::Uuid::new_v4(),
        name: "Bone 1".to_string(),
        parent_bone_id: None,
        position: click_pos,
        length: 80.0,
        rotation: 0.0,
        bound_object_ids: Vec::new(),
    };

    let armature = Armature {
        id: uuid::Uuid::new_v4(),
        name: format!("Armature {}", app.project.armatures.len() + 1),
        bones: vec![new_bone],
    };

    app.project.armatures.push(armature);
}

fn bone_end_position(bone: &Bone, armature: &Armature) -> [f32; 2] {
    let start = compute_bone_world_position(bone, armature);
    [
        start[0] + bone.length * bone.rotation.cos(),
        start[1] + bone.length * bone.rotation.sin(),
    ]
}

fn find_nearest_joint(app: &AnimateApp, pos: [f32; 2]) -> Option<(usize, usize, bool)> {
    let threshold = 15.0 / app.canvas_view.zoom;
    let threshold_sq = threshold * threshold;

    for (armature_index, armature) in app.project.armatures.iter().enumerate() {
        for (bone_index, bone) in armature.bones.iter().enumerate() {
            let start = compute_bone_world_position(bone, armature);
            let end = bone_end_position(bone, armature);

            let end_dx = pos[0] - end[0];
            let end_dy = pos[1] - end[1];
            if end_dx * end_dx + end_dy * end_dy < threshold_sq {
                return Some((armature_index, bone_index, true));
            }

            let start_dx = pos[0] - start[0];
            let start_dy = pos[1] - start[1];
            if start_dx * start_dx + start_dy * start_dy < threshold_sq {
                return Some((armature_index, bone_index, false));
            }
        }
    }
    None
}

fn try_start_ik_drag(app: &mut AnimateApp, canvas_pos: egui::Pos2) {
    let click_pos = [canvas_pos.x, canvas_pos.y];
    if let Some((armature_index, bone_index, true)) = find_nearest_joint(app, click_pos) {
        app.ik_drag_bone = Some((armature_index, bone_index));
    }
}

fn update_ik_drag(app: &mut AnimateApp, canvas_pos: egui::Pos2) {
    let Some((armature_index, bone_index)) = app.ik_drag_bone else {
        return;
    };

    if armature_index >= app.project.armatures.len() {
        app.ik_drag_bone = None;
        return;
    }

    let target = [canvas_pos.x, canvas_pos.y];
    fabrik_solve(
        &mut app.project.armatures[armature_index],
        bone_index,
        target,
    );
}

fn fabrik_solve(armature: &mut Armature, end_bone_index: usize, target: [f32; 2]) {
    let chain = build_bone_chain(armature, end_bone_index);
    if chain.is_empty() {
        return;
    }

    let mut positions: Vec<[f32; 2]> = Vec::new();
    let mut lengths: Vec<f32> = Vec::new();

    for &bone_index in &chain {
        let bone = &armature.bones[bone_index];
        let start = compute_bone_world_position(bone, armature);
        positions.push(start);
        lengths.push(bone.length);
    }

    let last_bone = &armature.bones[*chain.last().unwrap()];
    let last_end = bone_end_position(last_bone, armature);
    positions.push(last_end);

    let root = positions[0];

    for _ in 0..10 {
        let last = positions.len() - 1;
        positions[last] = target;

        for index in (0..last).rev() {
            let dx = positions[index][0] - positions[index + 1][0];
            let dy = positions[index][1] - positions[index + 1][1];
            let dist = (dx * dx + dy * dy).sqrt().max(0.001);
            let ratio = lengths[index] / dist;
            positions[index][0] = positions[index + 1][0] + dx * ratio;
            positions[index][1] = positions[index + 1][1] + dy * ratio;
        }

        positions[0] = root;

        for index in 0..last {
            let dx = positions[index + 1][0] - positions[index][0];
            let dy = positions[index + 1][1] - positions[index][1];
            let dist = (dx * dx + dy * dy).sqrt().max(0.001);
            let ratio = lengths[index] / dist;
            positions[index + 1][0] = positions[index][0] + dx * ratio;
            positions[index + 1][1] = positions[index][1] + dy * ratio;
        }
    }

    for (chain_index, &bone_index) in chain.iter().enumerate() {
        let start = positions[chain_index];
        let end = positions[chain_index + 1];
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        armature.bones[bone_index].rotation = dy.atan2(dx);
    }
}

fn build_bone_chain(armature: &Armature, end_bone_index: usize) -> Vec<usize> {
    let mut chain = vec![end_bone_index];
    let mut current = end_bone_index;

    loop {
        let bone = &armature.bones[current];
        if let Some(parent_id) = bone.parent_bone_id
            && let Some(parent_index) = armature.bones.iter().position(|b| b.id == parent_id)
        {
            chain.push(parent_index);
            current = parent_index;
            continue;
        }
        break;
    }

    chain.reverse();
    chain
}

pub fn draw_bone_properties(app: &mut AnimateApp, ui: &mut egui::Ui) {
    if app.project.armatures.is_empty() {
        return;
    }

    ui.separator();
    ui.heading("Armatures");

    let mut delete_armature = None;

    for armature_index in 0..app.project.armatures.len() {
        let armature_name = app.project.armatures[armature_index].name.clone();
        let bone_count = app.project.armatures[armature_index].bones.len();

        ui.horizontal(|ui| {
            ui.label(format!("{} ({} bones)", armature_name, bone_count));
            if ui.small_button("X").clicked() {
                delete_armature = Some(armature_index);
            }
        });
    }

    if let Some(index) = delete_armature {
        app.history.push(app.project.clone());
        app.project.armatures.remove(index);
    }
}
