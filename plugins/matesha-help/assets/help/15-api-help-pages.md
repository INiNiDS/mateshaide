# Plugin API: help pages

Any plugin can add pages to this help. No code is needed.

- Put Markdown files into assets/help/ of your package, for example assets/help/graph.md.
- The first line "# Title" is the page title; pages are sorted by file name inside your section.
- The section is named after your plugin; inactive plugins are marked (off).
- Every line is a paragraph and is wrapped automatically. Supported: # headings (three levels), "- " bullets, lines
  starting with four spaces as code, empty lines as gaps.
- Up to 32 pages per plugin, 64 KB per page.
- In the browser build, list the files in the bundled! line of src/plugins/package.rs.

### Opening a page from code

Declare org.matesha.help as a dependency, request plugins.call and

```lua
return { kind = "call", plugin = "org.matesha.help", service = "open.v1", args = { page = "org.example.graph/graph.md" } }
```

The page id is "<plugin id>/<file name>".
