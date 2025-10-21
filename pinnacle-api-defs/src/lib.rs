pub mod pinnacle {

    pub mod v1 {
        tonic::include_proto!("pinnacle.v1");
    }

    pub mod input {
        pub mod v1 {
            tonic::include_proto!("pinnacle.input.v1");
        }
    }

    pub mod output {
        pub mod v1 {
            tonic::include_proto!("pinnacle.output.v1");
        }
    }

    pub mod tag {
        pub mod v1 {
            tonic::include_proto!("pinnacle.tag.v1");
        }
    }

    pub mod window {
        pub mod v1 {
            tonic::include_proto!("pinnacle.window.v1");
        }
    }

    pub mod process {
        pub mod v1 {
            tonic::include_proto!("pinnacle.process.v1");
        }
    }

    pub mod signal {
        pub mod v1 {
            tonic::include_proto!("pinnacle.signal.v1");

            pub trait SignalRequest {
                fn from_control(control: StreamControl) -> Self;
                fn control(&self) -> StreamControl;
            }

            macro_rules! impl_signal_request {
                ( $( $request:ident ),* ) => {
                    $(
                        impl SignalRequest for $request {
                            fn from_control(control: StreamControl) -> Self {
                                $request {
                                    control: control.into(),
                                }
                            }

                            fn control(&self) -> StreamControl {
                                self.control()
                            }
                        }
                    )*
                };
            }

            impl_signal_request!(
                OutputConnectRequest,
                OutputDisconnectRequest,
                OutputResizeRequest,
                OutputMoveRequest,
                OutputPointerEnterRequest,
                OutputPointerLeaveRequest,
                OutputFocusedRequest,
                WindowPointerEnterRequest,
                WindowPointerLeaveRequest,
                WindowFocusedRequest,
                WindowTitleChangedRequest,
                WindowLayoutModeChangedRequest,
                WindowCreatedRequest,
                WindowDestroyedRequest,
                TagActiveRequest,
                TagCreatedRequest,
                TagRemovedRequest,
                InputDeviceAddedRequest
            );
        }
    }

    pub mod layout {
        pub mod v1 {
            tonic::include_proto!("pinnacle.layout.v1");
        }
    }

    pub mod render {
        pub mod v1 {
            tonic::include_proto!("pinnacle.render.v1");
        }
    }

    pub mod util {
        pub mod v1 {
            tonic::include_proto!("pinnacle.util.v1");
        }
    }

    pub mod debug {
        pub mod v1 {
            tonic::include_proto!("pinnacle.debug.v1");
        }
    }
}

pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("pinnacle");
