# Layouts

Pinnacle's layout system dynamically tiles windows according to a tree structure. More specifically, we use
[Taffy](https://github.com/DioxusLabs/taffy) to compute layouts, so if you're familiar with
[CSS Flexbox](https://css-tricks.com/snippets/css/a-guide-to-flexbox/), you have a good idea of how
the layout system works.

## Layout basics

### General architecture

When Pinnacle wants to layout a set of windows, it sends a request to your config. The request contains
information like the amount of windows being laid out, what tags are currently active, and the output
the layout is occurring on. In response to the layout request, your config generates a layout tree
that is then sent to the compositor. Pinnacle processes that tree and computes a new layout. Finally,
windows are assigned the resulting geometries and are updated.

### Managing layouts

Of course, you need a way to get the config to respond to layout requests. You can do this through the
`manage` function. You must provide a function that takes layout arguments as input and returns a layout tree.

::: tabs key:langs
== Lua
```lua
require("pinnacle.layout").manage(function(layout_args)
    -- Calculate and return a layout tree
end)
```
== Rust
```rust
layout::manage(|layout_args| {
    // Calculate and return a layout tree
});
```
:::

### The layout tree

Before we get into how to calculate a layout tree, we should understand what a layout tree *is*.

A layout tree consists of layout nodes arranged in a tree structure.
The amount of leaf nodes in the tree determine how many windows can be laid out.
A layout node has the following properties:

| Property | Type | Description |
| -------- | ---- | ----------- |
| Label | String | Provides Pinnacle with information that helps in tree diffing |
| Layout direction | Either horizonal or vertical | Determines what direction the node lays out its children nodes |
| Gaps | Four floats | Determines the gaps the node will surround its children nodes with |
| Size proportion | Float | Determines the amount of space that it will fill up relative to its sibling nodes |
| Children | LayoutNode\[] | The children layout nodes |
| Traversal index | Int | Determines the order Pinnacle traverses the tree to assign geometries |
| Traversal overrides | Int\[]\[] | Overrides the default traversal strategy per window |

The last two may seem arcane, but we'll see what they do further down.

## Layout calculation

Now that we have a rough idea of what happens during a layout, we can calculate one.

### Layout generators

The API provides an abstraction to help with generating a layout through the layout generator interface.
A layout generator is a struct/table with a method called `layout` that receives a window count and returns
the root node of the calculated tree.

::: tabs key:langs
== Lua
```lua
local custom_generator = {}
function custom_generator:layout(window_count)
    -- TODO: generate a layout here
end
```
== Rust
```rust
struct CustomGenerator;
impl LayoutGenerator for CustomGenerator {
    fn layout(&self, window_count: u32) -> LayoutNode {
        todo!()
    }
}
```
:::

Layout generators are composable. You can create a layout generator that calls out to other layout generators
to create a layout, joining their calculated trees into a new one.
For example, the master stack generator builds off of two line generators.

#### Builtin generators

The API provides a set of builtin layout generators that should suffice for most people.
They cover most of the builtin layouts that Awesome has; specifically, the
master stack, spiral, dwindle, corner, and fair layouts.

Additionally, there is the line layout generator to assist in building custom layouts, as well as
the cycle layout generator. The cycle layout generator delegates to a provided list of layout generators,
picking the "active" one on request. It allows you to cycle between the given layouts.

#### Creating a custom generator

Before we dive into how to make a custom layout, we need to know how Pinnacle assigns geometries to windows
upon receiving a layout tree.

##### Window geometry assignment

Internally, Pinnacle keeps a list of windows. Candidate windows up for layout are assigned geometries
from the start of the list to the end. Leaf nodes determine the geometries of windows and how many windows can be laid out.

> [!TIP]
> For the rest of this page, we'll establish a convention for showing a layout tree graphically:
> 
> - `•` represents a non-leaf node
> - `○` represents a leaf node whose geometry hasn't been assigned to a window
> - `●` represents a leaf node whose geometry *has* been assigned to a window

Let's look at a simple example.

```
  •    
 /|\   
○ ○ ○  
```

This layout tree will lay out three windows in a line that fill up the screen.

To "fill out" the tree (assign all leaf nodes' geometries to windows), Pinnacle iteratively traverses
the tree from the root depth-first. When it finds an "empty" leaf node (one whose geometry hasn't been assigned to a window),
it assigns that node's geometry to a window. This is represented by filling out the node.

```
Window  0          1          2
        •          •          •  
       /|\        /|\        /|\ 
      ● ○ ○      ● ● ○      ● ● ●
```

Note: this process is *iterative*. When an empty leaf node is found, traversal restarts from the root to find
the next empty leaf node. The reason for this is explained in the [advanced](./layout#advanced-generator-techniques)
section below.

##### Layout trees to a layout

Let's show what actually shows up on screen when we submit a layout tree for computation.
We'll bring in the gaps, size proportion, and layout direction properties as well.

```
  •
 /|\
● ● ●
```

Properties are listed in `root, child 0, child 1, child 2` order.

<table>
<tr>
<td> Gaps </td> <td> Size proportion </td> <td> Layout direction </td> <td> Layout on-screen </td>
</tr>

<tr>
<td> 0, 0, 0, 0 </td> <td> 1.0, 1.0, 1.0, 1.0 </td> <td> Row, Row, Row, Row </td>
<td>
<div class="compress-lines">

```
┌───────┬───────┬───────┐
│       │       │       │
│       │       │       │
│   0   │   1   │   2   │
│       │       │       │
│       │       │       │
└───────┴───────┴───────┘
```

</div>
</td>
</tr>

<tr>
<td> 0, 0, 0, 0 </td> <td> 1.0, 1.0, 1.0, 1.0 </td> <td> <u>Col</u>, Row, Row, Row </td>
<td>
<div class="compress-lines">

```
┌───────────────────────┐
│           0           │
├───────────────────────┤
│           1           │
├───────────────────────┤
│           2           │
└───────────────────────┘ 
```

</div>
</td>
</tr>

<tr>
<td> <u>4.0 all sides</u>, 0, 0, 0 </td> <td> 1.0, 1.0, 1.0, 1.0 </td> <td> Row, Row, Row, Row </td>
<td>
<div class="compress-lines">

```
            4 px gaps ├┤
┌──────────────────────┐
│┌──────┬──────┬──────┐│
││      │      │      ││
││  0   │  1   │  2   ││
││      │      │      ││
│└──────┴──────┴──────┘│
└──────────────────────┘
```

</div>
</td>
</tr>

<tr>
<td> <u>4.0 all sides, 4.0 all sides, 4.0 all sides, 4.0 all sides</u> </td>
<td> 1.0, 1.0, 1.0, 1.0 </td> <td> Row, Row, Row, Row </td>
<td>
<div class="compress-lines">

```
    8 px gaps ├┤
┌─────────────────────┐
│┌─────┐┌─────┐┌─────┐│
││     ││     ││     ││
││  0  ││  1  ││  2  ││
││     ││     ││     ││
│└─────┘└─────┘└─────┘│
└─────────────────────┘
```

</div>
</td>
</tr>

<tr>
<td> 0,0,0,0 </td>
<td> 1.0, 1.0, <u>2.0</u>, 1.0 </td> <td> Row, Row, Row, Row </td>
<td>
<div class="compress-lines">

```
┌─────┬──────────┬─────┐
│     │          │     │
│     │          │     │
│  0  │    1     │  2  │
│     │          │     │
│     │          │     │
└─────┴──────────┴─────┘
```

</div>
</td>
</tr>
</table>

##### Creating an actual layout

To implement the `layout` method, use the window count and any state in your struct/table
to create a layout tree. Add children nodes by appending or setting the `children` property.
You can set gaps, the size proportion, and the layout direction as well.

The following implements a simple layout generator that lays out windows in a row.

```
Window count:   1     2       3     ...
                •     •       •
                |    / \     /|\
                ●   ●   ●   ● ● ●
```

::: tabs key:langs
== Lua
```lua
local custom_generator = {
    gaps = 4.0, -- Custom state
}
function custom_generator:layout(window_count)
    local root = {
        gaps = self.gaps,
        children = {},
        -- Layout direction defaults to row
        -- Size proportion defaults to 1.0
    }

    for i = 1,window_count do
        table.insert(root.children, {
            gaps = self.gaps,
            children = {}
        })
    end

    return root
end
```
== Rust


```rust
struct CustomGenerator {
    gaps: Gaps, // Custom state
}
impl LayoutGenerator for CustomGenerator {
    fn layout(&self, window_count: u32) -> LayoutNode {
        let root = LayoutNode::new();
        root.set_gaps(self.gaps);
        // Layout direction defaults to row
        // Size proportion defaults to 1.0

        for _ in 0..window_count {
            let child = LayoutNode::new();
            child.set_gaps(self.gaps);
            root.add_child(child);
        }

        root
    }
}
```

<div class="pad-content">

> [!NOTE]
> `LayoutNode`s are ref-counted. A cloned `LayoutNode` refers to the same node as the original node.

</div>
:::

Remember, layout generators are composable. You could simplify the above to the following:

::: tabs key:langs
== Lua
```lua
local custom_generator = {
    gaps = 4.0, -- Custom state
}
function custom_generator:layout(window_count)
    local line_generator = require("pinnacle.layout").builtin.line({
        outer_gaps = 0.0,
        inner_gaps = self.gaps,
    })

    local root = line_generator:layout(window_count)

    return root
end
```
== Rust
```rust
struct CustomGenerator {
    gaps: Gaps, // Custom state
}
impl LayoutGenerator for CustomGenerator {
    fn layout(&self, window_count: u32) -> LayoutNode {
        let line_generator = Line {
            outer_gaps: 0.0.into(),
            inner_gaps: self.gaps,
            direction: LayoutDir::Row,
            reversed: false
        };

        let root = line_generator.layout(window_count);

        root
    }
}
```
:::

Of course, this just wraps the line generator for no reason, but you get the idea.

##### Advanced generator techniques

When we discussed layout node properties, we mentioned a traversal index and traversal overrides.
Let's dive deeper into those two properties.

We discussed how Pinnacle traverses the layout tree to create a layout. However, a simple depth-first traversal
doesn't permit more complicated insertion techniques.
What if we want to, say, *reverse* the order windows are inserted? For example,
AwesomeWM inserts new windows in the master stack layout on the master side and pushes every
other window on the stack side down. If we traverse with depth-first normally, new windows
will always be inserted at the end of the side stack.

###### Traversal index

To enable different orders of insertion, all nodes can have a *traversal index* set.
The traversal index dictates the order in which depth-first traversal chooses children to visit.
Let's copy the row generator we wrote above and set traversal indices on the leaf nodes backwards.

::: tabs key:langs
== Lua
```lua
local custom_generator = {
    gaps = 4.0, -- Custom state
}
function custom_generator:layout(window_count)
    local root = {
        gaps = self.gaps,
        children = {},
        -- Layout direction defaults to row
        -- Size proportion defaults to 1.0
    }

    for i = 1,window_count do -- [!code --]
    for i = window_count,1,-1 do -- [!code ++]
        table.insert(root.children, {
            gaps = self.gaps,
            children = {},
            traversal_index = i, -- [!code ++]
        })
    end

    return root
end
```
== Rust
```rust
struct CustomGenerator {
    gaps: Gaps, // Custom state
}
impl LayoutGenerator for CustomGenerator {
    fn layout(&self, window_count: u32) -> LayoutNode {
        let root = LayoutNode::new();
        root.set_gaps(self.gaps);
        // Layout direction defaults to row
        // Size proportion defaults to 1.0

        for _ in 0..window_count { // [!code --]
        for i in (0..window_count).rev() { // [!code ++]
            let child = LayoutNode::new(); // [!code --]
            let child = LayoutNode::new_with_traversal_index(i); // [!code ++]
            child.set_gaps(self.gaps);
            root.add_child(child);
        }

        root
    }
}
```
:::

Now, traversal will travel down nodes from last to first. As a result, we
have effectively reversed the order of insertion of windows into the layout.
This technique is used when you set `reverse` to true in the builtin master stack layout.

###### Traversal overrides

Even with the ability to reorder traversal, it turns out the static traversal strategy of
"go down the tree in the order provided" doesn't allow for more complicated insertion strategies.
Take AwesomeWM's corner layout, for example. When windows spawn, they are laid out in an
*alternating* fashion, with every even window being inserted into the vertical stack and
every odd window being inserted into the horizontal stack.

Currently, we have no way of changing the path of traversal *per window*; when a node is traversed,
we go through all of its children in sequence before returning to go down a different node.
This is where traversal overrides come in.

Traversal overrides can be applied to any node. A traversal override is a map of
window indices to lists of integer indices. Let's break that down with an example.

::: tabs key:langs
== Lua
```lua
local overrides = {
    [0] = { 1, 1, 2 },
    [2] = { 2 },
}
```
== Rust
```rust
let overrides: HashMap<_, _> = [
    (0, vec![1, 1, 2]),
    (2, vec![2])
].into_iter().collect();
```
:::

The map key represents the index of the window whose traversal gets overridden.
With 4 windows, the above overrides will override traversal for the first and third windows.

> [!IMPORTANT]
> Override indices are 0-based for you Lua users out there.

The map value determines the path of traversal for the given window at the node the override is set on.
When the above overrides are set on the root layout node, when Pinnacle lays out window 0, it
traverses from the root to child 1, then child 1, then child 2. Similarly, window 2
travels from the root to child 2. Nodes without traversal overrides will be filled according to
regular traversal.

```
Window  0*         1          2*         3          4          5
        •          •          •          •          •          •
       /|\        /|\        /|\        /|\        /|\        /|\
      ○ • ○      ● • ○      ● • ●      ● • ●      ● • ●      ● • ●
       / \        / \        / \        / \        / \        / \
      ○   •      ○   •      ○   •      ●   •      ●   •      ●   •
         /|\        /|\        /|\        /|\        /|\        /|\
        ○ ○ ●      ○ ○ ●      ○ ○ ●      ○ ○ ●      ● ○ ●      ● ● ●
```

If you look at the source code for the corner layout, you'll see it sets the
traversal overrides for the root node with an alternation of 0s and 1s in order
to send all even windows down the side stack and all odd windows down the
horizontal stack.

Of course, to support this more complex traversal strategy, we have to iteratively
restart traversal from the root whenever we fill in a leaf node. Luckily,
layout trees are small, so this shouldn't pose any significant performance penalty.

> [!NOTE]
> In order to support composable layout generators, if a child has traversal overrides
> while you are traversing according to an ancestor's overrides, the child's overrides
> will *replace* the current overridden path.
