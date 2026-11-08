# Plugin API: callbacks

The entry module returns a table with functions. The first argument is always the plugin state s: a table that the host
keeps between calls (JSON, up to 1 MB).

### activate (s)

Called once. Returns contributions:

```lua
return {
    tools = { { id = "graph", label = "Graph", text_input = false } },
    commands = { { id = "zoom_fit", label = "Fit" } },
    services = { "plot.v1" },
    object_types = { "plot" },
}
```

- tools appear as toolbar buttons; pointer and key events go to the plugin whose tool is selected.
- commands appear as quick buttons and arrive as the event command.

### handle (s, event, ctx)

Reacts to events and returns an array of commands. ctx has nodes (own objects), new_id (a free object id), tool, focused
and api.

### render (s, node, ctx)

Draws one object. Returns primitives in world coordinates: text (x, y, text, size, font, color), rect (x, y, width,
height, color), circle (x, y, radius, color), line (x1, y1, x2, y2, width, color). Colors are RGBA arrays 0..1. render
must depend only on node and ctx: its result is cached.

### overlay (s, ctx)

Optional. Draws caret, selection and handles for the selected tool; ctx has nodes, focused, zoom.

### panel (s, ctx)

Optional. A screen panel above the canvas; return nil to hide it. ctx has width, height, top, dark and, with help.read,
pages. Return x, y, width, height, scroll_max and items: text (x, y, text, size, color), rect (x, y, width, height,
color), button (id, x, y, width, height, label, selected). Coordinates are pixels relative to the panel; colors are
theme names: text, muted, accent, warning, error, success, border, surface.

### service (s, name, args, ctx)

Called by other plugins. Returns { value = ..., commands = {...} }.

### deactivate (s)

Optional cleanup.

### Limits

250 ms per call, 128 commands per callback, 64 commands per event chain, node data up to 256 KB, 5000 objects per
document.

## ink

Plugin contributions can list object types in `ink`. Each type must also be in `object_types`. Freehand strokes that start inside such an object are not drawn on the canvas. The plugin receives them as an `ink.stroke` event and stores them in its own data. The eraser sends an `ink.erase` event instead of deleting the object.
