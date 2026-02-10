use crate::app::AnimateApp;
use crate::tween;

pub fn bring_to_front(app: &mut AnimateApp) {
    let selected = app.selection.selected_objects.clone();
    if selected.is_empty() {
        return;
    }

    app.history.push(app.project.clone());

    for layer in &mut app.project.layers {
        let has_match = tween::resolve_frame(layer, app.current_frame)
            .map(|objects| objects.iter().any(|object| selected.contains(&object.id)))
            .unwrap_or(false);

        if has_match {
            tween::ensure_keyframe_at(layer, app.current_frame);
        }

        if let Some(keyframe) = layer.keyframes.get_mut(&app.current_frame) {
            let mut selected_objects = Vec::new();
            let mut remaining = Vec::new();
            for object in keyframe.objects.drain(..) {
                if selected.contains(&object.id) {
                    selected_objects.push(object);
                } else {
                    remaining.push(object);
                }
            }
            remaining.extend(selected_objects);
            keyframe.objects = remaining;
        }
    }
}

pub fn send_to_back(app: &mut AnimateApp) {
    let selected = app.selection.selected_objects.clone();
    if selected.is_empty() {
        return;
    }

    app.history.push(app.project.clone());

    for layer in &mut app.project.layers {
        let has_match = tween::resolve_frame(layer, app.current_frame)
            .map(|objects| objects.iter().any(|object| selected.contains(&object.id)))
            .unwrap_or(false);

        if has_match {
            tween::ensure_keyframe_at(layer, app.current_frame);
        }

        if let Some(keyframe) = layer.keyframes.get_mut(&app.current_frame) {
            let mut selected_objects = Vec::new();
            let mut remaining = Vec::new();
            for object in keyframe.objects.drain(..) {
                if selected.contains(&object.id) {
                    selected_objects.push(object);
                } else {
                    remaining.push(object);
                }
            }
            selected_objects.extend(remaining);
            keyframe.objects = selected_objects;
        }
    }
}

pub fn bring_forward(app: &mut AnimateApp) {
    let selected = app.selection.selected_objects.clone();
    if selected.is_empty() {
        return;
    }

    app.history.push(app.project.clone());

    for layer in &mut app.project.layers {
        let has_match = tween::resolve_frame(layer, app.current_frame)
            .map(|objects| objects.iter().any(|object| selected.contains(&object.id)))
            .unwrap_or(false);

        if has_match {
            tween::ensure_keyframe_at(layer, app.current_frame);
        }

        if let Some(keyframe) = layer.keyframes.get_mut(&app.current_frame) {
            let count = keyframe.objects.len();
            let mut index = count.saturating_sub(1);
            loop {
                if index < count.saturating_sub(1)
                    && selected.contains(&keyframe.objects[index].id)
                    && !selected.contains(&keyframe.objects[index + 1].id)
                {
                    keyframe.objects.swap(index, index + 1);
                }
                if index == 0 {
                    break;
                }
                index -= 1;
            }
        }
    }
}

pub fn send_backward(app: &mut AnimateApp) {
    let selected = app.selection.selected_objects.clone();
    if selected.is_empty() {
        return;
    }

    app.history.push(app.project.clone());

    for layer in &mut app.project.layers {
        let has_match = tween::resolve_frame(layer, app.current_frame)
            .map(|objects| objects.iter().any(|object| selected.contains(&object.id)))
            .unwrap_or(false);

        if has_match {
            tween::ensure_keyframe_at(layer, app.current_frame);
        }

        if let Some(keyframe) = layer.keyframes.get_mut(&app.current_frame) {
            let count = keyframe.objects.len();
            for index in 1..count {
                if selected.contains(&keyframe.objects[index].id)
                    && !selected.contains(&keyframe.objects[index - 1].id)
                {
                    keyframe.objects.swap(index, index - 1);
                }
            }
        }
    }
}
