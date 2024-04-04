use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::UNIX_EPOCH,
};

use smithay::{
    backend::{
        allocator::Buffer,
        renderer::{buffer_type, BufferType},
    },
    output::Output,
    reexports::{
        wayland_protocols_wlr::screencopy::v1::server::{
            zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1},
            zwlr_screencopy_manager_v1::{self, ZwlrScreencopyManagerV1},
        },
        wayland_server::{
            self,
            protocol::{wl_buffer::WlBuffer, wl_shm},
            Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, Resource,
        },
    },
    utils::{Physical, Point, Rectangle},
    wayland::{
        dmabuf::get_dmabuf,
        shm::{self, shm_format_to_fourcc},
    },
};
use tracing::trace;

const VERSION: u32 = 3;

pub struct ScreencopyManagerState;

pub struct ScreencopyManagerGlobalData {
    filter: Box<dyn Fn(&Client) -> bool + Send + Sync>,
}

impl ScreencopyManagerState {
    pub fn new<D, F>(display: &DisplayHandle, filter: F) -> Self
    where
        D: GlobalDispatch<ZwlrScreencopyManagerV1, ScreencopyManagerGlobalData>
            + Dispatch<ZwlrScreencopyManagerV1, ()>
            + Dispatch<ZwlrScreencopyFrameV1, ScreencopyFrameState>
            + ScreencopyHandler
            + 'static,
        F: Fn(&Client) -> bool + Send + Sync + 'static,
    {
        let global_data = ScreencopyManagerGlobalData {
            filter: Box::new(filter),
        };
        display.create_global::<D, ZwlrScreencopyManagerV1, _>(VERSION, global_data);
        Self
    }
}

impl<D> GlobalDispatch<ZwlrScreencopyManagerV1, ScreencopyManagerGlobalData, D>
    for ScreencopyManagerState
where
    D: GlobalDispatch<ZwlrScreencopyManagerV1, ScreencopyManagerGlobalData>
        + Dispatch<ZwlrScreencopyManagerV1, ()>
        + Dispatch<ZwlrScreencopyFrameV1, ScreencopyFrameState>
        + ScreencopyHandler
        + 'static,
{
    fn bind(
        _state: &mut D,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: wayland_server::New<ZwlrScreencopyManagerV1>,
        _global_data: &ScreencopyManagerGlobalData,
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(resource, ());
    }

    fn can_view(client: Client, global_data: &ScreencopyManagerGlobalData) -> bool {
        (global_data.filter)(&client)
    }
}

impl<D> Dispatch<ZwlrScreencopyManagerV1, (), D> for ScreencopyManagerState
where
    D: GlobalDispatch<ZwlrScreencopyManagerV1, ScreencopyManagerGlobalData>
        + Dispatch<ZwlrScreencopyManagerV1, ()>
        + Dispatch<ZwlrScreencopyFrameV1, ScreencopyFrameState>
        + ScreencopyHandler
        + 'static,
{
    fn request(
        _state: &mut D,
        _client: &Client,
        manager: &ZwlrScreencopyManagerV1,
        request: <ZwlrScreencopyManagerV1 as wayland_server::Resource>::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        let (frame, overlay_cursor, physical_region, output) = match request {
            zwlr_screencopy_manager_v1::Request::CaptureOutput {
                frame,
                overlay_cursor,
                output,
            } => {
                let output = Output::from_resource(&output).expect("no output for resource");
                let physical_size = output.current_mode().expect("output has no mode").size;
                let physical_region =
                    Rectangle::from_loc_and_size(Point::from((0, 0)), physical_size);

                (frame, overlay_cursor, physical_region, output)
            }
            zwlr_screencopy_manager_v1::Request::CaptureOutputRegion {
                frame,
                overlay_cursor,
                output,
                x,
                y,
                width,
                height,
            } => {
                if width <= 0 || height <= 0 {
                    trace!("Screencopy client requested region with negative size");
                    let frame = data_init.init(frame, ScreencopyFrameState::Failed);
                    frame.failed();
                    return;
                }

                let output = Output::from_resource(&output).expect("no output for resource");
                let output_transform = output.current_transform();
                let output_transformed_physical_size = output_transform
                    .transform_size(output.current_mode().expect("output no mode").size);
                let output_transformed_rect =
                    Rectangle::from_loc_and_size((0, 0), output_transformed_physical_size);

                // This is in the transformed space
                let screencopy_region = Rectangle::from_loc_and_size((x, y), (width, height));

                let output_scale = output.current_scale().fractional_scale();
                let physical_rect = screencopy_region.to_physical_precise_round(output_scale);

                // Clamp captured region to the output.
                let Some(clamped_rect) = physical_rect.intersection(output_transformed_rect) else {
                    trace!("screencopy client requested region outside of output");
                    let frame = data_init.init(frame, ScreencopyFrameState::Failed);
                    frame.failed();
                    return;
                };

                // Untransform the region to the actual physical rect
                let untransformed_region = output_transform
                    .invert()
                    .transform_rect_in(clamped_rect, &output_transformed_physical_size);

                (frame, overlay_cursor, untransformed_region, output)
            }
            zwlr_screencopy_manager_v1::Request::Destroy => return,
            _ => unreachable!(),
        };

        // Create the frame.
        let overlay_cursor = overlay_cursor != 0;
        let info = ScreencopyFrameInfo {
            output,
            overlay_cursor,
            physical_region,
        };
        let frame = data_init.init(
            frame,
            ScreencopyFrameState::Pending {
                info,
                copied: Arc::new(AtomicBool::new(false)),
            },
        );

        let buffer_size = physical_region.size;

        // Send desired SHM buffer parameters.
        frame.buffer(
            wl_shm::Format::Argb8888,
            buffer_size.w as u32,
            buffer_size.h as u32,
            buffer_size.w as u32 * 4,
        );

        if manager.version() >= 3 {
            // Send desired DMA buffer parameters.
            frame.linux_dmabuf(
                smithay::backend::allocator::Fourcc::Argb8888 as u32,
                buffer_size.w as u32,
                buffer_size.h as u32,
            );

            // Notify client that all supported buffers were enumerated.
            frame.buffer_done();
        }
    }
}

pub trait ScreencopyHandler {
    fn frame(&mut self, frame: Screencopy);
}

#[allow(missing_docs)]
#[macro_export]
macro_rules! delegate_screencopy {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        smithay::reexports::wayland_server::delegate_global_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::screencopy::v1::server::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1: $crate::protocol::screencopy::ScreencopyManagerGlobalData
        ] => $crate::protocol::screencopy::ScreencopyManagerState);

        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::screencopy::v1::server::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1: ()
        ] => $crate::protocol::screencopy::ScreencopyManagerState);

        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols_wlr::screencopy::v1::server::zwlr_screencopy_frame_v1::ZwlrScreencopyFrameV1: $crate::protocol::screencopy::ScreencopyFrameState
        ] => $crate::protocol::screencopy::ScreencopyManagerState);
    };
}

#[derive(Clone, Debug)]
pub struct ScreencopyFrameInfo {
    output: Output,
    physical_region: Rectangle<i32, Physical>,
    overlay_cursor: bool,
}

pub enum ScreencopyFrameState {
    Failed,
    Pending {
        info: ScreencopyFrameInfo,
        copied: Arc<AtomicBool>,
    },
}

impl<D> Dispatch<ZwlrScreencopyFrameV1, ScreencopyFrameState, D> for ScreencopyManagerState
where
    D: Dispatch<ZwlrScreencopyFrameV1, ScreencopyFrameState> + ScreencopyHandler + 'static,
{
    fn request(
        state: &mut D,
        _client: &Client,
        frame: &ZwlrScreencopyFrameV1,
        request: <ZwlrScreencopyFrameV1 as wayland_server::Resource>::Request,
        data: &ScreencopyFrameState,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        if matches!(request, zwlr_screencopy_frame_v1::Request::Destroy) {
            return;
        }

        let (info, copied) = match data {
            ScreencopyFrameState::Failed => return,
            ScreencopyFrameState::Pending { info, copied } => (info, copied),
        };

        if copied.load(Ordering::SeqCst) {
            frame.post_error(
                zwlr_screencopy_frame_v1::Error::AlreadyUsed,
                "copy was already requested",
            );
            return;
        }

        let (buffer, with_damage) = match request {
            zwlr_screencopy_frame_v1::Request::Copy { buffer } => (buffer, false),
            zwlr_screencopy_frame_v1::Request::CopyWithDamage { buffer } => (buffer, true),
            _ => unreachable!(),
        };

        match buffer_type(&buffer) {
            Some(BufferType::Shm) => {
                if !shm::with_buffer_contents(&buffer, |_buf, shm_len, buffer_data| {
                    buffer_data.format == wl_shm::Format::Argb8888
                        && buffer_data.stride == info.physical_region.size.w * 4
                        && buffer_data.height == info.physical_region.size.h
                        && shm_len as i32 == buffer_data.stride * buffer_data.height
                })
                .unwrap_or(false)
                {
                    frame.post_error(
                        zwlr_screencopy_frame_v1::Error::InvalidBuffer,
                        "invalid buffer",
                    );
                    return;
                }
            }
            Some(BufferType::Dma) => match get_dmabuf(&buffer) {
                Ok(dmabuf) => {
                    if !(Some(dmabuf.format().code)
                        == shm_format_to_fourcc(wl_shm::Format::Argb8888)
                        && dmabuf.width() == info.physical_region.size.w as u32
                        && dmabuf.height() == info.physical_region.size.h as u32)
                    {
                        frame.post_error(
                            zwlr_screencopy_frame_v1::Error::InvalidBuffer,
                            "invalid buffer",
                        );
                        return;
                    }
                }
                Err(err) => {
                    frame.post_error(
                        zwlr_screencopy_frame_v1::Error::InvalidBuffer,
                        err.to_string(),
                    );
                    return;
                }
            },
            _ => {
                frame.post_error(
                    zwlr_screencopy_frame_v1::Error::InvalidBuffer,
                    "invalid buffer",
                );
                return;
            }
        }

        copied.store(true, Ordering::SeqCst);

        state.frame(Screencopy {
            with_damage,
            buffer,
            frame: frame.clone(),
            info: info.clone(),
            submitted: false,
        });
    }
}

#[derive(Debug)]
pub struct Screencopy {
    info: ScreencopyFrameInfo,
    frame: ZwlrScreencopyFrameV1,
    with_damage: bool,
    buffer: WlBuffer,
    submitted: bool,
}

// If `Screencopy::submit` wasn't called, send the failed event.
impl Drop for Screencopy {
    fn drop(&mut self) {
        if !self.submitted {
            self.frame.failed();
        }
    }
}

impl Screencopy {
    pub fn buffer(&self) -> &WlBuffer {
        &self.buffer
    }

    /// Get the output-local physical region to be copied.
    pub fn physical_region(&self) -> Rectangle<i32, Physical> {
        self.info.physical_region
    }

    pub fn output(&self) -> &Output {
        &self.info.output
    }

    pub fn overlay_cursor(&self) -> bool {
        self.info.overlay_cursor
    }

    /// Get whether or not this screencopy should be done on damage.
    pub fn with_damage(&self) -> bool {
        self.with_damage
    }

    /// Mark damaged regions of the screencopy buffer.
    pub fn damage(&mut self, damage: &[Rectangle<i32, Physical>]) {
        if !self.with_damage {
            return;
        }

        for Rectangle { loc, size } in damage {
            self.frame
                .damage(loc.x as u32, loc.y as u32, size.w as u32, size.h as u32);
        }
    }

    /// Submit the copied content.
    pub fn submit(mut self, y_invert: bool) {
        // Notify client that buffer is ordinary.
        self.frame.flags(if y_invert {
            zwlr_screencopy_frame_v1::Flags::YInvert
        } else {
            zwlr_screencopy_frame_v1::Flags::empty()
        });

        // Notify client about successful copy.
        let time = UNIX_EPOCH.elapsed().unwrap();
        let tv_sec_hi = (time.as_secs() >> 32) as u32;
        let tv_sec_lo = (time.as_secs() & 0xFFFFFFFF) as u32;
        let tv_nsec = time.subsec_nanos();
        self.frame.ready(tv_sec_hi, tv_sec_lo, tv_nsec);

        // Mark frame as submitted to ensure destructor isn't run.
        self.submitted = true;
    }
}
