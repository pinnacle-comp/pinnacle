use std::{
    collections::HashMap,
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicBool, AtomicI32, Ordering},
    },
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
    utils::{Buffer, Point, Size},
};
use wayland_backend::server::ClientId;

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
    session: ExtImageCopyCaptureSessionV1,
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

    pub(super) fn new(session: ExtImageCopyCaptureSessionV1) -> Self {
        Self { session }
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
        if self
            .data()
            .cursor_session
            .as_ref()
            .is_some_and(|cursor_session| {
                !cursor_session
                    .data::<CursorSessionData>()
                    .unwrap()
                    .cursor_entered
                    .load(Ordering::Relaxed)
            })
        {
            // This is a cursor capture session and the cursor is not entered
            // (i.e. is not over the source).
            return None;
        }

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

    pub fn cursor_session(&self) -> Option<CursorSession> {
        self.data().cursor_session.clone().map(CursorSession::new)
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
    source: ExtImageCaptureSourceV1,
    cursor: Cursor,
    frame: Option<ExtImageCopyCaptureFrameV1>,
    shm_formats: Vec<wl_shm::Format>,
    dmabuf_formats: HashMap<DrmFourcc, Vec<DrmModifier>>,
    dmabuf_device: Option<DrmNode>,
    cursor_session: Option<ExtImageCopyCaptureCursorSessionV1>,
}

impl SessionData {
    pub(super) fn new(
        source: ExtImageCaptureSourceV1,
        cursor: Cursor,
        shm_formats: Vec<wl_shm::Format>,
        dmabuf_formats: HashMap<DrmFourcc, Vec<DrmModifier>>,
        dmabuf_device: Option<DrmNode>,
        cursor_session: Option<ExtImageCopyCaptureCursorSessionV1>,
    ) -> Self {
        Self {
            source,
            cursor,
            frame: None,
            shm_formats,
            dmabuf_formats,
            dmabuf_device,
            cursor_session,
        }
    }
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

/// An active cursor capture session.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CursorSession {
    session: ExtImageCopyCaptureCursorSessionV1,
}

impl CursorSession {
    /// The source for this cursor session.
    pub fn source(&self) -> &Source {
        self.data().source.data::<Source>().unwrap()
    }

    /// The pointer that this cursor session is capturing.
    pub fn pointer(&self) -> &WlPointer {
        &self.data().pointer
    }

    pub fn set_hotspot(&self, hotspot: Point<i32, Buffer>) {
        if self.data().hotspot() == hotspot {
            return;
        }

        self.data().set_hotspot(hotspot);

        if self.data().cursor_entered.load(Ordering::Relaxed) {
            self.session.hotspot(hotspot.x, hotspot.y);
        }
    }

    pub fn set_position(&self, position: Option<Point<i32, Buffer>>) {
        let cursor_entered = self.data().cursor_entered.load(Ordering::Relaxed);
        if cursor_entered != position.is_some() {
            self.data()
                .cursor_entered
                .store(position.is_some(), Ordering::Relaxed);
            if position.is_some() {
                self.session.enter();
                let hotspot = self.data().hotspot();
                self.session.hotspot(hotspot.x, hotspot.y);
            } else {
                self.session.leave();
            }
        }

        if let Some(position) = position {
            self.session.position(position.x, position.y);
        }
    }

    pub(super) fn new(session: ExtImageCopyCaptureCursorSessionV1) -> Self {
        Self { session }
    }

    fn data(&self) -> &CursorSessionData {
        self.session.data::<CursorSessionData>().unwrap()
    }
}

pub struct CursorSessionData {
    source: ExtImageCaptureSourceV1,
    pointer: WlPointer,
    capture_session_retrieved: AtomicBool,
    current_hotspot: (AtomicI32, AtomicI32),
    cursor_entered: AtomicBool,
}

impl CursorSessionData {
    pub(super) fn new(source: ExtImageCaptureSourceV1, pointer: WlPointer) -> Self {
        Self {
            source,
            pointer,
            capture_session_retrieved: Default::default(),
            current_hotspot: Default::default(),
            cursor_entered: Default::default(),
        }
    }

    fn hotspot(&self) -> Point<i32, Buffer> {
        (
            self.current_hotspot.0.load(Ordering::Relaxed),
            self.current_hotspot.1.load(Ordering::Relaxed),
        )
            .into()
    }

    fn set_hotspot(&self, hotspot: Point<i32, Buffer>) {
        self.current_hotspot.0.store(hotspot.x, Ordering::Relaxed);
        self.current_hotspot.1.store(hotspot.y, Ordering::Relaxed);
    }
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
        _client: ClientId,
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
        resource: &ExtImageCopyCaptureCursorSessionV1,
        request: <ExtImageCopyCaptureCursorSessionV1 as Resource>::Request,
        data: &CursorSessionData,
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            ext_image_copy_capture_cursor_session_v1::Request::Destroy => (),
            ext_image_copy_capture_cursor_session_v1::Request::GetCaptureSession { session } => {
                if data.capture_session_retrieved.load(Ordering::Relaxed) {
                    resource.post_error(
                        ext_image_copy_capture_cursor_session_v1::Error::DuplicateSession,
                        "get_capture_session already sent",
                    );
                    return;
                }

                data.capture_session_retrieved
                    .store(true, Ordering::Relaxed);

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
                        cursor_session: Some(resource.clone()),
                    }),
                );
                let session = Session { session };

                state
                    .image_copy_capture_state()
                    .sessions
                    .push(session.clone());

                state.new_session(session);
            }
            _ => (),
        }
    }

    fn destroyed(
        state: &mut D,
        _client: ClientId,
        resource: &ExtImageCopyCaptureCursorSessionV1,
        _data: &CursorSessionData,
    ) {
        state
            .image_copy_capture_state()
            .cursor_sessions
            .retain(|session| session.session != *resource);

        state.cursor_session_destroyed(CursorSession {
            session: resource.clone(),
        });
    }
}
