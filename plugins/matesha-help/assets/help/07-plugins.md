# Plugins

Text, help and auto-save are plugins. Plugins are Lua scripts that run in a sandbox: they see only their own objects,
cannot touch files or the network, and have time and size limits.

### Managing plugins

- Open the Plugins page with the Plugins button or Ctrl + Shift + P.
- Activate shows the plugin's permissions and dependencies; nothing runs before you confirm.
- If a plugin's files change, it has to be approved again.
- Deactivating a plugin keeps its objects in the document; they show a placeholder until the plugin is active again.

### Installing

- Desktop: put the plugin folder into plugins/ next to the program (or the working folder), then press F5 on the Plugins
  page. Import also accepts a .zip.
- Browser: use Import with a .zip; the package is kept in the browser storage.

### Writing your own

Start with "Plugin API: manifest" and continue page by page. A plugin is a folder with plugin.aam, a Lua entry file and
optional assets/.
