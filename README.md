# What is it?

MateshaIDE is just like an IDE, but for math. If you have ever used an IDE like PyCharm, Visual Studio, Eclipse, etc., you know there are many features that make writing code easier. So MateshaIDE has many features, but for math.

<!-- You litteraly don't need a video guide with new wiki plugin -->

## Where I can test

First of all, you can clone the repo:

```bash

git clone https://github.com/ininids/mateshaide --depth=1

```

and run it via Cargo:

```bash

cargo run --release

```

You can also use it in the browser (note native version works much smoother):

https://matesha.ininids.in.rs/

Also, you can download a version from the releases page.

## Features working by now

### The screen

The top toolbar has: Plugins, Open, Save, Draw, Eraser, Text, Undo, Redo, zoom (-, 100%, +), PNG and PDF.

In the top right corner you can switch the Dark and Halloween themes.

Under the toolbar you can see the tool buttons and quick commands of the active plugins (colors, font size, fonts, Help, Auto-Save).

At the bottom there is a notice line with messages and errors.

### Drawing

Press Ctrl + 9 to turn draw mode on or off. When it's on, hold the left mouse button and move the mouse to draw. Each stroke is saved in undo/redo history.

If you start a stroke inside a graph, it becomes a part of the graph (see Graph below).

### Eraser

Press E or the Eraser button to turn the eraser on. Ctrl + 8 toggles it, Esc turns it off.

The object under the cursor is highlighted in red before you erase it.

1. Click - removes the object under the cursor: a stroke, a text block, or a graph.

2. Press and drag - erases drawings only. Every stroke you touch is removed as a whole. Text blocks and graphs are not affected by a drag.

One click or one drag is one undo step, so Ctrl + Z brings it back.

Inside a graph, a click on a drawing removes that drawing, and a click on an empty part of the graph deletes the graph. A drag over a drawing inside a graph erases it.

### Text and formulas

The core editor can only draw. Text and other objects come from plugins.

Activate the Text Editor plugin once on the Plugins page, then press T or the Text button and click an empty place. A caret appears right there, and the block is created by the first typed character. You can also double-click an empty place (when Draw and Eraser are off).

Esc or a click outside finishes editing. An empty block is removed.

The Text tool turns plain typing into 2D math as you type:

1. x^2, x^{n+1} - superscript. x_i, x_{i,j} - subscript. Shift + letter after a character types a superscript, Ctrl + letter types a subscript.

2. sum, prod, int, iint, oint - big operators with limits: sum_{i=1}^{n}, int_0^1 x dx.

3. a//b - fraction. sqrt{x} - square root.

4. alpha, beta, pi, inf, nabla, forall, exists, <=, >=, !=, -> and other symbols are replaced while you type.

5. { inserts a pair {}. To show a literal brace, type \{ or \}.

Click inside a block to place the caret, drag to select, Ctrl + A / C / X / V work as usual. Drag the thin bar above a block to move it, drag a corner to resize it (the font size follows the block size).

Quick commands: Ink, Blue, Red, Green set the color, A- and A+ change the size.

Every block has its own font. Font > switches to the next font, Font... loads your own .ttf, .otf or .ttc file. Fonts that you picked are loaded again at the next start.

### Graph

The Graph plugin draws function plots on the canvas. Select the Graph tool and click an empty place to start a graph with y = sin(x).

You can enter:

1. y = f(x), or just f(x), e.g. y = sin(x)

2. x = g(y), e.g. x = y^2

3. (x(t), y(t)) - a parametric curve, e.g. (2cos(t), sin(2t)); t = 0..2pi

A graph holds up to 12 rows, each row has its own color. A row with an error shows a red "!" and the message.

Drag inside the plot to pan. The mouse wheel or Ctrl + drag zooms. Fit fits the view to the curves, Grid toggles the grid lines. Drag the bar above a graph to move it, drag a corner to resize it.

### Shapes

The shapes below are test tools. They are available in debug builds only (`cargo run`).

Get into test mode by pressing Ctrl + 1.

Now you can see the square in the corner. It shows the color that will be applied to the next shape. There are three modes for the colors:

1. Circle - The color will be taken from TEST_PALETTE in circular order.

2. Fixed - Each shape has a fixed color.

3. Picked - You can change the color for all shapes that you will draw by pressing Ctrl + 7.

You can change the mode (rotating) by pressing Ctrl + 6.

Now you have these shapes:

1. Rect - Ctrl + 2

2. Circle - Ctrl + 3

3. Hexagon - Ctrl + 4

4. Rounded rect - Ctrl + 5

To draw any shape, you need to press 1 (make sure you are in test mode). The shape is spawned at the cursor with the current size.

You can also hold the left mouse button and drag to set the size. The size that you dragged becomes the current size for the next shapes.

Press [ or ] to make the current size smaller or bigger. Shift + [ / ] changes only the width, Alt + [ / ] changes only the height.

To erase a shape, use the Eraser (see above).

### Selection

Selection is a test tool too (debug builds only).

Press V or Ctrl + Shift + 1 to switch to selection mode.

Click a core shape to select it and drag it to move it.

Drag a corner of the selected shape to resize it. Hold Shift to keep the aspect ratio.

Press Delete or Backspace to delete the selected shape.

Esc cancels the current drag, otherwise it deselects the shape.

Circles and hexagons keep their proportions. Lines and custom SDF shapes can be moved, but not resized.

### Zoom & Pan

You can zoom using Ctrl + Mouse Wheel, or by pressing Ctrl + = / Ctrl + + and Ctrl + - (Numpad + and Numpad - work too).

You can reset the view by pressing Ctrl + 0.

To pan, you need to hold the right or middle mouse button and move the mouse. When you release the button, the canvas keeps gliding for a moment.

Another way is to use the mouse wheel without Ctrl.

Without Shift, it pans from top to bottom; with Shift, from left to right.

### Undo/Redo

You can undo by pressing Ctrl + Z and redo by pressing Ctrl + Y, Ctrl + X (or Ctrl + Shift + Z).

Undo/redo also covers the objects that plugins create and edit.

### Plugins

The plugin page is the main feature of MateshaIDE.

Press Ctrl + Shift + P or the Plugins button in the top-left corner to open it.

The page lists every installed plugin with its version, permissions and status. To use a plugin, press Activate and confirm the dialog. Plugin code starts running only after you confirm, and its dependencies are activated too. If you disable a plugin, the plugins that depend on it are disabled too. If the files of a plugin change, it has to be approved again.

Press F5 to rescan the `plugins/` folder after you add or change something there. Esc or Close returns to the canvas.

In the browser, use Import ZIP to install a plugin. The package is kept in the browser storage. The desktop version accepts a .zip too.

If a plugin is disabled, its objects stay in the document and are shown as placeholders until you activate the plugin again.

Plugins are Lua scripts that run in a sandbox: they see only their own objects, cannot touch files or the network, and have time and size limits.

Plugins that come with MateshaIDE:

1. Text Editor - text blocks and 2D math.

2. Graph - function plots.

3. Help - the built-in wiki.

4. Auto-Save - automatic recovery copy of the canvas.

### Help

Press F1 or the ? Help button to open the built-in help. It explains how to use MateshaIDE and how to write plugins.

Any plugin can add its own pages to the help without code: put Markdown files into `assets/help/` of the package.

### Auto-Save

The Auto-Save plugin saves the whole canvas (drawings, text blocks, objects of all plugins) without asking:

1. 2 seconds after your last change.

2. At least every 30 seconds while you keep editing without a pause.

3. When you close the window (desktop).

On Linux the copy is in `~/.config/mateshaide/autosave/document.matesha.json` (`%APPDATA%` on Windows). In the browser it is in the browser storage of the site.

If the canvas is empty when the app starts, the last auto-saved canvas is loaded. Ctrl + Z undoes it. The Restore button loads the copy at any time, Save Now saves immediately, On/Off pauses and resumes saving.

### Save & Open

Press Ctrl + S or the Save button to save the document to `document.matesha.json`.

Press Ctrl + O or the Open button to open a saved document.

In the browser the file is downloaded and picked through the browser dialogs; in the native build you get a file dialog.

### Export

Press Ctrl + E to export the canvas to PNG. A preview will appear. Press S to save the image, C to copy it to the clipboard, or Esc to close the preview. You can also click the Save and Copy buttons in the preview.

Press Ctrl + P to export the canvas to PDF. The file is saved automatically when all pages are rendered.

The export includes the objects drawn by plugins.

### Themes

The Dark button switches the dark theme. The Halloween button turns on the Halloween theme. The choice is remembered.

### Hotkeys

1. T - Text tool. E - Eraser. Ctrl + 8 - toggle the eraser. Ctrl + 9 - toggle drawing.

2. Esc - leave the eraser or a tool; while typing it finishes the text block.

3. F1 - help. Ctrl + Shift + P - plugins. F5 - rescan plugins (on the plugins page).

4. Ctrl + Z - undo. Ctrl + Y / Ctrl + X / Ctrl + Shift + Z - redo.

5. Ctrl + S - save. Ctrl + O - open. Ctrl + E - PNG. Ctrl + P - PDF.

6. Ctrl + = / Ctrl + - / Ctrl + 0 - zoom in / zoom out / 100%.

## Writing a plugin

A plugin is a folder with `plugin.aam`, a Lua entry file and optional `assets/`:

```aam
id = "org.example.graph"
name = "Graph"
version = "1.0.0"
description = "What the plugin does."
authors = ["Your Name"]
repository = "https://github.com/you/graph"
icon = "assets/icon.png"
entry = "main.lua"
api = "^1.0"
permissions = ["document.write"]
dependencies = []
```

Plugins can create their own objects, add tools and quick commands, call services of other plugins (for example `plot.v1` from Graph or `insert.v1` from Text Editor) and take over the freehand strokes drawn inside their objects. The permissions are shown to the user before activation.

The full API is described page by page in the built-in help (Plugin API pages).

## License

The project is licensed under AGPL-3.0 and made for Hack Club Terra.

# Thanks for using MateshaIDE!
