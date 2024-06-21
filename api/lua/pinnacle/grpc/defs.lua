local util = require("pinnacle.util")

local defs = {}

-- Pinnacle

---@class GrpcRequestArgs
---@field service string
---@field method string
---@field request string
---@field response string

---@class pinnacle.v0alpha1.Geometry
---@field x integer?
---@field y integer?
---@field width integer?
---@field height integer?

---@class pinnacle.v0alpha1.QuitRequest

---@class pinnacle.v0alpha1.ReloadConfigRequest

---@class pinnacle.v0alpha1.PingRequest
---@field payload string

---@class pinnacle.v0alpha1.PingResponse
---@field payload string

---@enum pinnacle.v0alpha1.SetOrToggle
local pinnacle_v0alpha1_SetOrToggle = {
    SET_OR_TOGGLE_UNSPECIFIED = 0,
    SET_OR_TOGGLE_SET = 1,
    SET_OR_TOGGLE_UNSET = 2,
    SET_OR_TOGGLE_TOGGLE = 3,
}

-- Output

---@class pinnacle.output.v0alpha1.Mode
---@field pixel_width integer?
---@field pixel_height integer?
---@field refresh_rate_millihz integer?

---@enum pinnacle.output.v0alpha1.Transform
local pinnacle_output_v0alpha1_Transform = {
    TRANSFORM_UNSPECIFIED = 0,
    TRANSFORM_NORMAL = 1,
    TRANSFORM_90 = 2,
    TRANSFORM_180 = 3,
    TRANSFORM_270 = 4,
    TRANSFORM_FLIPPED = 5,
    TRANSFORM_FLIPPED_90 = 6,
    TRANSFORM_FLIPPED_180 = 7,
    TRANSFORM_FLIPPED_270 = 8,
}

---@class pinnacle.output.v0alpha1.SetLocationRequest
---@field output_name string?
---@field x integer?
---@field y integer?

---@class pinnacle.output.v0alpha1.SetModeRequest
---@field output_name string?
---@field pixel_width integer?
---@field pixel_height integer?
---@field refresh_rate_millihz integer?

---@class pinnacle.output.v0alpha1.SetModelineRequest
---@field output_name string?
---@field clock number?
---@field hdisplay integer?
---@field hsync_start integer?
---@field hsync_end integer?
---@field htotal integer?
---@field vdisplay integer?
---@field vsync_start integer?
---@field vsync_end integer?
---@field vtotal integer?
---@field hsync_pos boolean?
---@field vsync_pos boolean?

---@class pinnacle.output.v0alpha1.SetScaleRequest
---@field output_name string?
---@field absolute number?
---@field relative number?

---@class pinnacle.output.v0alpha1.SetTransformRequest
---@field output_name string?
---@field transform pinnacle.output.v0alpha1.Transform

---@class pinnacle.output.v0alpha1.SetPoweredRequest
---@field output_name string?
---@field powered boolean

---@class pinnacle.output.v0alpha1.GetRequest

---@class pinnacle.output.v0alpha1.GetResponse
---@field output_names string[]?

---@class pinnacle.output.v0alpha1.GetPropertiesRequest
---@field output_name string?

---@class pinnacle.output.v0alpha1.GetPropertiesResponse
---@field make string?
---@field model string?
---@field x integer?
---@field y integer?
---@field logical_width integer?
---@field logical_height integer?
---@field current_mode pinnacle.output.v0alpha1.Mode?
---@field preferred_mode pinnacle.output.v0alpha1.Mode?
---@field modes pinnacle.output.v0alpha1.Mode[]?
---@field physical_width integer?
---@field physical_height integer?
---@field focused boolean?
---@field tag_ids integer[]?
---@field scale number?
---@field transform pinnacle.output.v0alpha1.Transform?
---@field serial integer?
---@field keyboard_focus_stack_window_ids integer[]?
---@field enabled boolean?
---@field powered boolean?

-- Window

---@class pinnacle.window.v0alpha1.CloseRequest
---@field window_id integer?

---@class pinnacle.window.v0alpha1.SetGeometryRequest
---@field window_id integer?
---@field geometry pinnacle.v0alpha1.Geometry?

---@class pinnacle.window.v0alpha1.SetFullscreenRequest
---@field window_id integer?
---@field set_or_toggle pinnacle.v0alpha1.SetOrToggle?

---@class pinnacle.window.v0alpha1.SetMaximizedRequest
---@field window_id integer?
---@field set_or_toggle pinnacle.v0alpha1.SetOrToggle?

---@class pinnacle.window.v0alpha1.SetFloatingRequest
---@field window_id integer?
---@field set_or_toggle pinnacle.v0alpha1.SetOrToggle?

---@class pinnacle.window.v0alpha1.SetFocusedRequest
---@field window_id integer?
---@field set_or_toggle pinnacle.v0alpha1.SetOrToggle?

---@class pinnacle.window.v0alpha1.MoveToTagRequest
---@field window_id integer?
---@field tag_id integer?

---@class pinnacle.window.v0alpha1.SetTagRequest
---@field window_id integer?
---@field tag_id integer?
---@field set_or_toggle pinnacle.v0alpha1.SetOrToggle?

---@class pinnacle.window.v0alpha1.RaiseRequest
---@field window_id integer?

---@class pinnacle.window.v0alpha1.MoveGrabRequest
---@field button integer?

---@class pinnacle.window.v0alpha1.ResizeGrabRequest
---@field button integer?

---@class pinnacle.window.v0alpha1.GetRequest

---@class pinnacle.window.v0alpha1.GetResponse
---@field window_ids integer[]?

---@class pinnacle.window.v0alpha1.GetPropertiesRequest
---@field window_id integer?

---@class pinnacle.window.v0alpha1.GetPropertiesResponse
---@field geometry pinnacle.v0alpha1.Geometry?
---@field class string?
---@field title string?
---@field focused boolean?
---@field floating boolean?
---@field fullscreen_or_maximized pinnacle.window.v0alpha1.FullscreenOrMaximized?
---@field tag_ids integer[]?

---@enum pinnacle.window.v0alpha1.FullscreenOrMaximized
local pinnacle_window_v0alpha1_FullscreenOrMaximized = {
    FULLSCREEN_OR_MAXIMIZED_UNSPECIFIED = 0,
    FULLSCREEN_OR_MAXIMIZED_NEITHER = 1,
    FULLSCREEN_OR_MAXIMIZED_FULLSCREEN = 2,
    FULLSCREEN_OR_MAXIMIZED_MAXIMIZED = 3,
}

---@class pinnacle.window.v0alpha1.AddWindowRuleRequest
---@field cond pinnacle.window.v0alpha1.WindowRuleCondition?
---@field rule pinnacle.window.v0alpha1.WindowRule?

---@class pinnacle.window.v0alpha1.WindowRuleCondition
---@field any pinnacle.window.v0alpha1.WindowRuleCondition[]?
---@field all pinnacle.window.v0alpha1.WindowRuleCondition[]?
---@field classes string[]?
---@field titles string[]?
---@field tags integer[]?

---@class pinnacle.window.v0alpha1.WindowRule
---@field output string?
---@field tags integer[]?
---@field floating boolean?
---@field fullscreen_or_maximized pinnacle.window.v0alpha1.FullscreenOrMaximized?
---@field x integer?
---@field y integer?
---@field width integer?
---@field height integer?

-- Tag

---@class pinnacle.tag.v0alpha1.SetActiveRequest
---@field tag_id integer?
---@field set_or_toggle pinnacle.v0alpha1.SetOrToggle?

---@class pinnacle.tag.v0alpha1.SwitchToRequest
---@field tag_id integer?

---@class pinnacle.tag.v0alpha1.AddRequest
---@field output_name string?
---@field tag_names string[]?

---@class pinnacle.tag.v0alpha1.AddResponse
---@field tag_ids integer[]?

---@class pinnacle.tag.v0alpha1.RemoveRequest
---@field tag_ids integer[]?

---@class pinnacle.tag.v0alpha1.GetRequest

---@class pinnacle.tag.v0alpha1.GetResponse
---@field tag_ids integer[]?

---@class pinnacle.tag.v0alpha1.GetPropertiesRequest
---@field tag_id integer?

---@class pinnacle.tag.v0alpha1.GetPropertiesResponse
---@field active boolean?
---@field name string?
---@field output_name string?
---@field window_ids integer[]?

-- Input

---@enum pinnacle.input.v0alpha1.Modifier
local pinnacle_input_v0alpha1_Modifier = {
    MODIFIER_UNSPECIFIED = 0,
    MODIFIER_SHIFT = 1,
    MODIFIER_CTRL = 2,
    MODIFIER_ALT = 3,
    MODIFIER_SUPER = 4,
}

---@class pinnacle.input.v0alpha1.SetKeybindRequest
---@field modifiers pinnacle.input.v0alpha1.Modifier[]?
---@field raw_code integer?
---@field xkb_name string?
---@field group string?
---@field description string?

---@class pinnacle.input.v0alpha1.SetKeybindResponse

---@enum pinnacle.input.v0alpha1.SetMousebindRequest.MouseEdge
local pinnacle_input_v0alpha1_SetMousebindRequest_MouseEdge = {
    MOUSE_EDGE_UNSPECIFIED = 0,
    MOUSE_EDGE_PRESS = 1,
    MOUSE_EDGE_RELEASE = 2,
}

---@class pinnacle.input.v0alpha1.SetMousebindRequest
---@field button integer?
---@field edge pinnacle.input.v0alpha1.SetMousebindRequest.MouseEdge?

---@class pinnacle.input.v0alpha1.SetMousebindResponse

---@class pinnacle.input.v0alpha1.KeybindDescriptionsRequest

---@class pinnacle.input.v0alpha1.KeybindDescriptionsResponse
---@field descriptions pinnacle.input.v0alpha1.KeybindDescription[]?

---@class pinnacle.input.v0alpha1.KeybindDescription
---@field modifiers pinnacle.input.v0alpha1.Modifier[]?
---@field raw_code integer?
---@field xkb_name string?
---@field group string?
---@field description string?

---@class SetXkbConfigRequest
---@field rules string?
---@field variant string?
---@field layout string?
---@field model string?
---@field options string?

---@class SetRepeatRateRequest
---@field rate integer?
---@field delay integer?

---@enum pinnacle.input.v0alpha1.SetLibinputSettingRequest.AccelProfile
local pinnacle_input_v0alpha1_SetLibinputSettingRequest_AccelProfile = {
    ACCEL_PROFILE_UNSPECIFIED = 0,
    ACCEL_PROFILE_FLAT = 1,
    ACCEL_PROFILE_ADAPTIVE = 2,
}

---@class pinnacle.input.v0alpha1.SetLibinputSettingRequest.CalibrationMatrix
---@field matrix number[]?

---@enum pinnacle.input.v0alpha1.SetLibinputSettingRequest.ClickMethod
local pinnacle_input_v0alpha1_SetLibinputSettingRequest_ClickMethod = {
    CLICK_METHOD_UNSPECIFIED = 0,
    CLICK_METHOD_BUTTON_AREAS = 1,
    CLICK_METHOD_CLICK_FINGER = 2,
}

---@enum pinnacle.input.v0alpha1.SetLibinputSettingRequest.ScrollMethod
local pinnacle_input_v0alpha1_SetLibinputSettingRequest_ScrollMethod = {
    SCROLL_METHOD_UNSPECIFIED = 0,
    SCROLL_METHOD_NO_SCROLL = 1,
    SCROLL_METHOD_TWO_FINGER = 2,
    SCROLL_METHOD_EDGE = 3,
    SCROLL_METHOD_ON_BUTTON_DOWN = 4,
}

---@enum pinnacle.input.v0alpha1.SetLibinputSettingRequest.TapButtonMap
local pinnacle_input_v0alpha1_SetLibinputSettingRequest_TapButtonMap = {
    TAP_BUTTON_MAP_UNSPECIFIED = 0,
    TAP_BUTTON_MAP_LEFT_RIGHT_MIDDLE = 1,
    TAP_BUTTON_MAP_LEFT_MIDDLE_RIGHT = 2,
}

---@class pinnacle.input.v0alpha1.SetLibinputSettingRequest
---@field accel_profile pinnacle.input.v0alpha1.SetLibinputSettingRequest.AccelProfile?
---@field accel_speed number?
---@field calibration_matrix pinnacle.input.v0alpha1.SetLibinputSettingRequest.CalibrationMatrix?
---@field click_method pinnacle.input.v0alpha1.SetLibinputSettingRequest.ClickMethod?
---@field disable_while_typing boolean?
---@field left_handed boolean?
---@field middle_emulation boolean?
---@field rotation_angle integer?
---@field scroll_button integer?
---@field scroll_button_lock boolean?
---@field scroll_method pinnacle.input.v0alpha1.SetLibinputSettingRequest.ScrollMethod?
---@field natural_scroll boolean?
---@field tap_button_map pinnacle.input.v0alpha1.SetLibinputSettingRequest.TapButtonMap?
---@field tap_drag boolean?
---@field tap_drag_lock boolean?
---@field tap boolean?

---@class SetXcursorRequest
---@field theme string?
---@field size integer?

-- Process

---@class pinnacle.process.v0alpha1.SpawnRequest
---@field args string[]?
---@field once boolean?
---@field has_callback boolean?

---@class pinnacle.process.v0alpha1.SpawnResponse
---@field stdout string?
---@field stderr string?
---@field exit_code integer?
---@field exit_message string?

---@class pinnacle.process.v0alpha1.SetEnvRequest
---@field key string?
---@field value string?

-- Layout

---@class pinnacle.layout.v0alpha1.LayoutRequest.Geometries
---@field request_id integer?
---@field output_name string?
---@field geometries pinnacle.v0alpha1.Geometry[]?

---@class pinnacle.layout.v0alpha1.LayoutRequest.ExplicitLayout
---@field output_name string?

---@class pinnacle.layout.v0alpha1.LayoutRequest
---@field geometries pinnacle.layout.v0alpha1.LayoutRequest.Geometries?
---@field layout pinnacle.layout.v0alpha1.LayoutRequest.ExplicitLayout?

---@class pinnacle.layout.v0alpha1.LayoutResponse
---@field request_id integer?
---@field output_name string?
---@field window_ids integer[]?
---@field tag_ids integer[]?
---@field output_width integer?
---@field output_height integer?

-- Render

---@enum pinnacle.render.v0alpha1.Filter
local pinnacle_render_v0alpha1_Filter = {
    FILTER_UNSPECIFIED = 0,
    FILTER_BILINEAR = 1,
    FILTER_NEAREST_NEIGHBOR = 2,
}

---@class pinnacle.render.v0alpha1.SetUpscaleFilterRequest
---@field filter pinnacle.render.v0alpha1.Filter?

---@class pinnacle.render.v0alpha1.SetDownscaleFilterRequest
---@field filter pinnacle.render.v0alpha1.Filter?

-- Signal

---@enum pinnacle.signal.v0alpha1.StreamControl
local pinnacle_signal_v0alpha1_StreamControl = {
    STREAM_CONTROL_UNSPECIFIED = 0,
    STREAM_CONTROL_READY = 1,
    STREAM_CONTROL_DISCONNECT = 2,
}

---@class pinnacle.signal.v0alpha1.OutputConnectRequest
---@field control pinnacle.signal.v0alpha1.StreamControl?

---@class pinnacle.signal.v0alpha1.OutputConnectResponse
---@field output_name string?

---@class pinnacle.signal.v0alpha1.OutputDisconnectRequest
---@field control pinnacle.signal.v0alpha1.StreamControl?

---@class pinnacle.signal.v0alpha1.OutputDisconnectResponse
---@field output_name string?

---@class pinnacle.signal.v0alpha1.OutputResizeRequest
---@field control pinnacle.signal.v0alpha1.StreamControl?

---@class pinnacle.signal.v0alpha1.OutputResizeResponse
---@field output_name string?
---@field logical_width integer?
---@field logical_height integer?

---@class pinnacle.signal.v0alpha1.OutputMoveRequest
---@field control pinnacle.signal.v0alpha1.StreamControl?

---@class pinnacle.signal.v0alpha1.OutputMoveResponse
---@field output_name string?
---@field x integer?
---@field y integer?

---@class pinnacle.signal.v0alpha1.WindowPointerEnterRequest
---@field control pinnacle.signal.v0alpha1.StreamControl?

---@class pinnacle.signal.v0alpha1.WindowPointerEnterResponse
---@field window_id integer?

---@class pinnacle.signal.v0alpha1.WindowPointerLeaveRequest
---@field control pinnacle.signal.v0alpha1.StreamControl?

---@class pinnacle.signal.v0alpha1.WindowPointerLeaveResponse
---@field window_id integer?

---@class pinnacle.signal.v0alpha1.TagActiveRequest
---@field control pinnacle.signal.v0alpha1.StreamControl?

---@class pinnacle.signal.v0alpha1.TagActiveResponse
---@field tag_id integer?
---@field active boolean?

defs.pinnacle = {
    v0alpha1 = {
        SetOrToggle = util.bijective_table(pinnacle_v0alpha1_SetOrToggle),
        PinnacleService = {
            ---@type GrpcRequestArgs
            Quit = {
                service = "pinnacle.v0alpha1.PinnacleService",
                method = "Quit",
                request = "pinnacle.v0alpha1.QuitRequest",
                response = "google.protobuf.Empty",
            },
            ---@type GrpcRequestArgs
            ReloadConfig = {
                service = "pinnacle.v0alpha1.PinnacleService",
                method = "ReloadConfig",
                request = "pinnacle.v0alpha1.ReloadConfigRequest",
                response = "google.protobuf.Empty",
            },
            ---@type GrpcRequestArgs
            Ping = {
                service = "pinnacle.v0alpha1.PinnacleService",
                method = "Ping",
                request = "pinnacle.v0alpha1.PingRequest",
                response = "pinnacle.v0alpha1.PingResponse",
            },
        },
    },
    output = {
        v0alpha1 = {
            Transform = util.bijective_table(pinnacle_output_v0alpha1_Transform),
            OutputService = {
                ---@type GrpcRequestArgs
                SetLocation = {
                    service = "pinnacle.output.v0alpha1.OutputService",
                    method = "SetLocation",
                    request = "pinnacle.output.v0alpha1.SetLocationRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                SetMode = {
                    service = "pinnacle.output.v0alpha1.OutputService",
                    method = "SetMode",
                    request = "pinnacle.output.v0alpha1.SetModeRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                SetModeline = {
                    service = "pinnacle.output.v0alpha1.OutputService",
                    method = "SetModeline",
                    request = "pinnacle.output.v0alpha1.SetModelineRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                SetScale = {
                    service = "pinnacle.output.v0alpha1.OutputService",
                    method = "SetScale",
                    request = "pinnacle.output.v0alpha1.SetScaleRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                SetTransform = {
                    service = "pinnacle.output.v0alpha1.OutputService",
                    method = "SetTransform",
                    request = "pinnacle.output.v0alpha1.SetTransformRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                SetPowered = {
                    service = "pinnacle.output.v0alpha1.OutputService",
                    method = "SetPowered",
                    request = "pinnacle.output.v0alpha1.SetPoweredRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                Get = {
                    service = "pinnacle.output.v0alpha1.OutputService",
                    method = "Get",
                    request = "pinnacle.output.v0alpha1.GetRequest",
                    response = "pinnacle.output.v0alpha1.GetResponse",
                },
                ---@type GrpcRequestArgs
                GetProperties = {
                    service = "pinnacle.output.v0alpha1.OutputService",
                    method = "GetProperties",
                    request = "pinnacle.output.v0alpha1.GetPropertiesRequest",
                    response = "pinnacle.output.v0alpha1.GetPropertiesResponse",
                },
            },
        },
    },
    window = {
        v0alpha1 = {
            FullscreenOrMaximized = util.bijective_table(
                pinnacle_window_v0alpha1_FullscreenOrMaximized
            ),
            WindowService = {
                ---@type GrpcRequestArgs
                Close = {
                    service = "pinnacle.window.v0alpha1.WindowService",
                    method = "Close",
                    request = "pinnacle.window.v0alpha1.CloseRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                SetGeometry = {
                    service = "pinnacle.window.v0alpha1.WindowService",
                    method = "SetGeometry",
                    request = "pinnacle.window.v0alpha1.SetGeometryRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                SetFullscreen = {
                    service = "pinnacle.window.v0alpha1.WindowService",
                    method = "SetFullscreen",
                    request = "pinnacle.window.v0alpha1.SetFullscreenRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                SetMaximized = {
                    service = "pinnacle.window.v0alpha1.WindowService",
                    method = "SetMaximized",
                    request = "pinnacle.window.v0alpha1.SetMaximizedRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                SetFloating = {
                    service = "pinnacle.window.v0alpha1.WindowService",
                    method = "SetFloating",
                    request = "pinnacle.window.v0alpha1.SetFloatingRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                SetFocused = {
                    service = "pinnacle.window.v0alpha1.WindowService",
                    method = "SetFocused",
                    request = "pinnacle.window.v0alpha1.SetFocusedRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                MoveToTag = {
                    service = "pinnacle.window.v0alpha1.WindowService",
                    method = "MoveToTag",
                    request = "pinnacle.window.v0alpha1.MoveToTagRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                SetTag = {
                    service = "pinnacle.window.v0alpha1.WindowService",
                    method = "SetTag",
                    request = "pinnacle.window.v0alpha1.SetTagRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                Raise = {
                    service = "pinnacle.window.v0alpha1.WindowService",
                    method = "Raise",
                    request = "pinnacle.window.v0alpha1.RaiseRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                MoveGrab = {
                    service = "pinnacle.window.v0alpha1.WindowService",
                    method = "MoveGrab",
                    request = "pinnacle.window.v0alpha1.MoveGrabRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                ResizeGrab = {
                    service = "pinnacle.window.v0alpha1.WindowService",
                    method = "ResizeGrab",
                    request = "pinnacle.window.v0alpha1.ResizeGrabRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                Get = {
                    service = "pinnacle.window.v0alpha1.WindowService",
                    method = "Get",
                    request = "pinnacle.window.v0alpha1.GetRequest",
                    response = "pinnacle.window.v0alpha1.GetResponse",
                },
                ---@type GrpcRequestArgs
                GetProperties = {
                    service = "pinnacle.window.v0alpha1.WindowService",
                    method = "GetProperties",
                    request = "pinnacle.window.v0alpha1.GetPropertiesRequest",
                    response = "pinnacle.window.v0alpha1.GetPropertiesResponse",
                },
                ---@type GrpcRequestArgs
                AddWindowRule = {
                    service = "pinnacle.window.v0alpha1.WindowService",
                    method = "AddWindowRule",
                    request = "pinnacle.window.v0alpha1.AddWindowRuleRequest",
                    response = "google.protobuf.Empty",
                },
            },
        },
    },
    tag = {
        v0alpha1 = {
            TagService = {
                ---@type GrpcRequestArgs
                SetActive = {
                    service = "pinnacle.tag.v0alpha1.TagService",
                    method = "SetActive",
                    request = "pinnacle.tag.v0alpha1.SetActiveRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                SwitchTo = {
                    service = "pinnacle.tag.v0alpha1.TagService",
                    method = "SwitchTo",
                    request = "pinnacle.tag.v0alpha1.SwitchToRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                Add = {
                    service = "pinnacle.tag.v0alpha1.TagService",
                    method = "Add",
                    request = "pinnacle.tag.v0alpha1.AddRequest",
                    response = "pinnacle.tag.v0alpha1.AddResponse",
                },
                ---@type GrpcRequestArgs
                Remove = {
                    service = "pinnacle.tag.v0alpha1.TagService",
                    method = "Remove",
                    request = "pinnacle.tag.v0alpha1.RemoveRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                Get = {
                    service = "pinnacle.tag.v0alpha1.TagService",
                    method = "Get",
                    request = "pinnacle.tag.v0alpha1.GetRequest",
                    response = "pinnacle.tag.v0alpha1.GetResponse",
                },
                ---@type GrpcRequestArgs
                GetProperties = {
                    service = "pinnacle.tag.v0alpha1.TagService",
                    method = "GetProperties",
                    request = "pinnacle.tag.v0alpha1.GetPropertiesRequest",
                    response = "pinnacle.tag.v0alpha1.GetPropertiesResponse",
                },
            },
        },
    },
    input = {
        v0alpha1 = {
            Modifier = util.bijective_table(pinnacle_input_v0alpha1_Modifier),
            SetMousebindRequest = {
                MouseEdge = util.bijective_table(
                    pinnacle_input_v0alpha1_SetMousebindRequest_MouseEdge
                ),
            },
            SetLibinputSettingRequest = {
                AccelProfile = util.bijective_table(
                    pinnacle_input_v0alpha1_SetLibinputSettingRequest_AccelProfile
                ),
                ClickMethod = util.bijective_table(
                    pinnacle_input_v0alpha1_SetLibinputSettingRequest_ClickMethod
                ),
                ScrollMethod = util.bijective_table(
                    pinnacle_input_v0alpha1_SetLibinputSettingRequest_ScrollMethod
                ),
                TapButtonMap = util.bijective_table(
                    pinnacle_input_v0alpha1_SetLibinputSettingRequest_TapButtonMap
                ),
            },
            InputService = {
                ---@type GrpcRequestArgs
                SetKeybind = {
                    service = "pinnacle.input.v0alpha1.InputService",
                    method = "SetKeybind",
                    request = "pinnacle.input.v0alpha1.SetKeybindRequest",
                    response = "pinnacle.input.v0alpha1.SetKeybindResponse",
                },
                ---@type GrpcRequestArgs
                SetMousebind = {
                    service = "pinnacle.input.v0alpha1.InputService",
                    method = "SetMousebind",
                    request = "pinnacle.input.v0alpha1.SetMousebindRequest",
                    response = "pinnacle.input.v0alpha1.SetMousebindResponse",
                },
                ---@type GrpcRequestArgs
                KeybindDescriptions = {
                    service = "pinnacle.input.v0alpha1.InputService",
                    method = "KeybindDescriptions",
                    request = "pinnacle.input.v0alpha1.KeybindDescriptionsRequest",
                    response = "pinnacle.input.v0alpha1.KeybindDescriptionsResponse",
                },
                ---@type GrpcRequestArgs
                SetXkbConfig = {
                    service = "pinnacle.input.v0alpha1.InputService",
                    method = "SetXkbConfig",
                    request = "pinnacle.input.v0alpha1.SetXkbConfigRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                SetRepeatRate = {
                    service = "pinnacle.input.v0alpha1.InputService",
                    method = "SetRepeatRate",
                    request = "pinnacle.input.v0alpha1.SetRepeatRateRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                SetLibinputSetting = {
                    service = "pinnacle.input.v0alpha1.InputService",
                    method = "SetLibinputSetting",
                    request = "pinnacle.input.v0alpha1.SetLibinputSettingRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                SetXcursor = {
                    service = "pinnacle.input.v0alpha1.InputService",
                    method = "SetXcursor",
                    request = "pinnacle.input.v0alpha1.SetXcursorRequest",
                    response = "google.protobuf.Empty",
                },
            },
        },
    },
    process = {
        v0alpha1 = {
            ProcessService = {
                ---@type GrpcRequestArgs
                Spawn = {
                    service = "pinnacle.process.v0alpha1.ProcessService",
                    method = "Spawn",
                    request = "pinnacle.process.v0alpha1.SpawnRequest",
                    response = "pinnacle.process.v0alpha1.SpawnResponse",
                },
                ---@type GrpcRequestArgs
                SetEnv = {
                    service = "pinnacle.process.v0alpha1.ProcessService",
                    method = "SetEnv",
                    request = "pinnacle.process.v0alpha1.SetEnvRequest",
                    response = "google.protobuf.Empty",
                },
            },
        },
    },
    layout = {
        v0alpha1 = {
            LayoutService = {
                ---@type GrpcRequestArgs
                Layout = {
                    service = "pinnacle.layout.v0alpha1.LayoutService",
                    method = "Layout",
                    request = "pinnacle.layout.v0alpha1.LayoutRequest",
                    response = "pinnacle.layout.v0alpha1.LayoutResponse",
                },
            },
        },
    },
    render = {
        v0alpha1 = {
            Filter = util.bijective_table(pinnacle_render_v0alpha1_Filter),
            RenderService = {
                ---@type GrpcRequestArgs
                SetUpscaleFilter = {
                    service = "pinnacle.render.v0alpha1.RenderService",
                    method = "SetUpscaleFilter",
                    request = "pinnacle.render.v0alpha1.SetUpscaleFilterRequest",
                    response = "google.protobuf.Empty",
                },
                ---@type GrpcRequestArgs
                SetDownscaleFilter = {
                    service = "pinnacle.render.v0alpha1.RenderService",
                    method = "SetDownscaleFilter",
                    request = "pinnacle.render.v0alpha1.SetDownscaleFilterRequest",
                    response = "google.protobuf.Empty",
                },
            },
        },
    },
    signal = {
        v0alpha1 = {
            StreamControl = util.bijective_table(pinnacle_signal_v0alpha1_StreamControl),
            ---@enum (key) SignalServiceMethod
            SignalService = {
                ---@type GrpcRequestArgs
                OutputConnect = {
                    service = "pinnacle.signal.v0alpha1.SignalService",
                    method = "OutputConnect",
                    request = "pinnacle.signal.v0alpha1.OutputConnectRequest",
                    response = "pinnacle.signal.v0alpha1.OutputConnectResponse",
                },
                ---@type GrpcRequestArgs
                OutputDisconnect = {
                    service = "pinnacle.signal.v0alpha1.SignalService",
                    method = "OutputDisconnect",
                    request = "pinnacle.signal.v0alpha1.OutputDisconnectRequest",
                    response = "pinnacle.signal.v0alpha1.OutputDisconnectResponse",
                },
                ---@type GrpcRequestArgs
                OutputResize = {
                    service = "pinnacle.signal.v0alpha1.SignalService",
                    method = "OutputResize",
                    request = "pinnacle.signal.v0alpha1.OutputResizeRequest",
                    response = "pinnacle.signal.v0alpha1.OutputResizeResponse",
                },
                ---@type GrpcRequestArgs
                OutputMove = {
                    service = "pinnacle.signal.v0alpha1.SignalService",
                    method = "OutputMove",
                    request = "pinnacle.signal.v0alpha1.OutputMoveRequest",
                    response = "pinnacle.signal.v0alpha1.OutputMoveResponse",
                },
                ---@type GrpcRequestArgs
                WindowPointerEnter = {
                    service = "pinnacle.signal.v0alpha1.SignalService",
                    method = "WindowPointerEnter",
                    request = "pinnacle.signal.v0alpha1.WindowPointerEnterRequest",
                    response = "pinnacle.signal.v0alpha1.WindowPointerEnterResponse",
                },
                ---@type GrpcRequestArgs
                WindowPointerLeave = {
                    service = "pinnacle.signal.v0alpha1.SignalService",
                    method = "WindowPointerLeave",
                    request = "pinnacle.signal.v0alpha1.WindowPointerLeaveRequest",
                    response = "pinnacle.signal.v0alpha1.WindowPointerLeaveResponse",
                },
                ---@type GrpcRequestArgs
                TagActive = {
                    service = "pinnacle.signal.v0alpha1.SignalService",
                    method = "TagActive",
                    request = "pinnacle.signal.v0alpha1.TagActiveRequest",
                    response = "pinnacle.signal.v0alpha1.TagActiveResponse",
                },
            },
        },
    },
}

return defs
