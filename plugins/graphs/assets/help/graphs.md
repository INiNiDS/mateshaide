# Graphs

Function graphs: y = f(x), x = g(y), and parametric curves (x(t), y(t)). Tool: Graph. Quick commands: + y(x), + (x(t), y(t)), Fit, Zoom +, Zoom -, Color, Grid, Delete row.

### Entering functions

- y = f(x), or just f(x): y as a function of x, e.g. y = sin(x), x^2 - 2x.
- x = g(y): x as a function of y, e.g. x = y^2.
- (x(t), y(t)): parametric curve, e.g. (cos(t), sin(t)). Add a range with t = a..b, e.g. (2cos(t), sin(2t)); t = 0..2pi. Without a range, t runs from 0 to 2pi.
- x = cos(t); y = sin(t) also works as a parametric row.

### Math syntax

- + - * / ^: arithmetic. ** also means ^. 2x and 2sin(x) multiply implicitly.
- ( ), { }, [ ]: all brackets are treated as ( ). |x|: absolute value.
- sin, cos, tan, cot, sec, csc, asin, acos, atan, sinh, cosh, tanh: trig and hyperbolic functions. tg and ctg are aliases for tan and cot.
- ln x: natural log. lg x or log x: base 10. log_2 x or log_{2}(x): any base.
- sqrt(x) or √x, cbrt(x), exp, abs, floor, ceil, round, sign (sgn): roots and other functions.
- min(a, b), max(a, b), mod(a, b): two-argument functions.
- pi (or π), e: constants.
- sin x and sin(x) both work. sin x^2 means sin(x^2). For the square of sine, write sin^2 x or (sin x)^2.
- x, y and t are the only variables. Any other letter gives an error.

### Editing

- Click an empty place to start a graph with y = sin(x). Your first keystroke replaces the sample text. If you are already editing, the first click only finishes editing.
- Enter moves to the next row, or adds a new row on the last one. Arrow Up and Down move between rows.
- Escape or a click outside finishes editing. Empty rows are removed, and a graph with no rows is deleted.
- Ctrl + V pastes. A graph holds up to 12 rows.
- A row with an error shows a red "!" and skips plotting. When the cursor is on that row, the error message appears above the plot.

### Viewing

- Drag inside the plot to pan. Ctrl + drag up or down to zoom. The mouse wheel zooms around the cursor.
- When a graph is selected but no row is being edited: + and - zoom, arrows pan, 0 or f fits the view, Enter starts editing the first row, Delete removes the graph.
- Fit fits the view to the curves. Auto-fit is on by default and follows the curves as you edit. Pan or zoom turns it off, and Fit turns it back on.
- Grid toggles grid lines. They are on by default.
- Jumps, such as in tan(x) or 1/x, are not joined across the gap.

### Rows and colors

- + y(x) adds an empty row. + (x(t), y(t)) adds a row with (cos(t), sin(t)).
- Color cycles the color of the selected row, or the last row if none is selected. There are six palette colors.
- Delete row removes the selected row, or the last row if none is selected. Deleting the last row deletes the graph.

### Moving and resizing

- Drag the bar above a graph to move it. Drag a corner to resize it. Minimum width is 200.

### Services for other plugins

- plot.v1: args { x, y, rows, width, height } creates a graph and returns { id }. rows is a list of formula strings, up to 12. If rows is empty, the graph shows y = x. Example: plot.v1 { x = 40, y = 300, rows = { "y = sin(x)", "(cos(t), sin(t))" } }

### Data

A graph is a node of type graph with schema_version 1 and data { rows, view, auto, grid }. Each row is { text, color }, where text is the formula as typed and color is RGBA with values from 0 to 1. view is [x_min, x_max, y_min, y_max]. auto is true when the view follows the curves. grid is true when grid lines are shown. Default size is 480 × 340. Formulas are stored as typed, so graphs stay editable.

## Drawing inside a graph

Turn on Draw and start a stroke inside the graph frame. The stroke becomes part of the graph.

- The stroke is stored in graph coordinates. It moves with the graph, follows zoom and pan of the view, and is clipped to the plot area.
- When the view moves away from the stroke, the stroke is hidden. It comes back when the view returns.
- Use the eraser to remove a drawing: click it to delete it, or drag over it.