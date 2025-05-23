# Screencasting

Pinnacle currently supports screencasting through the `wlr-screencopy` protocol.
This means you can only screencast full outputs, not individual windows.

To screencast to applications that use PipeWire screen capture (OBS, Discord, WebRTC, etc), you need:
- PipeWire
- An active D-Bus session
- [`xdg-desktop-portal-wlr`](https://github.com/emersion/xdg-desktop-portal-wlr)
- Pinnacle running as a [session](../getting-started/running#running-as-a-session)

Assuming everything is set up correctly, attempting to start a screencast will run
the chooser command configured for `xdg-desktop-portal-wlr`
(by default this is `slurp` then `wofi` then `bemenu`). You can then select an output
and the screencast should start.
