use std::{
    borrow::Cow,
    cell::RefCell,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    time::Duration,
};

use smithay::{
    desktop::{
        WindowSurfaceType,
        utils::{
            OutputPresentationFeedback, bbox_from_surface_tree, send_dmabuf_feedback_surface_tree,
            send_frames_surface_tree, take_presentation_feedback_surface_tree,
            under_from_surface_tree, with_surfaces_surface_tree,
        },
    },
    output::Output,
    reexports::{
        wayland_protocols::wp::presentation_time::server::wp_presentation_feedback,
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{IsAlive, Logical, Point, Rectangle, Serial, user_data::UserDataMap},
    wayland::{
        compositor::{self, SurfaceData},
        dmabuf::DmabufFeedback,
        seat::WaylandFocus,
    },
};

use crate::{
    protocol::snowcap_decoration::{self, Bounds, DecorationSurfaceCachedState},
    state::WithState,
    util::transaction::Transaction,
};

static DECORATION_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Clone)]
pub struct DecorationSurface(Arc<DecorationSurfaceInner>);

impl PartialEq for DecorationSurface {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl Eq for DecorationSurface {}

impl std::hash::Hash for DecorationSurface {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.id.hash(state);
    }
}

#[derive(Debug)]
struct DecorationSurfaceInner {
    id: u32,
    surface: snowcap_decoration::DecorationSurface,
    userdata: UserDataMap,
}

impl IsAlive for DecorationSurface {
    fn alive(&self) -> bool {
        self.0.surface.alive()
    }
}

impl DecorationSurface {
    pub fn new(surface: snowcap_decoration::DecorationSurface) -> Self {
        Self(Arc::new(DecorationSurfaceInner {
            id: DECORATION_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            surface,
            userdata: UserDataMap::new(),
        }))
    }

    pub fn decoration_surface(&self) -> &snowcap_decoration::DecorationSurface {
        &self.0.surface
    }

    pub fn wl_surface(&self) -> &WlSurface {
        self.0.surface.wl_surface()
    }

    pub fn cached_state(&self) -> DecorationSurfaceCachedState {
        compositor::with_states(self.0.surface.wl_surface(), |states| {
            *states
                .cached_state
                .get::<DecorationSurfaceCachedState>()
                .current()
        })
    }

    pub fn bounds(&self) -> Bounds {
        self.cached_state().bounds
    }

    pub fn geometry(&self) -> Rectangle<i32, Logical> {
        self.cached_state().geometry
    }

    pub fn bbox(&self) -> Rectangle<i32, Logical> {
        bbox_from_surface_tree(self.0.surface.wl_surface(), (0, 0))
    }

    pub fn surface_under<P: Into<Point<f64, Logical>>>(
        &self,
        point: P,
        surface_type: WindowSurfaceType,
    ) -> Option<(WlSurface, Point<i32, Logical>)> {
        let point = point.into();
        let surface = self.wl_surface();

        if surface_type.contains(WindowSurfaceType::TOPLEVEL) {
            return under_from_surface_tree(surface, point, (0, 0), surface_type);
        }

        None
    }

    pub fn send_frame<T, F>(
        &self,
        output: &Output,
        time: T,
        throttle: Option<Duration>,
        primary_scan_out_output: F,
    ) where
        T: Into<Duration>,
        F: FnMut(&WlSurface, &SurfaceData) -> Option<Output> + Copy,
    {
        let time = time.into();
        let surface = self.0.surface.wl_surface();

        send_frames_surface_tree(surface, output, time, throttle, primary_scan_out_output);

        // TODO: popups
    }

    pub fn send_dmabuf_feedback<'a, P, F>(
        &self,
        output: &Output,
        primary_scan_out_output: P,
        select_dmabuf_feedback: F,
    ) where
        P: FnMut(&WlSurface, &SurfaceData) -> Option<Output> + Copy,
        F: Fn(&WlSurface, &SurfaceData) -> &'a DmabufFeedback + Copy,
    {
        let surface = self.0.surface.wl_surface();

        send_dmabuf_feedback_surface_tree(
            surface,
            output,
            primary_scan_out_output,
            select_dmabuf_feedback,
        );

        // TODO: popups
    }

    pub fn take_presentation_feedback<F1, F2>(
        &self,
        output_feedback: &mut OutputPresentationFeedback,
        primary_scan_out_output: F1,
        presentation_feedback_flags: F2,
    ) where
        F1: FnMut(&WlSurface, &SurfaceData) -> Option<Output> + Copy,
        F2: FnMut(&WlSurface, &SurfaceData) -> wp_presentation_feedback::Kind + Copy,
    {
        let surface = self.0.surface.wl_surface();
        take_presentation_feedback_surface_tree(
            surface,
            output_feedback,
            primary_scan_out_output,
            presentation_feedback_flags,
        );

        // TODO: popups
    }

    pub fn with_surfaces<F>(&self, mut processor: F)
    where
        F: FnMut(&WlSurface, &SurfaceData),
    {
        let surface = self.0.surface.wl_surface();

        with_surfaces_surface_tree(surface, &mut processor);

        // TODO: popups
    }

    pub fn user_data(&self) -> &UserDataMap {
        &self.0.userdata
    }

    /// Takes and returns the most recent transaction that has been committed.
    pub fn take_pending_transaction(&self, commit_serial: Serial) -> Option<Transaction> {
        let mut txn = None;

        while let Some(previous_txn_serial) =
            self.with_state(|state| state.pending_transactions.first().map(|tx| tx.0))
        {
            // This drops all transactions older than the most recently committed to release them.
            if previous_txn_serial <= commit_serial {
                let (_, transaction) =
                    self.with_state_mut(|state| state.pending_transactions.remove(0));

                txn = Some(transaction);
            } else {
                break;
            }
        }

        txn
    }
}

impl WaylandFocus for DecorationSurface {
    fn wl_surface(&self) -> Option<Cow<'_, WlSurface>> {
        Some(Cow::Borrowed(self.0.surface.wl_surface()))
    }
}

#[derive(Debug, Default)]
pub struct DecorationSurfaceState {
    pub bounds_changed: AtomicBool,
    pub pending_transactions: Vec<(Serial, Transaction)>,
}

impl WithState for DecorationSurface {
    type State = DecorationSurfaceState;

    fn with_state<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&Self::State) -> T,
    {
        let state = self
            .user_data()
            .get_or_insert(RefCell::<DecorationSurfaceState>::default);
        func(&state.borrow())
    }

    fn with_state_mut<F, T>(&self, func: F) -> T
    where
        F: FnOnce(&mut Self::State) -> T,
    {
        let state = self
            .user_data()
            .get_or_insert(RefCell::<DecorationSurfaceState>::default);
        func(&mut state.borrow_mut())
    }
}
