# Window Management

## Tiling

Pinnacle is a dynamic-tiling compositor. When a new window opens, if it is to be tiled, a new layout will
be computed that will tile the new window along with existing tiled windows. This happens dynamically,
meaning you, the user, have no say in where exactly the window will be placed, outside of defining
the layout. Windows will shift around as necessary to insert the new window into the tiled layout.

The tiled layout will fill the output in order to maximize the working area windows have,
excluding any exclusive layer surfaces.

![Tiled layout](/assets/tiling.png)

Starting an interactive move on a tiled window will cause it to swap with any tiled window that the mouse moves over.

Tiled windows currently cannot be resized. This is planned for the future.

## Other layout modes

Floating windows are separate from the tiled layout. They can be moved and resized freely.

![Floating layout](/assets/floating.png)

Maximized windows are resized and relocated to fill the non-exclusive area of an output.
This means things like bars will still show.

![Maximized layout](/assets/maximized.png)

Fullscreen windows completely fill the screen. They will also render above all layer surfaces
except for those on the overlay layer.

![Fullscreen layout](/assets/fullscreen.png)
