# Plugin API: permissions

Permissions are listed in plugin.aam and shown to the user before activation.

- document.read: see objects of all plugins in ctx.nodes (otherwise only your own).
- document.write: create, update and delete your own objects.
- plugins.call: call services of declared dependencies.
- clipboard.read, clipboard.write: the system clipboard.
- files.pick: ask the user to pick a file (fonts for now).
- help.read: receive help pages of all plugins in panel ctx.pages.
