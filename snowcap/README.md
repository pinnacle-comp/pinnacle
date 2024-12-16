# Snowcap
A very, *very* Wayland widget system built for Pinnacle

Currently in early development with preliminary integration into Pinnacle.

## What is Snowcap?
Snowcap is a widget system for Wayland, made for [Pinnacle](https://github.com/pinnacle-comp/pinnacle),
my WIP Wayland compositor.

It uses Smithay's [client toolkit](https://github.com/Smithay/client-toolkit) along with the
[Iced](https://github.com/iced-rs/iced) GUI library to draw various widgets on screen.

## Compositor Requirements
While I'm making this for Pinnacle, a side-goal is to have it at least somewhat compositor-agnostic.
To that end, compatible compositors must implement the wlr-layer-shell protocol for Snowcap to work.
