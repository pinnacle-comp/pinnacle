use std::mem;

use smithay::{
    backend::{
        egl::EGLDevice,
        renderer::{
            Bind, Color32F, ExportMem, Offscreen, buffer_dimensions,
            damage::OutputDamageTracker,
            element::RenderElement,
            gles::{GlesRenderbuffer, GlesRenderer},
        },
    },
    output::Output,
    reexports::wayland_server::protocol::wl_shm,
    utils::{Buffer, Physical, Point, Rectangle, Size, Transform},
    wayland::{
        compositor,
        dmabuf::get_dmabuf,
        fractional_scale::with_fractional_scale,
        seat::WaylandFocus,
        shm::{shm_format_to_fourcc, with_buffer_contents},
    },
};
use tracing::error;

use crate::{
    protocol::{
        image_capture_source::Source,
        image_copy_capture::{
            ImageCopyCaptureHandler, ImageCopyCaptureState, delegate_image_copy_capture,
            frame::Frame,
            session::{Cursor, CursorSession, Session},
        },
    },
    render::{
        OutputRenderElement, output_render_elements,
        pointer::pointer_render_elements,
        util::{DynElement, damage::BufferDamageElement},
    },
    state::{Pinnacle, State, WithState},
};

impl ImageCopyCaptureHandler for State {
    fn image_copy_capture_state(&mut self) -> &mut ImageCopyCaptureState {
        &mut self.pinnacle.image_copy_capture_state
    }

    fn new_session(&mut self, session: Session) {
        let Some((buffer_size, scale)) = self.pinnacle.buffer_size_and_scale_for_session(&session)
        else {
            session.stopped();
            return;
        };

        session.resized(buffer_size);
        let trackers = SessionDamageTrackers::new(buffer_size, scale);

        match session.source() {
            Source::Output(wl_output) => {
                let Some(output) = Output::from_resource(&wl_output) else {
                    session.stopped();
                    return;
                };

                output.with_state_mut(|state| state.capture_sessions.insert(session, trackers));
            }
            Source::ForeignToplevel(ext_foreign_toplevel_handle_v1) => {
                let Some(window) = self
                    .pinnacle
                    .window_for_foreign_toplevel_handle(&ext_foreign_toplevel_handle_v1)
                else {
                    session.stopped();
                    return;
                };

                window.with_state_mut(|state| state.capture_sessions.insert(session, trackers));
            }
        }
    }

    fn new_cursor_session(&mut self, cursor_session: CursorSession) {
        match cursor_session.source() {
            Source::Output(wl_output) => {
                let Some(output) = Output::from_resource(wl_output) else {
                    return;
                };

                output.with_state_mut(|state| state.cursor_sessions.push(cursor_session));
            }
            Source::ForeignToplevel(ext_foreign_toplevel_handle_v1) => {
                let Some(window) = self
                    .pinnacle
                    .window_for_foreign_toplevel_handle(ext_foreign_toplevel_handle_v1)
                else {
                    return;
                };

                window.with_state_mut(|state| state.cursor_sessions.push(cursor_session));
            }
        }
    }

    fn session_destroyed(&mut self, session: Session) {
        for output in self.pinnacle.outputs.iter() {
            output.with_state_mut(|state| state.capture_sessions.remove(&session));
        }

        for win in self.pinnacle.windows.iter() {
            win.with_state_mut(|state| state.capture_sessions.remove(&session));
        }
    }

    fn cursor_session_destroyed(&mut self, cursor_session: CursorSession) {
        for output in self.pinnacle.outputs.iter() {
            output.with_state_mut(|state| {
                state
                    .cursor_sessions
                    .retain(|session| *session != cursor_session)
            });
        }

        for win in self.pinnacle.windows.iter() {
            win.with_state_mut(|state| {
                state
                    .cursor_sessions
                    .retain(|session| *session != cursor_session)
            });
        }
    }
}
delegate_image_copy_capture!(State);

impl State {
    /// Sets buffer constraints for all [`Session`]s globally.
    pub fn set_copy_capture_buffer_constraints(&mut self) {
        let shm_formats = [wl_shm::Format::Argb8888];

        let dmabuf_device = self
            .backend
            .with_renderer(|renderer| {
                EGLDevice::device_for_display(renderer.egl_context().display())
                    .ok()
                    .and_then(|device| device.try_get_render_node().ok().flatten())
            })
            .flatten();

        let dmabuf_formats = self
            .backend
            .with_renderer(|renderer| renderer.egl_context().dmabuf_render_formats().clone())
            .unwrap_or_default();

        self.pinnacle
            .image_copy_capture_state
            .set_buffer_constraints(shm_formats, dmabuf_device, dmabuf_formats);
    }

    /// Processes all active [`Session`]s by updating cursor positions and
    /// rendering pending frames.
    pub fn process_capture_sessions(&mut self) {
        let _span = tracy_client::span!();

        self.update_cursor_capture_positions();

        for win in self.pinnacle.windows.clone() {
            let mut sessions = win.with_state_mut(|state| mem::take(&mut state.capture_sessions));
            for (session, trackers) in sessions.iter_mut() {
                let Some((size, scale)) = self.pinnacle.buffer_size_and_scale_for_session(session)
                else {
                    session.stopped();
                    continue;
                };

                if (size, scale) != (trackers.size(), trackers.scale()) {
                    if size != trackers.size() {
                        session.resized(size);
                    }
                    *trackers = SessionDamageTrackers::new(size, scale);
                }

                let Some(frame) = session.get_pending_frame(size) else {
                    continue;
                };

                if let Some(cursor_session) = session.cursor_session() {
                    let hotspot = self
                        .pinnacle
                        .cursor_state
                        .cursor_hotspot(self.pinnacle.clock.now(), scale)
                        .unwrap_or_default();
                    cursor_session.set_hotspot(hotspot);
                }

                let elements = match session.cursor() {
                    Cursor::Hidden => self
                        .backend
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
                                .map(OutputRenderElement::from)
                                .collect::<Vec<_>>()
                        })
                        .unwrap(),
                    Cursor::Composited => self
                        .backend
                        .with_renderer(|renderer| {
                            let win_loc = self.pinnacle.space.element_location(&win);
                            let pointer_elements = if let Some(win_loc) = win_loc {
                                let hotspot = self
                                    .pinnacle
                                    .cursor_state
                                    .cursor_hotspot(self.pinnacle.clock.now(), scale)
                                    .unwrap_or_default();

                                let pointer_loc =
                                    self.pinnacle.seat.get_pointer().unwrap().current_location()
                                        - win_loc.to_f64()
                                        - win.total_decoration_offset().to_f64();

                                let (pointer_elements, _) = pointer_render_elements(
                                    pointer_loc.to_physical_precise_round(scale)
                                        - Point::new(hotspot.x, hotspot.y),
                                    scale,
                                    renderer,
                                    &mut self.pinnacle.cursor_state,
                                    self.pinnacle.dnd_icon.as_ref(),
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
                                .map(OutputRenderElement::from)
                                .chain(
                                    elements
                                        .popup_elements
                                        .into_iter()
                                        .chain(elements.surface_elements)
                                        .map(OutputRenderElement::from),
                                )
                                .collect::<Vec<_>>();
                            elements
                        })
                        .unwrap(),
                    Cursor::Standalone { pointer: _ } => self
                        .backend
                        .with_renderer(|renderer| {
                            let (pointer_elements, _) = pointer_render_elements(
                                (0, 0).into(),
                                scale,
                                renderer,
                                &mut self.pinnacle.cursor_state,
                                self.pinnacle.dnd_icon.as_ref(),
                                &self.pinnacle.clock,
                            );
                            pointer_elements
                                .into_iter()
                                .map(OutputRenderElement::from)
                                .collect()
                        })
                        .unwrap(),
                };

                self.handle_frame(frame, &elements, trackers);
            }
            win.with_state_mut(|state| state.capture_sessions = sessions);
        }

        for output in self.pinnacle.outputs.clone() {
            let mut sessions =
                output.with_state_mut(|state| mem::take(&mut state.capture_sessions));
            for (session, trackers) in sessions.iter_mut() {
                let Some((size, scale)) = self.pinnacle.buffer_size_and_scale_for_session(session)
                else {
                    session.stopped();
                    continue;
                };

                if (size, scale) != (trackers.size(), trackers.scale()) {
                    if size != trackers.size() {
                        session.resized(size);
                    }
                    *trackers = SessionDamageTrackers::new(size, scale);
                }

                let Some(frame) = session.get_pending_frame(size) else {
                    continue;
                };

                if let Some(cursor_session) = session.cursor_session() {
                    let hotspot = self
                        .pinnacle
                        .cursor_state
                        .cursor_hotspot(self.pinnacle.clock.now(), scale)
                        .unwrap_or_default();
                    cursor_session.set_hotspot(hotspot);
                }

                let elements = match session.cursor() {
                    Cursor::Hidden => self
                        .backend
                        .with_renderer(|renderer| {
                            output_render_elements(
                                &output,
                                renderer,
                                &self.pinnacle.space,
                                &self.pinnacle.z_index_stack,
                            )
                        })
                        .unwrap(),
                    Cursor::Composited => {
                        let Some(output_geo) = self.pinnacle.space.output_geometry(&output) else {
                            continue;
                        };

                        let hotspot = self
                            .pinnacle
                            .cursor_state
                            .cursor_hotspot(self.pinnacle.clock.now(), scale)
                            .unwrap_or_default();

                        let pointer_loc =
                            self.pinnacle.seat.get_pointer().unwrap().current_location()
                                - output_geo.loc.to_f64();
                        let scale = output.current_scale().fractional_scale();

                        self.backend
                            .with_renderer(|renderer| {
                                let (pointer_elements, _) = pointer_render_elements(
                                    pointer_loc.to_physical_precise_round(scale)
                                        - Point::new(hotspot.x, hotspot.y),
                                    scale,
                                    renderer,
                                    &mut self.pinnacle.cursor_state,
                                    self.pinnacle.dnd_icon.as_ref(),
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
                                    .map(OutputRenderElement::from)
                                    .chain(elements)
                                    .collect::<Vec<_>>()
                            })
                            .unwrap()
                    }
                    Cursor::Standalone { pointer: _ } => {
                        let scale = output.current_scale().fractional_scale();
                        self.backend
                            .with_renderer(|renderer| {
                                let (pointer_elements, _) = pointer_render_elements(
                                    (0, 0).into(),
                                    scale,
                                    renderer,
                                    &mut self.pinnacle.cursor_state,
                                    self.pinnacle.dnd_icon.as_ref(),
                                    &self.pinnacle.clock,
                                );
                                pointer_elements
                                    .into_iter()
                                    .map(OutputRenderElement::from)
                                    .collect()
                            })
                            .unwrap()
                    }
                };

                self.handle_frame(frame, &elements, trackers);
            }

            output.with_state_mut(|state| state.capture_sessions = sessions);
        }
    }

    /// Sends copy-capture clients updated cursor positions relative to their source.
    fn update_cursor_capture_positions(&mut self) {
        let _span = tracy_client::span!();

        let cursor_loc = self.pinnacle.seat.get_pointer().unwrap().current_location();

        for output in self.pinnacle.outputs.iter() {
            let sessions = output.with_state(|state| state.cursor_sessions.clone());

            if sessions.is_empty() {
                continue;
            }

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

            for session in sessions {
                session.set_position(position);
            }
        }

        for window in self.pinnacle.windows.iter() {
            let sessions = window.with_state(|state| state.cursor_sessions.clone());

            if sessions.is_empty() {
                continue;
            }

            let Some(window_loc) = self.pinnacle.space.element_location(window) else {
                continue;
            };

            let Some(surface) = window.wl_surface() else {
                continue;
            };

            let fractional_scale = compositor::with_states(&surface, |data| {
                with_fractional_scale(data, |scale| scale.preferred_scale())
            })
            .unwrap_or(1.0);

            let cursor_loc =
                cursor_loc - window_loc.to_f64() - window.total_decoration_offset().to_f64();

            let cursor_loc: Point<i32, Physical> =
                cursor_loc.to_physical_precise_round(fractional_scale);
            let cursor_loc: Point<i32, Buffer> = (cursor_loc.x, cursor_loc.y).into();

            let mut cursor_geo = self
                .pinnacle
                .cursor_state
                .cursor_geometry(self.pinnacle.clock.now(), fractional_scale)
                .unwrap_or_default();

            cursor_geo.loc += cursor_loc;

            let buffer_size: Size<i32, Physical> = (**window)
                .geometry()
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

            for session in sessions {
                session.set_position(position);
            }
        }
    }

    /// Renders elements to a [`Frame`] if they caused damage, then notifies the client.
    fn handle_frame(
        &mut self,
        frame: Frame,
        elements: &[impl RenderElement<GlesRenderer>],
        trackers: &mut SessionDamageTrackers,
    ) {
        let (damage, _) = trackers.damage.damage_output(1, elements).unwrap();
        let damage = damage.map(|damage| damage.as_slice()).unwrap_or_default();
        if damage.is_empty() {
            frame.submit(Transform::Normal, []);
            return;
        }

        let buffer = frame.buffer();
        let buffer_size = buffer_dimensions(&buffer).expect("this buffer is handled");

        let client_damage = frame
            .buffer_damage()
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
                .map(DynElement::new)
                .chain(elements.iter().map(DynElement::new))
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

            frame.submit(
                Transform::Normal,
                damage.iter().map(|rect| {
                    Rectangle::new(
                        (rect.loc.x, rect.loc.y).into(),
                        (rect.size.w, rect.size.h).into(),
                    )
                }),
            );
        });
    }
}

impl Pinnacle {
    /// Returns the target buffer size and scale for a [`Session`].
    ///
    /// Returns `None` if the source doesn't exist.
    fn buffer_size_and_scale_for_session(
        &mut self,
        session: &Session,
    ) -> Option<(Size<i32, Buffer>, f64)> {
        match session.source() {
            Source::Output(wl_output) => {
                let output = Output::from_resource(&wl_output)?;
                let scale = output.current_scale().fractional_scale();

                if matches!(session.cursor(), Cursor::Standalone { .. }) {
                    let geo = self
                        .cursor_state
                        .cursor_geometry(self.clock.now(), scale)
                        .unwrap_or(Rectangle::from_size((1, 1).into()));
                    Some((geo.size, scale))
                } else {
                    let size = output.current_mode()?.size;
                    Some(((size.w, size.h).into(), scale))
                }
            }
            Source::ForeignToplevel(ext_foreign_toplevel_handle_v1) => {
                let window =
                    self.window_for_foreign_toplevel_handle(&ext_foreign_toplevel_handle_v1)?;

                let surface = window.wl_surface()?;

                let fractional_scale = compositor::with_states(&surface, |data| {
                    with_fractional_scale(data, |scale| scale.preferred_scale())
                })?;

                if matches!(session.cursor(), Cursor::Standalone { .. }) {
                    let geo = self
                        .cursor_state
                        .cursor_geometry(self.clock.now(), fractional_scale)
                        .unwrap_or(Rectangle::from_size((1, 1).into()));
                    Some((geo.size, fractional_scale))
                } else {
                    let size = (*window)
                        .geometry()
                        .size
                        .to_f64()
                        .to_buffer(fractional_scale, Transform::Normal)
                        .to_i32_round();

                    Some((size, fractional_scale))
                }
            }
        }
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
