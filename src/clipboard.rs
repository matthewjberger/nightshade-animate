use crate::app::AnimateApp;
use crate::tween;

pub fn copy_selected(app: &mut AnimateApp) {
    let selected = &app.selection.selected_objects;
    if selected.is_empty() {
        return;
    }

    app.clipboard.objects.clear();
    for layer in &app.project.layers {
        if let Some(objects) = tween::resolve_frame(layer, app.current_frame) {
            for object in &objects {
                if selected.contains(&object.id) {
                    app.clipboard.objects.push(object.clone());
                }
            }
        }
    }
}

pub fn cut_selected(app: &mut AnimateApp) {
    copy_selected(app);
    if app.clipboard.objects.is_empty() {
        return;
    }
    crate::menu::delete_selected(app);
}

pub fn paste(app: &mut AnimateApp) {
    if app.clipboard.objects.is_empty() {
        return;
    }
    if app.active_layer >= app.project.layers.len() {
        return;
    }

    app.history.push(app.project.clone());

    let layer = &mut app.project.layers[app.active_layer];
    tween::ensure_keyframe_at(layer, app.current_frame);

    let mut new_ids = Vec::new();
    if let Some(keyframe) = layer.keyframes.get_mut(&app.current_frame) {
        for original in &app.clipboard.objects {
            let mut pasted = original.clone();
            pasted.id = uuid::Uuid::new_v4();
            pasted.position[0] += 10.0;
            pasted.position[1] += 10.0;
            new_ids.push(pasted.id);
            keyframe.objects.push(pasted);
        }
    }

    app.selection.selected_objects = new_ids;
}

pub fn duplicate_selected(app: &mut AnimateApp) {
    copy_selected(app);
    paste(app);
}
