---@lcat nodoc

---@enum pinnacle.signal.v0alpha1.StreamControl
local pinnacle_signal_v0alpha1_StreamControl = {
    STREAM_CONTROL_UNSPECIFIED = 0,
    STREAM_CONTROL_READY = 1,
    STREAM_CONTROL_DISCONNECT = 2,
}

---@enum pinnacle.input.v0alpha1.Modifier
local pinnacle_input_v0alpha1_Modifier = {
    MODIFIER_UNSPECIFIED = 0,
    MODIFIER_SHIFT = 1,
    MODIFIER_CTRL = 2,
    MODIFIER_ALT = 3,
    MODIFIER_SUPER = 4,
}

---@enum pinnacle.input.v0alpha1.SetMousebindRequest.MouseEdge
local pinnacle_input_v0alpha1_SetMousebindRequest_MouseEdge = {
    MOUSE_EDGE_UNSPECIFIED = 0,
    MOUSE_EDGE_PRESS = 1,
    MOUSE_EDGE_RELEASE = 2,
}

---@enum pinnacle.input.v0alpha1.SetLibinputSettingRequest.AccelProfile
local pinnacle_input_v0alpha1_SetLibinputSettingRequest_AccelProfile = {
    ACCEL_PROFILE_UNSPECIFIED = 0,
    ACCEL_PROFILE_FLAT = 1,
    ACCEL_PROFILE_ADAPTIVE = 2,
}

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

---@enum pinnacle.v0alpha1.SetOrToggle
local pinnacle_v0alpha1_SetOrToggle = {
    SET_OR_TOGGLE_UNSPECIFIED = 0,
    SET_OR_TOGGLE_SET = 1,
    SET_OR_TOGGLE_UNSET = 2,
    SET_OR_TOGGLE_TOGGLE = 3,
}

---@enum pinnacle.v0alpha1.Backend
local pinnacle_v0alpha1_Backend = {
    BACKEND_UNSPECIFIED = 0,
    BACKEND_WINDOW = 1,
    BACKEND_TTY = 2,
}

---@enum pinnacle.window.v0alpha1.FullscreenOrMaximized
local pinnacle_window_v0alpha1_FullscreenOrMaximized = {
    FULLSCREEN_OR_MAXIMIZED_UNSPECIFIED = 0,
    FULLSCREEN_OR_MAXIMIZED_NEITHER = 1,
    FULLSCREEN_OR_MAXIMIZED_FULLSCREEN = 2,
    FULLSCREEN_OR_MAXIMIZED_MAXIMIZED = 3,
}

---@enum pinnacle.window.v0alpha1.WindowState
local pinnacle_window_v0alpha1_WindowState = {
    WINDOW_STATE_UNSPECIFIED = 0,
    WINDOW_STATE_TILED = 1,
    WINDOW_STATE_FLOATING = 2,
    WINDOW_STATE_FULLSCREEN = 3,
    WINDOW_STATE_MAXIMIZED = 4,
}

---@enum pinnacle.render.v0alpha1.Filter
local pinnacle_render_v0alpha1_Filter = {
    FILTER_UNSPECIFIED = 0,
    FILTER_BILINEAR = 1,
    FILTER_NEAREST_NEIGHBOR = 2,
}

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


---@class google.protobuf.Empty

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

---@class pinnacle.input.v0alpha1.SetKeybindRequest
---@field modifiers pinnacle.input.v0alpha1.Modifier[]?
---@field raw_code integer?
---@field xkb_name string?
---@field group string?
---@field description string?

---@class pinnacle.input.v0alpha1.SetKeybindResponse

---@class pinnacle.input.v0alpha1.KeybindDescriptionsRequest

---@class pinnacle.input.v0alpha1.KeybindDescriptionsResponse
---@field descriptions pinnacle.input.v0alpha1.KeybindDescription[]?

---@class pinnacle.input.v0alpha1.KeybindDescription
---@field modifiers pinnacle.input.v0alpha1.Modifier[]?
---@field raw_code integer?
---@field xkb_name string?
---@field group string?
---@field description string?

---@class pinnacle.input.v0alpha1.SetMousebindRequest
---@field modifiers pinnacle.input.v0alpha1.Modifier[]?
---@field button integer?
---@field edge pinnacle.input.v0alpha1.SetMousebindRequest.MouseEdge?

---@class pinnacle.input.v0alpha1.SetMousebindResponse

---@class pinnacle.input.v0alpha1.SetXkbConfigRequest
---@field rules string?
---@field variant string?
---@field layout string?
---@field model string?
---@field options string?

---@class pinnacle.input.v0alpha1.SetRepeatRateRequest
---@field rate integer?
---@field delay integer?

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

---@class pinnacle.input.v0alpha1.SetLibinputSettingRequest.CalibrationMatrix
---@field matrix number[]?

---@class pinnacle.input.v0alpha1.SetXcursorRequest
---@field theme string?
---@field size integer?

---@class pinnacle.v0alpha1.Geometry
---@field x integer?
---@field y integer?
---@field width integer?
---@field height integer?

---@class pinnacle.v0alpha1.QuitRequest

---@class pinnacle.v0alpha1.ReloadConfigRequest

---@class pinnacle.v0alpha1.PingRequest
---@field payload string?

---@class pinnacle.v0alpha1.PingResponse
---@field payload string?

---@class pinnacle.v0alpha1.ShutdownWatchRequest

---@class pinnacle.v0alpha1.ShutdownWatchResponse

---@class pinnacle.v0alpha1.BackendRequest

---@class pinnacle.v0alpha1.BackendResponse
---@field backend pinnacle.v0alpha1.Backend?

---@class pinnacle.layout.v0alpha1.LayoutRequest
---@field geometries pinnacle.layout.v0alpha1.LayoutRequest.Geometries?
---@field layout pinnacle.layout.v0alpha1.LayoutRequest.ExplicitLayout?

---@class pinnacle.layout.v0alpha1.LayoutRequest.Geometries
---@field request_id integer?
---@field output_name string?
---@field geometries pinnacle.v0alpha1.Geometry[]?

---@class pinnacle.layout.v0alpha1.LayoutRequest.ExplicitLayout
---@field output_name string?

---@class pinnacle.layout.v0alpha1.LayoutResponse
---@field request_id integer?
---@field output_name string?
---@field window_ids integer[]?
---@field tag_ids integer[]?
---@field output_width integer?
---@field output_height integer?

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
---@field state pinnacle.window.v0alpha1.WindowState?

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
---@field ssd boolean?
---@field state pinnacle.window.v0alpha1.WindowState?

---@class pinnacle.render.v0alpha1.SetUpscaleFilterRequest
---@field filter pinnacle.render.v0alpha1.Filter?

---@class pinnacle.render.v0alpha1.SetDownscaleFilterRequest
---@field filter pinnacle.render.v0alpha1.Filter?

---@class pinnacle.output.v0alpha1.Mode
---@field pixel_width integer?
---@field pixel_height integer?
---@field refresh_rate_millihz integer?

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
---@field transform pinnacle.output.v0alpha1.Transform?

---@class pinnacle.output.v0alpha1.SetPoweredRequest
---@field output_name string?
---@field powered boolean?

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


local google = {}
google.protobuf = {}
google.protobuf.Empty = {}
local pinnacle = {}
pinnacle.signal = {}
pinnacle.signal.v0alpha1 = {}
pinnacle.signal.v0alpha1.OutputConnectRequest = {}
pinnacle.signal.v0alpha1.OutputConnectResponse = {}
pinnacle.signal.v0alpha1.OutputDisconnectRequest = {}
pinnacle.signal.v0alpha1.OutputDisconnectResponse = {}
pinnacle.signal.v0alpha1.OutputResizeRequest = {}
pinnacle.signal.v0alpha1.OutputResizeResponse = {}
pinnacle.signal.v0alpha1.OutputMoveRequest = {}
pinnacle.signal.v0alpha1.OutputMoveResponse = {}
pinnacle.signal.v0alpha1.WindowPointerEnterRequest = {}
pinnacle.signal.v0alpha1.WindowPointerEnterResponse = {}
pinnacle.signal.v0alpha1.WindowPointerLeaveRequest = {}
pinnacle.signal.v0alpha1.WindowPointerLeaveResponse = {}
pinnacle.signal.v0alpha1.TagActiveRequest = {}
pinnacle.signal.v0alpha1.TagActiveResponse = {}
pinnacle.input = {}
pinnacle.input.v0alpha1 = {}
pinnacle.input.v0alpha1.SetKeybindRequest = {}
pinnacle.input.v0alpha1.SetKeybindResponse = {}
pinnacle.input.v0alpha1.KeybindDescriptionsRequest = {}
pinnacle.input.v0alpha1.KeybindDescriptionsResponse = {}
pinnacle.input.v0alpha1.KeybindDescription = {}
pinnacle.input.v0alpha1.SetMousebindRequest = {}
pinnacle.input.v0alpha1.SetMousebindResponse = {}
pinnacle.input.v0alpha1.SetXkbConfigRequest = {}
pinnacle.input.v0alpha1.SetRepeatRateRequest = {}
pinnacle.input.v0alpha1.SetLibinputSettingRequest = {}
pinnacle.input.v0alpha1.SetLibinputSettingRequest.CalibrationMatrix = {}
pinnacle.input.v0alpha1.SetXcursorRequest = {}
pinnacle.v0alpha1 = {}
pinnacle.v0alpha1.Geometry = {}
pinnacle.v0alpha1.QuitRequest = {}
pinnacle.v0alpha1.ReloadConfigRequest = {}
pinnacle.v0alpha1.PingRequest = {}
pinnacle.v0alpha1.PingResponse = {}
pinnacle.v0alpha1.ShutdownWatchRequest = {}
pinnacle.v0alpha1.ShutdownWatchResponse = {}
pinnacle.v0alpha1.BackendRequest = {}
pinnacle.v0alpha1.BackendResponse = {}
pinnacle.layout = {}
pinnacle.layout.v0alpha1 = {}
pinnacle.layout.v0alpha1.LayoutRequest = {}
pinnacle.layout.v0alpha1.LayoutRequest.Geometries = {}
pinnacle.layout.v0alpha1.LayoutRequest.ExplicitLayout = {}
pinnacle.layout.v0alpha1.LayoutResponse = {}
pinnacle.tag = {}
pinnacle.tag.v0alpha1 = {}
pinnacle.tag.v0alpha1.SetActiveRequest = {}
pinnacle.tag.v0alpha1.SwitchToRequest = {}
pinnacle.tag.v0alpha1.AddRequest = {}
pinnacle.tag.v0alpha1.AddResponse = {}
pinnacle.tag.v0alpha1.RemoveRequest = {}
pinnacle.tag.v0alpha1.GetRequest = {}
pinnacle.tag.v0alpha1.GetResponse = {}
pinnacle.tag.v0alpha1.GetPropertiesRequest = {}
pinnacle.tag.v0alpha1.GetPropertiesResponse = {}
pinnacle.process = {}
pinnacle.process.v0alpha1 = {}
pinnacle.process.v0alpha1.SpawnRequest = {}
pinnacle.process.v0alpha1.SpawnResponse = {}
pinnacle.process.v0alpha1.SetEnvRequest = {}
pinnacle.window = {}
pinnacle.window.v0alpha1 = {}
pinnacle.window.v0alpha1.CloseRequest = {}
pinnacle.window.v0alpha1.SetGeometryRequest = {}
pinnacle.window.v0alpha1.SetFullscreenRequest = {}
pinnacle.window.v0alpha1.SetMaximizedRequest = {}
pinnacle.window.v0alpha1.SetFloatingRequest = {}
pinnacle.window.v0alpha1.SetFocusedRequest = {}
pinnacle.window.v0alpha1.MoveToTagRequest = {}
pinnacle.window.v0alpha1.SetTagRequest = {}
pinnacle.window.v0alpha1.RaiseRequest = {}
pinnacle.window.v0alpha1.MoveGrabRequest = {}
pinnacle.window.v0alpha1.ResizeGrabRequest = {}
pinnacle.window.v0alpha1.GetRequest = {}
pinnacle.window.v0alpha1.GetResponse = {}
pinnacle.window.v0alpha1.GetPropertiesRequest = {}
pinnacle.window.v0alpha1.GetPropertiesResponse = {}
pinnacle.window.v0alpha1.AddWindowRuleRequest = {}
pinnacle.window.v0alpha1.WindowRuleCondition = {}
pinnacle.window.v0alpha1.WindowRule = {}
pinnacle.render = {}
pinnacle.render.v0alpha1 = {}
pinnacle.render.v0alpha1.SetUpscaleFilterRequest = {}
pinnacle.render.v0alpha1.SetDownscaleFilterRequest = {}
pinnacle.output = {}
pinnacle.output.v0alpha1 = {}
pinnacle.output.v0alpha1.Mode = {}
pinnacle.output.v0alpha1.SetLocationRequest = {}
pinnacle.output.v0alpha1.SetModeRequest = {}
pinnacle.output.v0alpha1.SetModelineRequest = {}
pinnacle.output.v0alpha1.SetScaleRequest = {}
pinnacle.output.v0alpha1.SetTransformRequest = {}
pinnacle.output.v0alpha1.SetPoweredRequest = {}
pinnacle.output.v0alpha1.GetRequest = {}
pinnacle.output.v0alpha1.GetResponse = {}
pinnacle.output.v0alpha1.GetPropertiesRequest = {}
pinnacle.output.v0alpha1.GetPropertiesResponse = {}

pinnacle.signal.v0alpha1.StreamControl = pinnacle_signal_v0alpha1_StreamControl
pinnacle.input.v0alpha1.Modifier = pinnacle_input_v0alpha1_Modifier
pinnacle.input.v0alpha1.SetMousebindRequest.MouseEdge = pinnacle_input_v0alpha1_SetMousebindRequest_MouseEdge
pinnacle.input.v0alpha1.SetLibinputSettingRequest.AccelProfile = pinnacle_input_v0alpha1_SetLibinputSettingRequest_AccelProfile
pinnacle.input.v0alpha1.SetLibinputSettingRequest.ClickMethod = pinnacle_input_v0alpha1_SetLibinputSettingRequest_ClickMethod
pinnacle.input.v0alpha1.SetLibinputSettingRequest.ScrollMethod = pinnacle_input_v0alpha1_SetLibinputSettingRequest_ScrollMethod
pinnacle.input.v0alpha1.SetLibinputSettingRequest.TapButtonMap = pinnacle_input_v0alpha1_SetLibinputSettingRequest_TapButtonMap
pinnacle.v0alpha1.SetOrToggle = pinnacle_v0alpha1_SetOrToggle
pinnacle.v0alpha1.Backend = pinnacle_v0alpha1_Backend
pinnacle.window.v0alpha1.FullscreenOrMaximized = pinnacle_window_v0alpha1_FullscreenOrMaximized
pinnacle.window.v0alpha1.WindowState = pinnacle_window_v0alpha1_WindowState
pinnacle.render.v0alpha1.Filter = pinnacle_render_v0alpha1_Filter
pinnacle.output.v0alpha1.Transform = pinnacle_output_v0alpha1_Transform

pinnacle.signal.v0alpha1.SignalService = {}
pinnacle.signal.v0alpha1.SignalService.OutputConnect = {}
pinnacle.signal.v0alpha1.SignalService.OutputConnect.service = "pinnacle.signal.v0alpha1.SignalService"
pinnacle.signal.v0alpha1.SignalService.OutputConnect.method = "OutputConnect"
pinnacle.signal.v0alpha1.SignalService.OutputConnect.request = ".pinnacle.signal.v0alpha1.OutputConnectRequest"
pinnacle.signal.v0alpha1.SignalService.OutputConnect.response = ".pinnacle.signal.v0alpha1.OutputConnectResponse"
pinnacle.signal.v0alpha1.SignalService.OutputDisconnect = {}
pinnacle.signal.v0alpha1.SignalService.OutputDisconnect.service = "pinnacle.signal.v0alpha1.SignalService"
pinnacle.signal.v0alpha1.SignalService.OutputDisconnect.method = "OutputDisconnect"
pinnacle.signal.v0alpha1.SignalService.OutputDisconnect.request = ".pinnacle.signal.v0alpha1.OutputDisconnectRequest"
pinnacle.signal.v0alpha1.SignalService.OutputDisconnect.response = ".pinnacle.signal.v0alpha1.OutputDisconnectResponse"
pinnacle.signal.v0alpha1.SignalService.OutputResize = {}
pinnacle.signal.v0alpha1.SignalService.OutputResize.service = "pinnacle.signal.v0alpha1.SignalService"
pinnacle.signal.v0alpha1.SignalService.OutputResize.method = "OutputResize"
pinnacle.signal.v0alpha1.SignalService.OutputResize.request = ".pinnacle.signal.v0alpha1.OutputResizeRequest"
pinnacle.signal.v0alpha1.SignalService.OutputResize.response = ".pinnacle.signal.v0alpha1.OutputResizeResponse"
pinnacle.signal.v0alpha1.SignalService.OutputMove = {}
pinnacle.signal.v0alpha1.SignalService.OutputMove.service = "pinnacle.signal.v0alpha1.SignalService"
pinnacle.signal.v0alpha1.SignalService.OutputMove.method = "OutputMove"
pinnacle.signal.v0alpha1.SignalService.OutputMove.request = ".pinnacle.signal.v0alpha1.OutputMoveRequest"
pinnacle.signal.v0alpha1.SignalService.OutputMove.response = ".pinnacle.signal.v0alpha1.OutputMoveResponse"
pinnacle.signal.v0alpha1.SignalService.WindowPointerEnter = {}
pinnacle.signal.v0alpha1.SignalService.WindowPointerEnter.service = "pinnacle.signal.v0alpha1.SignalService"
pinnacle.signal.v0alpha1.SignalService.WindowPointerEnter.method = "WindowPointerEnter"
pinnacle.signal.v0alpha1.SignalService.WindowPointerEnter.request = ".pinnacle.signal.v0alpha1.WindowPointerEnterRequest"
pinnacle.signal.v0alpha1.SignalService.WindowPointerEnter.response = ".pinnacle.signal.v0alpha1.WindowPointerEnterResponse"
pinnacle.signal.v0alpha1.SignalService.WindowPointerLeave = {}
pinnacle.signal.v0alpha1.SignalService.WindowPointerLeave.service = "pinnacle.signal.v0alpha1.SignalService"
pinnacle.signal.v0alpha1.SignalService.WindowPointerLeave.method = "WindowPointerLeave"
pinnacle.signal.v0alpha1.SignalService.WindowPointerLeave.request = ".pinnacle.signal.v0alpha1.WindowPointerLeaveRequest"
pinnacle.signal.v0alpha1.SignalService.WindowPointerLeave.response = ".pinnacle.signal.v0alpha1.WindowPointerLeaveResponse"
pinnacle.signal.v0alpha1.SignalService.TagActive = {}
pinnacle.signal.v0alpha1.SignalService.TagActive.service = "pinnacle.signal.v0alpha1.SignalService"
pinnacle.signal.v0alpha1.SignalService.TagActive.method = "TagActive"
pinnacle.signal.v0alpha1.SignalService.TagActive.request = ".pinnacle.signal.v0alpha1.TagActiveRequest"
pinnacle.signal.v0alpha1.SignalService.TagActive.response = ".pinnacle.signal.v0alpha1.TagActiveResponse"
pinnacle.input.v0alpha1.InputService = {}
pinnacle.input.v0alpha1.InputService.SetKeybind = {}
pinnacle.input.v0alpha1.InputService.SetKeybind.service = "pinnacle.input.v0alpha1.InputService"
pinnacle.input.v0alpha1.InputService.SetKeybind.method = "SetKeybind"
pinnacle.input.v0alpha1.InputService.SetKeybind.request = ".pinnacle.input.v0alpha1.SetKeybindRequest"
pinnacle.input.v0alpha1.InputService.SetKeybind.response = ".pinnacle.input.v0alpha1.SetKeybindResponse"
pinnacle.input.v0alpha1.InputService.SetMousebind = {}
pinnacle.input.v0alpha1.InputService.SetMousebind.service = "pinnacle.input.v0alpha1.InputService"
pinnacle.input.v0alpha1.InputService.SetMousebind.method = "SetMousebind"
pinnacle.input.v0alpha1.InputService.SetMousebind.request = ".pinnacle.input.v0alpha1.SetMousebindRequest"
pinnacle.input.v0alpha1.InputService.SetMousebind.response = ".pinnacle.input.v0alpha1.SetMousebindResponse"
pinnacle.input.v0alpha1.InputService.KeybindDescriptions = {}
pinnacle.input.v0alpha1.InputService.KeybindDescriptions.service = "pinnacle.input.v0alpha1.InputService"
pinnacle.input.v0alpha1.InputService.KeybindDescriptions.method = "KeybindDescriptions"
pinnacle.input.v0alpha1.InputService.KeybindDescriptions.request = ".pinnacle.input.v0alpha1.KeybindDescriptionsRequest"
pinnacle.input.v0alpha1.InputService.KeybindDescriptions.response = ".pinnacle.input.v0alpha1.KeybindDescriptionsResponse"
pinnacle.input.v0alpha1.InputService.SetXkbConfig = {}
pinnacle.input.v0alpha1.InputService.SetXkbConfig.service = "pinnacle.input.v0alpha1.InputService"
pinnacle.input.v0alpha1.InputService.SetXkbConfig.method = "SetXkbConfig"
pinnacle.input.v0alpha1.InputService.SetXkbConfig.request = ".pinnacle.input.v0alpha1.SetXkbConfigRequest"
pinnacle.input.v0alpha1.InputService.SetXkbConfig.response = ".google.protobuf.Empty"
pinnacle.input.v0alpha1.InputService.SetRepeatRate = {}
pinnacle.input.v0alpha1.InputService.SetRepeatRate.service = "pinnacle.input.v0alpha1.InputService"
pinnacle.input.v0alpha1.InputService.SetRepeatRate.method = "SetRepeatRate"
pinnacle.input.v0alpha1.InputService.SetRepeatRate.request = ".pinnacle.input.v0alpha1.SetRepeatRateRequest"
pinnacle.input.v0alpha1.InputService.SetRepeatRate.response = ".google.protobuf.Empty"
pinnacle.input.v0alpha1.InputService.SetLibinputSetting = {}
pinnacle.input.v0alpha1.InputService.SetLibinputSetting.service = "pinnacle.input.v0alpha1.InputService"
pinnacle.input.v0alpha1.InputService.SetLibinputSetting.method = "SetLibinputSetting"
pinnacle.input.v0alpha1.InputService.SetLibinputSetting.request = ".pinnacle.input.v0alpha1.SetLibinputSettingRequest"
pinnacle.input.v0alpha1.InputService.SetLibinputSetting.response = ".google.protobuf.Empty"
pinnacle.input.v0alpha1.InputService.SetXcursor = {}
pinnacle.input.v0alpha1.InputService.SetXcursor.service = "pinnacle.input.v0alpha1.InputService"
pinnacle.input.v0alpha1.InputService.SetXcursor.method = "SetXcursor"
pinnacle.input.v0alpha1.InputService.SetXcursor.request = ".pinnacle.input.v0alpha1.SetXcursorRequest"
pinnacle.input.v0alpha1.InputService.SetXcursor.response = ".google.protobuf.Empty"
pinnacle.v0alpha1.PinnacleService = {}
pinnacle.v0alpha1.PinnacleService.Quit = {}
pinnacle.v0alpha1.PinnacleService.Quit.service = "pinnacle.v0alpha1.PinnacleService"
pinnacle.v0alpha1.PinnacleService.Quit.method = "Quit"
pinnacle.v0alpha1.PinnacleService.Quit.request = ".pinnacle.v0alpha1.QuitRequest"
pinnacle.v0alpha1.PinnacleService.Quit.response = ".google.protobuf.Empty"
pinnacle.v0alpha1.PinnacleService.ReloadConfig = {}
pinnacle.v0alpha1.PinnacleService.ReloadConfig.service = "pinnacle.v0alpha1.PinnacleService"
pinnacle.v0alpha1.PinnacleService.ReloadConfig.method = "ReloadConfig"
pinnacle.v0alpha1.PinnacleService.ReloadConfig.request = ".pinnacle.v0alpha1.ReloadConfigRequest"
pinnacle.v0alpha1.PinnacleService.ReloadConfig.response = ".google.protobuf.Empty"
pinnacle.v0alpha1.PinnacleService.Ping = {}
pinnacle.v0alpha1.PinnacleService.Ping.service = "pinnacle.v0alpha1.PinnacleService"
pinnacle.v0alpha1.PinnacleService.Ping.method = "Ping"
pinnacle.v0alpha1.PinnacleService.Ping.request = ".pinnacle.v0alpha1.PingRequest"
pinnacle.v0alpha1.PinnacleService.Ping.response = ".pinnacle.v0alpha1.PingResponse"
pinnacle.v0alpha1.PinnacleService.ShutdownWatch = {}
pinnacle.v0alpha1.PinnacleService.ShutdownWatch.service = "pinnacle.v0alpha1.PinnacleService"
pinnacle.v0alpha1.PinnacleService.ShutdownWatch.method = "ShutdownWatch"
pinnacle.v0alpha1.PinnacleService.ShutdownWatch.request = ".pinnacle.v0alpha1.ShutdownWatchRequest"
pinnacle.v0alpha1.PinnacleService.ShutdownWatch.response = ".pinnacle.v0alpha1.ShutdownWatchResponse"
pinnacle.v0alpha1.PinnacleService.Backend = {}
pinnacle.v0alpha1.PinnacleService.Backend.service = "pinnacle.v0alpha1.PinnacleService"
pinnacle.v0alpha1.PinnacleService.Backend.method = "Backend"
pinnacle.v0alpha1.PinnacleService.Backend.request = ".pinnacle.v0alpha1.BackendRequest"
pinnacle.v0alpha1.PinnacleService.Backend.response = ".pinnacle.v0alpha1.BackendResponse"
pinnacle.layout.v0alpha1.LayoutService = {}
pinnacle.layout.v0alpha1.LayoutService.Layout = {}
pinnacle.layout.v0alpha1.LayoutService.Layout.service = "pinnacle.layout.v0alpha1.LayoutService"
pinnacle.layout.v0alpha1.LayoutService.Layout.method = "Layout"
pinnacle.layout.v0alpha1.LayoutService.Layout.request = ".pinnacle.layout.v0alpha1.LayoutRequest"
pinnacle.layout.v0alpha1.LayoutService.Layout.response = ".pinnacle.layout.v0alpha1.LayoutResponse"
pinnacle.tag.v0alpha1.TagService = {}
pinnacle.tag.v0alpha1.TagService.SetActive = {}
pinnacle.tag.v0alpha1.TagService.SetActive.service = "pinnacle.tag.v0alpha1.TagService"
pinnacle.tag.v0alpha1.TagService.SetActive.method = "SetActive"
pinnacle.tag.v0alpha1.TagService.SetActive.request = ".pinnacle.tag.v0alpha1.SetActiveRequest"
pinnacle.tag.v0alpha1.TagService.SetActive.response = ".google.protobuf.Empty"
pinnacle.tag.v0alpha1.TagService.SwitchTo = {}
pinnacle.tag.v0alpha1.TagService.SwitchTo.service = "pinnacle.tag.v0alpha1.TagService"
pinnacle.tag.v0alpha1.TagService.SwitchTo.method = "SwitchTo"
pinnacle.tag.v0alpha1.TagService.SwitchTo.request = ".pinnacle.tag.v0alpha1.SwitchToRequest"
pinnacle.tag.v0alpha1.TagService.SwitchTo.response = ".google.protobuf.Empty"
pinnacle.tag.v0alpha1.TagService.Add = {}
pinnacle.tag.v0alpha1.TagService.Add.service = "pinnacle.tag.v0alpha1.TagService"
pinnacle.tag.v0alpha1.TagService.Add.method = "Add"
pinnacle.tag.v0alpha1.TagService.Add.request = ".pinnacle.tag.v0alpha1.AddRequest"
pinnacle.tag.v0alpha1.TagService.Add.response = ".pinnacle.tag.v0alpha1.AddResponse"
pinnacle.tag.v0alpha1.TagService.Remove = {}
pinnacle.tag.v0alpha1.TagService.Remove.service = "pinnacle.tag.v0alpha1.TagService"
pinnacle.tag.v0alpha1.TagService.Remove.method = "Remove"
pinnacle.tag.v0alpha1.TagService.Remove.request = ".pinnacle.tag.v0alpha1.RemoveRequest"
pinnacle.tag.v0alpha1.TagService.Remove.response = ".google.protobuf.Empty"
pinnacle.tag.v0alpha1.TagService.Get = {}
pinnacle.tag.v0alpha1.TagService.Get.service = "pinnacle.tag.v0alpha1.TagService"
pinnacle.tag.v0alpha1.TagService.Get.method = "Get"
pinnacle.tag.v0alpha1.TagService.Get.request = ".pinnacle.tag.v0alpha1.GetRequest"
pinnacle.tag.v0alpha1.TagService.Get.response = ".pinnacle.tag.v0alpha1.GetResponse"
pinnacle.tag.v0alpha1.TagService.GetProperties = {}
pinnacle.tag.v0alpha1.TagService.GetProperties.service = "pinnacle.tag.v0alpha1.TagService"
pinnacle.tag.v0alpha1.TagService.GetProperties.method = "GetProperties"
pinnacle.tag.v0alpha1.TagService.GetProperties.request = ".pinnacle.tag.v0alpha1.GetPropertiesRequest"
pinnacle.tag.v0alpha1.TagService.GetProperties.response = ".pinnacle.tag.v0alpha1.GetPropertiesResponse"
pinnacle.process.v0alpha1.ProcessService = {}
pinnacle.process.v0alpha1.ProcessService.Spawn = {}
pinnacle.process.v0alpha1.ProcessService.Spawn.service = "pinnacle.process.v0alpha1.ProcessService"
pinnacle.process.v0alpha1.ProcessService.Spawn.method = "Spawn"
pinnacle.process.v0alpha1.ProcessService.Spawn.request = ".pinnacle.process.v0alpha1.SpawnRequest"
pinnacle.process.v0alpha1.ProcessService.Spawn.response = ".pinnacle.process.v0alpha1.SpawnResponse"
pinnacle.process.v0alpha1.ProcessService.SetEnv = {}
pinnacle.process.v0alpha1.ProcessService.SetEnv.service = "pinnacle.process.v0alpha1.ProcessService"
pinnacle.process.v0alpha1.ProcessService.SetEnv.method = "SetEnv"
pinnacle.process.v0alpha1.ProcessService.SetEnv.request = ".pinnacle.process.v0alpha1.SetEnvRequest"
pinnacle.process.v0alpha1.ProcessService.SetEnv.response = ".google.protobuf.Empty"
pinnacle.window.v0alpha1.WindowService = {}
pinnacle.window.v0alpha1.WindowService.Close = {}
pinnacle.window.v0alpha1.WindowService.Close.service = "pinnacle.window.v0alpha1.WindowService"
pinnacle.window.v0alpha1.WindowService.Close.method = "Close"
pinnacle.window.v0alpha1.WindowService.Close.request = ".pinnacle.window.v0alpha1.CloseRequest"
pinnacle.window.v0alpha1.WindowService.Close.response = ".google.protobuf.Empty"
pinnacle.window.v0alpha1.WindowService.SetGeometry = {}
pinnacle.window.v0alpha1.WindowService.SetGeometry.service = "pinnacle.window.v0alpha1.WindowService"
pinnacle.window.v0alpha1.WindowService.SetGeometry.method = "SetGeometry"
pinnacle.window.v0alpha1.WindowService.SetGeometry.request = ".pinnacle.window.v0alpha1.SetGeometryRequest"
pinnacle.window.v0alpha1.WindowService.SetGeometry.response = ".google.protobuf.Empty"
pinnacle.window.v0alpha1.WindowService.SetFullscreen = {}
pinnacle.window.v0alpha1.WindowService.SetFullscreen.service = "pinnacle.window.v0alpha1.WindowService"
pinnacle.window.v0alpha1.WindowService.SetFullscreen.method = "SetFullscreen"
pinnacle.window.v0alpha1.WindowService.SetFullscreen.request = ".pinnacle.window.v0alpha1.SetFullscreenRequest"
pinnacle.window.v0alpha1.WindowService.SetFullscreen.response = ".google.protobuf.Empty"
pinnacle.window.v0alpha1.WindowService.SetMaximized = {}
pinnacle.window.v0alpha1.WindowService.SetMaximized.service = "pinnacle.window.v0alpha1.WindowService"
pinnacle.window.v0alpha1.WindowService.SetMaximized.method = "SetMaximized"
pinnacle.window.v0alpha1.WindowService.SetMaximized.request = ".pinnacle.window.v0alpha1.SetMaximizedRequest"
pinnacle.window.v0alpha1.WindowService.SetMaximized.response = ".google.protobuf.Empty"
pinnacle.window.v0alpha1.WindowService.SetFloating = {}
pinnacle.window.v0alpha1.WindowService.SetFloating.service = "pinnacle.window.v0alpha1.WindowService"
pinnacle.window.v0alpha1.WindowService.SetFloating.method = "SetFloating"
pinnacle.window.v0alpha1.WindowService.SetFloating.request = ".pinnacle.window.v0alpha1.SetFloatingRequest"
pinnacle.window.v0alpha1.WindowService.SetFloating.response = ".google.protobuf.Empty"
pinnacle.window.v0alpha1.WindowService.SetFocused = {}
pinnacle.window.v0alpha1.WindowService.SetFocused.service = "pinnacle.window.v0alpha1.WindowService"
pinnacle.window.v0alpha1.WindowService.SetFocused.method = "SetFocused"
pinnacle.window.v0alpha1.WindowService.SetFocused.request = ".pinnacle.window.v0alpha1.SetFocusedRequest"
pinnacle.window.v0alpha1.WindowService.SetFocused.response = ".google.protobuf.Empty"
pinnacle.window.v0alpha1.WindowService.MoveToTag = {}
pinnacle.window.v0alpha1.WindowService.MoveToTag.service = "pinnacle.window.v0alpha1.WindowService"
pinnacle.window.v0alpha1.WindowService.MoveToTag.method = "MoveToTag"
pinnacle.window.v0alpha1.WindowService.MoveToTag.request = ".pinnacle.window.v0alpha1.MoveToTagRequest"
pinnacle.window.v0alpha1.WindowService.MoveToTag.response = ".google.protobuf.Empty"
pinnacle.window.v0alpha1.WindowService.SetTag = {}
pinnacle.window.v0alpha1.WindowService.SetTag.service = "pinnacle.window.v0alpha1.WindowService"
pinnacle.window.v0alpha1.WindowService.SetTag.method = "SetTag"
pinnacle.window.v0alpha1.WindowService.SetTag.request = ".pinnacle.window.v0alpha1.SetTagRequest"
pinnacle.window.v0alpha1.WindowService.SetTag.response = ".google.protobuf.Empty"
pinnacle.window.v0alpha1.WindowService.Raise = {}
pinnacle.window.v0alpha1.WindowService.Raise.service = "pinnacle.window.v0alpha1.WindowService"
pinnacle.window.v0alpha1.WindowService.Raise.method = "Raise"
pinnacle.window.v0alpha1.WindowService.Raise.request = ".pinnacle.window.v0alpha1.RaiseRequest"
pinnacle.window.v0alpha1.WindowService.Raise.response = ".google.protobuf.Empty"
pinnacle.window.v0alpha1.WindowService.MoveGrab = {}
pinnacle.window.v0alpha1.WindowService.MoveGrab.service = "pinnacle.window.v0alpha1.WindowService"
pinnacle.window.v0alpha1.WindowService.MoveGrab.method = "MoveGrab"
pinnacle.window.v0alpha1.WindowService.MoveGrab.request = ".pinnacle.window.v0alpha1.MoveGrabRequest"
pinnacle.window.v0alpha1.WindowService.MoveGrab.response = ".google.protobuf.Empty"
pinnacle.window.v0alpha1.WindowService.ResizeGrab = {}
pinnacle.window.v0alpha1.WindowService.ResizeGrab.service = "pinnacle.window.v0alpha1.WindowService"
pinnacle.window.v0alpha1.WindowService.ResizeGrab.method = "ResizeGrab"
pinnacle.window.v0alpha1.WindowService.ResizeGrab.request = ".pinnacle.window.v0alpha1.ResizeGrabRequest"
pinnacle.window.v0alpha1.WindowService.ResizeGrab.response = ".google.protobuf.Empty"
pinnacle.window.v0alpha1.WindowService.Get = {}
pinnacle.window.v0alpha1.WindowService.Get.service = "pinnacle.window.v0alpha1.WindowService"
pinnacle.window.v0alpha1.WindowService.Get.method = "Get"
pinnacle.window.v0alpha1.WindowService.Get.request = ".pinnacle.window.v0alpha1.GetRequest"
pinnacle.window.v0alpha1.WindowService.Get.response = ".pinnacle.window.v0alpha1.GetResponse"
pinnacle.window.v0alpha1.WindowService.GetProperties = {}
pinnacle.window.v0alpha1.WindowService.GetProperties.service = "pinnacle.window.v0alpha1.WindowService"
pinnacle.window.v0alpha1.WindowService.GetProperties.method = "GetProperties"
pinnacle.window.v0alpha1.WindowService.GetProperties.request = ".pinnacle.window.v0alpha1.GetPropertiesRequest"
pinnacle.window.v0alpha1.WindowService.GetProperties.response = ".pinnacle.window.v0alpha1.GetPropertiesResponse"
pinnacle.window.v0alpha1.WindowService.AddWindowRule = {}
pinnacle.window.v0alpha1.WindowService.AddWindowRule.service = "pinnacle.window.v0alpha1.WindowService"
pinnacle.window.v0alpha1.WindowService.AddWindowRule.method = "AddWindowRule"
pinnacle.window.v0alpha1.WindowService.AddWindowRule.request = ".pinnacle.window.v0alpha1.AddWindowRuleRequest"
pinnacle.window.v0alpha1.WindowService.AddWindowRule.response = ".google.protobuf.Empty"
pinnacle.render.v0alpha1.RenderService = {}
pinnacle.render.v0alpha1.RenderService.SetUpscaleFilter = {}
pinnacle.render.v0alpha1.RenderService.SetUpscaleFilter.service = "pinnacle.render.v0alpha1.RenderService"
pinnacle.render.v0alpha1.RenderService.SetUpscaleFilter.method = "SetUpscaleFilter"
pinnacle.render.v0alpha1.RenderService.SetUpscaleFilter.request = ".pinnacle.render.v0alpha1.SetUpscaleFilterRequest"
pinnacle.render.v0alpha1.RenderService.SetUpscaleFilter.response = ".google.protobuf.Empty"
pinnacle.render.v0alpha1.RenderService.SetDownscaleFilter = {}
pinnacle.render.v0alpha1.RenderService.SetDownscaleFilter.service = "pinnacle.render.v0alpha1.RenderService"
pinnacle.render.v0alpha1.RenderService.SetDownscaleFilter.method = "SetDownscaleFilter"
pinnacle.render.v0alpha1.RenderService.SetDownscaleFilter.request = ".pinnacle.render.v0alpha1.SetDownscaleFilterRequest"
pinnacle.render.v0alpha1.RenderService.SetDownscaleFilter.response = ".google.protobuf.Empty"
pinnacle.output.v0alpha1.OutputService = {}
pinnacle.output.v0alpha1.OutputService.SetLocation = {}
pinnacle.output.v0alpha1.OutputService.SetLocation.service = "pinnacle.output.v0alpha1.OutputService"
pinnacle.output.v0alpha1.OutputService.SetLocation.method = "SetLocation"
pinnacle.output.v0alpha1.OutputService.SetLocation.request = ".pinnacle.output.v0alpha1.SetLocationRequest"
pinnacle.output.v0alpha1.OutputService.SetLocation.response = ".google.protobuf.Empty"
pinnacle.output.v0alpha1.OutputService.SetMode = {}
pinnacle.output.v0alpha1.OutputService.SetMode.service = "pinnacle.output.v0alpha1.OutputService"
pinnacle.output.v0alpha1.OutputService.SetMode.method = "SetMode"
pinnacle.output.v0alpha1.OutputService.SetMode.request = ".pinnacle.output.v0alpha1.SetModeRequest"
pinnacle.output.v0alpha1.OutputService.SetMode.response = ".google.protobuf.Empty"
pinnacle.output.v0alpha1.OutputService.SetModeline = {}
pinnacle.output.v0alpha1.OutputService.SetModeline.service = "pinnacle.output.v0alpha1.OutputService"
pinnacle.output.v0alpha1.OutputService.SetModeline.method = "SetModeline"
pinnacle.output.v0alpha1.OutputService.SetModeline.request = ".pinnacle.output.v0alpha1.SetModelineRequest"
pinnacle.output.v0alpha1.OutputService.SetModeline.response = ".google.protobuf.Empty"
pinnacle.output.v0alpha1.OutputService.SetScale = {}
pinnacle.output.v0alpha1.OutputService.SetScale.service = "pinnacle.output.v0alpha1.OutputService"
pinnacle.output.v0alpha1.OutputService.SetScale.method = "SetScale"
pinnacle.output.v0alpha1.OutputService.SetScale.request = ".pinnacle.output.v0alpha1.SetScaleRequest"
pinnacle.output.v0alpha1.OutputService.SetScale.response = ".google.protobuf.Empty"
pinnacle.output.v0alpha1.OutputService.SetTransform = {}
pinnacle.output.v0alpha1.OutputService.SetTransform.service = "pinnacle.output.v0alpha1.OutputService"
pinnacle.output.v0alpha1.OutputService.SetTransform.method = "SetTransform"
pinnacle.output.v0alpha1.OutputService.SetTransform.request = ".pinnacle.output.v0alpha1.SetTransformRequest"
pinnacle.output.v0alpha1.OutputService.SetTransform.response = ".google.protobuf.Empty"
pinnacle.output.v0alpha1.OutputService.SetPowered = {}
pinnacle.output.v0alpha1.OutputService.SetPowered.service = "pinnacle.output.v0alpha1.OutputService"
pinnacle.output.v0alpha1.OutputService.SetPowered.method = "SetPowered"
pinnacle.output.v0alpha1.OutputService.SetPowered.request = ".pinnacle.output.v0alpha1.SetPoweredRequest"
pinnacle.output.v0alpha1.OutputService.SetPowered.response = ".google.protobuf.Empty"
pinnacle.output.v0alpha1.OutputService.Get = {}
pinnacle.output.v0alpha1.OutputService.Get.service = "pinnacle.output.v0alpha1.OutputService"
pinnacle.output.v0alpha1.OutputService.Get.method = "Get"
pinnacle.output.v0alpha1.OutputService.Get.request = ".pinnacle.output.v0alpha1.GetRequest"
pinnacle.output.v0alpha1.OutputService.Get.response = ".pinnacle.output.v0alpha1.GetResponse"
pinnacle.output.v0alpha1.OutputService.GetProperties = {}
pinnacle.output.v0alpha1.OutputService.GetProperties.service = "pinnacle.output.v0alpha1.OutputService"
pinnacle.output.v0alpha1.OutputService.GetProperties.method = "GetProperties"
pinnacle.output.v0alpha1.OutputService.GetProperties.request = ".pinnacle.output.v0alpha1.GetPropertiesRequest"
pinnacle.output.v0alpha1.OutputService.GetProperties.response = ".pinnacle.output.v0alpha1.GetPropertiesResponse"

return {
    google = google,
    pinnacle = pinnacle,
}

