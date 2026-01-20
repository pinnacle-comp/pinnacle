pub mod v1;

use std::mem;

use indexmap::IndexSet;
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

/// A unique id for a [`Tag`].
#[derive(Debug, PartialEq, Clone)]
pub enum TagMoveToOutputError {
    OutputDoesNotExist(OutputName),
    SameWindowOnTwoOutputs(Vec<Tag>),
}

impl core::fmt::Display for TagMoveToOutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TagMoveToOutputError::OutputDoesNotExist(output) => write!(
                f,
                "Tried to move tags to output {} but it doesn't exist",
                output.0
            ),
            TagMoveToOutputError::SameWindowOnTwoOutputs(tags) => write!(
                f,
                "executing this operation would put the same windows in tags {:?} on two separate outputs at once. This is forbidden.",
                tags.iter().map(|tag| tag.name())
            ),
        }
    }
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
        return Err(TagMoveToOutputError::OutputDoesNotExist(output_name));
    };

    let tags_to_move: IndexSet<Tag> = tags_to_move.into_iter().collect();
    let mut to_evaluate_tags = IndexSet::new();

    for window in state.pinnacle.windows.iter() {
        window.with_state(|state| {
            // is window affected by move
            let mut affected = false;
            let mut contains_other_tags = false;

            for tag in state.tags.iter() {
                let contains = tags_to_move.contains(tag);
                affected = contains || affected;
                contains_other_tags = !contains || contains_other_tags;

                if affected && contains_other_tags {
                    to_evaluate_tags.insert(tag.clone());
                }
            }
        })
    }

    let mut tags_on_other_output = Vec::new();

    for output in state.pinnacle.outputs.iter() {
        if output.name() != output_name.0 {
            output.with_state(|state| {
                for tag in to_evaluate_tags
                    .extract_if(.., |to_evalaute_tag| state.tags.contains(to_evalaute_tag))
                {
                    tags_on_other_output.push(tag);
                }
            })
        }
    }

    if !tags_on_other_output.is_empty() {
        return Err(TagMoveToOutputError::SameWindowOnTwoOutputs(
            tags_on_other_output,
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

    state.pinnacle.update_xwayland_stacking_order();
    Ok(())
}
