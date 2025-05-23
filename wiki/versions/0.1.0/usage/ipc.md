# IPC

Because Pinnacle is configured at runtime, IPC is implemented using the already existing APIs.

## Lua API REPL

Many compositors expose a CLI that provides access to IPC commands. Pinnacle uses the Lua API for this purpose.

You can start an interactive Lua REPL that loads the Lua API with `pinnacle client`,
allowing you to run most API functions on demand.

```
$ pinnacle client
Lua 5.4.7  Copyright (C) 1994-2024 Lua.org, PUC-Rio
DEBUG Building protos
INFO Connected to socket at /run/user/1000/pinnacle-grpc-25293.sock
Available globals: Pinnacle, Input, Libinput, Process, Output, Tag, Window, Layout, Util, Snowcap
pinnacle> Window.get_focused():app_id()
Alacritty
pinnacle>
```

The REPL loads the API into the following globals:
```lua
Pinnacle = require("pinnacle")
Input = require("pinnacle.input")
Libinput = require("pinnacle.input.libinput")
Process = require("pinnacle.process")
Output = require("pinnacle.output")
Tag = require("pinnacle.tag")
Window = require("pinnacle.window")
Layout = require("pinnacle.layout")
Util = require("pinnacle.util")
Snowcap = require("pinnacle.snowcap")
```

Alternatively, to run a one-off function non-interactively,
pipe a Lua string into the client or use the `-e` flag:

```
$ echo "print(Output.get_focused().name)" | pinnacle client
DEBUG Building protos
INFO Connected to socket at /run/user/1000/pinnacle-grpc-25293.sock
Available globals: Pinnacle, Input, Libinput, Process, Output, Tag, Window, Layout, Util, Snowcap
DP-1

$ pinnacle client -e "print(Output.get_focused().name)"
DEBUG Building protos
INFO Connected to socket at /run/user/1000/pinnacle-grpc-25293.sock
Available globals: Pinnacle, Input, Libinput, Process, Output, Tag, Window, Layout, Util, Snowcap
DP-1
```

> [!NOTE]
> The logs and globals print makes it difficult to easily retrieve information from the function call.
> This will be fixed soon™️.
