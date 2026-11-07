# Plugin API: events and commands

### Events (event.name)

- pointer: phase down, move or up, x, y in world coordinates, shift, ctrl.
- key: key (e.g. "a", "ArrowLeft", "Escape"), ctrl, shift, alt. text: typed text. ime.preedit: text being composed.
- command: id of a quick button. tool.selected, blur, document.changed.
- clipboard: ok, text (answer to clipboard.read). file.picked: ok, kind, family, file, token.
- service.result: token, value, plugin, service.
- panel.click: id of a panel button. panel.scroll: dy, max.
- autosave.tick, app.exit, document.saved, document.restored: only with document.autosave.

### Commands (returned from handle and service)

- { kind = "node.create", node = {...} }, { kind = "node.update", node = {...} }, { kind = "node.delete", id = "..." }.
- { kind = "input.focus" } and { kind = "input.blur" }: start or stop text input for your tool.
- { kind = "call", plugin = "...", service = "...", args = {...}, token = ... }.
- { kind = "clipboard.write", text = "..." }, { kind = "clipboard.read" }.
- { kind = "file.pick", accept = "font", token = ... }.
- { kind = "status.set", text = "..." }: a short status next to your quick buttons.
- { kind = "document.save" }, { kind = "document.restore" }: auto-save slot, needs document.autosave.

### Objects

A node is { id, type, x, y, width, height, data }. Create it with ctx.new_id. y grows upward: (x, y) is the top-left
corner and the object extends down to y - height.

### Transactions

All commands of one event are applied together. If anything fails, nothing is changed and the error is shown in the
notice line. All changes of one pointer drag form one undo step.
