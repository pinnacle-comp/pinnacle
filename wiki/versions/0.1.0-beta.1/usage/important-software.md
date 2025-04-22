# Important Software

Pinnacle is just a compositor. We are still a ways off from having custom bars, notifications,
etc. like Awesome, so this page should lay out a handful of useful software that you
will want.

## Bars, panels, and other widgets

> [!TIP]
> I *highly* recommend a bar because you can currently lose a floating window behind tiled ones.
> A bar allows you to bring the floating window back to the front.

https://github.com/rcalixte/awesome-wayland?tab=readme-ov-file#widgets-bars-panels-etc

## Notification daemons

https://github.com/rcalixte/awesome-wayland?tab=readme-ov-file#notifications

## XDG desktop portals

Portals allow you to screenshare and use Flatpak apps. They require you to run Pinnacle
[as a session](../getting-started/running#running-as-a-session).

The following portals are recommended and are used in
[`pinnacle-portals.conf`](https://github.com/pinnacle-comp/pinnacle/blob/main/resources/pinnacle-portals.conf):
- `xdg-desktop-portal-gtk`: As a fallback for most portal activities like file picking and other basic things
- `xdg-desktop-portal-wlr`: For screencasting
- `gnome-keyring`: For the Secret portal

## Authentication agents

Authentication agents allow applications to request superuser privileges for things like writing
to root-owned directories.

See the [Polkit](https://wiki.archlinux.org/title/Polkit#Authentication_agents)
page on the Arch wiki for a list of authentication agents.
