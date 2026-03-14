use crate::app::{AppVars, NoteDraft, RefAV};
use crate::canvas::CanvasKind;
use crate::dom::Tabs;
use crate::view_draw::notes_dom::{focus_note_editor, update_notes_view};
use kurbo::Vec2;

pub(crate) enum NoteDraftOutcome {
    None,
    Started,
    Updated,
    Finished(usize),
}

pub(crate) fn handle_note_draft_move(avb: &mut AppVars) -> NoteDraftOutcome {
    if avb.notes.draft.is_none() || avb.active_view != Tabs::Draw {
        return NoteDraftOutcome::None;
    }
    let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
    let draw_pos = canvas.get_user_ui().draw_pos;
    if let Some(draft) = avb.notes.draft.clone() {
        let min = Vec2::new(draft.start.x.min(draw_pos.x), draft.start.y.min(draw_pos.y));
        let max = Vec2::new(draft.start.x.max(draw_pos.x), draft.start.y.max(draw_pos.y));
        if let Some(note) = canvas.notes.get_mut(draft.id) {
            note.pos = min;
            note.size = max - min;
        }
        return NoteDraftOutcome::Updated;
    }
    NoteDraftOutcome::None
}

pub(crate) fn handle_note_draft_start(avb: &mut AppVars, button: i16) -> NoteDraftOutcome {
    if !avb.notes.mode || avb.active_view != Tabs::Draw || button != 0 {
        return NoteDraftOutcome::None;
    }
    let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
    let start = canvas.get_user_ui().draw_pos;
    let id = canvas.notes.add(start, Vec2::new(1.0, 1.0), String::new());
    avb.notes.draft = Some(NoteDraft { id, start });
    NoteDraftOutcome::Started
}

pub(crate) fn handle_note_draft_finish(avb: &mut AppVars) -> NoteDraftOutcome {
    let Some(draft) = avb.notes.draft.take() else {
        return NoteDraftOutcome::None;
    };
    let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
    let min_size = 80.0 / canvas.get_scale().max(0.001);
    if let Some(note) = canvas.notes.get_mut(draft.id) {
        note.size = Vec2::new(note.size.x.max(min_size), note.size.y.max(min_size));
    }
    NoteDraftOutcome::Finished(draft.id)
}

pub(crate) fn delete_note_if_selected(av: &RefAV, active_note_id: Option<usize>) -> bool {
    let Ok(mut avb) = av.try_borrow_mut() else {
        return false;
    };
    let note_id = active_note_id.or(avb.notes.selected);
    let Some(note_id) = note_id else {
        return false;
    };
    let canvas = &mut avb.canvases[CanvasKind::Draw.idx()];
    if canvas.notes.remove(note_id).is_some() {
        avb.notes.selected = None;
        return true;
    }
    false
}

pub(crate) fn focus_note_after_create(av: RefAV, note_id: usize) {
    update_notes_view(av.clone());
    focus_note_editor(av, note_id);
}
