pub mod v1;

use std::mem;

use tracing::warn;

use crate::{
    output::OutputName,
    state::{State, WithState},
    tag::Tag,
    window::UnmappedState,
};

use super::{StateFnSender, signal::Signal};

pub struct TagService {
    sender: StateFnSender,
}

impl TagService {
    pub fn new(sender: StateFnSender) -> Self {
        Self { sender }
    }
}

pub fn set_active(state: &mut State, tag: &Tag, set: Option<bool>) {
    let Some(output) = tag.output(&state.pinnacle) else {
        return;
    };

    let active = set.unwrap_or(!tag.active());

    if tag.set_active(active) {
        state.pinnacle.signal_state.tag_active.signal(tag);
    }

    state.pinnacle.update_xwayland_stacking_order();

    state.pinnacle.request_layout(&output);

    state.schedule_render(&output);
}

pub fn switch_to(state: &mut State, tag: &Tag) {
    let Some(output) = tag.output(&state.pinnacle) else {
        return;
    };

    output.with_state(|op_state| {
        for op_tag in op_state.tags.iter() {
            if op_tag.set_active(false) {
                state.pinnacle.signal_state.tag_active.signal(op_tag);
            }
        }
        if tag.set_active(true) {
            state.pinnacle.signal_state.tag_active.signal(tag);
        }
    });

    state.pinnacle.update_xwayland_stacking_order();

    state.pinnacle.request_layout(&output);

    state.schedule_render(&output);
}

pub fn add(
    state: &mut State,
    tag_names: impl IntoIterator<Item = String>,
    output_name: OutputName,
) -> Vec<Tag> {
    let Some(output) = output_name.output(&state.pinnacle) else {
        warn!(
            "Tried to add tags to output {} but it doesn't exist",
            output_name.0
        );
        return Vec::new();
    };

    let new_tags = tag_names.into_iter().map(Tag::new).collect::<Vec<_>>();

    output.with_state_mut(|state| {
        state.add_tags(new_tags.clone());
    });

    if !new_tags.is_empty() {
        let mut unmapped_windows = mem::take(&mut state.pinnacle.unmapped_windows);
        for unmapped in unmapped_windows.iter_mut() {
            if !matches!(unmapped.state, UnmappedState::WaitingForTags { .. }) {
                continue;
            };

            unmapped.window.with_state_mut(|state| {
                state.tags = new_tags.first().cloned().into_iter().collect();
            });

            state.pinnacle.request_window_rules(unmapped);
        }
        state.pinnacle.unmapped_windows = unmapped_windows;
    }

    state.pinnacle.update_xwayland_stacking_order();

    for tag in new_tags.iter() {
        state.pinnacle.signal_state.tag_created.signal(tag);
    }

    new_tags
}

pub fn remove(state: &mut State, tags_to_remove: Vec<Tag>) {
    for window in state.pinnacle.windows.iter() {
        window.with_state_mut(|state| {
            for tag_to_remove in tags_to_remove.iter() {
                state.tags.shift_remove(tag_to_remove);
            }
        })
    }

    for output in state.pinnacle.outputs.clone() {
        let mut changed = false;

        output.with_state_mut(|state| {
            for tag_to_remove in tags_to_remove.iter() {
                changed = state.tags.shift_remove(tag_to_remove) || changed;
            }
        });

        if changed {
            state.pinnacle.request_layout(&output);
            state.schedule_render(&output);
        }
    }

    for conn_saved_state in state.pinnacle.config.connector_saved_states.values_mut() {
        for tag_to_remove in tags_to_remove.iter() {
            conn_saved_state.tags.shift_remove(tag_to_remove);
        }
    }

    state.pinnacle.update_xwayland_stacking_order();

    for tag_to_remove in tags_to_remove.iter() {
        state
            .pinnacle
            .signal_state
            .tag_removed
            .signal(tag_to_remove);
    }
}

pub fn move_to_output(state: &mut State, tags_to_move: Vec<Tag>, output_name: OutputName) {
    let Some(new_output) = output_name.output(&state.pinnacle) else {
        warn!(
            "Tried to move tags to output {} but it doesn't exist",
            output_name.0
        );
        return;
    };

    for output in state.pinnacle.outputs.clone() {
        let mut changed = false;

        if output.name() == new_output.name() {
            output.with_state_mut(|state| {
                for tag in tags_to_move.iter() {
                    changed = state.tags.insert(tag.clone()) || changed;
                }
            });
        } else {
            output.with_state_mut(|state| {
                for tag in tags_to_move.iter() {
                    changed = state.tags.shift_remove(tag) || changed;
                }
            });
        }

        if changed {
            state.pinnacle.request_layout(&output);
            state.schedule_render(&output);
        }
    }

    state.pinnacle.update_xwayland_stacking_order();
}
