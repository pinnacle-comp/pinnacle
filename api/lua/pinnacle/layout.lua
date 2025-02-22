-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

local client = require("pinnacle.grpc.client").client
local protobuf = require("pinnacle.grpc.protobuf")
local layout_service = require("pinnacle.grpc.defs").pinnacle.layout.v1.LayoutService
local defs = require("pinnacle.grpc.defs")
local log = require("pinnacle.log")

---@class pinnacle.layout.LayoutArgs
---@field output pinnacle.output.OutputHandle
---@field window_count integer
---@field tags pinnacle.tag.TagHandle[]

---@alias pinnacle.layout.LayoutDir
---| "row"
---| "column"

---@alias pinnacle.layout.Gaps
---| { left: number, right: number, top: number, bottom: number }
---| number

---@class pinnacle.layout.LayoutNode
---A label that helps Pinnacle decide how to diff layout trees.
---@field label string?
---An index that determines how Pinnacle traverses a layout tree.
---@field traversal_index integer?
---A set of indices per window index that changes how that window is assigned a geometry.
---@field traversal_overrides table<integer, integer[]>?
---@field layout_dir pinnacle.layout.LayoutDir?
---@field gaps (number | pinnacle.layout.Gaps)?
---The proportion the node takes up relative to its siblings.
---@field size_proportion number?
---@field children pinnacle.layout.LayoutNode[]

---A layout generator.
---@class pinnacle.layout.LayoutGenerator
---Generate an array of geometries from the given `LayoutArgs`.
---@field layout fun(self: self, window_count: integer): pinnacle.layout.LayoutNode

---Builtin layout generators.
---
---This contains functions that create various builtin generators.
---@class pinnacle.layout.builtin
local builtin = {}

---@class pinnacle.layout.builtin.Line : pinnacle.layout.LayoutGenerator
---@field outer_gaps pinnacle.layout.Gaps
---@field inner_gaps pinnacle.layout.Gaps
---@field direction pinnacle.layout.LayoutDir
---@field reversed boolean

---@class pinnacle.layout.builtin.LineOpts
---@field outer_gaps pinnacle.layout.Gaps?
---@field inner_gaps pinnacle.layout.Gaps?
---@field direction pinnacle.layout.LayoutDir?
---@field reversed boolean?

---Creates a layout generator that lays out windows in a line.
---
---@param options pinnacle.layout.builtin.LineOpts?
---
---@return pinnacle.layout.builtin.Line
function builtin.line(options)
    ---@type pinnacle.layout.builtin.Line
    return {
        outer_gaps = options and options.outer_gaps or 4.0,
        inner_gaps = options and options.inner_gaps or 4.0,
        direction = options and options.direction or "row",
        reversed = options and options.reversed or false,
        ---@param self pinnacle.layout.builtin.Line
        layout = function(self, window_count)
            ---@type pinnacle.layout.LayoutNode
            local root = {
                gaps = self.outer_gaps,
                layout_dir = self.direction,
                label = "builtin.line",
                children = {},
            }

            if window_count == 0 then
                return root
            end

            ---@type pinnacle.layout.LayoutNode[]
            local children = {}
            if not self.reversed then
                for i = 0, window_count - 1 do
                    table.insert(children, {
                        traversal_index = i,
                        gaps = self.inner_gaps,
                        children = {},
                    })
                end
            else
                for i = window_count - 1, 0, -1 do
                    table.insert(children, {
                        traversal_index = i,
                        gaps = self.inner_gaps,
                        children = {},
                    })
                end
            end

            root.children = children

            return root
        end,
    }
end

---@class pinnacle.layout.builtin.MasterStack : pinnacle.layout.LayoutGenerator
---@field outer_gaps pinnacle.layout.Gaps
---@field inner_gaps pinnacle.layout.Gaps
---@field master_factor number
---@field master_side "left" | "right" | "top" | "bottom"
---@field master_count integer
---@field reversed boolean

---@class pinnacle.layout.builtin.MasterStackOpts
---@field outer_gaps pinnacle.layout.Gaps?
---@field inner_gaps pinnacle.layout.Gaps?
---@field master_factor number?
---@field master_side ("left" | "right" | "top" | "bottom")?
---@field master_count integer?
---@field reversed boolean?

---Creates a layout generator that lays windows out in two stacks: a master and side stack.
---
---@param options pinnacle.layout.builtin.MasterStackOpts?
---@return pinnacle.layout.builtin.MasterStack
function builtin.master_stack(options)
    ---@type pinnacle.layout.builtin.MasterStack
    return {
        outer_gaps = options and options.outer_gaps or 4.0,
        inner_gaps = options and options.inner_gaps or 4.0,
        master_factor = options and options.master_factor or 0.5,
        master_side = options and options.master_side or "left",
        master_count = options and options.master_count or 1,
        reversed = options and options.reversed or false,
        ---@param self pinnacle.layout.builtin.MasterStack
        layout = function(self, window_count)
            ---@type pinnacle.layout.LayoutNode
            local root = {
                gaps = self.outer_gaps,
                layout_dir = (self.master_side == "left" or self.master_side == "right") and "row"
                    or "column",
                label = "builtin.master_stack",
                children = {},
            }

            if window_count == 0 then
                return root
            end

            local master_factor = math.min(math.max(0.1, self.master_factor), 0.9)

            local master_tv_idx, stack_tv_idx = 0, 1
            if self.reversed then
                master_tv_idx, stack_tv_idx = 1, 0
            end

            local master_count = math.min(self.master_count, window_count)

            local line = builtin.line({
                outer_gaps = 0.0,
                inner_gaps = self.inner_gaps,
                direction = (self.master_side == "left" or self.master_side == "right")
                        and "column"
                    or "row",
                reversed = self.reversed,
            })

            local master_side = line:layout(master_count)

            master_side.traversal_index = master_tv_idx
            master_side.size_proportion = master_factor * 10.0

            if window_count <= self.master_count then
                root.children = { master_side }
                return root
            end

            local stack_count = window_count - master_count
            local stack_side = line:layout(stack_count)
            stack_side.traversal_index = stack_tv_idx
            stack_side.size_proportion = (1.0 - master_factor) * 10.0

            if self.master_side == "left" or self.master_side == "top" then
                root.children = { master_side, stack_side }
            else
                root.children = { stack_side, master_side }
            end

            return root
        end,
    }
end

---@class pinnacle.layout.builtin.Dwindle : pinnacle.layout.LayoutGenerator
---@field outer_gaps pinnacle.layout.Gaps
---@field inner_gaps pinnacle.layout.Gaps

---@class pinnacle.layout.builtin.DwindleOpts
---@field outer_gaps pinnacle.layout.Gaps?
---@field inner_gaps pinnacle.layout.Gaps?

---Creates a layout generator that lays windows out dwindling down to the bottom right.
---
---@param options pinnacle.layout.builtin.DwindleOpts?
---
---@return pinnacle.layout.builtin.Dwindle
function builtin.dwindle(options)
    ---@type pinnacle.layout.builtin.Dwindle
    return {
        outer_gaps = options and options.outer_gaps or 4.0,
        inner_gaps = options and options.inner_gaps or 4.0,
        ---@param self pinnacle.layout.builtin.Dwindle
        layout = function(self, window_count)
            ---@type pinnacle.layout.LayoutNode
            local root = {
                gaps = self.outer_gaps,
                label = "builtin.dwindle",
                children = {},
            }

            if window_count == 0 then
                return root
            end

            if window_count == 1 then
                ---@type pinnacle.layout.LayoutNode
                local child = {
                    gaps = self.inner_gaps,
                    children = {},
                }
                root.children = { child }
                return root
            end

            local current_node = root

            for i = 0, window_count - 2 do
                if current_node ~= root then
                    current_node.label = "builtin.dwindle.split"
                    current_node.gaps = 0.0
                end

                ---@type pinnacle.layout.LayoutNode
                local child1 = {
                    traversal_index = 0,
                    layout_dir = (i % 2 == 0) and "column" or "row",
                    gaps = self.inner_gaps,
                    children = {},
                }

                ---@type pinnacle.layout.LayoutNode
                local child2 = {
                    traversal_index = 1,
                    layout_dir = (i % 2 == 0) and "column" or "row",
                    gaps = self.inner_gaps,
                    children = {},
                }

                current_node.children = { child1, child2 }

                current_node = child2
            end

            return root
        end,
    }
end

---@class pinnacle.layout.builtin.Spiral : pinnacle.layout.LayoutGenerator
---@field outer_gaps pinnacle.layout.Gaps
---@field inner_gaps pinnacle.layout.Gaps

---@class pinnacle.layout.builtin.SpiralOpts
---@field outer_gaps pinnacle.layout.Gaps?
---@field inner_gaps pinnacle.layout.Gaps?

---Creates a layout generator that lays windows out in a spiral.
---
---@param options pinnacle.layout.builtin.SpiralOpts?
---
---@return pinnacle.layout.builtin.Spiral
function builtin.spiral(options)
    ---@type pinnacle.layout.builtin.Spiral
    return {
        outer_gaps = options and options.outer_gaps or 4.0,
        inner_gaps = options and options.inner_gaps or 4.0,
        ---@param self pinnacle.layout.builtin.Spiral
        layout = function(self, window_count)
            ---@type pinnacle.layout.LayoutNode
            local root = {
                gaps = self.outer_gaps,
                label = "builtin.spiral",
                children = {},
            }

            if window_count == 0 then
                return root
            end

            if window_count == 1 then
                ---@type pinnacle.layout.LayoutNode
                local child = {
                    gaps = self.inner_gaps,
                    children = {},
                }
                root.children = { child }
                return root
            end

            local current_node = root

            for i = 0, window_count - 2 do
                if current_node ~= root then
                    current_node.label = "builtin.dwindle.split"
                    current_node.gaps = 0.0
                end

                ---@type pinnacle.layout.LayoutNode
                local child1 = {
                    traversal_index = 0,
                    layout_dir = (i % 2 == 0) and "column" or "row",
                    gaps = self.inner_gaps,
                    children = {},
                }

                ---@type pinnacle.layout.LayoutNode
                local child2 = {
                    traversal_index = 1,
                    layout_dir = (i % 2 == 0) and "column" or "row",
                    gaps = self.inner_gaps,
                    children = {},
                }

                current_node.children = { child1, child2 }

                if i % 4 == 0 or i % 4 == 1 then
                    current_node = child2
                else
                    current_node = child1
                end
            end

            return root
        end,
    }
end

---@class pinnacle.layout.builtin.Corner : pinnacle.layout.LayoutGenerator
---@field outer_gaps pinnacle.layout.Gaps
---@field inner_gaps pinnacle.layout.Gaps
---@field corner_width_factor number
---@field corner_height_factor number
---@field corner_loc "top_left" | "top_right" | "bottom_left" | "bottom_right"

---@class pinnacle.layout.builtin.CornerOpts
---@field outer_gaps pinnacle.layout.Gaps?
---@field inner_gaps pinnacle.layout.Gaps?
---@field corner_width_factor number?
---@field corner_height_factor number?
---@field corner_loc ("top_left" | "top_right" | "bottom_left" | "bottom_right")?

---Creates a layout generator that lays windows out with one main corner window and
---a horizontal and vertical stack flanking the other two sides.
---
---@param options pinnacle.layout.builtin.CornerOpts?
---@return pinnacle.layout.builtin.Corner
function builtin.corner(options)
    ---@type pinnacle.layout.builtin.Corner
    return {
        outer_gaps = options and options.outer_gaps or 4.0,
        inner_gaps = options and options.inner_gaps or 4.0,
        corner_width_factor = options and options.corner_width_factor or 0.5,
        corner_height_factor = options and options.corner_height_factor or 0.5,
        corner_loc = options and options.corner_loc or "top_left",
        ---@param self pinnacle.layout.builtin.Corner
        layout = function(self, window_count)
            ---@type pinnacle.layout.LayoutNode
            local root = {
                gaps = self.outer_gaps,
                label = "builtin.corner",
                children = {},
            }

            if window_count == 0 then
                return root
            end

            if window_count == 1 then
                ---@type pinnacle.layout.LayoutNode
                local child = {
                    gaps = self.inner_gaps,
                    children = {},
                }
                root.children = { child }
                return root
            end

            local corner_width_factor = math.min(math.max(0.1, self.corner_width_factor), 0.9)
            local corner_height_factor = math.min(math.max(0.1, self.corner_height_factor), 0.9)

            ---@type pinnacle.layout.LayoutNode
            local corner_and_horiz_stack_node = {
                traversal_index = 0,
                label = "builtin.corner.corner_and_stack",
                layout_dir = "column",
                size_proportion = corner_width_factor * 10.0,
                children = {},
            }

            local vert_count = math.ceil((window_count - 1) / 2)
            local horiz_count = math.floor((window_count - 1) / 2)

            local vert_stack = builtin.line({
                outer_gaps = 0.0,
                inner_gaps = self.inner_gaps,
                direction = "column",
                reversed = false,
            })

            local vert_stack_node = vert_stack:layout(vert_count)
            vert_stack_node.size_proportion = (1.0 - corner_width_factor) * 10.0
            vert_stack_node.traversal_index = 1

            if self.corner_loc == "top_left" or self.corner_loc == "bottom_left" then
                root.children = { corner_and_horiz_stack_node, vert_stack_node }
            else
                root.children = { vert_stack_node, corner_and_horiz_stack_node }
            end

            if horiz_count == 0 then
                corner_and_horiz_stack_node.gaps = self.inner_gaps
                return root
            end

            ---@type pinnacle.layout.LayoutNode
            local corner_node = {
                traversal_index = 0,
                size_proportion = corner_height_factor * 10.0,
                gaps = self.inner_gaps,
                children = {},
            }

            local horiz_stack = builtin.line({
                outer_gaps = 0.0,
                inner_gaps = self.inner_gaps,
                direction = "row",
                reversed = false,
            })

            local horiz_stack_node = horiz_stack:layout(horiz_count)
            horiz_stack_node.size_proportion = (1.0 - corner_height_factor) * 10.0
            horiz_stack_node.traversal_index = 1

            if self.corner_loc == "top_left" or self.corner_loc == "top_right" then
                corner_and_horiz_stack_node.children = { corner_node, horiz_stack_node }
            else
                corner_and_horiz_stack_node.children = { horiz_stack_node, corner_node }
            end

            local traversal_overrides = {}
            for i = 0, window_count - 1 do
                traversal_overrides[i] = { i % 2 }
            end

            root.traversal_overrides = traversal_overrides

            return root
        end,
    }
end

---@class pinnacle.layout.builtin.Fair : pinnacle.layout.LayoutGenerator
---@field outer_gaps pinnacle.layout.Gaps
---@field inner_gaps pinnacle.layout.Gaps
---@field axis "horizontal" | "vertical"

---@class pinnacle.layout.builtin.FairOpts
---@field outer_gaps pinnacle.layout.Gaps?
---@field inner_gaps pinnacle.layout.Gaps?
---@field axis ("horizontal" | "vertical")?

---Creates a layout generator that lays windows out keeping their sizes roughly the same.
---
---@param options pinnacle.layout.builtin.FairOpts?
---
---@return pinnacle.layout.builtin.Fair
function builtin.fair(options)
    ---@type pinnacle.layout.builtin.Fair
    return {
        outer_gaps = options and options.outer_gaps or 4.0,
        inner_gaps = options and options.inner_gaps or 4.0,
        axis = options and options.axis or "vertical",
        ---@param self pinnacle.layout.builtin.Fair
        layout = function(self, window_count)
            ---@type pinnacle.layout.LayoutNode
            local root = {
                gaps = self.outer_gaps,
                label = "builtin.fair",
                children = {},
            }

            if window_count == 0 then
                return root
            end

            if window_count == 1 then
                ---@type pinnacle.layout.LayoutNode
                local child = {
                    gaps = self.inner_gaps,
                    children = {},
                }
                root.children = { child }
                return root
            end

            if window_count == 2 then
                ---@type pinnacle.layout.LayoutNode
                local child1 = {
                    gaps = self.inner_gaps,
                    children = {},
                }
                ---@type pinnacle.layout.LayoutNode
                local child2 = {
                    gaps = self.inner_gaps,
                    children = {},
                }
                root.children = { child1, child2 }
                return root
            end

            local line_count = math.floor(math.sqrt(window_count) + 0.5)
            local wins_per_line = {}

            local max_per_line = (window_count > line_count * line_count) and line_count + 1
                or line_count

            for i = 1, window_count do
                local index = math.ceil(i / max_per_line)
                if not wins_per_line[index] then
                    wins_per_line[index] = 0
                end
                wins_per_line[index] = wins_per_line[index] + 1
            end

            local line = builtin.line({
                outer_gaps = 0.0,
                inner_gaps = self.inner_gaps,
                direction = self.axis == "horizontal" and "row" or "column",
                reversed = false,
            })

            local lines = {}
            for i = 1, line_count do
                lines[i] = line:layout(wins_per_line[i])
            end

            root.children = lines

            root.layout_dir = self.axis == "horizontal" and "column" or "row"

            return root
        end,
    }
end

---@class pinnacle.layout.builtin.Cycle : pinnacle.layout.LayoutGenerator
---@field layouts pinnacle.layout.LayoutGenerator[]
---@field private tag_indices table<integer, integer>
---@field current_tag pinnacle.tag.TagHandle?
local Cycle = {}

---Cycles the layout forward for the given tag.
---
---@param tag pinnacle.tag.TagHandle
function Cycle:cycle_layout_forward(tag)
    if not self.tag_indices[tag.id] then
        self.tag_indices[tag.id] = 1
    end
    self.tag_indices[tag.id] = self.tag_indices[tag.id] + 1
    if self.tag_indices[tag.id] > #self.layouts then
        self.tag_indices[tag.id] = 1
    end
end

---Cycles the layout backward for the given tag.
---
---@param tag pinnacle.tag.TagHandle
function Cycle:cycle_layout_backward(tag)
    if not self.tag_indices[tag.id] then
        self.tag_indices[tag.id] = 1
    end
    self.tag_indices[tag.id] = self.tag_indices[tag.id] - 1
    if self.tag_indices[tag.id] < 1 then
        self.tag_indices[tag.id] = #self.layouts
    end
end

---Gets the current layout generator for the given tag.
---
---@param tag pinnacle.tag.TagHandle
---
---@return pinnacle.layout.LayoutGenerator?
function Cycle:current_layout(tag)
    return self.layouts[self.tag_indices[tag.id] or 1]
end

---Creates a layout generator that delegates to other layout generators depending on the tag
---and allows you to cycle between the generators.
---
---@param layouts pinnacle.layout.LayoutGenerator[]
---
---@return pinnacle.layout.builtin.Cycle
function builtin.cycle(layouts)
    ---@type pinnacle.layout.builtin.Cycle
    local cycler = {
        layouts = layouts,
        tag_indices = {},
        current_tag = nil,
        ---@param self pinnacle.layout.builtin.Cycle
        layout = function(self, window_count)
            if self.current_tag then
                local curr_layout = self:current_layout(self.current_tag)
                if curr_layout then
                    return curr_layout:layout(window_count)
                end
            end

            ---@type pinnacle.layout.LayoutNode
            local node = {
                children = {},
            }

            return node
        end,
    }

    setmetatable(cycler, { __index = Cycle })
    return cycler
end

---Layout management.
---
---@class pinnacle.layout
local layout = {
    builtin = builtin,
}

---@class pinnacle.layout.LayoutRequester
---@field private sender grpc_client.h2.Stream
local LayoutRequester = {}

---Causes the compositor to emit a layout request.
---
---@param output pinnacle.output.OutputHandle?
function LayoutRequester:request_layout(output)
    local output = output or require("pinnacle.output").get_focused()
    if not output then
        return
    end

    local chunk = require("pinnacle.grpc.protobuf").encode("pinnacle.layout.v1.LayoutRequest", {
        force_layout = {
            output_name = output.name,
        },
    })

    local success, err = pcall(self.sender.write_chunk, self.sender, chunk)

    if not success then
        print("error sending to stream:", err)
    end
end

---@param node pinnacle.layout.LayoutNode
---
---@return pinnacle.layout.v1.LayoutNode
local function layout_node_to_api_node(node)
    local traversal_overrides = {}
    for idx, overrides in pairs(node.traversal_overrides or {}) do
        traversal_overrides[idx] = {
            overrides = overrides,
        }
    end

    local gaps = node.gaps or 0.0
    if type(gaps) == "number" then
        local gaps_num = gaps
        gaps = {
            left = gaps_num,
            right = gaps_num,
            top = gaps_num,
            bottom = gaps_num,
        }
    end

    local children = {}
    for _, child in ipairs(node.children or {}) do
        table.insert(children, layout_node_to_api_node(child))
    end

    ---@type pinnacle.layout.v1.LayoutNode
    return {
        label = node.label,
        traversal_overrides = traversal_overrides,
        traversal_index = node.traversal_index or 0,
        style = {
            size_proportion = node.size_proportion or 1.0,
            flex_dir = ((node.layout_dir or "row") == "row")
                    and defs.pinnacle.layout.v1.FlexDir.FLEX_DIR_ROW
                or defs.pinnacle.layout.v1.FlexDir.FLEX_DIR_COLUMN,
            gaps = gaps,
        },
        children = children,
    }
end

---Begins managing layout requests from the compositor.
---
---You must call this function to get windows to lay out.
---The provided function will be run with the arguments of the layout request.
---It must return a `LayoutNode` that represents the root of a layout tree.
---
---#### Example
---
---```lua
---local layout_requester = Layout.manage(function(args)
---    local first_tag = args.tags[1]
---    if not first_tag then
---        return {
---            children = {},
---        }
---    end
---    layout_cycler.current_tag = first_tag
---    return layout_cycler:layout(args.window_count)
---end)
---```
---
---@param on_layout fun(args: pinnacle.layout.LayoutArgs): pinnacle.layout.LayoutNode
---
---@return pinnacle.layout.LayoutRequester # A requester that allows you to force the compositor to request a layout.
---@nodiscard
function layout.manage(on_layout)
    local stream, err = client:bidirectional_streaming_request(
        layout_service.Layout,
        function(response, stream)
            ---@type pinnacle.layout.LayoutArgs
            local args = {
                output = require("pinnacle.output").handle.new(response.output_name),
                window_count = response.window_count,
                tags = require("pinnacle.tag").handle.new_from_table(response.tag_ids or {}),
            }

            local node = on_layout(args)

            local chunk =
                require("pinnacle.grpc.protobuf").encode("pinnacle.layout.v1.LayoutRequest", {
                    tree_response = {
                        request_id = response.request_id,
                        tree_id = 0, -- TODO:
                        output_name = response.output_name,
                        root_node = layout_node_to_api_node(node),
                    },
                })

            local success, err = pcall(stream.write_chunk, stream, chunk)

            if not success then
                print("error sending to stream:", err)
            end
        end
    )

    if err then
        log.error("failed to start bidir stream")
        os.exit(1)
    end

    local requester = { sender = stream }
    setmetatable(requester, { __index = LayoutRequester })

    return requester
end

return layout
