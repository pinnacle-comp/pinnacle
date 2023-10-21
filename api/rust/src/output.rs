//! Output management.

use crate::{
    msg::{Args, CallbackId, Msg, Request, RequestResponse},
    request, send_msg,
    tag::TagHandle,
    CallbackVec,
};

/// A unique identifier for an output.
///
/// An empty string represents an invalid output.
#[derive(Debug, Hash, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub(crate) struct OutputName(pub String);

/// Get an [`OutputHandle`] by its name.
///
/// `name` is the name of the port the output is plugged in to.
/// This is something like `HDMI-1` or `eDP-0`.
pub fn get_by_name(name: &str) -> Option<OutputHandle> {
    let RequestResponse::Outputs { output_names } = request(Request::GetOutputs) else {
        unreachable!()
    };

    output_names
        .into_iter()
        .find(|s| s == name)
        .map(|s| OutputHandle(OutputName(s)))
}

/// Get a handle to all connected outputs.
pub fn get_all() -> impl Iterator<Item = OutputHandle> {
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
pub fn get_focused() -> Option<OutputHandle> {
    let RequestResponse::Outputs { output_names } = request(Request::GetOutputs) else {
        unreachable!()
    };

    output_names
        .into_iter()
        .map(|s| OutputHandle(OutputName(s)))
        .find(|op| op.properties().focused == Some(true))
}

/// Connect a function to be run on all current and future outputs.
///
/// When called, `connect_for_all` will run `func` with all currently connected outputs.
/// If a new output is connected, `func` will also be called with it.
///
/// `func` takes in two parameters:
/// - `0`: An [`OutputHandle`] you can act on.
/// - `1`: A `&mut `[`CallbackVec`] for use in the closure.
///
/// This will *not* be called if it has already been called for a given connector.
/// This means turning your monitor off and on or unplugging and replugging it *to the same port*
/// won't trigger `func`. Plugging it in to a new port *will* trigger `func`.
/// This is intended to prevent duplicate setup.
///
/// Please note: this function will be run *after* Pinnacle processes your entire config.
/// For example, if you define tags in `func` but toggle them directly after `connect_for_all`,
/// nothing will happen as the tags haven't been added yet.
pub fn connect_for_all<'a, F>(mut func: F, callback_vec: &mut CallbackVec<'a>)
where
    F: FnMut(OutputHandle, &mut CallbackVec) + 'a,
{
    let args_callback = move |args: Option<Args>, callback_vec: &mut CallbackVec<'_>| {
        if let Some(Args::ConnectForAllOutputs { output_name }) = args {
            func(OutputHandle(OutputName(output_name)), callback_vec);
        }
    };

    let len = callback_vec.callbacks.len();
    callback_vec.callbacks.push(Box::new(args_callback));

    let msg = Msg::ConnectForAllOutputs {
        callback_id: CallbackId(len as u32),
    };

    send_msg(msg).unwrap();
}

/// An output handle.
///
/// This is a handle to one of your monitors.
/// It serves to make it easier to deal with them, defining methods for getting properties and
/// helpers for things like positioning multiple monitors.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct OutputHandle(pub(crate) OutputName);

/// Properties of an output.
pub struct OutputProperties {
    /// The make.
    pub make: Option<String>,
    /// The model.
    ///
    /// This is something like `27GL850` or whatever gibberish monitor manufacturers name their
    /// displays.
    pub model: Option<String>,
    /// The location of the output in the global space.
    pub loc: Option<(i32, i32)>,
    /// The resolution of the output in pixels, where `res.0` is the width and `res.1` is the
    /// height.
    pub res: Option<(i32, i32)>,
    /// The refresh rate of the output in millihertz.
    ///
    /// For example, 60Hz is returned as 60000.
    pub refresh_rate: Option<i32>,
    /// The physical size of the output in millimeters.
    pub physical_size: Option<(i32, i32)>,
    /// Whether or not the output is focused.
    pub focused: Option<bool>,
    /// The tags on this output.
    pub tags: Vec<TagHandle>,
}

impl OutputHandle {
    /// Get this output's name.
    pub fn name(&self) -> String {
        self.0 .0.clone()
    }

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
            tags: tag_ids
                .unwrap_or(vec![])
                .into_iter()
                .map(TagHandle)
                .collect(),
        }
    }

    /// Add tags with the given `names` to this output.
    pub fn add_tags(&self, names: &[&str]) {
        crate::tag::add(self, names);
    }

    /// Set this output's location in the global space.
    pub fn set_loc(&self, x: Option<i32>, y: Option<i32>) {
        let msg = Msg::SetOutputLocation {
            output_name: self.0.clone(),
            x,
            y,
        };

        send_msg(msg).unwrap();
    }

    /// Set this output's location to the right of `other`.
    ///
    /// It will be aligned vertically based on the given `alignment`.
    pub fn set_loc_right_of(&self, other: &OutputHandle, alignment: AlignmentVertical) {
        self.set_loc_horizontal(other, LeftOrRight::Right, alignment);
    }

    /// Set this output's location to the left of `other`.
    ///
    /// It will be aligned vertically based on the given `alignment`.
    pub fn set_loc_left_of(&self, other: &OutputHandle, alignment: AlignmentVertical) {
        self.set_loc_horizontal(other, LeftOrRight::Left, alignment);
    }

    /// Set this output's location to the top of `other`.
    ///
    /// It will be aligned horizontally based on the given `alignment`.
    pub fn set_loc_top_of(&self, other: &OutputHandle, alignment: AlignmentHorizontal) {
        self.set_loc_vertical(other, TopOrBottom::Top, alignment);
    }

    /// Set this output's location to the bottom of `other`.
    ///
    /// It will be aligned horizontally based on the given `alignment`.
    pub fn set_loc_bottom_of(&self, other: &OutputHandle, alignment: AlignmentHorizontal) {
        self.set_loc_vertical(other, TopOrBottom::Bottom, alignment);
    }

    fn set_loc_horizontal(
        &self,
        other: &OutputHandle,
        left_or_right: LeftOrRight,
        alignment: AlignmentVertical,
    ) {
        let op1_props = self.properties();
        let op2_props = other.properties();

        let (Some(_self_loc), Some(self_res), Some(other_loc), Some(other_res)) =
            (op1_props.loc, op1_props.res, op2_props.loc, op2_props.res)
        else {
            return;
        };

        let x = match left_or_right {
            LeftOrRight::Left => other_loc.0 - self_res.0,
            LeftOrRight::Right => other_loc.0 + self_res.0,
        };

        let y = match alignment {
            AlignmentVertical::Top => other_loc.1,
            AlignmentVertical::Center => other_loc.1 + (other_res.1 - self_res.1) / 2,
            AlignmentVertical::Bottom => other_loc.1 + (other_res.1 - self_res.1),
        };

        self.set_loc(Some(x), Some(y));
    }

    fn set_loc_vertical(
        &self,
        other: &OutputHandle,
        top_or_bottom: TopOrBottom,
        alignment: AlignmentHorizontal,
    ) {
        let op1_props = self.properties();
        let op2_props = other.properties();

        let (Some(_self_loc), Some(self_res), Some(other_loc), Some(other_res)) =
            (op1_props.loc, op1_props.res, op2_props.loc, op2_props.res)
        else {
            return;
        };

        let y = match top_or_bottom {
            TopOrBottom::Top => other_loc.1 - self_res.1,
            TopOrBottom::Bottom => other_loc.1 + other_res.1,
        };

        let x = match alignment {
            AlignmentHorizontal::Left => other_loc.0,
            AlignmentHorizontal::Center => other_loc.0 + (other_res.0 - self_res.0) / 2,
            AlignmentHorizontal::Right => other_loc.0 + (other_res.0 - self_res.0),
        };

        self.set_loc(Some(x), Some(y));
    }
}

enum TopOrBottom {
    Top,
    Bottom,
}

enum LeftOrRight {
    Left,
    Right,
}

/// Horizontal alignment.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AlignmentHorizontal {
    /// Align the outputs such that the left edges are in line.
    Left,
    /// Center the outputs horizontally.
    Center,
    /// Align the outputs such that the right edges are in line.
    Right,
}

/// Vertical alignment.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AlignmentVertical {
    /// Align the outputs such that the top edges are in line.
    Top,
    /// Center the outputs vertically.
    Center,
    /// Align the outputs such that the bottom edges are in line.
    Bottom,
}
