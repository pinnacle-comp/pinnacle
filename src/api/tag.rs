pub mod v1;

use std::mem;

use indexmap::IndexSet;

use crate::{
    output::OutputName,
    state::{State, WithState},
    tag::Tag,
    window::{UnmappedState, window_state::WindowId},
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
) -> Result<Vec<Tag>, TagAddError> {
    let Some(output) = output_name.output(&state.pinnacle) else {
        return Err(TagAddError::OutputDoesNotExist);
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

    Ok(new_tags)
}

#[derive(Debug, PartialEq, Clone)]
pub enum TagAddError {
    /// Its impossible to add tags to an output that does not exist. Create it first
    OutputDoesNotExist,
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

#[derive(Debug, PartialEq, Clone)]
pub enum TagMoveToOutputError {
    /// Its impossible to move tags to an output that does not exist. Create it first
    OutputDoesNotExist,
    /// Moving the task would result in a situation where each of the following windows are on multiple outputs. This would be an invalid state for pinnacle.
    SameWindowOnTwoOutputs(Vec<WindowId>),
}

pub fn move_to_output<T>(
    state: &mut State,
    tags_to_move: T,
    output_name: OutputName,
) -> Result<(), TagMoveToOutputError>
where
    T: IntoIterator<Item = Tag>,
{
    let Some(new_output) = output_name.output(&state.pinnacle) else {
        return Err(TagMoveToOutputError::OutputDoesNotExist);
    };

    let tags_to_move: IndexSet<Tag> = tags_to_move.into_iter().collect();
    let tags_on_other_outputs = state
        .pinnacle
        .outputs
        .iter()
        .filter(|output| **output != new_output)
        .flat_map(|output| output.with_state(|state| state.tags.clone()))
        .filter(|tag| !tags_to_move.contains(tag))
        .collect::<IndexSet<_>>();

    let mut problematic_windows = IndexSet::new();
    let mut windows_to_update = Vec::new();

    for window in state.pinnacle.windows.iter() {
        let (window_id, is_affected_by_move, has_other_output_tag) = window.with_state(|state| {
            let is_affected_by_move = !state.tags.is_disjoint(&tags_to_move);
            let has_other_output_tag = !state.tags.is_disjoint(&tags_on_other_outputs);

            (state.id, is_affected_by_move, has_other_output_tag)
        });

        if is_affected_by_move {
            if has_other_output_tag {
                problematic_windows.insert(window_id);
            }
            windows_to_update.push(window.clone());
        }
    }

    if !problematic_windows.is_empty() {
        return Err(TagMoveToOutputError::SameWindowOnTwoOutputs(
            problematic_windows.into_iter().collect(),
        ));
    }

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

    if state.pinnacle.space.output_geometry(&new_output).is_some() {
        let loc = state.pinnacle.floating_loc_for_output(&new_output);
        for window in windows_to_update {
            window.with_state_mut(|state| state.set_floating_loc(Some(loc)));

            let layout_mode = window.with_state(|state| state.layout_mode);
            state.pinnacle.update_window_geometry(
                &window,
                layout_mode.is_tiled() || layout_mode.is_spilled(),
            );
        }
    }

    state.pinnacle.update_xwayland_stacking_order();
    Ok(())
}
