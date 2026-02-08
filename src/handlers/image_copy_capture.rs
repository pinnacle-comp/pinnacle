use std::{cell::RefCell, collections::HashMap};

use smithay::{
    backend::{
        allocator::{Fourcc as DrmFourcc, Modifier as DrmModifier},
        egl::EGLDevice,
        renderer::{
            Bind, Color32F, ExportMem, Offscreen, buffer_dimensions,
            damage::OutputDamageTracker,
            element::RenderElement,
            gles::{GlesRenderbuffer, GlesRenderer},
        },
    },
    delegate_image_copy_capture,
    output::Output,
    reexports::wayland_server::protocol::wl_shm,
    utils::{Buffer, Physical, Point, Rectangle, Size, Transform},
    wayland::{
        compositor,
        dmabuf::get_dmabuf,
        fractional_scale::with_fractional_scale,
        image_capture_source::ImageCaptureSource,
        image_copy_capture::{
            BufferConstraints, CaptureFailureReason, CursorSession, CursorSessionRef,
            DmabufConstraints, Frame, FrameRef, ImageCopyCaptureHandler, ImageCopyCaptureState,
            Session, SessionRef,
        },
        seat::WaylandFocus,
        shm::{shm_format_to_fourcc, with_buffer_contents},
    },
};
use tracing::error;

use crate::{
    handlers::image_capture_source::ImageCaptureSourceKind,
    render::{
        output_render_elements,
        pointer::pointer_render_elements,
        util::{DynElement, damage::BufferDamageElement},
    },
    state::{Pinnacle, State, WithState},
    window::WindowElement,
};

const SUPPORTED_SHM_FORMATS: &[wl_shm::Format] = &[wl_shm::Format::Argb8888];

impl ImageCopyCaptureHandler for State {
    fn image_copy_capture_state(&mut self) -> &mut ImageCopyCaptureState {
        &mut self.pinnacle.image_copy_capture_state
    }

    fn capture_constraints(&mut self, source: &ImageCaptureSource) -> Option<BufferConstraints> {
        let (size, _scale) = self.pinnacle.buffer_size_and_scale_for_source(source)?;

        Some(self.buffer_constraints(size))
    }

    fn new_session(&mut self, session: Session) {
        let Some((size, scale)) = self
            .pinnacle
            .buffer_size_and_scale_for_source(&session.source())
        else {
            return;
        };

        session
            .user_data()
            .insert_if_missing(|| RefCell::new(SessionDamageTrackers::new(size, scale)));

        self.pinnacle.capture_sessions.push(session);
    }

    fn frame(&mut self, session: &SessionRef, frame: Frame) {
        let f = session.user_data().get_or_insert(|| RefCell::new(None));
        *f.borrow_mut() = Some(frame);
    }

    fn cursor_capture_constraints(
        &mut self,
        source: &ImageCaptureSource,
    ) -> Option<BufferConstraints> {
        let (size, _scale) = self
            .pinnacle
            .buffer_size_and_scale_for_cursor_source(source)?;

        Some(self.buffer_constraints(size))
    }

    fn new_cursor_session(&mut self, session: CursorSession) {
        let Some((size, scale)) = self
            .pinnacle
            .buffer_size_and_scale_for_cursor_source(&session.source())
        else {
            return;
        };

        session
            .user_data()
            .insert_if_missing(|| RefCell::new(SessionDamageTrackers::new(size, scale)));

        self.pinnacle.cursor_capture_sessions.push(session);
    }

    fn cursor_frame(&mut self, session: &CursorSessionRef, frame: Frame) {
        let f = session.user_data().get_or_insert(|| RefCell::new(None));
        *f.borrow_mut() = Some(frame);
    }

    fn frame_aborted(&mut self, frame: FrameRef) {
        let _span = tracy_client::span!();

        for session in self.pinnacle.capture_sessions.iter() {
            session
                .user_data()
                .get_or_insert(|| RefCell::new(None::<Frame>))
                .borrow_mut()
                .take_if(|f| **f == frame);
        }

        for session in self.pinnacle.cursor_capture_sessions.iter() {
            session
                .user_data()
                .get_or_insert(|| RefCell::new(None::<Frame>))
                .borrow_mut()
                .take_if(|f| **f == frame);
        }
    }

    fn session_destroyed(&mut self, session: SessionRef) {
        self.pinnacle
            .capture_sessions
            .retain(|sess| *sess != session);
    }

    fn cursor_session_destroyed(&mut self, session: CursorSessionRef) {
        self.pinnacle
            .cursor_capture_sessions
            .retain(|sess| *sess != session);
    }
}
delegate_image_copy_capture!(State);

impl State {
    pub fn process_capture_sessions(&mut self) {
        let _span = tracy_client::span!();

        self.update_cursor_capture_positions();

        for session in self
            .pinnacle
            .capture_sessions
            .iter()
            .map(|session| session.as_ref())
            .collect::<Vec<_>>()
        {
            self.process_capture_session(session);
        }

        for session in self
            .pinnacle
            .cursor_capture_sessions
            .iter()
            .map(|session| session.as_ref())
            .collect::<Vec<_>>()
        {
            self.process_cursor_capture_session(session);
        }
    }

    fn process_capture_session(&mut self, session: SessionRef) {
        let _span = tracy_client::span!();

        let maybe_frame = session
            .user_data()
            .get_or_insert(|| RefCell::new(None::<Frame>));

        let Some(frame) = maybe_frame.borrow_mut().take() else {
            return;
        };

        let Some((size, scale)) = self
            .pinnacle
            .buffer_size_and_scale_for_source(&session.source())
        else {
            return;
        };

        let trackers = session
            .user_data()
            .get_or_insert(|| RefCell::new(SessionDamageTrackers::new(size, scale)));

        let mut trackers = trackers.borrow_mut();

        if (size, scale) != (trackers.size(), trackers.scale()) {
            session.update_constraints(self.buffer_constraints(size));
            frame.fail(CaptureFailureReason::BufferConstraints);
            *trackers = SessionDamageTrackers::new(size, scale);
            return;
        }

        let elements = match session
            .source()
            .user_data()
            .get::<ImageCaptureSourceKind>()
            .unwrap()
        {
            ImageCaptureSourceKind::Output(output) => {
                let Some(output) = output.upgrade() else {
                    frame.fail(CaptureFailureReason::Stopped);
                    return;
                };

                let elements = if session.draw_cursor() {
                    let Some(output_geo) = self.pinnacle.space.output_geometry(&output) else {
                        frame.fail(CaptureFailureReason::Stopped);
                        return;
                    };

                    let pointer_loc = self.pinnacle.seat.get_pointer().unwrap().current_location()
                        - output_geo.loc.to_f64();
                    let scale = output.current_scale().fractional_scale();

                    self.backend
                        .with_renderer(|renderer| {
                            let (pointer_elements, _) = pointer_render_elements(
                                pointer_loc,
                                scale,
                                renderer,
                                &mut self.pinnacle.cursor_state,
                                &self.pinnacle.clock,
                            );
                            let elements = output_render_elements(
                                &output,
                                renderer,
                                &self.pinnacle.space,
                                &self.pinnacle.z_index_stack,
                            );
                            pointer_elements
                                .into_iter()
                                .map(DynElement::owned)
                                .chain(elements.into_iter().map(DynElement::owned))
                                .collect::<Vec<_>>()
                        })
                        .unwrap()
                } else {
                    self.backend
                        .with_renderer(|renderer| {
                            output_render_elements(
                                &output,
                                renderer,
                                &self.pinnacle.space,
                                &self.pinnacle.z_index_stack,
                            )
                            .into_iter()
                            .map(DynElement::owned)
                            .collect::<Vec<_>>()
                        })
                        .unwrap()
                };

                elements
            }
            ImageCaptureSourceKind::Toplevel(foreign_toplevel) => {
                let Some(foreign_toplevel) = foreign_toplevel.upgrade() else {
                    frame.fail(CaptureFailureReason::Stopped);
                    return;
                };

                let Some(win) = self
                    .pinnacle
                    .window_for_foreign_toplevel_handle(&foreign_toplevel)
                    .cloned()
                else {
                    frame.fail(CaptureFailureReason::Stopped);
                    return;
                };

                let elements = if session.draw_cursor() {
                    self.backend
                        .with_renderer(|renderer| {
                            let win_loc = self.pinnacle.space.element_location(&win);
                            let pointer_elements = if let Some(win_loc) = win_loc {
                                let pointer_loc =
                                    self.pinnacle.seat.get_pointer().unwrap().current_location()
                                        - win_loc.to_f64()
                                        - win.total_decoration_offset().to_f64();

                                let (pointer_elements, _) = pointer_render_elements(
                                    pointer_loc,
                                    scale,
                                    renderer,
                                    &mut self.pinnacle.cursor_state,
                                    &self.pinnacle.clock,
                                );
                                pointer_elements
                            } else {
                                Vec::new()
                            };
                            let elements = win.render_elements(
                                renderer,
                                (0, 0).into(),
                                scale.into(),
                                1.0,
                                false,
                            );
                            let elements = pointer_elements
                                .into_iter()
                                .map(DynElement::owned)
                                .chain(
                                    elements
                                        .popup_elements
                                        .into_iter()
                                        .chain(elements.surface_elements)
                                        .map(DynElement::owned),
                                )
                                .collect::<Vec<_>>();
                            elements
                        })
                        .unwrap()
                } else {
                    self.backend
                        .with_renderer(|renderer| {
                            let elements = win.render_elements(
                                renderer,
                                (0, 0).into(),
                                scale.into(),
                                1.0,
                                false,
                            );

                            elements
                                .popup_elements
                                .into_iter()
                                .chain(elements.surface_elements)
                                .map(DynElement::owned)
                                .collect::<Vec<_>>()
                        })
                        .unwrap()
                };

                elements
            }
        };

        if let Err(frame) = self.handle_frame(frame, &elements, &mut trackers) {
            *maybe_frame.borrow_mut() = Some(frame);
        }
    }

    fn process_cursor_capture_session(&mut self, session: CursorSessionRef) {
        let _span = tracy_client::span!();

        let maybe_frame = session
            .user_data()
            .get_or_insert(|| RefCell::new(None::<Frame>));

        let Some(frame) = maybe_frame.borrow_mut().take() else {
            return;
        };

        let Some((size, scale)) = self
            .pinnacle
            .buffer_size_and_scale_for_cursor_source(&session.source())
        else {
            return;
        };

        let trackers = session
            .user_data()
            .get_or_insert(|| RefCell::new(SessionDamageTrackers::new(size, scale)));

        let mut trackers = trackers.borrow_mut();

        if (size, scale) != (trackers.size(), trackers.scale()) {
            session.update_constraints(self.buffer_constraints(size));
            frame.fail(CaptureFailureReason::BufferConstraints);
            *trackers = SessionDamageTrackers::new(size, scale);
            return;
        }

        let cursor_offset = self
            .pinnacle
            .cursor_state
            .cursor_geometry(self.pinnacle.clock.now(), scale)
            .unwrap_or_default()
            .loc;

        session.set_cursor_hotspot(Point::default() - cursor_offset);

        let elements = self
            .backend
            .with_renderer(|renderer| {
                let (pointer_elements, _) = pointer_render_elements(
                    (0.0, 0.0).into(),
                    scale,
                    renderer,
                    &mut self.pinnacle.cursor_state,
                    &self.pinnacle.clock,
                );
                let pointer_elements =
                    crate::render::util::to_local_coord_space(pointer_elements, scale);
                pointer_elements
                    .into_iter()
                    .map(DynElement::owned)
                    .collect::<Vec<_>>()
            })
            .unwrap();

        if let Err(frame) = self.handle_frame(frame, &elements, &mut trackers) {
            *maybe_frame.borrow_mut() = Some(frame);
        }
    }

    /// Sends copy-capture clients updated cursor positions relative to their source.
    fn update_cursor_capture_positions(&mut self) {
        let _span = tracy_client::span!();

        let cursor_loc = self.pinnacle.seat.get_pointer().unwrap().current_location();

        for session in self.pinnacle.cursor_capture_sessions.iter() {
            // PERF: We recompute the location every time regardless if we've already done it for a
            // window or output. We should be able to cache this if needed.

            match session
                .source()
                .user_data()
                .get::<ImageCaptureSourceKind>()
                .unwrap()
            {
                ImageCaptureSourceKind::Output(output) => {
                    let Some(output) = output.upgrade() else {
                        return;
                    };

                    let cursor_loc: Point<i32, Physical> = (cursor_loc
                        - output.current_location().to_f64())
                    .to_physical_precise_round(output.current_scale().fractional_scale());
                    let cursor_loc: Point<i32, Buffer> = (cursor_loc.x, cursor_loc.y).into();

                    let mut cursor_geo = self
                        .pinnacle
                        .cursor_state
                        .cursor_geometry(
                            self.pinnacle.clock.now(),
                            output.current_scale().fractional_scale(),
                        )
                        .unwrap_or_default();

                    cursor_geo.loc += cursor_loc;

                    let mode_size = output
                        .current_mode()
                        .map(|mode| mode.size)
                        .unwrap_or_default();
                    let mode_rect: Rectangle<i32, Buffer> =
                        Rectangle::from_size((mode_size.w, mode_size.h).into());

                    let position = if cursor_geo.overlaps(mode_rect) {
                        Some(cursor_loc)
                    } else {
                        None
                    };

                    session.set_cursor_pos(position);
                }
                ImageCaptureSourceKind::Toplevel(foreign_toplevel) => {
                    let Some(foreign_toplevel) = foreign_toplevel.upgrade() else {
                        return;
                    };

                    let Some(window) = self
                        .pinnacle
                        .window_for_foreign_toplevel_handle(&foreign_toplevel)
                        .cloned()
                    else {
                        return;
                    };

                    let Some(window_loc) = self.pinnacle.space.element_location(&window) else {
                        return;
                    };

                    let Some(surface) = window.wl_surface() else {
                        return;
                    };

                    let fractional_scale = compositor::with_states(&surface, |data| {
                        with_fractional_scale(data, |scale| scale.preferred_scale())
                    })
                    .unwrap_or(1.0);

                    let cursor_loc = cursor_loc
                        - window_loc.to_f64()
                        - window.total_decoration_offset().to_f64();

                    let cursor_loc: Point<i32, Physical> =
                        cursor_loc.to_physical_precise_round(fractional_scale);
                    let cursor_loc: Point<i32, Buffer> = (cursor_loc.x, cursor_loc.y).into();

                    let mut cursor_geo = self
                        .pinnacle
                        .cursor_state
                        .cursor_geometry(self.pinnacle.clock.now(), fractional_scale)
                        .unwrap_or_default();

                    cursor_geo.loc += cursor_loc;

                    let buffer_size: Size<i32, Physical> = window
                        .geometry_without_decorations()
                        .size
                        .to_f64()
                        .to_physical_precise_round(fractional_scale);
                    let buffer_geo: Rectangle<i32, Buffer> =
                        Rectangle::from_size((buffer_size.w, buffer_size.h).into());

                    let position = if cursor_geo.overlaps(buffer_geo) {
                        Some(cursor_loc)
                    } else {
                        None
                    };

                    session.set_cursor_pos(position);
                }
            }
        }
    }

    fn buffer_constraints(&mut self, size: Size<i32, Buffer>) -> BufferConstraints {
        let shm_formats = SUPPORTED_SHM_FORMATS.to_vec();

        // TODO: maybe cache these
        let dmabuf_device = self
            .backend
            .with_renderer(|renderer| {
                EGLDevice::device_for_display(renderer.egl_context().display())
                    .ok()
                    .and_then(|device| device.try_get_render_node().ok().flatten())
            })
            .flatten();

        let dmabuf_constraints = dmabuf_device.map(|device| {
            let dmabuf_formats = self
                .backend
                .with_renderer(|renderer| renderer.egl_context().dmabuf_render_formats().clone())
                .unwrap_or_default();

            let mut formats: HashMap<DrmFourcc, Vec<DrmModifier>> = HashMap::new();

            for format in dmabuf_formats.into_iter() {
                formats
                    .entry(format.code)
                    .or_default()
                    .push(format.modifier);
            }

            DmabufConstraints {
                node: device,
                formats: formats.into_iter().collect(),
            }
        });

        BufferConstraints {
            size,
            shm: shm_formats,
            dma: dmabuf_constraints,
        }
    }

    /// Renders elements to a [`Frame`] if they caused damage, then notifies the client.
    ///
    /// If there was no damage, an `Err` containing the frame is returned.
    fn handle_frame(
        &mut self,
        frame: Frame,
        elements: &[impl RenderElement<GlesRenderer>],
        trackers: &mut SessionDamageTrackers,
    ) -> Result<(), Frame> {
        let _span = tracy_client::span!();

        let (damage, _) = trackers.damage.damage_output(1, elements).unwrap();
        let damage = damage.map(|damage| damage.as_slice()).unwrap_or_default();
        if damage.is_empty() {
            return Err(frame);
        }

        let buffer = frame.buffer();
        let buffer_size = buffer_dimensions(&buffer).expect("this buffer is handled");

        let client_damage = frame
            .damage()
            .into_iter()
            .map(BufferDamageElement::new)
            .collect::<Vec<_>>();

        self.backend.with_renderer(|renderer| {
            let mut dmabuf;
            let mut renderbuffer: GlesRenderbuffer;
            let mut shm_format = None;

            let mut framebuffer = if let Ok(dma) = get_dmabuf(&buffer).cloned() {
                dmabuf = dma;
                renderer.bind(&mut dmabuf).unwrap()
            } else if let Ok(format) = with_buffer_contents(&buffer, |_, _, data| data.format) {
                shm_format = Some(format);
                renderbuffer = renderer
                    .create_buffer(shm_format_to_fourcc(format).unwrap(), buffer_size)
                    .unwrap();
                renderer.bind(&mut renderbuffer).unwrap()
            } else {
                panic!("captured frame that doesn't have a shm or dma buffer");
            };

            let elements = client_damage
                .iter()
                .map(DynElement::borrowed)
                .chain(elements.iter().map(DynElement::borrowed))
                .collect::<Vec<_>>();

            let rendered_damage = trackers
                .render
                .render_output(
                    renderer,
                    &mut framebuffer,
                    1,
                    &elements,
                    Color32F::TRANSPARENT,
                )
                .unwrap();

            if let Some(shm_format) = shm_format {
                let mapping = renderer
                    .copy_framebuffer(
                        &framebuffer,
                        Rectangle::from_size(buffer_size),
                        shm_format_to_fourcc(shm_format).unwrap(),
                    )
                    .unwrap();

                let bytes = renderer.map_texture(&mapping).unwrap();

                for rect in rendered_damage.damage.unwrap() {
                    if let Err(err) = crate::render::util::blit(
                        bytes,
                        buffer_size,
                        Rectangle::new(
                            (rect.loc.x, rect.loc.y).into(),
                            (rect.size.w, rect.size.h).into(),
                        ),
                        &buffer,
                    ) {
                        error!("failed to copy capture: {err}");
                        return;
                    }
                }
            }

            let damage = damage
                .iter()
                .map(|rect| {
                    Rectangle::new(
                        (rect.loc.x, rect.loc.y).into(),
                        (rect.size.w, rect.size.h).into(),
                    )
                })
                .collect::<Vec<_>>();

            frame.success(Transform::Normal, damage, self.pinnacle.clock.now());
        });

        Ok(())
    }
}

impl Pinnacle {
    pub fn stop_capture_sessions_for_output(&mut self, output: &Output) {
        self.capture_sessions.retain(|session| {
            let session_is_for_this_output = matches!(
                session
                    .source()
                    .user_data()
                    .get::<ImageCaptureSourceKind>()
                    .unwrap(),
                ImageCaptureSourceKind::Output(weak) if weak == output
            );
            !session_is_for_this_output
        });

        self.cursor_capture_sessions.retain(|session| {
            let session_is_for_this_output = matches!(
                session
                    .source()
                    .user_data()
                    .get::<ImageCaptureSourceKind>()
                    .unwrap(),
                ImageCaptureSourceKind::Output(weak) if weak == output
            );
            !session_is_for_this_output
        });
    }

    pub fn stop_capture_sessions_for_window(&mut self, window: &WindowElement) {
        self.capture_sessions.retain(|session| {
            let session_is_for_this_window = matches!(
                session
                    .source()
                    .user_data()
                    .get::<ImageCaptureSourceKind>()
                    .unwrap(),
                ImageCaptureSourceKind::Toplevel(weak)
                    if window.with_state(|state| {
                        // TODO: figure out if this upgrade is correct
                        weak.upgrade().map(|handle| handle.identifier())
                            == state
                                .foreign_toplevel_list_handle
                                .as_ref()
                                .map(|handle| handle.identifier())
                    })
            );
            !session_is_for_this_window
        });

        self.cursor_capture_sessions.retain(|session| {
            let session_is_for_this_window = matches!(
                session
                    .source()
                    .user_data()
                    .get::<ImageCaptureSourceKind>()
                    .unwrap(),
                ImageCaptureSourceKind::Toplevel(weak)
                    if window.with_state(|state| {
                        weak.upgrade().map(|handle| handle.identifier())
                            == state
                                .foreign_toplevel_list_handle
                                .as_ref()
                                .map(|handle| handle.identifier())
                    })
            );
            !session_is_for_this_window
        });
    }

    /// Returns the target buffer size and scale for an [`ImageCaptureSource`].
    ///
    /// Returns `None` if the source doesn't exist.
    fn buffer_size_and_scale_for_source(
        &mut self,
        source: &ImageCaptureSource,
    ) -> Option<(Size<i32, Buffer>, f64)> {
        let kind = source
            .user_data()
            .get::<ImageCaptureSourceKind>()
            .expect("source should have source here");

        match kind {
            ImageCaptureSourceKind::Output(output) => {
                let output = output.upgrade()?;
                let scale = output.current_scale().fractional_scale();

                let size = output.current_mode()?.size;
                Some(((size.w, size.h).into(), scale))
            }
            ImageCaptureSourceKind::Toplevel(foreign_toplevel) => {
                let foreign_toplevel = foreign_toplevel.upgrade()?;
                let window = self.window_for_foreign_toplevel_handle(&foreign_toplevel)?;

                let surface = window.wl_surface()?;

                let fractional_scale = compositor::with_states(&surface, |data| {
                    with_fractional_scale(data, |scale| scale.preferred_scale())
                })?;

                let size = window
                    .geometry_without_decorations()
                    .size
                    .to_f64()
                    .to_buffer(fractional_scale, Transform::Normal)
                    .to_i32_round();

                Some((size, fractional_scale))
            }
        }
    }

    fn buffer_size_and_scale_for_cursor_source(
        &mut self,
        source: &ImageCaptureSource,
    ) -> Option<(Size<i32, Buffer>, f64)> {
        let kind = source
            .user_data()
            .get::<ImageCaptureSourceKind>()
            .expect("source should have source here");

        let scale = match kind {
            ImageCaptureSourceKind::Output(output) => {
                let output = output.upgrade()?;
                output.current_scale().fractional_scale()
            }
            ImageCaptureSourceKind::Toplevel(foreign_toplevel) => {
                let foreign_toplevel = foreign_toplevel.upgrade()?;
                let window = self.window_for_foreign_toplevel_handle(&foreign_toplevel)?;

                let surface = window.wl_surface()?;

                let fractional_scale = compositor::with_states(&surface, |data| {
                    with_fractional_scale(data, |scale| scale.preferred_scale())
                })?;

                fractional_scale
            }
        };

        let geo = self
            .cursor_state
            .cursor_geometry(self.clock.now(), scale)
            .unwrap_or(Rectangle::from_size((1, 1).into()));
        Some((geo.size, scale))
    }
}

/// Damage trackers for copy-capture sessions.
#[derive(Debug)]
pub struct SessionDamageTrackers {
    /// The "something has changed on-screen" damage tracker.
    ///
    /// This tracks actual screen damage to see if a new frame
    /// should be rendered.
    damage: OutputDamageTracker,
    /// The rendering damage tracker.
    ///
    /// This is used to render to session buffers, and the returned damage
    /// is used to optimize blitting into said buffers.
    render: OutputDamageTracker,
}

impl SessionDamageTrackers {
    /// Creates a new set of damage trackers for handling copy-capture sessions.
    fn new(size: Size<i32, Buffer>, scale: f64) -> Self {
        Self {
            damage: OutputDamageTracker::new((size.w, size.h), scale, Transform::Normal),
            render: OutputDamageTracker::new((size.w, size.h), scale, Transform::Normal),
        }
    }

    /// Returns the current buffer size of these trackers.
    fn size(&self) -> Size<i32, Buffer> {
        let (size, _, _) = self.render.mode().try_into().unwrap();
        (size.w, size.h).into()
    }

    /// Returns the current scale of these trackers.
    fn scale(&self) -> f64 {
        let (_, scale, _) = self.render.mode().try_into().unwrap();
        scale.x
    }
}
