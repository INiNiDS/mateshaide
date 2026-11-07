# Plugin API: assets, fonts, files

Functions available to every plugin:

- assets.list (): files under assets/ of your own package: { {path = "assets/...", size = n}, ... }.
- assets.read (path): the text of a UTF-8 file from your package, up to 256 KB.
- fonts.load (path): registers a .ttf/.otf/.ttc file from your assets and returns its family name.
- fonts.list (): available families; the first one is the default.
- text_layout (text, size, font): width, height, line_height, lines and carets (x, y, line for every caret position).
- graphemes (text): an array of user-visible characters. join_text (array) joins them back.
- replace_text (s, needle, replacement): plain-text replace.
- require ("module"): loads another .lua file of the package.

### Files from the user

Return { kind = "file.pick", accept = "font", token = ... } (permission files.pick). After the user picks a file you get
file.picked with ok, family and your token.

### Not available

io, os, debug, load, pcall, coroutine and network access. Use the host functions above.
