# Auto-Save

Saves the whole canvas (drawings, text blocks, objects of all plugins) without asking.

### When it saves

- 2 seconds after your last change.
- At least every 30 seconds while you keep editing without a pause.
- When you close the window (desktop).

### Where

- Desktop: mateshaide/autosave/document.matesha.json in your config folder (~/.config on Linux, %APPDATA% on Windows).
- Browser: the browser storage of this site (key matesha.autosave.v1).
- This is a recovery copy. Save (Ctrl + S) still writes a document wherever you want.

### Restoring

- If the canvas is empty when the app starts, the last auto-saved canvas is loaded. Ctrl + Z undoes it.
- Restore loads the auto-saved canvas at any time; Ctrl + Z undoes that too.

### Status

The text next to the Auto-Save buttons: on, unsaved, saving..., saved, restored, error, no backup, off. On error the
reason is shown in the notice line; Auto-Save retries in 10 seconds.

- On/Off pauses and resumes saving. Save now saves immediately.
