# Text Editor

Text and 2D math blocks. Tool: Text / Math (T). Quick commands: colors, A- / A+, Font >, Font...

### Typing math

- x^2, x_i, x^{n+1}: scripts; Shift + letter: superscript, Ctrl + letter: subscript.
- sum, prod, int, iint, oint, sqrt: big operators and the root; limits with _{...}^{...}.
- a//b: fraction. { inserts {}; \} and \{ are literal braces.
- alpha, pi, inf, <=, >=, !=, ->: symbols are replaced while you type.

### Blocks

- Click an empty place to start a block, Escape or a click outside to finish; empty blocks are removed.
- Drag the bar above a block to move it, drag a corner to resize it.

### Services for other plugins

- insert.v1: args { x, y, text, color, size } creates a block and focuses it; returns { id }.
- command.v1: args { action, text } runs an editor action: color_black, color_blue, color_red, color_green, size_up,
  size_down, font_next, font_pick, sup_mode, sub_mode, insert_text, delete_block.

### Data

A block is a node of type block with data { text, size, color, font }. The text is stored in the source form (x^{2},
a//b), so documents stay editable.
