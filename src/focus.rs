// SPDX-License-Identifier: GPL-3.0-or-later

use keyboard::KeyboardFocusTarget;
use smithay::{
    desktop::layer_map_for_output,
    output::Output,
    utils::{IsAlive, SERIAL_COUNTER},
    wayland::shell::wlr_layer::{self, KeyboardInteractivity},
};

use crate::{
    api::signal::Signal,
    state::{Pinnacle, State, WithState},
    window::{WindowElement, ZIndexElement},
};

pub mod keyboard;
pub mod pointer;

impl State {
    /// Updates the keyboard focus.
    ///
    /// This computes the current keyboard focus in the following order:
    /// 1. Any lock surface if locked,
    /// 2. The topmost exclusive layer surface,
    /// 3. Any on-demand layer surface, and finally
    /// 4. The focused window on the focused output.
    ///
    /// Some focus behavior (subject to change):
    /// - The first lock surface that a client sends in gets lock surface focus.
    ///   Lock surface focus can be changed by clicking on another lock surface.
    /// - The topmost exclusive layer surface gets focus, prioritizing surfaces
    ///   on the focused output. This currently cannot be changed by click,
    ///   but this may change in the future.
    /// - On-demand layer surfaces can only be focused by clicking on them.
    ///   They retain focus unless a window is focused or it is clicked off of.
    /// - Only the focused window on the focused output gets focus.
    ///   If the focused output changes, the window may lose focus.
    pub fn update_keyboard_focus(&mut self) {
        let _span = tracy_client::span!("State::update_keyboard_focus");

        let Some(keyboard) = self.pinnacle.seat.get_keyboard() else {
            return;
        };

        if keyboard.current_focus().is_some_and(|focus| !focus.alive()) {
            keyboard.set_focus(self, None, SERIAL_COUNTER.next_serial());
        }

        self.pinnacle
            .lock_surface_focus
            .take_if(|lock| !lock.alive());

        // Only allow keyboard focus on lock surfaces when locked
        if !self.pinnacle.lock_state.is_unlocked() {
            let lock_surface = self
                .pinnacle
                .lock_surface_focus
                .clone()
                .map(KeyboardFocusTarget::LockSurface);

            if keyboard.current_focus() == lock_surface {
                return;
            }

            keyboard.set_focus(self, lock_surface, SERIAL_COUNTER.next_serial());

            for win in self.pinnacle.windows.iter() {
                win.set_activated(false);
                if let Some(toplevel) = win.toplevel() {
                    toplevel.send_pending_configure();
                }
            }

            return;
        }

        // Refresh exclusive layer shell focus
        let mut exclusive_layer_focus: Option<smithay::desktop::LayerSurface> = None;

        for op in self.pinnacle.output_focus_stack.outputs().rev() {
            let possible_overlay_focus = layer_map_for_output(op)
                .layers_on(wlr_layer::Layer::Overlay)
                .rev()
                .find(|layer| {
                    layer.cached_state().keyboard_interactivity == KeyboardInteractivity::Exclusive
                })
                .cloned();

            if possible_overlay_focus.is_some() {
                exclusive_layer_focus = possible_overlay_focus;
                break;
            }

            // Only allow the topmost `top` exlcusive layer but keep searching
            // for overlay layers
            if exclusive_layer_focus.is_none() {
                let possible_top_focus = layer_map_for_output(op)
                    .layers_on(wlr_layer::Layer::Top)
                    .rev()
                    .find(|layer| {
                        layer.cached_state().keyboard_interactivity
                            == KeyboardInteractivity::Exclusive
                    })
                    .cloned();

                if possible_top_focus.is_some() {
                    exclusive_layer_focus = possible_top_focus;
                }
            }
        }

        if let Some(exclusive_layer_focus) = exclusive_layer_focus {
            let layer_target = KeyboardFocusTarget::LayerSurface(exclusive_layer_focus);
            if keyboard.current_focus().as_ref() == Some(&layer_target) {
                return;
            }

            keyboard.set_focus(self, Some(layer_target), SERIAL_COUNTER.next_serial());

            for win in self.pinnacle.windows.iter() {
                win.set_activated(false);
                if let Some(toplevel) = win.toplevel() {
                    toplevel.send_pending_configure();
                }
            }

            return;
        }

        // Handle on-demand layer shell focus
        self.pinnacle
            .on_demand_layer_focus
            .take_if(|layer| !layer.alive());

        if let Some(layer) = self.pinnacle.on_demand_layer_focus.as_ref() {
            let layer_target = KeyboardFocusTarget::LayerSurface(layer.clone());
            if keyboard.current_focus().as_ref() == Some(&layer_target) {
                return;
            }

            keyboard.set_focus(self, Some(layer_target), SERIAL_COUNTER.next_serial());

            for win in self.pinnacle.windows.iter() {
                win.set_activated(false);
                if let Some(toplevel) = win.toplevel() {
                    toplevel.send_pending_configure();
                }
            }

            return;
        }

        // And lastly, window focus
        let focused_window = self
            .pinnacle
            .focused_output()
            .and_then(|op| self.pinnacle.focus_stack_for_output(op).last().cloned())
            .filter(|_| self.pinnacle.keyboard_focus_stack.focused);

        if keyboard.current_focus().is_some_and(
            |focus| matches!(&focus, KeyboardFocusTarget::Window(w) if Some(w) == focused_window.as_ref()),
        ) {
            return;
        }

        for win in self.pinnacle.windows.iter() {
            let focused = Some(win) == focused_window.as_ref();
            win.set_activated(focused);
            if let Some(toplevel) = win.toplevel() {
                toplevel.send_pending_configure();
            }
            if focused {
                self.pinnacle.signal_state.window_focused.signal(win);
            }
        }

        keyboard.set_focus(
            self,
            focused_window.map(KeyboardFocusTarget::Window),
            SERIAL_COUNTER.next_serial(),
        );
    }
}

impl Pinnacle {
    pub fn fixup_z_layering(&mut self) {
        let _span = tracy_client::span!("Pinnacle::fixup_z_layering");

        self.z_index_stack.retain(|z| match z {
            ZIndexElement::Window(win) => {
                self.space.raise_element(win, false);
                true
            }
            ZIndexElement::Unmapping(weak) => weak.upgrade().is_some(),
        });
    }

    /// Raise a window to the top of the z-index stack.
    pub fn raise_window(&mut self, window: WindowElement) {
        let _span = tracy_client::span!("Pinnacle::raise_window");

        self.space.raise_element(&window, false);

        self.z_index_stack
            .retain(|win| !matches!(win, ZIndexElement::Window(win) if win == window));
        self.z_index_stack.push(ZIndexElement::Window(window));

        self.update_xwayland_stacking_order();
    }

    /// Lower a window to the bottom of the z-index stack.
    pub fn lower_window(&mut self, window: WindowElement) {
        let _span = tracy_client::span!("Pinnacle::lower_window");

        self.z_index_stack
            .retain(|win| !matches!(win, ZIndexElement::Window(win) if win == window));
        self.z_index_stack.insert(0, ZIndexElement::Window(window));

        for win in self.z_index_stack.iter() {
            if let ZIndexElement::Window(win) = win {
                self.space.raise_element(win, false);
            }
        }

        self.update_xwayland_stacking_order();
    }

    /// Get the currently focused output, or the first mapped output if there is none, or None.
    pub fn focused_output(&self) -> Option<&Output> {
        let _span = tracy_client::span!("Pinnacle::focused_output");

        self.output_focus_stack
            .outputs()
            .last()
            .or_else(|| self.space.outputs().next())
    }

    pub fn focus_stack_for_output(
        &self,
        output: &Output,
    ) -> impl DoubleEndedIterator<Item = &WindowElement> {
        let output = output.clone();
        self.keyboard_focus_stack.windows().filter(move |win| {
            let win_geo = self.space.element_geometry(win);
            let op_geo = self.space.output_geometry(&output);

            if let (Some(win_geo), Some(op_geo)) = (win_geo, op_geo) {
                win_geo.overlaps(op_geo)
            } else {
                false
            }
        })
    }

    pub fn focus_output(&mut self, output: &Output) {
        if self.output_focus_stack.current_focus() == Some(output) {
            return;
        }
        self.output_focus_stack.set_focus(output.clone());
        self.signal_state.output_focused.signal(output);
    }
}

#[derive(Debug, Clone, Default)]
pub struct OutputFocusStack {
    stack: Vec<Output>,
}

impl OutputFocusStack {
    // Sets the new focused output.
    fn set_focus(&mut self, output: Output) {
        self.stack.retain(|op| op != &output);
        self.stack.push(output);
    }

    pub fn add_to_end(&mut self, output: Output) {
        self.stack.retain(|op| op != &output);
        self.stack.insert(0, output);
    }

    pub fn remove(&mut self, output: &Output) {
        self.stack.retain(|op| op != output);
    }

    pub fn current_focus(&self) -> Option<&Output> {
        self.outputs().last()
    }

    fn outputs(&self) -> impl DoubleEndedIterator<Item = &Output> {
        self.stack
            .iter()
            .filter(|op| op.with_state(|state| state.enabled_global_id.is_some()))
    }
}

/// A stack of windows, with the top one being the one in focus.
#[derive(Debug, Default)]
pub struct WindowKeyboardFocusStack {
    stack: Vec<WindowElement>,
    focused: bool,
}

impl WindowKeyboardFocusStack {
    /// Sets `window` to be focused.
    ///
    /// If it's already in the stack, it will be removed then pushed.
    /// If it isn't, it will just be pushed.
    pub fn set_focus(&mut self, window: WindowElement) {
        self.stack.retain(|win| win != window);
        self.stack.push(window);
        self.focused = true;
    }

    /// Adds a window to the focus stack while keeping the currently focused window
    /// still focused.
    ///
    /// This will insert the window one below the top of the stack.
    pub fn add_focus(&mut self, window: WindowElement) {
        self.stack.retain(|win| win != window);
        let insert_idx = self.stack.len().saturating_sub(1);
        self.stack.insert(insert_idx, window);
    }

    /// Unsets the focus by marking this stack as unfocused.
    ///
    /// This will cause [`Self::current_focus`] to return `None`.
    pub fn unset_focus(&mut self) {
        self.focused = false;
    }

    /// Removes a window from the focus stack.
    pub fn remove(&mut self, window: &WindowElement) {
        self.stack.retain(|win| win != window);
    }

    pub fn windows(&self) -> impl DoubleEndedIterator<Item = &WindowElement> {
        self.stack.iter()
    }

    /// Gets the currently focused window on this stack.
    ///
    /// This is the topmost window that is on an active tag and not
    /// an OR window.
    pub fn current_focus(&self) -> Option<&WindowElement> {
        if !self.focused {
            return None;
        };

        self.stack
            .iter()
            .rev()
            .filter(|win| win.is_on_active_tag())
            .find(|win| !win.is_x11_override_redirect())
    }
}
