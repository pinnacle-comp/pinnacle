use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard},
};

use smithay::{
    backend::{
        allocator::{Fourcc as DrmFourcc, Modifier as DrmModifier},
        drm::DrmNode,
        renderer::buffer_dimensions,
    },
    reexports::{
        wayland_protocols::ext::{
            image_capture_source::v1::server::ext_image_capture_source_v1::ExtImageCaptureSourceV1,
            image_copy_capture::v1::server::{
                ext_image_copy_capture_cursor_session_v1::{
                    self, ExtImageCopyCaptureCursorSessionV1,
                },
                ext_image_copy_capture_frame_v1::{self, ExtImageCopyCaptureFrameV1},
                ext_image_copy_capture_session_v1::{self, ExtImageCopyCaptureSessionV1},
            },
        },
        wayland_server::{
            Client, DataInit, Dispatch, DisplayHandle, Resource,
            protocol::{wl_pointer::WlPointer, wl_shm},
        },
    },
    utils::{Buffer, Size},
};

use crate::protocol::{
    image_capture_source::Source,
    image_copy_capture::{
        ImageCopyCaptureHandler, ImageCopyCaptureState,
        frame::{Frame, FrameData, FrameState},
    },
};

/// An active capture session.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Session {
    pub(super) session: ExtImageCopyCaptureSessionV1,
}

impl Session {
    /// Returns the [`Source`] of this session.
    pub fn source(&self) -> Source {
        self.data().source.data::<Source>().unwrap().clone()
    }

    /// Returns how the cursor should be handled during captures.
    pub fn cursor(&self) -> Cursor {
        self.data().cursor.clone()
    }

    /// Notifies the client that this session has stopped.
    pub fn stopped(&self) {
        self.session.stopped();
    }

    /// Notifies the client that the session has resized.
    ///
    /// This will also send buffer constraints afterward.
    pub fn resized(&self, new_size: Size<i32, Buffer>) {
        self.session
            .buffer_size(new_size.w as u32, new_size.h as u32);
        self.send_buffer_constraints();
    }

    pub(super) fn set_buffer_constraints(
        &self,
        shm_formats: Vec<wl_shm::Format>,
        dmabuf_device: Option<DrmNode>,
        dmabuf_formats: HashMap<DrmFourcc, Vec<DrmModifier>>,
    ) {
        self.data().shm_formats = shm_formats.into_iter().collect();
        self.data().dmabuf_device = dmabuf_device;
        self.data().dmabuf_formats = dmabuf_formats;

        self.send_buffer_constraints();
    }

    /// If the frame belongs to this session, removes it.
    ///
    /// Returns whether a frame was removed.
    pub(super) fn frame_destroyed(&self, frame: &ExtImageCopyCaptureFrameV1) -> bool {
        self.data().frame.take_if(|f| f == frame).is_some()
    }

    /// Sends buffer constraints for this session to the client.
    ///
    /// Constraints are taken from the last call to
    /// [`ImageCopyCaptureState::set_buffer_constraints`][super::ImageCopyCaptureState::set_buffer_constraints].
    ///
    /// This will fail any frame pending for capture.
    fn send_buffer_constraints(&self) {
        for format in self.data().shm_formats.iter().copied() {
            self.session.shm_format(format);
        }

        for (&code, modifiers) in self.data().dmabuf_formats.iter() {
            if code != DrmFourcc::Xrgb8888 && code != DrmFourcc::Argb8888 {
                // TODO: Sending all formats causes pipewire over xdg-desktop-portal-wlr to stop
                // working when resizing the buffer, figure that out
                continue;
            }
            let modifiers = modifiers
                .iter()
                .flat_map(|&modifier| u64::from(modifier).to_ne_bytes())
                .collect();
            self.session.dmabuf_format(code as u32, modifiers);
        }

        if let Some(device) = self.data().dmabuf_device {
            self.session
                .dmabuf_device(device.dev_id().to_ne_bytes().to_vec());
        }

        self.session.done();

        if let Some(frame) = self.data().frame.clone() {
            let mut frame_data = frame.data::<Mutex<FrameData>>().unwrap().lock().unwrap();
            if let FrameState::CaptureRequested | FrameState::Capturing = frame_data.frame_state {
                frame.failed(ext_image_copy_capture_frame_v1::FailureReason::BufferConstraints);
                frame_data.frame_state = FrameState::Failed;
            }
        }
    }

    /// Retrieves a requested frame for the given buffer size.
    ///
    /// This returns a [`Frame`] when the client has requested capture and
    /// the attached buffer is the same size as the provided size.
    /// If the sizes are different, the frame fails.
    pub fn get_pending_frame(&self, size: Size<i32, Buffer>) -> Option<Frame> {
        self.data()
            .frame
            .clone()
            .filter(|frame| {
                let mut data = frame.data::<Mutex<FrameData>>().unwrap().lock().unwrap();
                let capture_requested = data.frame_state == FrameState::CaptureRequested;

                capture_requested && {
                    let buffer = data.buffer.as_ref().unwrap();
                    let buffer_size = buffer_dimensions(buffer).unwrap_or_default();
                    if buffer_size != size {
                        data.frame_state = FrameState::Failed;
                        frame.failed(
                            ext_image_copy_capture_frame_v1::FailureReason::BufferConstraints,
                        );
                        false
                    } else {
                        true
                    }
                }
            })
            .map(Frame::new)
    }

    pub(super) fn frame(&self) -> Option<ExtImageCopyCaptureFrameV1> {
        self.data().frame.clone()
    }

    fn data(&self) -> MutexGuard<'_, SessionData> {
        self.session
            .data::<Mutex<SessionData>>()
            .unwrap()
            .lock()
            .unwrap()
    }
}

pub struct SessionData {
    pub(super) source: ExtImageCaptureSourceV1,
    pub(super) cursor: Cursor,
    pub(super) frame: Option<ExtImageCopyCaptureFrameV1>,
    pub(super) shm_formats: Vec<wl_shm::Format>,
    pub(super) dmabuf_formats: HashMap<DrmFourcc, Vec<DrmModifier>>,
    pub(super) dmabuf_device: Option<DrmNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// How cursors should be handled during copies.
pub enum Cursor {
    /// The cursor should be hidden.
    Hidden,
    /// The cursor should be composited onto the frame.
    Composited,
    /// Only the cursor should be drawn.
    Standalone { pointer: WlPointer },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorSession {
    pub(super) session: ExtImageCopyCaptureCursorSessionV1,
}

impl CursorSession {
    pub fn source(&self) -> &Source {
        self.session
            .data::<CursorSessionData>()
            .unwrap()
            .source
            .data::<Source>()
            .unwrap()
    }

    pub fn pointer(&self) -> &WlPointer {
        &self.session.data::<CursorSessionData>().unwrap().pointer
    }
}

pub struct CursorSessionData {
    pub(super) source: ExtImageCaptureSourceV1,
    pub(super) pointer: WlPointer,
}

impl<D> Dispatch<ExtImageCopyCaptureSessionV1, Mutex<SessionData>, D> for ImageCopyCaptureState
where
    D: Dispatch<ExtImageCopyCaptureFrameV1, Mutex<FrameData>> + ImageCopyCaptureHandler,
{
    fn request(
        _state: &mut D,
        _client: &Client,
        resource: &ExtImageCopyCaptureSessionV1,
        request: <ExtImageCopyCaptureSessionV1 as Resource>::Request,
        data: &Mutex<SessionData>,
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            ext_image_copy_capture_session_v1::Request::CreateFrame { frame } => {
                let frame = data_init.init(frame, Mutex::default());
                if data.lock().unwrap().frame.is_some() {
                    resource.post_error(
                        ext_image_copy_capture_session_v1::Error::DuplicateFrame,
                        "the previous frame must be destroyed before creating a new one",
                    );
                    return;
                }

                data.lock().unwrap().frame = Some(frame);
            }
            ext_image_copy_capture_session_v1::Request::Destroy => (),
            _ => (),
        }
    }

    fn destroyed(
        state: &mut D,
        _client: wayland_backend::server::ClientId,
        resource: &ExtImageCopyCaptureSessionV1,
        _data: &Mutex<SessionData>,
    ) {
        state
            .image_copy_capture_state()
            .sessions
            .retain(|session| session.session != *resource);

        state.session_destroyed(Session {
            session: resource.clone(),
        });
    }
}

impl<D> Dispatch<ExtImageCopyCaptureCursorSessionV1, CursorSessionData, D> for ImageCopyCaptureState
where
    D: Dispatch<ExtImageCopyCaptureSessionV1, Mutex<SessionData>> + ImageCopyCaptureHandler,
{
    fn request(
        state: &mut D,
        _client: &Client,
        _resource: &ExtImageCopyCaptureCursorSessionV1,
        request: <ExtImageCopyCaptureCursorSessionV1 as Resource>::Request,
        data: &CursorSessionData,
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            ext_image_copy_capture_cursor_session_v1::Request::Destroy => (),
            ext_image_copy_capture_cursor_session_v1::Request::GetCaptureSession { session } => {
                let source = data.source.clone();
                let cursor = Cursor::Standalone {
                    pointer: data.pointer.clone(),
                };

                let shm_formats = state.image_copy_capture_state().shm_formats.clone();
                let dmabuf_formats = state.image_copy_capture_state().dmabuf_formats.clone();
                let dmabuf_device = state.image_copy_capture_state().dmabuf_device;

                let session = data_init.init(
                    session,
                    Mutex::new(SessionData {
                        source,
                        cursor,
                        frame: Default::default(),
                        shm_formats,
                        dmabuf_formats,
                        dmabuf_device,
                    }),
                );
                let session = Session { session };

                state
                    .image_copy_capture_state()
                    .sessions
                    .push(session.clone());

                state.new_session(session);
            }
            _ => todo!(),
        }
    }

    fn destroyed(
        _state: &mut D,
        _client: wayland_backend::server::ClientId,
        _resource: &ExtImageCopyCaptureCursorSessionV1,
        _data: &CursorSessionData,
    ) {
        todo!()
    }
}
