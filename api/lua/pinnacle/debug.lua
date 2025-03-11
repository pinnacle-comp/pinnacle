-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local client = require("pinnacle.grpc.client").client
local util_v1 = require("pinnacle.grpc.defs").pinnacle.util.v1

---Debugging utilities.
---
---> [!WARNING]
---> This module is not governed by the API stability guarantees.
---
---@class pinnacle.debug
local debug = {}

---Sets damage visualization.
---
---When on, parts of the screen that are damaged after rendering will have
---red rectangles drawn where the damage is.
---
---@param set boolean
function debug.set_damage_visualization(set)
    local _, err = client:pinnacle_debug_v1_DebugService_SetDamageVisualization({
        set_or_toggle = set and util_v1.SetOrToggle.SET_OR_TOGGLE_SET
            or util_v1.SetOrToggle.SET_OR_TOGGLE_UNSET,
    })
end

---Toggles damage visualization.
---
---When on, parts of the screen that are damaged after rendering will have
---red rectangles drawn where the damage is.
function debug.toggle_damage_visualization()
    local _, err = client:pinnacle_debug_v1_DebugService_SetDamageVisualization({
        set_or_toggle = util_v1.SetOrToggle.SET_OR_TOGGLE_TOGGLE,
    })
end

---Sets opaque region visualization.
---
---When on, parts of the screen that are opaque will have a transparent blue rectangle
---drawn over it, while parts that are not opaque will have a transparent red rectangle
---drawn.
---
---@param set boolean
function debug.set_opaque_region_visualization(set)
    local _, err = client:pinnacle_debug_v1_DebugService_SetOpaqueRegionVisualization({
        set_or_toggle = set and util_v1.SetOrToggle.SET_OR_TOGGLE_SET
            or util_v1.SetOrToggle.SET_OR_TOGGLE_UNSET,
    })
end

---Toggles opaque region visualization.
---
---When on, parts of the screen that are opaque will have a transparent blue rectangle
---drawn over it, while parts that are not opaque will have a transparent red rectangle
---drawn.
function debug.toggle_opaque_region_visualization()
    local _, err = client:pinnacle_debug_v1_DebugService_SetOpaqueRegionVisualization({
        set_or_toggle = util_v1.SetOrToggle.SET_OR_TOGGLE_TOGGLE,
    })
end

return debug
