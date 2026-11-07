# Fonts

Every text block has its own font. The default font is Roboto.

- Select a block (or nothing, to set the font for new blocks) and press Font > to cycle through the loaded fonts.
- Font... opens a file dialog: pick a .ttf, .otf or .ttc file. The font is registered and applied to the selected block.
- Fonts picked by you are remembered and loaded again at the next start.
- Plugins can ship fonts in assets/fonts/ inside their package; the Text Editor loads them on activation.
- If a document uses a font that is not available, Roboto is used instead and the font name is kept in the document.
