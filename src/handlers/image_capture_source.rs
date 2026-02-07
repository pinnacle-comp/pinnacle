use smithay::{
    delegate_image_capture_source, delegate_output_capture_source,
    delegate_toplevel_capture_source,
    output::WeakOutput,
    wayland::{
        foreign_toplevel_list::{ForeignToplevelHandle, ForeignToplevelWeakHandle},
        image_capture_source::{
            ImageCaptureSource, ImageCaptureSourceHandler, OutputCaptureSourceHandler,
            OutputCaptureSourceState, ToplevelCaptureSourceHandler, ToplevelCaptureSourceState,
        },
    },
};

use crate::state::State;

#[derive(Clone, Debug)]
pub enum ImageCaptureSourceKind {
    Output(WeakOutput),
    Toplevel(ForeignToplevelWeakHandle),
}

impl ImageCaptureSourceHandler for State {}
delegate_image_capture_source!(State);

impl OutputCaptureSourceHandler for State {
    fn output_capture_source_state(&mut self) -> &mut OutputCaptureSourceState {
        &mut self.pinnacle.output_capture_source_state
    }

    fn output_source_created(
        &mut self,
        source: ImageCaptureSource,
        output: &smithay::output::Output,
    ) {
        source
            .user_data()
            .insert_if_missing(|| ImageCaptureSourceKind::Output(output.downgrade()));
    }
}
delegate_output_capture_source!(State);

impl ToplevelCaptureSourceHandler for State {
    fn toplevel_capture_source_state(&mut self) -> &mut ToplevelCaptureSourceState {
        &mut self.pinnacle.toplevel_capture_source_state
    }

    fn toplevel_source_created(
        &mut self,
        source: ImageCaptureSource,
        toplevel: &ForeignToplevelHandle,
    ) {
        source
            .user_data()
            .insert_if_missing(|| ImageCaptureSourceKind::Toplevel(toplevel.downgrade()));
    }
}
delegate_toplevel_capture_source!(State);
