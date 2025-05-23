# Integration with external applications

You may want to provide information about windows, tags, outputs, etc. to external
applications.

You can spin up both one-off and long-running scripts with the Lua API to
provide information to the application. You could do it with the Rust API as well,
but I recommend using the Lua API to not have to deal with Cargo projects and
build artifacts everywhere.

For example, [eww](https://github.com/elkowar/eww) allows you to run scripts to provide
data to widgets. You can write a small Lua script that uses the `run` function to
provide one-off information, or use the `setup` function similarly to a normal config
for something that subscribes to data changes.

```lua
require("pinnacle").run(function()
    print(require("pinnacle.output").get_focused().name)
end)

require("pinnacle").setup(function()
    require("pinnacle.tag").connect_signal({
        tag_active = function(tag, active)
            -- Output data in the necessary format here
        end
    })
end)
```
