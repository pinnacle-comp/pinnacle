# Resizing tiles

Tiles can be resized with the mouse using `WindowHandle::begin_resize`,
or through the config with `WindowHandle::resize_tile`.
For API specifics, search the corresponding API reference.

## Remembering tile sizing

It is usually expected that resized tiles stay resized whenever the
layout changes. Because of the tag system, the layout may change
in sporadic ways; the window count may suddenly double, for example,
when a tag is activated. Because of this, and because of the way
layout trees are more of a "client-side" construct, rather than a
server-side object, Pinnacle opts to use a diffing algorithm to
try to remember the exact proportions of tiles. The specific algorithm
used is the [GumTree algorithm](https://github.com/GumTreeDiff/gumtree).

## Layout generators with resizing in mind

The diffing algorithm works best when layout nodes are properly labeled.
If you look at the source code for layout generators like `Fair` and `Dwindle`,
you will see that certain nodes are labeled. In the case of the `Fair` generator,
each specific column/row is labeled to track size changes within each column/row.

In general, you should try to construct layout generators such that important nodes
retain a stable label across layouts for the most intuitive sizing behavior.

## Known issues

The following are some known issues with resizing and tracking sizes:

- There may be some slight single-pixel jittering in certain circumstances
  when resizing windows due to floating point rounding.
- Sizes are lost when fullscreening/maximizing a window in a row/column with
  only two windows. This is because fullscreening/maximizing removes the window
  from the layout, causing its node to disappear, meaning Pinnacle forgets
  its size when unfullscreening/unmaximizing.
