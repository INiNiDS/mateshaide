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
cargo run
```

You can also use it in the browser (note native version works much smoother):
https://ininids.github.io/mateshaide/

At the release it will be available on GitHub releases too.

## Features working by now

### Shapes

The core editor can only draw shapes. Text and other objects come from plugins.
Test tools are available in debug builds only (`cargo run`).
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

By pressing E, you will switch to erase mode. Press 1 or click the left mouse button to erase a shape or a text block.

### Selection

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

### Drawing

Press Ctrl + 9 to turn draw mode on or off. When it's on, hold the left mouse button and move the mouse to draw. Each stroke is saved in undo/redo history.

### Plugins

The plugin page is the main feature of MateshaIDE.
Press Ctrl + Shift + P or the Plugins button in the top-left corner to open it.

The page lists every installed plugin with its version, permissions and status. To use a plugin, press Activate and confirm the dialog. Plugin code starts running only after you confirm, and its dependencies are activated too. If you disable a plugin, the plugins that depend on it are disabled too.

Press F5 to rescan the `plugins/` folder after you add or change something there. Esc or Close returns to the canvas.
In the browser, use Import ZIP to install a plugin.

If a plugin is disabled, its objects stay in the document and are shown as placeholders until you activate the plugin again.

### Save & Open

Press Ctrl + S or the Save button to save the document to `document.matesha.json`.
Press Ctrl + O or the Open button to open a saved document.
In the browser the file is downloaded and picked through the browser dialogs; in the native build you get a file dialog.

### Export

Press Ctrl + E to export the canvas to PNG. A preview will appear. Press S to save the image, C to copy it to the clipboard, or Esc to close the preview. You can also click the Save and Copy buttons in the preview.

Press Ctrl + P to export the canvas to PDF. The file is saved automatically when all pages are rendered.

The export includes the objects drawn by plugins.

## License

The project is licensed under AGPL-3.0 and made for Hack Club Terra.

# Thanks for using MateshaIDE!
