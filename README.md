# What is it?

MateshaIDE is just like an IDE, but for math. If you have ever used an IDE like PyCharm, Visual Studio, Eclipse, etc., you know there are many features that make writing code easier. So MateshaIDE has many features, but for math.

## Status

I'm still working on it. So far it's more of a coloring book. You can draw but can't write.
It's about 10% done.

## What's next

After that, I will make a full PluginManager. This is the main feature of MateshaIDE, since the other features are plugins. It's a plugin architecture.

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

Now you can only draw shapes.
Get into test mode by pressing Ctrl + 1.
Now you can see the square in the corner. It shows the color that will be applied to the next shape. There are three modes for the colors:
1. Circle - The color will be taken from TEST_PALETTE in circular order.
2. Fixed - Each shape has a fixed color.
3. Picked - You can change the color for all shapes that you will draw by pressing Ctrl + 7.
You can change the mode (rotating) by pressing Ctrl + 6.

Now you have these shapes:
1. Rect - Ctrl + 2 or Ctrl + 5
2. Circle - Ctrl + 3
3. Hexagon - Ctrl + 4

To draw any shape, you need to press 1 (make sure you are in test mode).

By pressing Ctrl + 8, you will switch to erase mode. Press 1 to erase a shape.

### Zoom & Pan

You can zoom using Ctrl + Mouse Wheel, or by pressing + or -.

To pan, you need to hold the right mouse button and move the mouse.
Another way is to use the mouse wheel without Ctrl.
Without Shift, it pans from top to bottom; with Shift, from left to right.

### Undo/Redo

You can undo by pressing Ctrl + Z and redo by pressing Ctrl + Y (or Ctrl + Shift + Z).

## License

The project is licensed under AGPL-3.0 and made for Hack Club Terra.

# Thanks for using MateshaIDE!