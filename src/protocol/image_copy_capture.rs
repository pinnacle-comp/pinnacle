use std::{collections::HashMap, sync::Mutex};

use smithay::{
    backend::{
        allocator::{Format as DrmFormat, Fourcc as DrmFourcc, Modifier as DrmModifier},
        drm::DrmNode,
    },
    reexports::{
        wayland_protocols::ext::image_copy_capture::v1::server::{
            ext_image_copy_capture_cursor_session_v1::ExtImageCopyCaptureCursorSessionV1,
            ext_image_copy_capture_manager_v1::{self, ExtImageCopyCaptureManagerV1},
            ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
        },
        wayland_server::{
            Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
            protocol::wl_shm,
        },
    },
};

use crate::protocol::image_copy_capture::session::{
    Cursor, CursorSession, CursorSessionData, Session, SessionData,
};

pub mod frame;
pub mod session;

const VERSION: u32 = 1;

#[derive(Debug)]
pub struct ImageCopyCaptureState {
    sessions: Vec<Session>,
    cursor_sessions: Vec<CursorSession>,
    shm_formats: Vec<wl_shm::Format>,
    dmabuf_formats: HashMap<DrmFourcc, Vec<DrmModifier>>,
    dmabuf_device: Option<DrmNode>,
}

pub struct ImageCopyCaptureGlobalData {
    filter: Box<dyn Fn(&Client) -> bool + Send + Sync>,
}

impl ImageCopyCaptureState {
    pub fn new<D, F>(display: &DisplayHandle, filter: F) -> Self
    where
        D: GlobalDispatch<ExtImageCopyCaptureManagerV1, ImageCopyCaptureGlobalData> + 'static,
        F: Fn(&Client) -> bool + Send + Sync + 'static,
    {
        let global_data = ImageCopyCaptureGlobalData {
            filter: Box::new(filter),
        };
        display.create_global::<D, ExtImageCopyCaptureManagerV1, _>(VERSION, global_data);

        Self {
            sessions: Vec::new(),
            cursor_sessions: Vec::new(),
            shm_formats: Vec::new(),
            dmabuf_formats: HashMap::new(),
            dmabuf_device: None,
        }
    }

    /// Sets format and device constraints for all current and new capture sessions.
    pub fn set_buffer_constraints(
        &mut self,
        shm_formats: impl IntoIterator<Item = wl_shm::Format>,
        dmabuf_device: Option<DrmNode>,
        dmabuf_formats: impl IntoIterator<Item = DrmFormat>,
    ) {
        self.shm_formats = shm_formats.into_iter().collect();

        self.dmabuf_device = dmabuf_device;

        self.dmabuf_formats.clear();
        for format in dmabuf_formats.into_iter() {
            self.dmabuf_formats
                .entry(format.code)
                .or_default()
                .push(format.modifier);
        }

        for session in self.sessions.iter() {
            session.set_buffer_constraints(
                self.shm_formats.clone(),
                self.dmabuf_device,
                self.dmabuf_formats.clone(),
            );
        }
    }
}

pub trait ImageCopyCaptureHandler {
    fn image_copy_capture_state(&mut self) -> &mut ImageCopyCaptureState;
    fn new_session(&mut self, session: Session);
    fn new_cursor_session(&mut self, cursor_session: CursorSession);
    fn session_destroyed(&mut self, session: Session);
    fn cursor_session_destroyed(&mut self, cursor_session: CursorSession);
}

impl<D> GlobalDispatch<ExtImageCopyCaptureManagerV1, ImageCopyCaptureGlobalData, D>
    for ImageCopyCaptureState
where
    D: Dispatch<ExtImageCopyCaptureManagerV1, ()>,
{
    fn bind(
        _state: &mut D,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<ExtImageCopyCaptureManagerV1>,
        _global_data: &ImageCopyCaptureGlobalData,
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(resource, ());
    }

    fn can_view(client: Client, global_data: &ImageCopyCaptureGlobalData) -> bool {
        (global_data.filter)(&client)
    }
}

impl<D> Dispatch<ExtImageCopyCaptureManagerV1, (), D> for ImageCopyCaptureState
where
    D: Dispatch<ExtImageCopyCaptureSessionV1, Mutex<SessionData>>
        + Dispatch<ExtImageCopyCaptureCursorSessionV1, CursorSessionData>
        + ImageCopyCaptureHandler,
{
    fn request(
        state: &mut D,
        _client: &Client,
        resource: &ExtImageCopyCaptureManagerV1,
        request: <ExtImageCopyCaptureManagerV1 as Resource>::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            ext_image_copy_capture_manager_v1::Request::CreateSession {
                session,
                source,
                options,
            } => match options.into_result() {
                Ok(options) => {
                    let cursor = if options
                        .contains(ext_image_copy_capture_manager_v1::Options::PaintCursors)
                    {
                        Cursor::Composited
                    } else {
                        Cursor::Hidden
                    };

                    let shm_formats = state.image_copy_capture_state().shm_formats.clone();
                    let dmabuf_formats = state.image_copy_capture_state().dmabuf_formats.clone();
                    let dmabuf_device = state.image_copy_capture_state().dmabuf_device;

                    let session = data_init.init(
                        session,
                        Mutex::new(SessionData::new(
                            source,
                            cursor,
                            shm_formats,
                            dmabuf_formats,
                            dmabuf_device,
                            None,
                        )),
                    );
                    let session = Session::new(session);

                    state
                        .image_copy_capture_state()
                        .sessions
                        .push(session.clone());

                    state.new_session(session);
                }
                Err(err) => {
                    data_init.init(
                        session,
                        Mutex::new(SessionData::new(
                            source,
                            Cursor::Hidden,
                            Default::default(),
                            Default::default(),
                            Default::default(),
                            None,
                        )),
                    );
                    resource.post_error(
                        ext_image_copy_capture_manager_v1::Error::InvalidOption,
                        err.to_string(),
                    );
                }
            },
            ext_image_copy_capture_manager_v1::Request::CreatePointerCursorSession {
                session,
                source,
                pointer,
            } => {
                let session = data_init.init(session, CursorSessionData::new(source, pointer));
                let session = CursorSession::new(session);

                state.new_cursor_session(session.clone());

                state
                    .image_copy_capture_state()
                    .cursor_sessions
                    .push(session);
            }
            ext_image_copy_capture_manager_v1::Request::Destroy => (),
            _ => (),
        }
    }
}

macro_rules! delegate_image_copy_capture {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        smithay::reexports::wayland_server::delegate_global_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols::ext::image_copy_capture::v1::server::ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1: $crate::protocol::image_copy_capture::ImageCopyCaptureGlobalData
        ] => $crate::protocol::image_copy_capture::ImageCopyCaptureState);

        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols::ext::image_copy_capture::v1::server::ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1: ()
        ] => $crate::protocol::image_copy_capture::ImageCopyCaptureState);

        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols::ext::image_copy_capture::v1::server::ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1: ::std::sync::Mutex<$crate::protocol::image_copy_capture::session::SessionData>
        ] => $crate::protocol::image_copy_capture::ImageCopyCaptureState);

        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols::ext::image_copy_capture::v1::server::ext_image_copy_capture_frame_v1::ExtImageCopyCaptureFrameV1: ::std::sync::Mutex<$crate::protocol::image_copy_capture::frame::FrameData>
        ] => $crate::protocol::image_copy_capture::ImageCopyCaptureState);

        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols::ext::image_copy_capture::v1::server::ext_image_copy_capture_cursor_session_v1::ExtImageCopyCaptureCursorSessionV1: $crate::protocol::image_copy_capture::session::CursorSessionData
        ] => $crate::protocol::image_copy_capture::ImageCopyCaptureState);
    };
}
pub(crate) use delegate_image_copy_capture;
