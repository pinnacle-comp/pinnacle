---@enum snowcap.widget.v0alpha1.Alignment
local snowcap_widget_v0alpha1_Alignment = {
    ALIGNMENT_UNSPECIFIED = 0,
    ALIGNMENT_START = 1,
    ALIGNMENT_CENTER = 2,
    ALIGNMENT_END = 3,
}

---@enum snowcap.widget.v0alpha1.ScrollableAlignment
local snowcap_widget_v0alpha1_ScrollableAlignment = {
    SCROLLABLE_ALIGNMENT_UNSPECIFIED = 0,
    SCROLLABLE_ALIGNMENT_START = 1,
    SCROLLABLE_ALIGNMENT_END = 2,
}

---@enum snowcap.widget.v0alpha1.Font.Weight
local snowcap_widget_v0alpha1_Font_Weight = {
    WEIGHT_UNSPECIFIED = 0,
    WEIGHT_THIN = 1,
    WEIGHT_EXTRA_LIGHT = 2,
    WEIGHT_LIGHT = 3,
    WEIGHT_NORMAL = 4,
    WEIGHT_MEDIUM = 5,
    WEIGHT_SEMIBOLD = 6,
    WEIGHT_BOLD = 7,
    WEIGHT_EXTRA_BOLD = 8,
    WEIGHT_BLACK = 9,
}

---@enum snowcap.widget.v0alpha1.Font.Stretch
local snowcap_widget_v0alpha1_Font_Stretch = {
    STRETCH_UNSPECIFIED = 0,
    STRETCH_ULTRA_CONDENSED = 1,
    STRETCH_EXTRA_CONDENSED = 2,
    STRETCH_CONDENSED = 3,
    STRETCH_SEMI_CONDENSED = 4,
    STRETCH_NORMAL = 5,
    STRETCH_SEMI_EXPANDED = 6,
    STRETCH_EXPANDED = 7,
    STRETCH_EXTRA_EXPANDED = 8,
    STRETCH_ULTRA_EXPANDED = 9,
}

---@enum snowcap.widget.v0alpha1.Font.Style
local snowcap_widget_v0alpha1_Font_Style = {
    STYLE_UNSPECIFIED = 0,
    STYLE_NORMAL = 1,
    STYLE_ITALIC = 2,
    STYLE_OBLIQUE = 3,
}

---@enum snowcap.layer.v0alpha1.Anchor
local snowcap_layer_v0alpha1_Anchor = {
    ANCHOR_UNSPECIFIED = 0,
    ANCHOR_TOP = 1,
    ANCHOR_BOTTOM = 2,
    ANCHOR_LEFT = 3,
    ANCHOR_RIGHT = 4,
    ANCHOR_TOP_LEFT = 5,
    ANCHOR_TOP_RIGHT = 6,
    ANCHOR_BOTTOM_LEFT = 7,
    ANCHOR_BOTTOM_RIGHT = 8,
}

---@enum snowcap.layer.v0alpha1.KeyboardInteractivity
local snowcap_layer_v0alpha1_KeyboardInteractivity = {
    KEYBOARD_INTERACTIVITY_UNSPECIFIED = 0,
    KEYBOARD_INTERACTIVITY_NONE = 1,
    KEYBOARD_INTERACTIVITY_ON_DEMAND = 2,
    KEYBOARD_INTERACTIVITY_EXCLUSIVE = 3,
}

---@enum snowcap.layer.v0alpha1.Layer
local snowcap_layer_v0alpha1_Layer = {
    LAYER_UNSPECIFIED = 0,
    LAYER_BACKGROUND = 1,
    LAYER_BOTTOM = 2,
    LAYER_TOP = 3,
    LAYER_OVERLAY = 4,
}


---@class snowcap.input.v0alpha1.Modifiers
---@field shift boolean?
---@field ctrl boolean?
---@field alt boolean?
---@field super boolean?

---@class snowcap.input.v0alpha1.KeyboardKeyRequest
---@field id integer?

---@class snowcap.input.v0alpha1.KeyboardKeyResponse
---@field key integer?
---@field modifiers snowcap.input.v0alpha1.Modifiers?
---@field pressed boolean?

---@class snowcap.input.v0alpha1.PointerButtonRequest
---@field id integer?

---@class snowcap.input.v0alpha1.PointerButtonResponse
---@field button integer?
---@field pressed boolean?

---@class google.protobuf.Empty

---@class snowcap.widget.v0alpha1.Padding
---@field top number?
---@field right number?
---@field bottom number?
---@field left number?

---@class snowcap.widget.v0alpha1.Length
---@field fill google.protobuf.Empty?
---@field fill_portion integer?
---@field shrink google.protobuf.Empty?
---@field fixed number?

---@class snowcap.widget.v0alpha1.Color
---@field red number?
---@field green number?
---@field blue number?
---@field alpha number?

---@class snowcap.widget.v0alpha1.Font
---@field family snowcap.widget.v0alpha1.Font.Family?
---@field weight snowcap.widget.v0alpha1.Font.Weight?
---@field stretch snowcap.widget.v0alpha1.Font.Stretch?
---@field style snowcap.widget.v0alpha1.Font.Style?

---@class snowcap.widget.v0alpha1.Font.Family
---@field name string?
---@field serif google.protobuf.Empty?
---@field sans_serif google.protobuf.Empty?
---@field cursive google.protobuf.Empty?
---@field fantasy google.protobuf.Empty?
---@field monospace google.protobuf.Empty?

---@class snowcap.widget.v0alpha1.WidgetDef
---@field text snowcap.widget.v0alpha1.Text?
---@field column snowcap.widget.v0alpha1.Column?
---@field row snowcap.widget.v0alpha1.Row?
---@field scrollable snowcap.widget.v0alpha1.Scrollable?
---@field container snowcap.widget.v0alpha1.Container?

---@class snowcap.widget.v0alpha1.Text
---@field text string?
---@field pixels number?
---@field width snowcap.widget.v0alpha1.Length?
---@field height snowcap.widget.v0alpha1.Length?
---@field horizontal_alignment snowcap.widget.v0alpha1.Alignment?
---@field vertical_alignment snowcap.widget.v0alpha1.Alignment?
---@field color snowcap.widget.v0alpha1.Color?
---@field font snowcap.widget.v0alpha1.Font?

---@class snowcap.widget.v0alpha1.Column
---@field spacing number?
---@field padding snowcap.widget.v0alpha1.Padding?
---@field item_alignment snowcap.widget.v0alpha1.Alignment?
---@field width snowcap.widget.v0alpha1.Length?
---@field height snowcap.widget.v0alpha1.Length?
---@field max_width number?
---@field clip boolean?
---@field children snowcap.widget.v0alpha1.WidgetDef[]?

---@class snowcap.widget.v0alpha1.Row
---@field spacing number?
---@field padding snowcap.widget.v0alpha1.Padding?
---@field item_alignment snowcap.widget.v0alpha1.Alignment?
---@field width snowcap.widget.v0alpha1.Length?
---@field height snowcap.widget.v0alpha1.Length?
---@field clip boolean?
---@field children snowcap.widget.v0alpha1.WidgetDef[]?

---@class snowcap.widget.v0alpha1.ScrollableDirection
---@field vertical snowcap.widget.v0alpha1.ScrollableProperties?
---@field horizontal snowcap.widget.v0alpha1.ScrollableProperties?

---@class snowcap.widget.v0alpha1.ScrollableProperties
---@field width number?
---@field margin number?
---@field scroller_width number?
---@field alignment snowcap.widget.v0alpha1.ScrollableAlignment?

---@class snowcap.widget.v0alpha1.Scrollable
---@field width snowcap.widget.v0alpha1.Length?
---@field height snowcap.widget.v0alpha1.Length?
---@field direction snowcap.widget.v0alpha1.ScrollableDirection?
---@field child snowcap.widget.v0alpha1.WidgetDef?

---@class snowcap.widget.v0alpha1.Container
---@field padding snowcap.widget.v0alpha1.Padding?
---@field width snowcap.widget.v0alpha1.Length?
---@field height snowcap.widget.v0alpha1.Length?
---@field max_width number?
---@field max_height number?
---@field horizontal_alignment snowcap.widget.v0alpha1.Alignment?
---@field vertical_alignment snowcap.widget.v0alpha1.Alignment?
---@field clip boolean?
---@field child snowcap.widget.v0alpha1.WidgetDef?
---@field text_color snowcap.widget.v0alpha1.Color?
---@field background_color snowcap.widget.v0alpha1.Color?
---@field border_radius number?
---@field border_thickness number?
---@field border_color snowcap.widget.v0alpha1.Color?

---@class snowcap.layer.v0alpha1.NewLayerRequest
---@field widget_def snowcap.widget.v0alpha1.WidgetDef?
---@field width integer?
---@field height integer?
---@field anchor snowcap.layer.v0alpha1.Anchor?
---@field keyboard_interactivity snowcap.layer.v0alpha1.KeyboardInteractivity?
---@field exclusive_zone integer?
---@field layer snowcap.layer.v0alpha1.Layer?

---@class snowcap.layer.v0alpha1.NewLayerResponse
---@field layer_id integer?

---@class snowcap.layer.v0alpha1.CloseRequest
---@field layer_id integer?

---@class snowcap.v0alpha1.Nothing


local snowcap = {}
snowcap.input = {}
snowcap.input.v0alpha1 = {}
snowcap.input.v0alpha1.Modifiers = {}
snowcap.input.v0alpha1.KeyboardKeyRequest = {}
snowcap.input.v0alpha1.KeyboardKeyResponse = {}
snowcap.input.v0alpha1.PointerButtonRequest = {}
snowcap.input.v0alpha1.PointerButtonResponse = {}
local google = {}
google.protobuf = {}
google.protobuf.Empty = {}
snowcap.widget = {}
snowcap.widget.v0alpha1 = {}
snowcap.widget.v0alpha1.Padding = {}
snowcap.widget.v0alpha1.Length = {}
snowcap.widget.v0alpha1.Color = {}
snowcap.widget.v0alpha1.Font = {}
snowcap.widget.v0alpha1.Font.Family = {}
snowcap.widget.v0alpha1.WidgetDef = {}
snowcap.widget.v0alpha1.Text = {}
snowcap.widget.v0alpha1.Column = {}
snowcap.widget.v0alpha1.Row = {}
snowcap.widget.v0alpha1.ScrollableDirection = {}
snowcap.widget.v0alpha1.ScrollableProperties = {}
snowcap.widget.v0alpha1.Scrollable = {}
snowcap.widget.v0alpha1.Container = {}
snowcap.layer = {}
snowcap.layer.v0alpha1 = {}
snowcap.layer.v0alpha1.NewLayerRequest = {}
snowcap.layer.v0alpha1.NewLayerResponse = {}
snowcap.layer.v0alpha1.CloseRequest = {}
snowcap.v0alpha1 = {}
snowcap.v0alpha1.Nothing = {}

snowcap.widget.v0alpha1.Alignment = snowcap_widget_v0alpha1_Alignment
snowcap.widget.v0alpha1.ScrollableAlignment = snowcap_widget_v0alpha1_ScrollableAlignment
snowcap.widget.v0alpha1.Font.Weight = snowcap_widget_v0alpha1_Font_Weight
snowcap.widget.v0alpha1.Font.Stretch = snowcap_widget_v0alpha1_Font_Stretch
snowcap.widget.v0alpha1.Font.Style = snowcap_widget_v0alpha1_Font_Style
snowcap.layer.v0alpha1.Anchor = snowcap_layer_v0alpha1_Anchor
snowcap.layer.v0alpha1.KeyboardInteractivity = snowcap_layer_v0alpha1_KeyboardInteractivity
snowcap.layer.v0alpha1.Layer = snowcap_layer_v0alpha1_Layer

snowcap.input.v0alpha1.InputService = {}
snowcap.input.v0alpha1.InputService.KeyboardKey = {}
snowcap.input.v0alpha1.InputService.KeyboardKey.service = "snowcap.input.v0alpha1.InputService"
snowcap.input.v0alpha1.InputService.KeyboardKey.method = "KeyboardKey"
snowcap.input.v0alpha1.InputService.KeyboardKey.request = ".snowcap.input.v0alpha1.KeyboardKeyRequest"
snowcap.input.v0alpha1.InputService.KeyboardKey.response = ".snowcap.input.v0alpha1.KeyboardKeyResponse"
snowcap.input.v0alpha1.InputService.PointerButton = {}
snowcap.input.v0alpha1.InputService.PointerButton.service = "snowcap.input.v0alpha1.InputService"
snowcap.input.v0alpha1.InputService.PointerButton.method = "PointerButton"
snowcap.input.v0alpha1.InputService.PointerButton.request = ".snowcap.input.v0alpha1.PointerButtonRequest"
snowcap.input.v0alpha1.InputService.PointerButton.response = ".snowcap.input.v0alpha1.PointerButtonResponse"
snowcap.layer.v0alpha1.LayerService = {}
snowcap.layer.v0alpha1.LayerService.NewLayer = {}
snowcap.layer.v0alpha1.LayerService.NewLayer.service = "snowcap.layer.v0alpha1.LayerService"
snowcap.layer.v0alpha1.LayerService.NewLayer.method = "NewLayer"
snowcap.layer.v0alpha1.LayerService.NewLayer.request = ".snowcap.layer.v0alpha1.NewLayerRequest"
snowcap.layer.v0alpha1.LayerService.NewLayer.response = ".snowcap.layer.v0alpha1.NewLayerResponse"
snowcap.layer.v0alpha1.LayerService.Close = {}
snowcap.layer.v0alpha1.LayerService.Close.service = "snowcap.layer.v0alpha1.LayerService"
snowcap.layer.v0alpha1.LayerService.Close.method = "Close"
snowcap.layer.v0alpha1.LayerService.Close.request = ".snowcap.layer.v0alpha1.CloseRequest"
snowcap.layer.v0alpha1.LayerService.Close.response = ".google.protobuf.Empty"

return {
    snowcap = snowcap,
    google = google,
}

