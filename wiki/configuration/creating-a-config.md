# Creating a Config

The first time you boot up Pinnacle, it will start up with the default Rust config, which is built into the compositor.
Obviously, you probably want to change stuff. In that case, you'll want to generate a new config.

Run the config generator with the following command:
```sh
pinnacle config gen
```

This will start an interactive TUI where you can specify where to copy a new config to, as well as what
language you want to use.
