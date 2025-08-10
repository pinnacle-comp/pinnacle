use smithay_client_toolkit::reexports::{
    calloop::LoopHandle,
    protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
};
use snowcap_protocols::snowcap_decoration_v1::client::snowcap_decoration_surface_v1::SnowcapDecorationSurfaceV1;

use crate::{state::State, surface::SnowcapSurface, widget::ViewFn};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct DecorationId(pub u32);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
pub struct DecorationIdCounter(DecorationId);

impl DecorationIdCounter {
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> DecorationId {
        let ret = self.0;
        self.0.0 += 1;
        ret
    }
}

impl State {
    pub fn decoration_for_id(&mut self, id: DecorationId) -> Option<&mut SnowcapDecoration> {
        self.decorations
            .iter_mut()
            .find(|deco| deco.decoration_id == id)
    }
}

pub struct SnowcapDecoration {
    pub surface: SnowcapSurface,

    pub decoration: SnowcapDecorationSurfaceV1,
    pub loop_handle: LoopHandle<'static, State>,
    pub foreign_toplevel_list_handle: ExtForeignToplevelHandleV1,

    pub decoration_id: DecorationId,

    pub initial_configure_received: bool,

    pub extents: Bounds,
    pub pending_extents: Option<Bounds>,
    pub toplevel_size: iced::Size<u32>,
    pub pending_toplevel_size: Option<iced::Size<u32>>,
    pub bounds: Bounds,
    pub pending_bounds: Option<Bounds>,
    pub pending_z_index: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bounds {
    pub left: u32,
    pub right: u32,
    pub top: u32,
    pub bottom: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

impl SnowcapDecoration {
    pub fn new(
        state: &mut State,
        toplevel_identifier: String,
        bounds: Bounds,
        z_index: i32,
        extents: Bounds,
        widgets: ViewFn,
    ) -> Option<Self> {
        let foreign_toplevel_list_handle = state
            .foreign_toplevel_list_handles
            .iter()
            .find_map(|(handle, ident)| {
                (ident.identifier() == Some(&toplevel_identifier)).then_some(handle)
            })
            .cloned()?;

        let surface = SnowcapSurface::new(state, widgets, true);

        let deco = state.snowcap_decoration_manager.get_decoration_surface(
            &surface.wl_surface,
            &foreign_toplevel_list_handle,
            &state.queue_handle,
            (),
        );

        deco.set_bounds(bounds.left, bounds.right, bounds.top, bounds.bottom);
        deco.set_z_index(z_index);
        deco.set_location(
            bounds.left as i32 - extents.left as i32,
            bounds.top as i32 - extents.top as i32,
        );

        surface.wl_surface.commit();

        let next_id = state.decoration_id_counter.next();

        Some(Self {
            surface,
            loop_handle: state.loop_handle.clone(),
            decoration: deco,
            foreign_toplevel_list_handle,
            decoration_id: next_id,
            initial_configure_received: false,
            extents,
            pending_extents: None,
            toplevel_size: iced::Size::new(1, 1),
            pending_toplevel_size: None,
            bounds,
            pending_bounds: None,
            pending_z_index: None,
        })
    }

    pub fn schedule_redraw(&mut self) {
        self.surface.schedule_redraw();
    }

    pub fn update_properties(
        &mut self,
        widgets: Option<ViewFn>,
        bounds: Option<Bounds>,
        extents: Option<Bounds>,
        z_index: Option<i32>,
    ) {
        if let Some(widgets) = widgets {
            self.surface.view_changed(widgets);
        }

        if let Some(bounds) = bounds {
            self.pending_bounds = Some(bounds);
        }

        if let Some(extents) = extents {
            self.pending_extents = Some(extents);
        }

        if let Some(z_index) = z_index {
            self.pending_z_index = Some(z_index);
        }

        self.surface.request_frame();
    }

    pub fn draw_if_scheduled(&mut self) {
        let _span = tracy_client::span!("SnowcapDecoration::draw_if_scheduled");
        self.surface.draw_if_scheduled();
    }

    pub fn update(
        &mut self,
        runtime: &mut crate::runtime::Runtime,
        compositor: &mut crate::compositor::Compositor,
    ) {
        let _span = tracy_client::span!("SnowcapDecoration::update");

        if let Some(extents) = self.pending_extents.take()
            && extents != self.extents
        {
            self.extents = extents;
        }
        if let Some(bounds) = self.pending_bounds.take()
            && bounds != self.bounds
        {
            self.bounds = bounds;
            self.decoration
                .set_bounds(bounds.left, bounds.right, bounds.top, bounds.bottom);
        }
        if let Some(toplevel_size) = self.pending_toplevel_size.take() {
            self.toplevel_size = toplevel_size;
        }
        if let Some(z_index) = self.pending_z_index.take() {
            self.decoration.set_z_index(z_index);
        }

        self.surface.bounds_changed(self.widget_bounds());

        self.surface.update(runtime, compositor);
    }

    pub fn widget_bounds(&self) -> iced::Size<u32> {
        iced::Size::new(
            self.toplevel_size.width + self.extents.left + self.extents.right,
            self.toplevel_size.height + self.extents.top + self.extents.bottom,
        )
    }
}
