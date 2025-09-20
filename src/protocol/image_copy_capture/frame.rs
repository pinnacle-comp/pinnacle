use std::{
    sync::{Mutex, MutexGuard},
    time::UNIX_EPOCH,
};

use smithay::{
    backend::renderer::{BufferType, buffer_type},
    reexports::{
        wayland_protocols::ext::image_copy_capture::v1::server::ext_image_copy_capture_frame_v1::{
            self, ExtImageCopyCaptureFrameV1, FailureReason,
        },
        wayland_server::{
            Client, DataInit, Dispatch, DisplayHandle, Resource, protocol::wl_buffer::WlBuffer,
        },
    },
    utils::{Buffer, Rectangle, Transform},
};
use wayland_backend::server::ClientId;

use crate::protocol::image_copy_capture::{ImageCopyCaptureHandler, ImageCopyCaptureState};

/// A frame that a client has requested capture for.
///
/// If this is dropped and [`Frame::submit`] has not been called, this will
/// send the `failed` event to the client.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Frame {
    frame: ExtImageCopyCaptureFrameV1,
}

impl Drop for Frame {
    fn drop(&mut self) {
        // There should've been no way for the frame to fail or become idle while this `Frame` exists.
        assert!(self.data().frame_state > FrameState::Idle);

        if self.data().frame_state == FrameState::CaptureRequested {
            // `submit` has been called with no damage
            return;
        }

        if self.data().frame_state != FrameState::Submitted {
            self.data().frame_state = FrameState::Failed;
            self.frame.failed(FailureReason::Unknown);
        }
    }
}

impl Frame {
    /// Creates a new frame.
    ///
    /// This should only be created once the client has sent the capture request.
    pub fn new(frame: ExtImageCopyCaptureFrameV1) -> Self {
        {
            let mut data = frame.data::<Mutex<FrameData>>().unwrap().lock().unwrap();
            assert_eq!(data.frame_state, FrameState::CaptureRequested);
            data.frame_state = FrameState::Capturing;
        }

        Self { frame }
    }

    /// Gets the buffer the client has attached to this frame.
    pub fn buffer(&self) -> WlBuffer {
        self.data()
            .buffer
            .clone()
            .expect("frame should have a buffer here")
    }

    /// Returns the buffer damage received by the client.
    pub fn buffer_damage(&self) -> Vec<Rectangle<i32, Buffer>> {
        self.data().client_buffer_damage.clone()
    }

    /// Submits this frame.
    ///
    /// If `damage` is empty, the frame returns to the state it had right after
    /// the client sent the capture request. Otherwise, the client is notified of completion.
    pub fn submit(
        &self,
        transform: Transform,
        damage: impl IntoIterator<Item = Rectangle<i32, Buffer>>,
    ) {
        let mut damage = damage.into_iter().peekable();
        if damage.peek().is_none() {
            self.data().frame_state = FrameState::CaptureRequested;
            return;
        }

        let time = UNIX_EPOCH.elapsed().unwrap();
        let tv_sec_hi = (time.as_secs() >> 32) as u32;
        let tv_sec_lo = (time.as_secs() & 0xFFFFFFFF) as u32;
        let tv_nsec = time.subsec_nanos();
        self.frame.presentation_time(tv_sec_hi, tv_sec_lo, tv_nsec);
        self.frame.transform(transform.into());
        for damage in damage {
            self.frame
                .damage(damage.loc.x, damage.loc.y, damage.size.w, damage.size.h);
        }
        self.frame.ready();
        self.data().frame_state = FrameState::Submitted;
    }

    fn data(&self) -> MutexGuard<'_, FrameData> {
        self.frame
            .data::<Mutex<FrameData>>()
            .unwrap()
            .lock()
            .unwrap()
    }
}

#[derive(Debug, Default)]
pub struct FrameData {
    pub(super) frame_state: FrameState,
    pub(super) client_buffer_damage: Vec<Rectangle<i32, Buffer>>,
    pub(super) buffer: Option<WlBuffer>,
}

/// The state of a frame.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FrameState {
    /// The frame has failed.
    Failed,
    /// The frame is idle.
    #[default]
    Idle,
    /// The client has requested capture.
    CaptureRequested,
    /// The frame is in the process of capturing.
    Capturing,
    /// The frame has been captured and the client has been notified.
    Submitted,
}

impl<D> Dispatch<ExtImageCopyCaptureFrameV1, Mutex<FrameData>, D> for ImageCopyCaptureState
where
    D: ImageCopyCaptureHandler,
{
    fn request(
        state: &mut D,
        _client: &Client,
        resource: &ExtImageCopyCaptureFrameV1,
        request: <ExtImageCopyCaptureFrameV1 as Resource>::Request,
        data: &Mutex<FrameData>,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            ext_image_copy_capture_frame_v1::Request::Destroy => (),
            ext_image_copy_capture_frame_v1::Request::AttachBuffer { buffer } => {
                if data.lock().unwrap().frame_state >= FrameState::CaptureRequested {
                    resource.post_error(
                        ext_image_copy_capture_frame_v1::Error::AlreadyCaptured,
                        "cannot attach a buffer after capturing",
                    );
                    return;
                }

                data.lock().unwrap().buffer = Some(buffer);
            }
            ext_image_copy_capture_frame_v1::Request::DamageBuffer {
                x,
                y,
                width,
                height,
            } => {
                if data.lock().unwrap().frame_state >= FrameState::CaptureRequested {
                    resource.post_error(
                        ext_image_copy_capture_frame_v1::Error::AlreadyCaptured,
                        "cannot damage buffer after capturing",
                    );
                    return;
                }

                if x < 0 || y < 0 || width <= 0 || height <= 0 {
                    resource.post_error(
                        ext_image_copy_capture_frame_v1::Error::InvalidBufferDamage,
                        format!(
                            "x or y were < 0, or width or height were <= 0 \
                            (x={x}, y={y}, width={width}, height={height})"
                        ),
                    );
                    return;
                }

                data.lock()
                    .unwrap()
                    .client_buffer_damage
                    .push(Rectangle::new((x, y).into(), (width, height).into()));
            }
            ext_image_copy_capture_frame_v1::Request::Capture => {
                if data.lock().unwrap().frame_state >= FrameState::CaptureRequested {
                    resource.post_error(
                        ext_image_copy_capture_frame_v1::Error::AlreadyCaptured,
                        "this frame was already captured",
                    );
                    return;
                }

                match data.lock().unwrap().buffer.clone() {
                    Some(buffer) => {
                        if !matches!(
                            buffer_type(&buffer),
                            Some(BufferType::Shm | BufferType::Dma)
                        ) {
                            if data.lock().unwrap().frame_state != FrameState::Failed {
                                resource.failed(FailureReason::BufferConstraints);
                                data.lock().unwrap().frame_state = FrameState::Failed;
                            }
                            return;
                        }
                    }
                    None => {
                        resource.post_error(
                            ext_image_copy_capture_frame_v1::Error::NoBuffer,
                            "a buffer must be attached before capturing",
                        );
                        return;
                    }
                }

                if !state
                    .image_copy_capture_state()
                    .sessions
                    .iter()
                    .any(|session| Some(resource) == session.frame().as_ref())
                {
                    if data.lock().unwrap().frame_state != FrameState::Failed {
                        resource.failed(FailureReason::Stopped);
                        data.lock().unwrap().frame_state = FrameState::Failed;
                    }
                    return;
                }

                data.lock().unwrap().frame_state = FrameState::CaptureRequested;
            }
            _ => (),
        }
    }

    fn destroyed(
        state: &mut D,
        _client: ClientId,
        resource: &ExtImageCopyCaptureFrameV1,
        _data: &Mutex<FrameData>,
    ) {
        for session in state.image_copy_capture_state().sessions.iter() {
            if session.frame_destroyed(resource) {
                break;
            }
        }
    }
}
