use smithay::reexports::{
    wayland_protocols::ext::{
        foreign_toplevel_list::v1::server::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1,
        image_capture_source::v1::server::{
            ext_foreign_toplevel_image_capture_source_manager_v1::{
                self, ExtForeignToplevelImageCaptureSourceManagerV1,
            },
            ext_image_capture_source_v1::ExtImageCaptureSourceV1,
            ext_output_image_capture_source_manager_v1::{
                self, ExtOutputImageCaptureSourceManagerV1,
            },
        },
    },
    wayland_server::{
        Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
        protocol::wl_output::WlOutput,
    },
};

const VERSION: u32 = 1;

#[derive(Debug)]
pub struct ImageCaptureSourceState;

impl ImageCaptureSourceState {
    pub fn new<D, F>(display: &DisplayHandle, filter: F) -> Self
    where
        D: GlobalDispatch<ExtOutputImageCaptureSourceManagerV1, ImageCaptureSourceGlobalData>,
        D: GlobalDispatch<
                ExtForeignToplevelImageCaptureSourceManagerV1,
                ImageCaptureSourceGlobalData,
            >,
        D: 'static,
        F: Fn(&Client) -> bool + Send + Sync + Clone + 'static,
    {
        let global_data = ImageCaptureSourceGlobalData {
            filter: Box::new(filter.clone()),
        };
        display.create_global::<D, ExtOutputImageCaptureSourceManagerV1, _>(VERSION, global_data);
        let global_data = ImageCaptureSourceGlobalData {
            filter: Box::new(filter.clone()),
        };
        display.create_global::<D, ExtForeignToplevelImageCaptureSourceManagerV1, _>(
            VERSION,
            global_data,
        );

        Self
    }
}

pub struct ImageCaptureSourceGlobalData {
    filter: Box<dyn Fn(&Client) -> bool + Send + Sync>,
}

#[derive(Debug, Clone)]
pub enum Source {
    Output(WlOutput),
    ForeignToplevel(ExtForeignToplevelHandleV1),
}

impl<D> GlobalDispatch<ExtOutputImageCaptureSourceManagerV1, ImageCaptureSourceGlobalData, D>
    for ImageCaptureSourceState
where
    D: Dispatch<ExtOutputImageCaptureSourceManagerV1, ()>,
{
    fn bind(
        _state: &mut D,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<ExtOutputImageCaptureSourceManagerV1>,
        _global_data: &ImageCaptureSourceGlobalData,
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(resource, ());
    }

    fn can_view(client: Client, global_data: &ImageCaptureSourceGlobalData) -> bool {
        (global_data.filter)(&client)
    }
}

impl<D>
    GlobalDispatch<ExtForeignToplevelImageCaptureSourceManagerV1, ImageCaptureSourceGlobalData, D>
    for ImageCaptureSourceState
where
    D: Dispatch<ExtForeignToplevelImageCaptureSourceManagerV1, ()>,
{
    fn bind(
        _state: &mut D,
        _handle: &DisplayHandle,
        _client: &Client,
        resource: New<ExtForeignToplevelImageCaptureSourceManagerV1>,
        _global_data: &ImageCaptureSourceGlobalData,
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(resource, ());
    }

    fn can_view(client: Client, global_data: &ImageCaptureSourceGlobalData) -> bool {
        (global_data.filter)(&client)
    }
}

impl<D> Dispatch<ExtOutputImageCaptureSourceManagerV1, (), D> for ImageCaptureSourceState
where
    D: Dispatch<ExtImageCaptureSourceV1, Source>,
{
    fn request(
        _state: &mut D,
        _client: &Client,
        _resource: &ExtOutputImageCaptureSourceManagerV1,
        request: <ExtOutputImageCaptureSourceManagerV1 as Resource>::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            ext_output_image_capture_source_manager_v1::Request::CreateSource {
                source,
                output,
            } => {
                data_init.init(source, Source::Output(output));
            }
            ext_output_image_capture_source_manager_v1::Request::Destroy => (),
            _ => (),
        }
    }
}

impl<D> Dispatch<ExtForeignToplevelImageCaptureSourceManagerV1, (), D> for ImageCaptureSourceState
where
    D: Dispatch<ExtImageCaptureSourceV1, Source>,
{
    fn request(
        _state: &mut D,
        _client: &Client,
        _resource: &ExtForeignToplevelImageCaptureSourceManagerV1,
        request: <ExtForeignToplevelImageCaptureSourceManagerV1 as Resource>::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            ext_foreign_toplevel_image_capture_source_manager_v1::Request::CreateSource {
                source,
                toplevel_handle,
            } => {
                data_init.init(source, Source::ForeignToplevel(toplevel_handle));
            }
            ext_foreign_toplevel_image_capture_source_manager_v1::Request::Destroy => (),
            _ => (),
        }
    }
}

impl<D> Dispatch<ExtImageCaptureSourceV1, Source, D> for ImageCaptureSourceState {
    fn request(
        _state: &mut D,
        _client: &Client,
        _resource: &ExtImageCaptureSourceV1,
        _request: <ExtImageCaptureSourceV1 as Resource>::Request,
        _data: &Source,
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
    }
}

macro_rules! delegate_image_capture_source {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        smithay::reexports::wayland_server::delegate_global_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols::ext::image_capture_source::v1::server::ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1: $crate::protocol::image_capture_source::ImageCaptureSourceGlobalData
        ] => $crate::protocol::image_capture_source::ImageCaptureSourceState);

        smithay::reexports::wayland_server::delegate_global_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols::ext::image_capture_source::v1::server::ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1: $crate::protocol::image_capture_source::ImageCaptureSourceGlobalData
        ] => $crate::protocol::image_capture_source::ImageCaptureSourceState);

        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols::ext::image_capture_source::v1::server::ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1: ()
        ] => $crate::protocol::image_capture_source::ImageCaptureSourceState);

        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols::ext::image_capture_source::v1::server::ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1: ()
        ] => $crate::protocol::image_capture_source::ImageCaptureSourceState);

        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            smithay::reexports::wayland_protocols::ext::image_capture_source::v1::server::ext_image_capture_source_v1::ExtImageCaptureSourceV1: $crate::protocol::image_capture_source::Source
        ] => $crate::protocol::image_capture_source::ImageCaptureSourceState);
    };
}
pub(crate) use delegate_image_capture_source;
