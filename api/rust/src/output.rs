use crate::{
    msg::{OutputName, Request, RequestResponse},
    request,
};

/// Output management.
pub struct Output;

impl Output {
    /// Get an [`OutputHandle`] by its name.
    ///
    /// `name` is the name of the port the output is plugged in to.
    /// This is something like `HDMI-1` or `eDP-0`.
    pub fn get_by_name(&self, name: &str) -> Option<OutputHandle> {
        let RequestResponse::Outputs { output_names } = request(Request::GetOutputs) else {
            unreachable!()
        };

        output_names
            .into_iter()
            .find(|s| s == name)
            .map(|s| OutputHandle(OutputName(s)))
    }

    /// Get a handle to all connected outputs.
    pub fn get_all(&self) -> impl Iterator<Item = OutputHandle> {
        let RequestResponse::Outputs { output_names } = request(Request::GetOutputs) else {
            unreachable!()
        };

        output_names
            .into_iter()
            .map(|name| OutputHandle(OutputName(name)))
    }

    /// Get the currently focused output.
    ///
    /// This is currently defined as the one with the cursor on it.
    pub fn get_focused(&self) -> Option<OutputHandle> {
        let RequestResponse::Outputs { output_names } = request(Request::GetOutputs) else {
            unreachable!()
        };

        output_names
            .into_iter()
            .map(|s| OutputHandle(OutputName(s)))
            .find(|op| op.properties().focused == Some(true))
    }
}

/// An output handle.
///
/// This is a handle to one of your monitors.
/// It serves to make it easier to deal with them, defining methods for getting properties and
/// helpers for things like positioning multiple monitors.
pub struct OutputHandle(OutputName);

/// Properties of an output.
pub struct OutputProperties {
    /// The make.
    make: Option<String>,
    /// The model.
    ///
    /// This is something like `27GL850` or whatever gibberish monitor manufacturers name their
    /// displays.
    model: Option<String>,
    /// The location of the output in the global space.
    loc: Option<(i32, i32)>,
    /// The resolution of the output in pixels, where `res.0` is the width and `res.1` is the
    /// height.
    res: Option<(i32, i32)>,
    /// The refresh rate of the output in millihertz.
    ///
    /// For example, 60Hz is returned as 60000.
    refresh_rate: Option<i32>,
    /// The physical size of the output in millimeters.
    physical_size: Option<(i32, i32)>,
    /// Whether or not the output is focused.
    focused: Option<bool>,
    // TODO: tags
}

impl OutputHandle {
    // TODO: Make OutputProperties an option, make non null fields not options
    /// Get all properties of this output.
    pub fn properties(&self) -> OutputProperties {
        let RequestResponse::OutputProps {
            make,
            model,
            loc,
            res,
            refresh_rate,
            physical_size,
            focused,
            tag_ids,
        } = request(Request::GetOutputProps {
            output_name: self.0 .0.clone(),
        })
        else {
            unreachable!()
        };

        OutputProperties {
            make,
            model,
            loc,
            res,
            refresh_rate,
            physical_size,
            focused,
        }
    }
}
