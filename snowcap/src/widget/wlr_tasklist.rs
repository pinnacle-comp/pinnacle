//! A widget to represent a list of open window.

use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
};

use anyhow::Context;
use iced::{Length, Size};
use iced_wgpu::core::{
    Element, Widget, layout, renderer,
    widget::{Tree, tree},
};
use smithay_client_toolkit::{
    output::OutputData,
    reexports::{
        client::{Proxy, Weak, protocol::wl_seat::WlSeat},
        protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1,
    },
};

use crate::{
    handlers::foreign_toplevel_management::{
        ForeignToplevelData, ForeignToplevelInfo, ToplevelState,
        WeakZwlrForeignToplevelManagementState, ZwlrForeignToplevelManagementState,
    },
    widget::output::OutputState,
};

pub mod operation {
    use iced_wgpu::core::widget::Operation;
    use smithay_client_toolkit::reexports::protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_handle_v1::ZwlrForeignToplevelHandleV1;

    pub fn new_toplevel(handle: ZwlrForeignToplevelHandleV1) -> impl Operation {
        struct AddToplevel {
            handle: ZwlrForeignToplevelHandleV1,
        }

        impl Operation for AddToplevel {
            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
                operate(self);
            }

            fn custom(
                &mut self,
                _id: Option<&iced::widget::Id>,
                _bounds: iced::Rectangle,
                state: &mut dyn std::any::Any,
            ) {
                let Some(state) = state.downcast_mut::<super::State>() else {
                    return;
                };

                state.add_toplevel(self.handle.clone());
            }
        }

        AddToplevel { handle }
    }

    pub fn update_toplevel(handle: ZwlrForeignToplevelHandleV1) -> impl Operation {
        struct UpdateToplevel {
            handle: ZwlrForeignToplevelHandleV1,
        }

        impl Operation for UpdateToplevel {
            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
                operate(self);
            }

            fn custom(
                &mut self,
                _id: Option<&iced::widget::Id>,
                _bounds: iced::Rectangle,
                state: &mut dyn std::any::Any,
            ) {
                let Some(state) = state.downcast_mut::<super::State>() else {
                    return;
                };

                state.update_toplevel(self.handle.clone());
            }
        }

        UpdateToplevel { handle }
    }

    pub fn remove_toplevel(handle: ZwlrForeignToplevelHandleV1) -> impl Operation {
        struct RemoveToplevel {
            handle: ZwlrForeignToplevelHandleV1,
        }

        impl Operation for RemoveToplevel {
            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
                operate(self);
            }

            fn custom(
                &mut self,
                _id: Option<&iced::widget::Id>,
                _bounds: iced::Rectangle,
                state: &mut dyn std::any::Any,
            ) {
                let Some(state) = state.downcast_mut::<super::State>() else {
                    return;
                };

                state.remove_toplevel(self.handle.clone());
            }
        }

        RemoveToplevel { handle }
    }

    pub fn toplevel_set_maximized(id: u64, maximized: bool) -> impl Operation {
        struct ToplevelSetMaximized {
            id: u64,
            maximized: bool,
        }

        impl Operation for ToplevelSetMaximized {
            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
                operate(self);
            }

            fn custom(
                &mut self,
                _id: Option<&iced::widget::Id>,
                _bounds: iced::Rectangle,
                state: &mut dyn std::any::Any,
            ) {
                let Some(state) = state.downcast_mut::<super::State>() else {
                    return;
                };

                state.toplevel_set_maximized(self.id, self.maximized);
            }
        }

        ToplevelSetMaximized { id, maximized }
    }

    pub fn toplevel_set_minimized(id: u64, minimized: bool) -> impl Operation {
        struct ToplevelSetMinimized {
            id: u64,
            minimized: bool,
        }

        impl Operation for ToplevelSetMinimized {
            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
                operate(self);
            }

            fn custom(
                &mut self,
                _id: Option<&iced::widget::Id>,
                _bounds: iced::Rectangle,
                state: &mut dyn std::any::Any,
            ) {
                let Some(state) = state.downcast_mut::<super::State>() else {
                    return;
                };

                state.toplevel_set_minimized(self.id, self.minimized);
            }
        }

        ToplevelSetMinimized { id, minimized }
    }

    pub fn toplevel_set_fullscreen(id: u64, fullscreen: bool) -> impl Operation {
        struct ToplevelSetFullscreen {
            id: u64,
            fullscreen: bool,
        }

        impl Operation for ToplevelSetFullscreen {
            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
                operate(self);
            }

            fn custom(
                &mut self,
                _id: Option<&iced::widget::Id>,
                _bounds: iced::Rectangle,
                state: &mut dyn std::any::Any,
            ) {
                let Some(state) = state.downcast_mut::<super::State>() else {
                    return;
                };

                state.toplevel_set_fullscreen(self.id, self.fullscreen);
            }
        }

        ToplevelSetFullscreen { id, fullscreen }
    }

    pub fn toplevel_activate(id: u64) -> impl Operation {
        struct ToplevelActivate {
            id: u64,
        }

        impl Operation for ToplevelActivate {
            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
                operate(self);
            }

            fn custom(
                &mut self,
                _id: Option<&iced::widget::Id>,
                _bounds: iced::Rectangle,
                state: &mut dyn std::any::Any,
            ) {
                let Some(state) = state.downcast_mut::<super::State>() else {
                    return;
                };

                state.toplevel_activate(self.id);
            }
        }

        ToplevelActivate { id }
    }

    pub fn toplevel_close(id: u64) -> impl Operation {
        struct ToplevelClose {
            id: u64,
        }

        impl Operation for ToplevelClose {
            fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
                operate(self);
            }

            fn custom(
                &mut self,
                _id: Option<&iced::widget::Id>,
                _bounds: iced::Rectangle,
                state: &mut dyn std::any::Any,
            ) {
                let Some(state) = state.downcast_mut::<super::State>() else {
                    return;
                };

                state.toplevel_close(self.id);
            }
        }

        ToplevelClose { id }
    }
}

#[derive(Debug, Clone)]
pub struct WlrTaskState {
    pub id: u64,
    pub title: String,
    pub app_id: String,
    pub state: ToplevelState,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum WlrTaskListEvent {
    ToplevelEnter(WlrTaskState),
    ToplevelUpdate(WlrTaskState),
    ToplevelLeave(u64),
}

/// Emits events on window changes.
pub struct WlrTaskList<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer> {
    content: Element<'a, Message, Theme, Renderer>,

    // FIXME: Ok, this feels like bad design 101. Ideally, we'd want a service that would maintain
    // both the protocol state and seat, but I'm currently undecided on how to do it, or how to
    // handle multi-seat if that becomes a thing.
    wlr_state: WeakZwlrForeignToplevelManagementState,
    seat: Option<Weak<WlSeat>>,

    on_enter: Option<Box<dyn Fn(WlrTaskState) -> Message + 'a>>,
    on_update: Option<Box<dyn Fn(WlrTaskState) -> Message + 'a>>,
    on_leave: Option<Box<dyn Fn(u64) -> Message + 'a>>,
    _all_output: bool,
}

/// Local state of the [`WlrTaskList`].
#[derive(Default)]
pub struct State {
    output_state: OutputState,

    toplevel_list: HashMap<u64, Weak<ZwlrForeignToplevelHandleV1>>,
    seat: Option<Weak<WlSeat>>,

    pending_enter: Vec<(WlrTaskState, Weak<ZwlrForeignToplevelHandleV1>)>,
    pending_update: Vec<(WlrTaskState, Weak<ZwlrForeignToplevelHandleV1>)>,
    pending_leave: Vec<(u64, Weak<ZwlrForeignToplevelHandleV1>)>,

    initial_state_sent: bool,
}

impl State {
    fn add_toplevel(&mut self, handle: ZwlrForeignToplevelHandleV1) {
        if !self.initial_state_sent {
            return;
        }

        let Ok(task_state) = handle.clone().try_into() else {
            return;
        };

        self.pending_enter.push((task_state, handle.downgrade()));
    }

    fn update_toplevel(&mut self, handle: ZwlrForeignToplevelHandleV1) {
        if !self.initial_state_sent {
            return;
        }

        let Ok(task_state): Result<WlrTaskState, anyhow::Error> = handle.clone().try_into() else {
            return;
        };

        let weak = handle.downgrade();
        if self.toplevel_list.contains_key(&task_state.id) {
            self.pending_update.push((task_state, weak));
        } else {
            self.pending_enter.push((task_state, weak));
        }
    }

    fn remove_toplevel(&mut self, handle: ZwlrForeignToplevelHandleV1) {
        if !self.initial_state_sent {
            return;
        }

        let id = make_id_from_handle(&handle);

        self.pending_leave.push((id, handle.downgrade()));
    }

    fn toplevel_set_maximized(&mut self, id: u64, maximized: bool) {
        if let Some(handle) = self.toplevel_list.get(&id).and_then(|v| v.upgrade().ok()) {
            if maximized {
                handle.set_maximized();
            } else {
                handle.unset_maximized();
            }
        }
    }

    fn toplevel_set_minimized(&mut self, id: u64, minimized: bool) {
        if let Some(handle) = self.toplevel_list.get(&id).and_then(|v| v.upgrade().ok()) {
            if minimized {
                handle.set_minimized();
            } else {
                handle.unset_minimized();
            }
        }
    }

    fn toplevel_set_fullscreen(&mut self, id: u64, fullscreen: bool) {
        if let Some(handle) = self.toplevel_list.get(&id).and_then(|v| v.upgrade().ok()) {
            if fullscreen {
                handle.set_fullscreen(None);
            } else {
                handle.unset_fullscreen();
            }
        }
    }

    fn toplevel_activate(&mut self, id: u64) {
        let Some(seat) = self.seat.as_ref().and_then(|s| s.upgrade().ok()) else {
            tracing::warn!("Activate was called, but the widget doesn't have an associated seat.");
            return;
        };

        if let Some(handle) = self.toplevel_list.get(&id).and_then(|v| v.upgrade().ok()) {
            handle.activate(&seat);
        }
    }

    fn toplevel_close(&mut self, id: u64) {
        if let Some(handle) = self.toplevel_list.get(&id).and_then(|v| v.upgrade().ok()) {
            handle.close();
        }
    }
}

impl<'a, Message, Theme, Renderer> WlrTaskList<'a, Message, Theme, Renderer> {
    #[must_use]
    pub fn on_enter(mut self, on_enter: impl Fn(WlrTaskState) -> Message + 'a) -> Self {
        self.on_enter = Some(Box::new(on_enter));
        self
    }

    #[must_use]
    pub fn on_update(mut self, on_update: impl Fn(WlrTaskState) -> Message + 'a) -> Self {
        self.on_update = Some(Box::new(on_update));
        self
    }

    #[must_use]
    pub fn on_leave(mut self, on_leave: impl Fn(u64) -> Message + 'a) -> Self {
        self.on_leave = Some(Box::new(on_leave));
        self
    }
}

impl<'a, Message, Theme, Renderer> WlrTaskList<'a, Message, Theme, Renderer> {
    /// Creates a [`WlrTaskList`] with the given content.
    pub fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        wlr_state: ZwlrForeignToplevelManagementState,
        seat: Option<Weak<WlSeat>>,
    ) -> Self {
        let wlr_state = wlr_state.downgrade();

        WlrTaskList {
            content: content.into(),
            wlr_state,
            seat,
            on_enter: None,
            on_update: None,
            on_leave: None,
            _all_output: false,
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for WlrTaskList<'_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
    Message: Clone,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: layout::Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced_wgpu::core::widget::Operation,
    ) {
        let state = tree.state.downcast_mut::<State>();

        state.seat = self.seat.clone();

        operation.custom(None, layout.bounds(), &mut state.output_state);
        operation.custom(None, layout.bounds(), state);

        operation.traverse(&mut |operation| {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                layout,
                renderer,
                operation,
            );
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: layout::Layout<'_>,
        cursor: iced_wgpu::core::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn iced_wgpu::core::Clipboard,
        shell: &mut iced_wgpu::core::Shell<'_, Message>,
        viewport: &iced::Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );

        let state = tree.state.downcast_mut::<State>();
        if let Some(wlr_state) = self.wlr_state.upgrade()
            && !state.initial_state_sent
        {
            wlr_state.with_toplevels(|toplevels| {
                for toplevel in toplevels {
                    if let Ok(task_state) = toplevel.clone().try_into() {
                        state.pending_enter.push((task_state, toplevel.downgrade()));
                    }
                }
            });

            state.initial_state_sent = true;
        }

        if let Some(on_enter) = self.on_enter.as_ref() {
            for (pending, weak) in state.pending_enter.drain(..) {
                if weak.upgrade().is_ok() {
                    let id = pending.id;

                    shell.publish((on_enter)(pending));
                    state.toplevel_list.insert(id, weak);
                }
            }
        }

        if let Some(on_update) = self.on_update.as_ref() {
            for (pending, weak) in state.pending_update.drain(..) {
                if weak.upgrade().is_ok() {
                    shell.publish((on_update)(pending));
                }
            }
        } else {
            state.pending_update.clear();
        }

        if let Some(on_leave) = self.on_leave.as_ref() {
            for (pending, _) in state.pending_leave.drain(..) {
                shell.publish((on_leave)(pending));

                state.toplevel_list.remove(&pending);
            }
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: layout::Layout<'_>,
        cursor: iced_wgpu::core::mouse::Cursor,
        viewport: &iced::Rectangle,
        renderer: &Renderer,
    ) -> iced_wgpu::core::mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        renderer_style: &iced_wgpu::core::renderer::Style,
        layout: layout::Layout<'_>,
        cursor: iced_wgpu::core::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            renderer_style,
            layout,
            cursor,
            viewport,
        )
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: layout::Layout<'b>,
        renderer: &Renderer,
        viewport: &iced::Rectangle,
        translation: iced::Vector,
    ) -> Option<iced_wgpu::core::overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, Message, Theme, Renderer> From<WlrTaskList<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a + Clone,
    Theme: 'a,
    Renderer: 'a + renderer::Renderer,
{
    fn from(value: WlrTaskList<'a, Message, Theme, Renderer>) -> Self {
        Element::new(value)
    }
}

fn make_id_from_handle(handle: &ZwlrForeignToplevelHandleV1) -> u64 {
    let mut hasher = DefaultHasher::default();
    handle.id().hash(&mut hasher);
    hasher.finish()
}

impl TryFrom<ZwlrForeignToplevelHandleV1> for WlrTaskState {
    type Error = anyhow::Error;

    fn try_from(value: ZwlrForeignToplevelHandleV1) -> anyhow::Result<Self> {
        let id = make_id_from_handle(&value);

        let data = value
            .data::<ForeignToplevelData>()
            .context("Proxy has no associated data")?;

        data.with_info(|info| {
            let ForeignToplevelInfo {
                app_id,
                title,
                outputs,
                state,
            } = info.clone();

            let outputs = outputs
                .iter()
                .flat_map(|o| {
                    o.data::<OutputData>()
                        .and_then(|d| d.with_output_info(|i| i.name.clone()))
                })
                .collect();

            Self {
                id,
                app_id,
                title,
                state,
                outputs,
            }
        })
        .context("Could not get TaskState from proxy.")
    }
}
