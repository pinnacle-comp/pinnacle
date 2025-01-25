---@lcat nodoc

---@enum pinnacle.signal.v1.StreamControl
local pinnacle_signal_v1_StreamControl = {
    STREAM_CONTROL_UNSPECIFIED = 0,
    STREAM_CONTROL_READY = 1,
    STREAM_CONTROL_DISCONNECT = 2,
}

---@enum pinnacle.input.v1.Modifier
local pinnacle_input_v1_Modifier = {
    MODIFIER_UNSPECIFIED = 0,
    MODIFIER_SHIFT = 1,
    MODIFIER_CTRL = 2,
    MODIFIER_ALT = 3,
    MODIFIER_SUPER = 4,
    MODIFIER_ISO_LEVEL3_SHIFT = 5,
    MODIFIER_ISO_LEVEL5_SHIFT = 6,
}

---@enum pinnacle.input.v1.Edge
local pinnacle_input_v1_Edge = {
    EDGE_UNSPECIFIED = 0,
    EDGE_PRESS = 1,
    EDGE_RELEASE = 2,
}

---@enum pinnacle.input.v1.ClickMethod
local pinnacle_input_v1_ClickMethod = {
    CLICK_METHOD_UNSPECIFIED = 0,
    CLICK_METHOD_BUTTON_AREAS = 1,
    CLICK_METHOD_CLICK_FINGER = 2,
}

---@enum pinnacle.input.v1.AccelProfile
local pinnacle_input_v1_AccelProfile = {
    ACCEL_PROFILE_UNSPECIFIED = 0,
    ACCEL_PROFILE_FLAT = 1,
    ACCEL_PROFILE_ADAPTIVE = 2,
}

---@enum pinnacle.input.v1.ScrollMethod
local pinnacle_input_v1_ScrollMethod = {
    SCROLL_METHOD_UNSPECIFIED = 0,
    SCROLL_METHOD_NO_SCROLL = 1,
    SCROLL_METHOD_TWO_FINGER = 2,
    SCROLL_METHOD_EDGE = 3,
    SCROLL_METHOD_ON_BUTTON_DOWN = 4,
}

---@enum pinnacle.input.v1.TapButtonMap
local pinnacle_input_v1_TapButtonMap = {
    TAP_BUTTON_MAP_UNSPECIFIED = 0,
    TAP_BUTTON_MAP_LEFT_RIGHT_MIDDLE = 1,
    TAP_BUTTON_MAP_LEFT_MIDDLE_RIGHT = 2,
}

---@enum pinnacle.input.v1.SendEventsMode
local pinnacle_input_v1_SendEventsMode = {
    SEND_EVENTS_MODE_UNSPECIFIED = 0,
    SEND_EVENTS_MODE_ENABLED = 1,
    SEND_EVENTS_MODE_DISABLED = 2,
    SEND_EVENTS_MODE_DISABLED_ON_EXTERNAL_MOUSE = 3,
}

---@enum pinnacle.input.v1.DeviceType
local pinnacle_input_v1_DeviceType = {
    DEVICE_TYPE_UNSPECIFIED = 0,
    DEVICE_TYPE_TOUCHPAD = 1,
    DEVICE_TYPE_TRACKBALL = 2,
    DEVICE_TYPE_TRACKPOINT = 3,
    DEVICE_TYPE_MOUSE = 4,
    DEVICE_TYPE_TABLET = 5,
    DEVICE_TYPE_KEYBOARD = 6,
    DEVICE_TYPE_SWITCH = 7,
}

---@enum pinnacle.util.v1.SetOrToggle
local pinnacle_util_v1_SetOrToggle = {
    SET_OR_TOGGLE_UNSPECIFIED = 0,
    SET_OR_TOGGLE_SET = 1,
    SET_OR_TOGGLE_UNSET = 2,
    SET_OR_TOGGLE_TOGGLE = 3,
}

---@enum pinnacle.util.v1.AbsOrRel
local pinnacle_util_v1_AbsOrRel = {
    ABS_OR_REL_UNSPECIFIED = 0,
    ABS_OR_REL_ABSOLUTE = 1,
    ABS_OR_REL_RELATIVE = 2,
}

---@enum pinnacle.layout.v1.FlexDir
local pinnacle_layout_v1_FlexDir = {
    FLEX_DIR_UNSPECIFIED = 0,
    FLEX_DIR_ROW = 1,
    FLEX_DIR_COLUMN = 2,
}

---@enum pinnacle.window.v1.LayoutMode
local pinnacle_window_v1_LayoutMode = {
    LAYOUT_MODE_UNSPECIFIED = 0,
    LAYOUT_MODE_TILED = 1,
    LAYOUT_MODE_FLOATING = 2,
    LAYOUT_MODE_FULLSCREEN = 3,
    LAYOUT_MODE_MAXIMIZED = 4,
}

---@enum pinnacle.window.v1.DecorationMode
local pinnacle_window_v1_DecorationMode = {
    DECORATION_MODE_UNSPECIFIED = 0,
    DECORATION_MODE_CLIENT_SIDE = 1,
    DECORATION_MODE_SERVER_SIDE = 2,
}

---@enum pinnacle.render.v1.Filter
local pinnacle_render_v1_Filter = {
    FILTER_UNSPECIFIED = 0,
    FILTER_BILINEAR = 1,
    FILTER_NEAREST_NEIGHBOR = 2,
}

---@enum pinnacle.output.v1.Transform
local pinnacle_output_v1_Transform = {
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

---@enum pinnacle.v1.Backend
local pinnacle_v1_Backend = {
    BACKEND_UNSPECIFIED = 0,
    BACKEND_WINDOW = 1,
    BACKEND_TTY = 2,
}


---@class google.protobuf.Empty

---@class pinnacle.signal.v1.OutputConnectRequest
---@field control pinnacle.signal.v1.StreamControl?

---@class pinnacle.signal.v1.OutputConnectResponse
---@field output_name string?

---@class pinnacle.signal.v1.OutputDisconnectRequest
---@field control pinnacle.signal.v1.StreamControl?

---@class pinnacle.signal.v1.OutputDisconnectResponse
---@field output_name string?

---@class pinnacle.signal.v1.OutputResizeRequest
---@field control pinnacle.signal.v1.StreamControl?

---@class pinnacle.signal.v1.OutputResizeResponse
---@field output_name string?
---@field logical_width integer?
---@field logical_height integer?

---@class pinnacle.signal.v1.OutputMoveRequest
---@field control pinnacle.signal.v1.StreamControl?

---@class pinnacle.signal.v1.OutputMoveResponse
---@field output_name string?
---@field x integer?
---@field y integer?

---@class pinnacle.signal.v1.WindowPointerEnterRequest
---@field control pinnacle.signal.v1.StreamControl?

---@class pinnacle.signal.v1.WindowPointerEnterResponse
---@field window_id integer?

---@class pinnacle.signal.v1.WindowPointerLeaveRequest
---@field control pinnacle.signal.v1.StreamControl?

---@class pinnacle.signal.v1.WindowPointerLeaveResponse
---@field window_id integer?

---@class pinnacle.signal.v1.TagActiveRequest
---@field control pinnacle.signal.v1.StreamControl?

---@class pinnacle.signal.v1.TagActiveResponse
---@field tag_id integer?
---@field active boolean?

---@class pinnacle.signal.v1.InputDeviceAddedRequest
---@field control pinnacle.signal.v1.StreamControl?

---@class pinnacle.signal.v1.InputDeviceAddedResponse
---@field device_sysname string?

---@class pinnacle.input.v1.Bind
---@field mods pinnacle.input.v1.Modifier[]?
---@field ignore_mods pinnacle.input.v1.Modifier[]?
---@field layer_name string?
---@field group string?
---@field description string?
---@field key pinnacle.input.v1.Keybind?
---@field mouse pinnacle.input.v1.Mousebind?

---@class pinnacle.input.v1.BindRequest
---@field bind pinnacle.input.v1.Bind?

---@class pinnacle.input.v1.BindResponse
---@field bind_id integer?

---@class pinnacle.input.v1.SetQuitBindRequest
---@field bind_id integer?

---@class pinnacle.input.v1.SetReloadConfigBindRequest
---@field bind_id integer?

---@class pinnacle.input.v1.Keybind
---@field key_code integer?
---@field xkb_name string?

---@class pinnacle.input.v1.KeybindStreamRequest
---@field bind_id integer?

---@class pinnacle.input.v1.KeybindStreamResponse
---@field edge pinnacle.input.v1.Edge?

---@class pinnacle.input.v1.KeybindOnPressRequest
---@field bind_id integer?

---@class pinnacle.input.v1.Mousebind
---@field button integer?

---@class pinnacle.input.v1.MousebindStreamRequest
---@field bind_id integer?

---@class pinnacle.input.v1.MousebindStreamResponse
---@field edge pinnacle.input.v1.Edge?

---@class pinnacle.input.v1.MousebindOnPressRequest
---@field bind_id integer?

---@class pinnacle.input.v1.SetBindGroupRequest
---@field bind_id integer?
---@field group string?

---@class pinnacle.input.v1.SetBindDescriptionRequest
---@field bind_id integer?
---@field desc string?

---@class pinnacle.input.v1.GetBindInfosRequest

---@class pinnacle.input.v1.GetBindInfosResponse
---@field bind_infos pinnacle.input.v1.BindInfo[]?

---@class pinnacle.input.v1.BindInfo
---@field bind_id integer?
---@field bind pinnacle.input.v1.Bind?

---@class pinnacle.input.v1.GetBindLayerStackRequest

---@class pinnacle.input.v1.GetBindLayerStackResponse
---@field layer_names string[]?

---@class pinnacle.input.v1.EnterBindLayerRequest
---@field layer_name string?

---@class pinnacle.input.v1.SetXkbConfigRequest
---@field rules string?
---@field variant string?
---@field layout string?
---@field model string?
---@field options string?

---@class pinnacle.input.v1.SetRepeatRateRequest
---@field rate integer?
---@field delay integer?

---@class pinnacle.input.v1.SetXcursorRequest
---@field theme string?
---@field size integer?

---@class pinnacle.input.v1.CalibrationMatrix
---@field matrix number[]?

---@class pinnacle.input.v1.GetDevicesRequest

---@class pinnacle.input.v1.GetDevicesResponse
---@field device_sysnames string[]?

---@class pinnacle.input.v1.GetDeviceCapabilitiesRequest
---@field device_sysname string?

---@class pinnacle.input.v1.GetDeviceCapabilitiesResponse
---@field keyboard boolean?
---@field pointer boolean?
---@field touch boolean?
---@field tablet_tool boolean?
---@field tablet_pad boolean?
---@field gesture boolean?
---@field switch boolean?

---@class pinnacle.input.v1.GetDeviceInfoRequest
---@field device_sysname string?

---@class pinnacle.input.v1.GetDeviceInfoResponse
---@field name string?
---@field product_id integer?
---@field vendor_id integer?

---@class pinnacle.input.v1.GetDeviceTypeRequest
---@field device_sysname string?

---@class pinnacle.input.v1.GetDeviceTypeResponse
---@field device_type pinnacle.input.v1.DeviceType?

---@class pinnacle.input.v1.SetDeviceLibinputSettingRequest
---@field device_sysname string?
---@field accel_profile pinnacle.input.v1.AccelProfile?
---@field accel_speed number?
---@field calibration_matrix pinnacle.input.v1.CalibrationMatrix?
---@field click_method pinnacle.input.v1.ClickMethod?
---@field disable_while_typing boolean?
---@field left_handed boolean?
---@field middle_emulation boolean?
---@field rotation_angle integer?
---@field scroll_button integer?
---@field scroll_button_lock boolean?
---@field scroll_method pinnacle.input.v1.ScrollMethod?
---@field natural_scroll boolean?
---@field tap_button_map pinnacle.input.v1.TapButtonMap?
---@field tap_drag boolean?
---@field tap_drag_lock boolean?
---@field tap boolean?
---@field send_events_mode pinnacle.input.v1.SendEventsMode?

---@class pinnacle.util.v1.Point
---@field x integer?
---@field y integer?

---@class pinnacle.util.v1.Size
---@field width integer?
---@field height integer?

---@class pinnacle.layout.v1.Gaps
---@field left number?
---@field right number?
---@field top number?
---@field bottom number?

---@class pinnacle.layout.v1.LayoutNode
---@field label string?
---@field traversal_index integer?
---@field traversal_overrides pinnacle.layout.v1.LayoutNode.TraversalOverridesEntry[]?
---@field style pinnacle.layout.v1.NodeStyle?
---@field children pinnacle.layout.v1.LayoutNode[]?

---@class pinnacle.layout.v1.LayoutNode.TraversalOverridesEntry
---@field key integer?
---@field value pinnacle.layout.v1.TraversalOverrides?

---@class pinnacle.layout.v1.TraversalOverrides
---@field overrides integer[]?

---@class pinnacle.layout.v1.NodeStyle
---@field flex_dir pinnacle.layout.v1.FlexDir?
---@field size_proportion number?
---@field gaps pinnacle.layout.v1.Gaps?

---@class pinnacle.layout.v1.LayoutRequest
---@field tree_response pinnacle.layout.v1.LayoutRequest.TreeResponse?
---@field force_layout pinnacle.layout.v1.LayoutRequest.ForceLayout?

---@class pinnacle.layout.v1.LayoutRequest.TreeResponse
---@field request_id integer?
---@field tree_id integer?
---@field root_node pinnacle.layout.v1.LayoutNode?
---@field output_name string?

---@class pinnacle.layout.v1.LayoutRequest.ForceLayout
---@field output_name string?

---@class pinnacle.layout.v1.LayoutResponse
---@field request_id integer?
---@field output_name string?
---@field window_count integer?
---@field tag_ids integer[]?

---@class pinnacle.tag.v1.GetRequest

---@class pinnacle.tag.v1.GetResponse
---@field tag_ids integer[]?

---@class pinnacle.tag.v1.AddRequest
---@field output_name string?
---@field tag_names string[]?

---@class pinnacle.tag.v1.AddResponse
---@field tag_ids integer[]?

---@class pinnacle.tag.v1.RemoveRequest
---@field tag_ids integer[]?

---@class pinnacle.tag.v1.GetActiveRequest
---@field tag_id integer?

---@class pinnacle.tag.v1.GetActiveResponse
---@field active boolean?

---@class pinnacle.tag.v1.GetNameRequest
---@field tag_id integer?

---@class pinnacle.tag.v1.GetNameResponse
---@field name string?

---@class pinnacle.tag.v1.GetOutputNameRequest
---@field tag_id integer?

---@class pinnacle.tag.v1.GetOutputNameResponse
---@field output_name string?

---@class pinnacle.tag.v1.SetActiveRequest
---@field tag_id integer?
---@field set_or_toggle pinnacle.util.v1.SetOrToggle?

---@class pinnacle.tag.v1.SwitchToRequest
---@field tag_id integer?

---@class pinnacle.process.v1.SpawnRequest
---@field cmd string[]?
---@field unique boolean?
---@field once boolean?
---@field shell_cmd string[]?
---@field envs pinnacle.process.v1.SpawnRequest.EnvsEntry[]?

---@class pinnacle.process.v1.SpawnRequest.EnvsEntry
---@field key string?
---@field value string?

---@class pinnacle.process.v1.SpawnData
---@field pid integer?
---@field fd_socket_path string?
---@field has_stdin boolean?
---@field has_stdout boolean?
---@field has_stderr boolean?

---@class pinnacle.process.v1.SpawnResponse
---@field spawn_data pinnacle.process.v1.SpawnData?

---@class pinnacle.process.v1.WaitOnSpawnRequest
---@field pid integer?

---@class pinnacle.process.v1.WaitOnSpawnResponse
---@field exit_code integer?
---@field exit_msg string?

---@class pinnacle.window.v1.GetRequest

---@class pinnacle.window.v1.GetResponse
---@field window_ids integer[]?

---@class pinnacle.window.v1.GetAppIdRequest
---@field window_id integer?

---@class pinnacle.window.v1.GetAppIdResponse
---@field app_id string?

---@class pinnacle.window.v1.GetTitleRequest
---@field window_id integer?

---@class pinnacle.window.v1.GetTitleResponse
---@field title string?

---@class pinnacle.window.v1.GetLocRequest
---@field window_id integer?

---@class pinnacle.window.v1.GetLocResponse
---@field loc pinnacle.util.v1.Point?

---@class pinnacle.window.v1.GetSizeRequest
---@field window_id integer?

---@class pinnacle.window.v1.GetSizeResponse
---@field size pinnacle.util.v1.Size?

---@class pinnacle.window.v1.GetFocusedRequest
---@field window_id integer?

---@class pinnacle.window.v1.GetFocusedResponse
---@field focused boolean?

---@class pinnacle.window.v1.GetLayoutModeRequest
---@field window_id integer?

---@class pinnacle.window.v1.GetLayoutModeResponse
---@field layout_mode pinnacle.window.v1.LayoutMode?

---@class pinnacle.window.v1.GetTagIdsRequest
---@field window_id integer?

---@class pinnacle.window.v1.GetTagIdsResponse
---@field tag_ids integer[]?

---@class pinnacle.window.v1.CloseRequest
---@field window_id integer?

---@class pinnacle.window.v1.SetGeometryRequest
---@field window_id integer?
---@field x integer?
---@field y integer?
---@field w integer?
---@field h integer?

---@class pinnacle.window.v1.SetFullscreenRequest
---@field window_id integer?
---@field set_or_toggle pinnacle.util.v1.SetOrToggle?

---@class pinnacle.window.v1.SetMaximizedRequest
---@field window_id integer?
---@field set_or_toggle pinnacle.util.v1.SetOrToggle?

---@class pinnacle.window.v1.SetFloatingRequest
---@field window_id integer?
---@field set_or_toggle pinnacle.util.v1.SetOrToggle?

---@class pinnacle.window.v1.SetFocusedRequest
---@field window_id integer?
---@field set_or_toggle pinnacle.util.v1.SetOrToggle?

---@class pinnacle.window.v1.SetDecorationModeRequest
---@field window_id integer?
---@field decoration_mode pinnacle.window.v1.DecorationMode?

---@class pinnacle.window.v1.MoveToTagRequest
---@field window_id integer?
---@field tag_id integer?

---@class pinnacle.window.v1.SetTagRequest
---@field window_id integer?
---@field tag_id integer?
---@field set_or_toggle pinnacle.util.v1.SetOrToggle?

---@class pinnacle.window.v1.RaiseRequest
---@field window_id integer?

---@class pinnacle.window.v1.MoveGrabRequest
---@field button integer?

---@class pinnacle.window.v1.ResizeGrabRequest
---@field button integer?

---@class pinnacle.window.v1.WindowRuleRequest
---@field finished pinnacle.window.v1.WindowRuleRequest.Finished?

---@class pinnacle.window.v1.WindowRuleRequest.Finished
---@field request_id integer?

---@class pinnacle.window.v1.WindowRuleResponse
---@field new_window pinnacle.window.v1.WindowRuleResponse.NewWindowRequest?

---@class pinnacle.window.v1.WindowRuleResponse.NewWindowRequest
---@field request_id integer?
---@field window_id integer?

---@class pinnacle.render.v1.SetUpscaleFilterRequest
---@field filter pinnacle.render.v1.Filter?

---@class pinnacle.render.v1.SetDownscaleFilterRequest
---@field filter pinnacle.render.v1.Filter?

---@class pinnacle.output.v1.SetLocRequest
---@field output_name string?
---@field x integer?
---@field y integer?

---@class pinnacle.output.v1.SetModeRequest
---@field output_name string?
---@field size pinnacle.util.v1.Size?
---@field refresh_rate_mhz integer?
---@field custom boolean?

---@class pinnacle.output.v1.Modeline
---@field clock number?
---@field hdisplay integer?
---@field hsync_start integer?
---@field hsync_end integer?
---@field htotal integer?
---@field vdisplay integer?
---@field vsync_start integer?
---@field vsync_end integer?
---@field vtotal integer?
---@field hsync boolean?
---@field vsync boolean?

---@class pinnacle.output.v1.SetModelineRequest
---@field output_name string?
---@field modeline pinnacle.output.v1.Modeline?

---@class pinnacle.output.v1.SetScaleRequest
---@field output_name string?
---@field scale number?
---@field abs_or_rel pinnacle.util.v1.AbsOrRel?

---@class pinnacle.output.v1.SetTransformRequest
---@field output_name string?
---@field transform pinnacle.output.v1.Transform?

---@class pinnacle.output.v1.SetPoweredRequest
---@field output_name string?
---@field set_or_toggle pinnacle.util.v1.SetOrToggle?

---@class pinnacle.output.v1.GetRequest

---@class pinnacle.output.v1.GetResponse
---@field output_names string[]?

---@class pinnacle.output.v1.GetInfoRequest
---@field output_name string?

---@class pinnacle.output.v1.GetInfoResponse
---@field make string?
---@field model string?
---@field serial string?

---@class pinnacle.output.v1.GetLocRequest
---@field output_name string?

---@class pinnacle.output.v1.GetLocResponse
---@field loc pinnacle.util.v1.Point?

---@class pinnacle.output.v1.GetLogicalSizeRequest
---@field output_name string?

---@class pinnacle.output.v1.GetLogicalSizeResponse
---@field logical_size pinnacle.util.v1.Size?

---@class pinnacle.output.v1.GetPhysicalSizeRequest
---@field output_name string?

---@class pinnacle.output.v1.GetPhysicalSizeResponse
---@field physical_size pinnacle.util.v1.Size?

---@class pinnacle.output.v1.Mode
---@field size pinnacle.util.v1.Size?
---@field refresh_rate_mhz integer?

---@class pinnacle.output.v1.GetModesRequest
---@field output_name string?

---@class pinnacle.output.v1.GetModesResponse
---@field current_mode pinnacle.output.v1.Mode?
---@field preferred_mode pinnacle.output.v1.Mode?
---@field modes pinnacle.output.v1.Mode[]?

---@class pinnacle.output.v1.GetFocusedRequest
---@field output_name string?

---@class pinnacle.output.v1.GetFocusedResponse
---@field focused boolean?

---@class pinnacle.output.v1.GetTagIdsRequest
---@field output_name string?

---@class pinnacle.output.v1.GetTagIdsResponse
---@field tag_ids integer[]?

---@class pinnacle.output.v1.GetScaleRequest
---@field output_name string?

---@class pinnacle.output.v1.GetScaleResponse
---@field scale number?

---@class pinnacle.output.v1.GetTransformRequest
---@field output_name string?

---@class pinnacle.output.v1.GetTransformResponse
---@field transform pinnacle.output.v1.Transform?

---@class pinnacle.output.v1.GetEnabledRequest
---@field output_name string?

---@class pinnacle.output.v1.GetEnabledResponse
---@field enabled boolean?

---@class pinnacle.output.v1.GetPoweredRequest
---@field output_name string?

---@class pinnacle.output.v1.GetPoweredResponse
---@field powered boolean?

---@class pinnacle.output.v1.GetFocusStackWindowIdsRequest
---@field output_name string?

---@class pinnacle.output.v1.GetFocusStackWindowIdsResponse
---@field window_ids integer[]?

---@class pinnacle.v1.QuitRequest

---@class pinnacle.v1.ReloadConfigRequest

---@class pinnacle.v1.KeepaliveRequest

---@class pinnacle.v1.KeepaliveResponse

---@class pinnacle.v1.BackendRequest

---@class pinnacle.v1.BackendResponse
---@field backend pinnacle.v1.Backend?


local google = {}
google.protobuf = {}
google.protobuf.Empty = {}
local pinnacle = {}
pinnacle.signal = {}
pinnacle.signal.v1 = {}
pinnacle.signal.v1.OutputConnectRequest = {}
pinnacle.signal.v1.OutputConnectResponse = {}
pinnacle.signal.v1.OutputDisconnectRequest = {}
pinnacle.signal.v1.OutputDisconnectResponse = {}
pinnacle.signal.v1.OutputResizeRequest = {}
pinnacle.signal.v1.OutputResizeResponse = {}
pinnacle.signal.v1.OutputMoveRequest = {}
pinnacle.signal.v1.OutputMoveResponse = {}
pinnacle.signal.v1.WindowPointerEnterRequest = {}
pinnacle.signal.v1.WindowPointerEnterResponse = {}
pinnacle.signal.v1.WindowPointerLeaveRequest = {}
pinnacle.signal.v1.WindowPointerLeaveResponse = {}
pinnacle.signal.v1.TagActiveRequest = {}
pinnacle.signal.v1.TagActiveResponse = {}
pinnacle.signal.v1.InputDeviceAddedRequest = {}
pinnacle.signal.v1.InputDeviceAddedResponse = {}
pinnacle.input = {}
pinnacle.input.v1 = {}
pinnacle.input.v1.Bind = {}
pinnacle.input.v1.BindRequest = {}
pinnacle.input.v1.BindResponse = {}
pinnacle.input.v1.SetQuitBindRequest = {}
pinnacle.input.v1.SetReloadConfigBindRequest = {}
pinnacle.input.v1.Keybind = {}
pinnacle.input.v1.KeybindStreamRequest = {}
pinnacle.input.v1.KeybindStreamResponse = {}
pinnacle.input.v1.KeybindOnPressRequest = {}
pinnacle.input.v1.Mousebind = {}
pinnacle.input.v1.MousebindStreamRequest = {}
pinnacle.input.v1.MousebindStreamResponse = {}
pinnacle.input.v1.MousebindOnPressRequest = {}
pinnacle.input.v1.SetBindGroupRequest = {}
pinnacle.input.v1.SetBindDescriptionRequest = {}
pinnacle.input.v1.GetBindInfosRequest = {}
pinnacle.input.v1.GetBindInfosResponse = {}
pinnacle.input.v1.BindInfo = {}
pinnacle.input.v1.GetBindLayerStackRequest = {}
pinnacle.input.v1.GetBindLayerStackResponse = {}
pinnacle.input.v1.EnterBindLayerRequest = {}
pinnacle.input.v1.SetXkbConfigRequest = {}
pinnacle.input.v1.SetRepeatRateRequest = {}
pinnacle.input.v1.SetXcursorRequest = {}
pinnacle.input.v1.CalibrationMatrix = {}
pinnacle.input.v1.GetDevicesRequest = {}
pinnacle.input.v1.GetDevicesResponse = {}
pinnacle.input.v1.GetDeviceCapabilitiesRequest = {}
pinnacle.input.v1.GetDeviceCapabilitiesResponse = {}
pinnacle.input.v1.GetDeviceInfoRequest = {}
pinnacle.input.v1.GetDeviceInfoResponse = {}
pinnacle.input.v1.GetDeviceTypeRequest = {}
pinnacle.input.v1.GetDeviceTypeResponse = {}
pinnacle.input.v1.SetDeviceLibinputSettingRequest = {}
pinnacle.util = {}
pinnacle.util.v1 = {}
pinnacle.util.v1.Point = {}
pinnacle.util.v1.Size = {}
pinnacle.layout = {}
pinnacle.layout.v1 = {}
pinnacle.layout.v1.Gaps = {}
pinnacle.layout.v1.LayoutNode = {}
pinnacle.layout.v1.LayoutNode.TraversalOverridesEntry = {}
pinnacle.layout.v1.TraversalOverrides = {}
pinnacle.layout.v1.NodeStyle = {}
pinnacle.layout.v1.LayoutRequest = {}
pinnacle.layout.v1.LayoutRequest.TreeResponse = {}
pinnacle.layout.v1.LayoutRequest.ForceLayout = {}
pinnacle.layout.v1.LayoutResponse = {}
pinnacle.tag = {}
pinnacle.tag.v1 = {}
pinnacle.tag.v1.GetRequest = {}
pinnacle.tag.v1.GetResponse = {}
pinnacle.tag.v1.AddRequest = {}
pinnacle.tag.v1.AddResponse = {}
pinnacle.tag.v1.RemoveRequest = {}
pinnacle.tag.v1.GetActiveRequest = {}
pinnacle.tag.v1.GetActiveResponse = {}
pinnacle.tag.v1.GetNameRequest = {}
pinnacle.tag.v1.GetNameResponse = {}
pinnacle.tag.v1.GetOutputNameRequest = {}
pinnacle.tag.v1.GetOutputNameResponse = {}
pinnacle.tag.v1.SetActiveRequest = {}
pinnacle.tag.v1.SwitchToRequest = {}
pinnacle.process = {}
pinnacle.process.v1 = {}
pinnacle.process.v1.SpawnRequest = {}
pinnacle.process.v1.SpawnRequest.EnvsEntry = {}
pinnacle.process.v1.SpawnData = {}
pinnacle.process.v1.SpawnResponse = {}
pinnacle.process.v1.WaitOnSpawnRequest = {}
pinnacle.process.v1.WaitOnSpawnResponse = {}
pinnacle.window = {}
pinnacle.window.v1 = {}
pinnacle.window.v1.GetRequest = {}
pinnacle.window.v1.GetResponse = {}
pinnacle.window.v1.GetAppIdRequest = {}
pinnacle.window.v1.GetAppIdResponse = {}
pinnacle.window.v1.GetTitleRequest = {}
pinnacle.window.v1.GetTitleResponse = {}
pinnacle.window.v1.GetLocRequest = {}
pinnacle.window.v1.GetLocResponse = {}
pinnacle.window.v1.GetSizeRequest = {}
pinnacle.window.v1.GetSizeResponse = {}
pinnacle.window.v1.GetFocusedRequest = {}
pinnacle.window.v1.GetFocusedResponse = {}
pinnacle.window.v1.GetLayoutModeRequest = {}
pinnacle.window.v1.GetLayoutModeResponse = {}
pinnacle.window.v1.GetTagIdsRequest = {}
pinnacle.window.v1.GetTagIdsResponse = {}
pinnacle.window.v1.CloseRequest = {}
pinnacle.window.v1.SetGeometryRequest = {}
pinnacle.window.v1.SetFullscreenRequest = {}
pinnacle.window.v1.SetMaximizedRequest = {}
pinnacle.window.v1.SetFloatingRequest = {}
pinnacle.window.v1.SetFocusedRequest = {}
pinnacle.window.v1.SetDecorationModeRequest = {}
pinnacle.window.v1.MoveToTagRequest = {}
pinnacle.window.v1.SetTagRequest = {}
pinnacle.window.v1.RaiseRequest = {}
pinnacle.window.v1.MoveGrabRequest = {}
pinnacle.window.v1.ResizeGrabRequest = {}
pinnacle.window.v1.WindowRuleRequest = {}
pinnacle.window.v1.WindowRuleRequest.Finished = {}
pinnacle.window.v1.WindowRuleResponse = {}
pinnacle.window.v1.WindowRuleResponse.NewWindowRequest = {}
pinnacle.render = {}
pinnacle.render.v1 = {}
pinnacle.render.v1.SetUpscaleFilterRequest = {}
pinnacle.render.v1.SetDownscaleFilterRequest = {}
pinnacle.output = {}
pinnacle.output.v1 = {}
pinnacle.output.v1.SetLocRequest = {}
pinnacle.output.v1.SetModeRequest = {}
pinnacle.output.v1.Modeline = {}
pinnacle.output.v1.SetModelineRequest = {}
pinnacle.output.v1.SetScaleRequest = {}
pinnacle.output.v1.SetTransformRequest = {}
pinnacle.output.v1.SetPoweredRequest = {}
pinnacle.output.v1.GetRequest = {}
pinnacle.output.v1.GetResponse = {}
pinnacle.output.v1.GetInfoRequest = {}
pinnacle.output.v1.GetInfoResponse = {}
pinnacle.output.v1.GetLocRequest = {}
pinnacle.output.v1.GetLocResponse = {}
pinnacle.output.v1.GetLogicalSizeRequest = {}
pinnacle.output.v1.GetLogicalSizeResponse = {}
pinnacle.output.v1.GetPhysicalSizeRequest = {}
pinnacle.output.v1.GetPhysicalSizeResponse = {}
pinnacle.output.v1.Mode = {}
pinnacle.output.v1.GetModesRequest = {}
pinnacle.output.v1.GetModesResponse = {}
pinnacle.output.v1.GetFocusedRequest = {}
pinnacle.output.v1.GetFocusedResponse = {}
pinnacle.output.v1.GetTagIdsRequest = {}
pinnacle.output.v1.GetTagIdsResponse = {}
pinnacle.output.v1.GetScaleRequest = {}
pinnacle.output.v1.GetScaleResponse = {}
pinnacle.output.v1.GetTransformRequest = {}
pinnacle.output.v1.GetTransformResponse = {}
pinnacle.output.v1.GetEnabledRequest = {}
pinnacle.output.v1.GetEnabledResponse = {}
pinnacle.output.v1.GetPoweredRequest = {}
pinnacle.output.v1.GetPoweredResponse = {}
pinnacle.output.v1.GetFocusStackWindowIdsRequest = {}
pinnacle.output.v1.GetFocusStackWindowIdsResponse = {}
pinnacle.v1 = {}
pinnacle.v1.QuitRequest = {}
pinnacle.v1.ReloadConfigRequest = {}
pinnacle.v1.KeepaliveRequest = {}
pinnacle.v1.KeepaliveResponse = {}
pinnacle.v1.BackendRequest = {}
pinnacle.v1.BackendResponse = {}

pinnacle.signal.v1.StreamControl = pinnacle_signal_v1_StreamControl
pinnacle.input.v1.Modifier = pinnacle_input_v1_Modifier
pinnacle.input.v1.Edge = pinnacle_input_v1_Edge
pinnacle.input.v1.ClickMethod = pinnacle_input_v1_ClickMethod
pinnacle.input.v1.AccelProfile = pinnacle_input_v1_AccelProfile
pinnacle.input.v1.ScrollMethod = pinnacle_input_v1_ScrollMethod
pinnacle.input.v1.TapButtonMap = pinnacle_input_v1_TapButtonMap
pinnacle.input.v1.SendEventsMode = pinnacle_input_v1_SendEventsMode
pinnacle.input.v1.DeviceType = pinnacle_input_v1_DeviceType
pinnacle.util.v1.SetOrToggle = pinnacle_util_v1_SetOrToggle
pinnacle.util.v1.AbsOrRel = pinnacle_util_v1_AbsOrRel
pinnacle.layout.v1.FlexDir = pinnacle_layout_v1_FlexDir
pinnacle.window.v1.LayoutMode = pinnacle_window_v1_LayoutMode
pinnacle.window.v1.DecorationMode = pinnacle_window_v1_DecorationMode
pinnacle.render.v1.Filter = pinnacle_render_v1_Filter
pinnacle.output.v1.Transform = pinnacle_output_v1_Transform
pinnacle.v1.Backend = pinnacle_v1_Backend

pinnacle.signal.v1.SignalService = {}
pinnacle.signal.v1.SignalService.OutputConnect = {}
pinnacle.signal.v1.SignalService.OutputConnect.service = "pinnacle.signal.v1.SignalService"
pinnacle.signal.v1.SignalService.OutputConnect.method = "OutputConnect"
pinnacle.signal.v1.SignalService.OutputConnect.request = ".pinnacle.signal.v1.OutputConnectRequest"
pinnacle.signal.v1.SignalService.OutputConnect.response = ".pinnacle.signal.v1.OutputConnectResponse"
pinnacle.signal.v1.SignalService.OutputDisconnect = {}
pinnacle.signal.v1.SignalService.OutputDisconnect.service = "pinnacle.signal.v1.SignalService"
pinnacle.signal.v1.SignalService.OutputDisconnect.method = "OutputDisconnect"
pinnacle.signal.v1.SignalService.OutputDisconnect.request = ".pinnacle.signal.v1.OutputDisconnectRequest"
pinnacle.signal.v1.SignalService.OutputDisconnect.response = ".pinnacle.signal.v1.OutputDisconnectResponse"
pinnacle.signal.v1.SignalService.OutputResize = {}
pinnacle.signal.v1.SignalService.OutputResize.service = "pinnacle.signal.v1.SignalService"
pinnacle.signal.v1.SignalService.OutputResize.method = "OutputResize"
pinnacle.signal.v1.SignalService.OutputResize.request = ".pinnacle.signal.v1.OutputResizeRequest"
pinnacle.signal.v1.SignalService.OutputResize.response = ".pinnacle.signal.v1.OutputResizeResponse"
pinnacle.signal.v1.SignalService.OutputMove = {}
pinnacle.signal.v1.SignalService.OutputMove.service = "pinnacle.signal.v1.SignalService"
pinnacle.signal.v1.SignalService.OutputMove.method = "OutputMove"
pinnacle.signal.v1.SignalService.OutputMove.request = ".pinnacle.signal.v1.OutputMoveRequest"
pinnacle.signal.v1.SignalService.OutputMove.response = ".pinnacle.signal.v1.OutputMoveResponse"
pinnacle.signal.v1.SignalService.WindowPointerEnter = {}
pinnacle.signal.v1.SignalService.WindowPointerEnter.service = "pinnacle.signal.v1.SignalService"
pinnacle.signal.v1.SignalService.WindowPointerEnter.method = "WindowPointerEnter"
pinnacle.signal.v1.SignalService.WindowPointerEnter.request = ".pinnacle.signal.v1.WindowPointerEnterRequest"
pinnacle.signal.v1.SignalService.WindowPointerEnter.response = ".pinnacle.signal.v1.WindowPointerEnterResponse"
pinnacle.signal.v1.SignalService.WindowPointerLeave = {}
pinnacle.signal.v1.SignalService.WindowPointerLeave.service = "pinnacle.signal.v1.SignalService"
pinnacle.signal.v1.SignalService.WindowPointerLeave.method = "WindowPointerLeave"
pinnacle.signal.v1.SignalService.WindowPointerLeave.request = ".pinnacle.signal.v1.WindowPointerLeaveRequest"
pinnacle.signal.v1.SignalService.WindowPointerLeave.response = ".pinnacle.signal.v1.WindowPointerLeaveResponse"
pinnacle.signal.v1.SignalService.TagActive = {}
pinnacle.signal.v1.SignalService.TagActive.service = "pinnacle.signal.v1.SignalService"
pinnacle.signal.v1.SignalService.TagActive.method = "TagActive"
pinnacle.signal.v1.SignalService.TagActive.request = ".pinnacle.signal.v1.TagActiveRequest"
pinnacle.signal.v1.SignalService.TagActive.response = ".pinnacle.signal.v1.TagActiveResponse"
pinnacle.signal.v1.SignalService.InputDeviceAdded = {}
pinnacle.signal.v1.SignalService.InputDeviceAdded.service = "pinnacle.signal.v1.SignalService"
pinnacle.signal.v1.SignalService.InputDeviceAdded.method = "InputDeviceAdded"
pinnacle.signal.v1.SignalService.InputDeviceAdded.request = ".pinnacle.signal.v1.InputDeviceAddedRequest"
pinnacle.signal.v1.SignalService.InputDeviceAdded.response = ".pinnacle.signal.v1.InputDeviceAddedResponse"
pinnacle.input.v1.InputService = {}
pinnacle.input.v1.InputService.Bind = {}
pinnacle.input.v1.InputService.Bind.service = "pinnacle.input.v1.InputService"
pinnacle.input.v1.InputService.Bind.method = "Bind"
pinnacle.input.v1.InputService.Bind.request = ".pinnacle.input.v1.BindRequest"
pinnacle.input.v1.InputService.Bind.response = ".pinnacle.input.v1.BindResponse"
pinnacle.input.v1.InputService.GetBindInfos = {}
pinnacle.input.v1.InputService.GetBindInfos.service = "pinnacle.input.v1.InputService"
pinnacle.input.v1.InputService.GetBindInfos.method = "GetBindInfos"
pinnacle.input.v1.InputService.GetBindInfos.request = ".pinnacle.input.v1.GetBindInfosRequest"
pinnacle.input.v1.InputService.GetBindInfos.response = ".pinnacle.input.v1.GetBindInfosResponse"
pinnacle.input.v1.InputService.SetBindGroup = {}
pinnacle.input.v1.InputService.SetBindGroup.service = "pinnacle.input.v1.InputService"
pinnacle.input.v1.InputService.SetBindGroup.method = "SetBindGroup"
pinnacle.input.v1.InputService.SetBindGroup.request = ".pinnacle.input.v1.SetBindGroupRequest"
pinnacle.input.v1.InputService.SetBindGroup.response = ".google.protobuf.Empty"
pinnacle.input.v1.InputService.SetBindDescription = {}
pinnacle.input.v1.InputService.SetBindDescription.service = "pinnacle.input.v1.InputService"
pinnacle.input.v1.InputService.SetBindDescription.method = "SetBindDescription"
pinnacle.input.v1.InputService.SetBindDescription.request = ".pinnacle.input.v1.SetBindDescriptionRequest"
pinnacle.input.v1.InputService.SetBindDescription.response = ".google.protobuf.Empty"
pinnacle.input.v1.InputService.SetQuitBind = {}
pinnacle.input.v1.InputService.SetQuitBind.service = "pinnacle.input.v1.InputService"
pinnacle.input.v1.InputService.SetQuitBind.method = "SetQuitBind"
pinnacle.input.v1.InputService.SetQuitBind.request = ".pinnacle.input.v1.SetQuitBindRequest"
pinnacle.input.v1.InputService.SetQuitBind.response = ".google.protobuf.Empty"
pinnacle.input.v1.InputService.SetReloadConfigBind = {}
pinnacle.input.v1.InputService.SetReloadConfigBind.service = "pinnacle.input.v1.InputService"
pinnacle.input.v1.InputService.SetReloadConfigBind.method = "SetReloadConfigBind"
pinnacle.input.v1.InputService.SetReloadConfigBind.request = ".pinnacle.input.v1.SetReloadConfigBindRequest"
pinnacle.input.v1.InputService.SetReloadConfigBind.response = ".google.protobuf.Empty"
pinnacle.input.v1.InputService.GetBindLayerStack = {}
pinnacle.input.v1.InputService.GetBindLayerStack.service = "pinnacle.input.v1.InputService"
pinnacle.input.v1.InputService.GetBindLayerStack.method = "GetBindLayerStack"
pinnacle.input.v1.InputService.GetBindLayerStack.request = ".pinnacle.input.v1.GetBindLayerStackRequest"
pinnacle.input.v1.InputService.GetBindLayerStack.response = ".pinnacle.input.v1.GetBindLayerStackResponse"
pinnacle.input.v1.InputService.EnterBindLayer = {}
pinnacle.input.v1.InputService.EnterBindLayer.service = "pinnacle.input.v1.InputService"
pinnacle.input.v1.InputService.EnterBindLayer.method = "EnterBindLayer"
pinnacle.input.v1.InputService.EnterBindLayer.request = ".pinnacle.input.v1.EnterBindLayerRequest"
pinnacle.input.v1.InputService.EnterBindLayer.response = ".google.protobuf.Empty"
pinnacle.input.v1.InputService.KeybindStream = {}
pinnacle.input.v1.InputService.KeybindStream.service = "pinnacle.input.v1.InputService"
pinnacle.input.v1.InputService.KeybindStream.method = "KeybindStream"
pinnacle.input.v1.InputService.KeybindStream.request = ".pinnacle.input.v1.KeybindStreamRequest"
pinnacle.input.v1.InputService.KeybindStream.response = ".pinnacle.input.v1.KeybindStreamResponse"
pinnacle.input.v1.InputService.MousebindStream = {}
pinnacle.input.v1.InputService.MousebindStream.service = "pinnacle.input.v1.InputService"
pinnacle.input.v1.InputService.MousebindStream.method = "MousebindStream"
pinnacle.input.v1.InputService.MousebindStream.request = ".pinnacle.input.v1.MousebindStreamRequest"
pinnacle.input.v1.InputService.MousebindStream.response = ".pinnacle.input.v1.MousebindStreamResponse"
pinnacle.input.v1.InputService.KeybindOnPress = {}
pinnacle.input.v1.InputService.KeybindOnPress.service = "pinnacle.input.v1.InputService"
pinnacle.input.v1.InputService.KeybindOnPress.method = "KeybindOnPress"
pinnacle.input.v1.InputService.KeybindOnPress.request = ".pinnacle.input.v1.KeybindOnPressRequest"
pinnacle.input.v1.InputService.KeybindOnPress.response = ".google.protobuf.Empty"
pinnacle.input.v1.InputService.MousebindOnPress = {}
pinnacle.input.v1.InputService.MousebindOnPress.service = "pinnacle.input.v1.InputService"
pinnacle.input.v1.InputService.MousebindOnPress.method = "MousebindOnPress"
pinnacle.input.v1.InputService.MousebindOnPress.request = ".pinnacle.input.v1.MousebindOnPressRequest"
pinnacle.input.v1.InputService.MousebindOnPress.response = ".google.protobuf.Empty"
pinnacle.input.v1.InputService.SetXkbConfig = {}
pinnacle.input.v1.InputService.SetXkbConfig.service = "pinnacle.input.v1.InputService"
pinnacle.input.v1.InputService.SetXkbConfig.method = "SetXkbConfig"
pinnacle.input.v1.InputService.SetXkbConfig.request = ".pinnacle.input.v1.SetXkbConfigRequest"
pinnacle.input.v1.InputService.SetXkbConfig.response = ".google.protobuf.Empty"
pinnacle.input.v1.InputService.SetRepeatRate = {}
pinnacle.input.v1.InputService.SetRepeatRate.service = "pinnacle.input.v1.InputService"
pinnacle.input.v1.InputService.SetRepeatRate.method = "SetRepeatRate"
pinnacle.input.v1.InputService.SetRepeatRate.request = ".pinnacle.input.v1.SetRepeatRateRequest"
pinnacle.input.v1.InputService.SetRepeatRate.response = ".google.protobuf.Empty"
pinnacle.input.v1.InputService.SetXcursor = {}
pinnacle.input.v1.InputService.SetXcursor.service = "pinnacle.input.v1.InputService"
pinnacle.input.v1.InputService.SetXcursor.method = "SetXcursor"
pinnacle.input.v1.InputService.SetXcursor.request = ".pinnacle.input.v1.SetXcursorRequest"
pinnacle.input.v1.InputService.SetXcursor.response = ".google.protobuf.Empty"
pinnacle.input.v1.InputService.GetDevices = {}
pinnacle.input.v1.InputService.GetDevices.service = "pinnacle.input.v1.InputService"
pinnacle.input.v1.InputService.GetDevices.method = "GetDevices"
pinnacle.input.v1.InputService.GetDevices.request = ".pinnacle.input.v1.GetDevicesRequest"
pinnacle.input.v1.InputService.GetDevices.response = ".pinnacle.input.v1.GetDevicesResponse"
pinnacle.input.v1.InputService.GetDeviceCapabilities = {}
pinnacle.input.v1.InputService.GetDeviceCapabilities.service = "pinnacle.input.v1.InputService"
pinnacle.input.v1.InputService.GetDeviceCapabilities.method = "GetDeviceCapabilities"
pinnacle.input.v1.InputService.GetDeviceCapabilities.request = ".pinnacle.input.v1.GetDeviceCapabilitiesRequest"
pinnacle.input.v1.InputService.GetDeviceCapabilities.response = ".pinnacle.input.v1.GetDeviceCapabilitiesResponse"
pinnacle.input.v1.InputService.GetDeviceInfo = {}
pinnacle.input.v1.InputService.GetDeviceInfo.service = "pinnacle.input.v1.InputService"
pinnacle.input.v1.InputService.GetDeviceInfo.method = "GetDeviceInfo"
pinnacle.input.v1.InputService.GetDeviceInfo.request = ".pinnacle.input.v1.GetDeviceInfoRequest"
pinnacle.input.v1.InputService.GetDeviceInfo.response = ".pinnacle.input.v1.GetDeviceInfoResponse"
pinnacle.input.v1.InputService.GetDeviceType = {}
pinnacle.input.v1.InputService.GetDeviceType.service = "pinnacle.input.v1.InputService"
pinnacle.input.v1.InputService.GetDeviceType.method = "GetDeviceType"
pinnacle.input.v1.InputService.GetDeviceType.request = ".pinnacle.input.v1.GetDeviceTypeRequest"
pinnacle.input.v1.InputService.GetDeviceType.response = ".pinnacle.input.v1.GetDeviceTypeResponse"
pinnacle.input.v1.InputService.SetDeviceLibinputSetting = {}
pinnacle.input.v1.InputService.SetDeviceLibinputSetting.service = "pinnacle.input.v1.InputService"
pinnacle.input.v1.InputService.SetDeviceLibinputSetting.method = "SetDeviceLibinputSetting"
pinnacle.input.v1.InputService.SetDeviceLibinputSetting.request = ".pinnacle.input.v1.SetDeviceLibinputSettingRequest"
pinnacle.input.v1.InputService.SetDeviceLibinputSetting.response = ".google.protobuf.Empty"
pinnacle.layout.v1.LayoutService = {}
pinnacle.layout.v1.LayoutService.Layout = {}
pinnacle.layout.v1.LayoutService.Layout.service = "pinnacle.layout.v1.LayoutService"
pinnacle.layout.v1.LayoutService.Layout.method = "Layout"
pinnacle.layout.v1.LayoutService.Layout.request = ".pinnacle.layout.v1.LayoutRequest"
pinnacle.layout.v1.LayoutService.Layout.response = ".pinnacle.layout.v1.LayoutResponse"
pinnacle.tag.v1.TagService = {}
pinnacle.tag.v1.TagService.Get = {}
pinnacle.tag.v1.TagService.Get.service = "pinnacle.tag.v1.TagService"
pinnacle.tag.v1.TagService.Get.method = "Get"
pinnacle.tag.v1.TagService.Get.request = ".pinnacle.tag.v1.GetRequest"
pinnacle.tag.v1.TagService.Get.response = ".pinnacle.tag.v1.GetResponse"
pinnacle.tag.v1.TagService.GetActive = {}
pinnacle.tag.v1.TagService.GetActive.service = "pinnacle.tag.v1.TagService"
pinnacle.tag.v1.TagService.GetActive.method = "GetActive"
pinnacle.tag.v1.TagService.GetActive.request = ".pinnacle.tag.v1.GetActiveRequest"
pinnacle.tag.v1.TagService.GetActive.response = ".pinnacle.tag.v1.GetActiveResponse"
pinnacle.tag.v1.TagService.GetName = {}
pinnacle.tag.v1.TagService.GetName.service = "pinnacle.tag.v1.TagService"
pinnacle.tag.v1.TagService.GetName.method = "GetName"
pinnacle.tag.v1.TagService.GetName.request = ".pinnacle.tag.v1.GetNameRequest"
pinnacle.tag.v1.TagService.GetName.response = ".pinnacle.tag.v1.GetNameResponse"
pinnacle.tag.v1.TagService.GetOutputName = {}
pinnacle.tag.v1.TagService.GetOutputName.service = "pinnacle.tag.v1.TagService"
pinnacle.tag.v1.TagService.GetOutputName.method = "GetOutputName"
pinnacle.tag.v1.TagService.GetOutputName.request = ".pinnacle.tag.v1.GetOutputNameRequest"
pinnacle.tag.v1.TagService.GetOutputName.response = ".pinnacle.tag.v1.GetOutputNameResponse"
pinnacle.tag.v1.TagService.Add = {}
pinnacle.tag.v1.TagService.Add.service = "pinnacle.tag.v1.TagService"
pinnacle.tag.v1.TagService.Add.method = "Add"
pinnacle.tag.v1.TagService.Add.request = ".pinnacle.tag.v1.AddRequest"
pinnacle.tag.v1.TagService.Add.response = ".pinnacle.tag.v1.AddResponse"
pinnacle.tag.v1.TagService.Remove = {}
pinnacle.tag.v1.TagService.Remove.service = "pinnacle.tag.v1.TagService"
pinnacle.tag.v1.TagService.Remove.method = "Remove"
pinnacle.tag.v1.TagService.Remove.request = ".pinnacle.tag.v1.RemoveRequest"
pinnacle.tag.v1.TagService.Remove.response = ".google.protobuf.Empty"
pinnacle.tag.v1.TagService.SetActive = {}
pinnacle.tag.v1.TagService.SetActive.service = "pinnacle.tag.v1.TagService"
pinnacle.tag.v1.TagService.SetActive.method = "SetActive"
pinnacle.tag.v1.TagService.SetActive.request = ".pinnacle.tag.v1.SetActiveRequest"
pinnacle.tag.v1.TagService.SetActive.response = ".google.protobuf.Empty"
pinnacle.tag.v1.TagService.SwitchTo = {}
pinnacle.tag.v1.TagService.SwitchTo.service = "pinnacle.tag.v1.TagService"
pinnacle.tag.v1.TagService.SwitchTo.method = "SwitchTo"
pinnacle.tag.v1.TagService.SwitchTo.request = ".pinnacle.tag.v1.SwitchToRequest"
pinnacle.tag.v1.TagService.SwitchTo.response = ".google.protobuf.Empty"
pinnacle.process.v1.ProcessService = {}
pinnacle.process.v1.ProcessService.Spawn = {}
pinnacle.process.v1.ProcessService.Spawn.service = "pinnacle.process.v1.ProcessService"
pinnacle.process.v1.ProcessService.Spawn.method = "Spawn"
pinnacle.process.v1.ProcessService.Spawn.request = ".pinnacle.process.v1.SpawnRequest"
pinnacle.process.v1.ProcessService.Spawn.response = ".pinnacle.process.v1.SpawnResponse"
pinnacle.process.v1.ProcessService.WaitOnSpawn = {}
pinnacle.process.v1.ProcessService.WaitOnSpawn.service = "pinnacle.process.v1.ProcessService"
pinnacle.process.v1.ProcessService.WaitOnSpawn.method = "WaitOnSpawn"
pinnacle.process.v1.ProcessService.WaitOnSpawn.request = ".pinnacle.process.v1.WaitOnSpawnRequest"
pinnacle.process.v1.ProcessService.WaitOnSpawn.response = ".pinnacle.process.v1.WaitOnSpawnResponse"
pinnacle.window.v1.WindowService = {}
pinnacle.window.v1.WindowService.Get = {}
pinnacle.window.v1.WindowService.Get.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.Get.method = "Get"
pinnacle.window.v1.WindowService.Get.request = ".pinnacle.window.v1.GetRequest"
pinnacle.window.v1.WindowService.Get.response = ".pinnacle.window.v1.GetResponse"
pinnacle.window.v1.WindowService.GetAppId = {}
pinnacle.window.v1.WindowService.GetAppId.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.GetAppId.method = "GetAppId"
pinnacle.window.v1.WindowService.GetAppId.request = ".pinnacle.window.v1.GetAppIdRequest"
pinnacle.window.v1.WindowService.GetAppId.response = ".pinnacle.window.v1.GetAppIdResponse"
pinnacle.window.v1.WindowService.GetTitle = {}
pinnacle.window.v1.WindowService.GetTitle.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.GetTitle.method = "GetTitle"
pinnacle.window.v1.WindowService.GetTitle.request = ".pinnacle.window.v1.GetTitleRequest"
pinnacle.window.v1.WindowService.GetTitle.response = ".pinnacle.window.v1.GetTitleResponse"
pinnacle.window.v1.WindowService.GetLoc = {}
pinnacle.window.v1.WindowService.GetLoc.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.GetLoc.method = "GetLoc"
pinnacle.window.v1.WindowService.GetLoc.request = ".pinnacle.window.v1.GetLocRequest"
pinnacle.window.v1.WindowService.GetLoc.response = ".pinnacle.window.v1.GetLocResponse"
pinnacle.window.v1.WindowService.GetSize = {}
pinnacle.window.v1.WindowService.GetSize.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.GetSize.method = "GetSize"
pinnacle.window.v1.WindowService.GetSize.request = ".pinnacle.window.v1.GetSizeRequest"
pinnacle.window.v1.WindowService.GetSize.response = ".pinnacle.window.v1.GetSizeResponse"
pinnacle.window.v1.WindowService.GetFocused = {}
pinnacle.window.v1.WindowService.GetFocused.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.GetFocused.method = "GetFocused"
pinnacle.window.v1.WindowService.GetFocused.request = ".pinnacle.window.v1.GetFocusedRequest"
pinnacle.window.v1.WindowService.GetFocused.response = ".pinnacle.window.v1.GetFocusedResponse"
pinnacle.window.v1.WindowService.GetLayoutMode = {}
pinnacle.window.v1.WindowService.GetLayoutMode.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.GetLayoutMode.method = "GetLayoutMode"
pinnacle.window.v1.WindowService.GetLayoutMode.request = ".pinnacle.window.v1.GetLayoutModeRequest"
pinnacle.window.v1.WindowService.GetLayoutMode.response = ".pinnacle.window.v1.GetLayoutModeResponse"
pinnacle.window.v1.WindowService.GetTagIds = {}
pinnacle.window.v1.WindowService.GetTagIds.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.GetTagIds.method = "GetTagIds"
pinnacle.window.v1.WindowService.GetTagIds.request = ".pinnacle.window.v1.GetTagIdsRequest"
pinnacle.window.v1.WindowService.GetTagIds.response = ".pinnacle.window.v1.GetTagIdsResponse"
pinnacle.window.v1.WindowService.Close = {}
pinnacle.window.v1.WindowService.Close.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.Close.method = "Close"
pinnacle.window.v1.WindowService.Close.request = ".pinnacle.window.v1.CloseRequest"
pinnacle.window.v1.WindowService.Close.response = ".google.protobuf.Empty"
pinnacle.window.v1.WindowService.SetGeometry = {}
pinnacle.window.v1.WindowService.SetGeometry.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.SetGeometry.method = "SetGeometry"
pinnacle.window.v1.WindowService.SetGeometry.request = ".pinnacle.window.v1.SetGeometryRequest"
pinnacle.window.v1.WindowService.SetGeometry.response = ".google.protobuf.Empty"
pinnacle.window.v1.WindowService.SetFullscreen = {}
pinnacle.window.v1.WindowService.SetFullscreen.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.SetFullscreen.method = "SetFullscreen"
pinnacle.window.v1.WindowService.SetFullscreen.request = ".pinnacle.window.v1.SetFullscreenRequest"
pinnacle.window.v1.WindowService.SetFullscreen.response = ".google.protobuf.Empty"
pinnacle.window.v1.WindowService.SetMaximized = {}
pinnacle.window.v1.WindowService.SetMaximized.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.SetMaximized.method = "SetMaximized"
pinnacle.window.v1.WindowService.SetMaximized.request = ".pinnacle.window.v1.SetMaximizedRequest"
pinnacle.window.v1.WindowService.SetMaximized.response = ".google.protobuf.Empty"
pinnacle.window.v1.WindowService.SetFloating = {}
pinnacle.window.v1.WindowService.SetFloating.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.SetFloating.method = "SetFloating"
pinnacle.window.v1.WindowService.SetFloating.request = ".pinnacle.window.v1.SetFloatingRequest"
pinnacle.window.v1.WindowService.SetFloating.response = ".google.protobuf.Empty"
pinnacle.window.v1.WindowService.SetFocused = {}
pinnacle.window.v1.WindowService.SetFocused.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.SetFocused.method = "SetFocused"
pinnacle.window.v1.WindowService.SetFocused.request = ".pinnacle.window.v1.SetFocusedRequest"
pinnacle.window.v1.WindowService.SetFocused.response = ".google.protobuf.Empty"
pinnacle.window.v1.WindowService.SetDecorationMode = {}
pinnacle.window.v1.WindowService.SetDecorationMode.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.SetDecorationMode.method = "SetDecorationMode"
pinnacle.window.v1.WindowService.SetDecorationMode.request = ".pinnacle.window.v1.SetDecorationModeRequest"
pinnacle.window.v1.WindowService.SetDecorationMode.response = ".google.protobuf.Empty"
pinnacle.window.v1.WindowService.MoveToTag = {}
pinnacle.window.v1.WindowService.MoveToTag.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.MoveToTag.method = "MoveToTag"
pinnacle.window.v1.WindowService.MoveToTag.request = ".pinnacle.window.v1.MoveToTagRequest"
pinnacle.window.v1.WindowService.MoveToTag.response = ".google.protobuf.Empty"
pinnacle.window.v1.WindowService.SetTag = {}
pinnacle.window.v1.WindowService.SetTag.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.SetTag.method = "SetTag"
pinnacle.window.v1.WindowService.SetTag.request = ".pinnacle.window.v1.SetTagRequest"
pinnacle.window.v1.WindowService.SetTag.response = ".google.protobuf.Empty"
pinnacle.window.v1.WindowService.Raise = {}
pinnacle.window.v1.WindowService.Raise.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.Raise.method = "Raise"
pinnacle.window.v1.WindowService.Raise.request = ".pinnacle.window.v1.RaiseRequest"
pinnacle.window.v1.WindowService.Raise.response = ".google.protobuf.Empty"
pinnacle.window.v1.WindowService.MoveGrab = {}
pinnacle.window.v1.WindowService.MoveGrab.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.MoveGrab.method = "MoveGrab"
pinnacle.window.v1.WindowService.MoveGrab.request = ".pinnacle.window.v1.MoveGrabRequest"
pinnacle.window.v1.WindowService.MoveGrab.response = ".google.protobuf.Empty"
pinnacle.window.v1.WindowService.ResizeGrab = {}
pinnacle.window.v1.WindowService.ResizeGrab.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.ResizeGrab.method = "ResizeGrab"
pinnacle.window.v1.WindowService.ResizeGrab.request = ".pinnacle.window.v1.ResizeGrabRequest"
pinnacle.window.v1.WindowService.ResizeGrab.response = ".google.protobuf.Empty"
pinnacle.window.v1.WindowService.WindowRule = {}
pinnacle.window.v1.WindowService.WindowRule.service = "pinnacle.window.v1.WindowService"
pinnacle.window.v1.WindowService.WindowRule.method = "WindowRule"
pinnacle.window.v1.WindowService.WindowRule.request = ".pinnacle.window.v1.WindowRuleRequest"
pinnacle.window.v1.WindowService.WindowRule.response = ".pinnacle.window.v1.WindowRuleResponse"
pinnacle.render.v1.RenderService = {}
pinnacle.render.v1.RenderService.SetUpscaleFilter = {}
pinnacle.render.v1.RenderService.SetUpscaleFilter.service = "pinnacle.render.v1.RenderService"
pinnacle.render.v1.RenderService.SetUpscaleFilter.method = "SetUpscaleFilter"
pinnacle.render.v1.RenderService.SetUpscaleFilter.request = ".pinnacle.render.v1.SetUpscaleFilterRequest"
pinnacle.render.v1.RenderService.SetUpscaleFilter.response = ".google.protobuf.Empty"
pinnacle.render.v1.RenderService.SetDownscaleFilter = {}
pinnacle.render.v1.RenderService.SetDownscaleFilter.service = "pinnacle.render.v1.RenderService"
pinnacle.render.v1.RenderService.SetDownscaleFilter.method = "SetDownscaleFilter"
pinnacle.render.v1.RenderService.SetDownscaleFilter.request = ".pinnacle.render.v1.SetDownscaleFilterRequest"
pinnacle.render.v1.RenderService.SetDownscaleFilter.response = ".google.protobuf.Empty"
pinnacle.output.v1.OutputService = {}
pinnacle.output.v1.OutputService.Get = {}
pinnacle.output.v1.OutputService.Get.service = "pinnacle.output.v1.OutputService"
pinnacle.output.v1.OutputService.Get.method = "Get"
pinnacle.output.v1.OutputService.Get.request = ".pinnacle.output.v1.GetRequest"
pinnacle.output.v1.OutputService.Get.response = ".pinnacle.output.v1.GetResponse"
pinnacle.output.v1.OutputService.SetLoc = {}
pinnacle.output.v1.OutputService.SetLoc.service = "pinnacle.output.v1.OutputService"
pinnacle.output.v1.OutputService.SetLoc.method = "SetLoc"
pinnacle.output.v1.OutputService.SetLoc.request = ".pinnacle.output.v1.SetLocRequest"
pinnacle.output.v1.OutputService.SetLoc.response = ".google.protobuf.Empty"
pinnacle.output.v1.OutputService.SetMode = {}
pinnacle.output.v1.OutputService.SetMode.service = "pinnacle.output.v1.OutputService"
pinnacle.output.v1.OutputService.SetMode.method = "SetMode"
pinnacle.output.v1.OutputService.SetMode.request = ".pinnacle.output.v1.SetModeRequest"
pinnacle.output.v1.OutputService.SetMode.response = ".google.protobuf.Empty"
pinnacle.output.v1.OutputService.SetModeline = {}
pinnacle.output.v1.OutputService.SetModeline.service = "pinnacle.output.v1.OutputService"
pinnacle.output.v1.OutputService.SetModeline.method = "SetModeline"
pinnacle.output.v1.OutputService.SetModeline.request = ".pinnacle.output.v1.SetModelineRequest"
pinnacle.output.v1.OutputService.SetModeline.response = ".google.protobuf.Empty"
pinnacle.output.v1.OutputService.SetScale = {}
pinnacle.output.v1.OutputService.SetScale.service = "pinnacle.output.v1.OutputService"
pinnacle.output.v1.OutputService.SetScale.method = "SetScale"
pinnacle.output.v1.OutputService.SetScale.request = ".pinnacle.output.v1.SetScaleRequest"
pinnacle.output.v1.OutputService.SetScale.response = ".google.protobuf.Empty"
pinnacle.output.v1.OutputService.SetTransform = {}
pinnacle.output.v1.OutputService.SetTransform.service = "pinnacle.output.v1.OutputService"
pinnacle.output.v1.OutputService.SetTransform.method = "SetTransform"
pinnacle.output.v1.OutputService.SetTransform.request = ".pinnacle.output.v1.SetTransformRequest"
pinnacle.output.v1.OutputService.SetTransform.response = ".google.protobuf.Empty"
pinnacle.output.v1.OutputService.SetPowered = {}
pinnacle.output.v1.OutputService.SetPowered.service = "pinnacle.output.v1.OutputService"
pinnacle.output.v1.OutputService.SetPowered.method = "SetPowered"
pinnacle.output.v1.OutputService.SetPowered.request = ".pinnacle.output.v1.SetPoweredRequest"
pinnacle.output.v1.OutputService.SetPowered.response = ".google.protobuf.Empty"
pinnacle.output.v1.OutputService.GetInfo = {}
pinnacle.output.v1.OutputService.GetInfo.service = "pinnacle.output.v1.OutputService"
pinnacle.output.v1.OutputService.GetInfo.method = "GetInfo"
pinnacle.output.v1.OutputService.GetInfo.request = ".pinnacle.output.v1.GetInfoRequest"
pinnacle.output.v1.OutputService.GetInfo.response = ".pinnacle.output.v1.GetInfoResponse"
pinnacle.output.v1.OutputService.GetLoc = {}
pinnacle.output.v1.OutputService.GetLoc.service = "pinnacle.output.v1.OutputService"
pinnacle.output.v1.OutputService.GetLoc.method = "GetLoc"
pinnacle.output.v1.OutputService.GetLoc.request = ".pinnacle.output.v1.GetLocRequest"
pinnacle.output.v1.OutputService.GetLoc.response = ".pinnacle.output.v1.GetLocResponse"
pinnacle.output.v1.OutputService.GetLogicalSize = {}
pinnacle.output.v1.OutputService.GetLogicalSize.service = "pinnacle.output.v1.OutputService"
pinnacle.output.v1.OutputService.GetLogicalSize.method = "GetLogicalSize"
pinnacle.output.v1.OutputService.GetLogicalSize.request = ".pinnacle.output.v1.GetLogicalSizeRequest"
pinnacle.output.v1.OutputService.GetLogicalSize.response = ".pinnacle.output.v1.GetLogicalSizeResponse"
pinnacle.output.v1.OutputService.GetPhysicalSize = {}
pinnacle.output.v1.OutputService.GetPhysicalSize.service = "pinnacle.output.v1.OutputService"
pinnacle.output.v1.OutputService.GetPhysicalSize.method = "GetPhysicalSize"
pinnacle.output.v1.OutputService.GetPhysicalSize.request = ".pinnacle.output.v1.GetPhysicalSizeRequest"
pinnacle.output.v1.OutputService.GetPhysicalSize.response = ".pinnacle.output.v1.GetPhysicalSizeResponse"
pinnacle.output.v1.OutputService.GetModes = {}
pinnacle.output.v1.OutputService.GetModes.service = "pinnacle.output.v1.OutputService"
pinnacle.output.v1.OutputService.GetModes.method = "GetModes"
pinnacle.output.v1.OutputService.GetModes.request = ".pinnacle.output.v1.GetModesRequest"
pinnacle.output.v1.OutputService.GetModes.response = ".pinnacle.output.v1.GetModesResponse"
pinnacle.output.v1.OutputService.GetFocused = {}
pinnacle.output.v1.OutputService.GetFocused.service = "pinnacle.output.v1.OutputService"
pinnacle.output.v1.OutputService.GetFocused.method = "GetFocused"
pinnacle.output.v1.OutputService.GetFocused.request = ".pinnacle.output.v1.GetFocusedRequest"
pinnacle.output.v1.OutputService.GetFocused.response = ".pinnacle.output.v1.GetFocusedResponse"
pinnacle.output.v1.OutputService.GetTagIds = {}
pinnacle.output.v1.OutputService.GetTagIds.service = "pinnacle.output.v1.OutputService"
pinnacle.output.v1.OutputService.GetTagIds.method = "GetTagIds"
pinnacle.output.v1.OutputService.GetTagIds.request = ".pinnacle.output.v1.GetTagIdsRequest"
pinnacle.output.v1.OutputService.GetTagIds.response = ".pinnacle.output.v1.GetTagIdsResponse"
pinnacle.output.v1.OutputService.GetScale = {}
pinnacle.output.v1.OutputService.GetScale.service = "pinnacle.output.v1.OutputService"
pinnacle.output.v1.OutputService.GetScale.method = "GetScale"
pinnacle.output.v1.OutputService.GetScale.request = ".pinnacle.output.v1.GetScaleRequest"
pinnacle.output.v1.OutputService.GetScale.response = ".pinnacle.output.v1.GetScaleResponse"
pinnacle.output.v1.OutputService.GetTransform = {}
pinnacle.output.v1.OutputService.GetTransform.service = "pinnacle.output.v1.OutputService"
pinnacle.output.v1.OutputService.GetTransform.method = "GetTransform"
pinnacle.output.v1.OutputService.GetTransform.request = ".pinnacle.output.v1.GetTransformRequest"
pinnacle.output.v1.OutputService.GetTransform.response = ".pinnacle.output.v1.GetTransformResponse"
pinnacle.output.v1.OutputService.GetEnabled = {}
pinnacle.output.v1.OutputService.GetEnabled.service = "pinnacle.output.v1.OutputService"
pinnacle.output.v1.OutputService.GetEnabled.method = "GetEnabled"
pinnacle.output.v1.OutputService.GetEnabled.request = ".pinnacle.output.v1.GetEnabledRequest"
pinnacle.output.v1.OutputService.GetEnabled.response = ".pinnacle.output.v1.GetEnabledResponse"
pinnacle.output.v1.OutputService.GetPowered = {}
pinnacle.output.v1.OutputService.GetPowered.service = "pinnacle.output.v1.OutputService"
pinnacle.output.v1.OutputService.GetPowered.method = "GetPowered"
pinnacle.output.v1.OutputService.GetPowered.request = ".pinnacle.output.v1.GetPoweredRequest"
pinnacle.output.v1.OutputService.GetPowered.response = ".pinnacle.output.v1.GetPoweredResponse"
pinnacle.output.v1.OutputService.GetFocusStackWindowIds = {}
pinnacle.output.v1.OutputService.GetFocusStackWindowIds.service = "pinnacle.output.v1.OutputService"
pinnacle.output.v1.OutputService.GetFocusStackWindowIds.method = "GetFocusStackWindowIds"
pinnacle.output.v1.OutputService.GetFocusStackWindowIds.request = ".pinnacle.output.v1.GetFocusStackWindowIdsRequest"
pinnacle.output.v1.OutputService.GetFocusStackWindowIds.response = ".pinnacle.output.v1.GetFocusStackWindowIdsResponse"
pinnacle.v1.PinnacleService = {}
pinnacle.v1.PinnacleService.Quit = {}
pinnacle.v1.PinnacleService.Quit.service = "pinnacle.v1.PinnacleService"
pinnacle.v1.PinnacleService.Quit.method = "Quit"
pinnacle.v1.PinnacleService.Quit.request = ".pinnacle.v1.QuitRequest"
pinnacle.v1.PinnacleService.Quit.response = ".google.protobuf.Empty"
pinnacle.v1.PinnacleService.ReloadConfig = {}
pinnacle.v1.PinnacleService.ReloadConfig.service = "pinnacle.v1.PinnacleService"
pinnacle.v1.PinnacleService.ReloadConfig.method = "ReloadConfig"
pinnacle.v1.PinnacleService.ReloadConfig.request = ".pinnacle.v1.ReloadConfigRequest"
pinnacle.v1.PinnacleService.ReloadConfig.response = ".google.protobuf.Empty"
pinnacle.v1.PinnacleService.Keepalive = {}
pinnacle.v1.PinnacleService.Keepalive.service = "pinnacle.v1.PinnacleService"
pinnacle.v1.PinnacleService.Keepalive.method = "Keepalive"
pinnacle.v1.PinnacleService.Keepalive.request = ".pinnacle.v1.KeepaliveRequest"
pinnacle.v1.PinnacleService.Keepalive.response = ".pinnacle.v1.KeepaliveResponse"
pinnacle.v1.PinnacleService.Backend = {}
pinnacle.v1.PinnacleService.Backend.service = "pinnacle.v1.PinnacleService"
pinnacle.v1.PinnacleService.Backend.method = "Backend"
pinnacle.v1.PinnacleService.Backend.request = ".pinnacle.v1.BackendRequest"
pinnacle.v1.PinnacleService.Backend.response = ".pinnacle.v1.BackendResponse"

return {
    google = google,
    pinnacle = pinnacle,
}

