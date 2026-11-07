# Plugin API: manifest

Every plugin folder has plugin.aam with these fields (all are required):
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

- id: letters, digits, dot, dash, underscore; must contain a dot. Object ids of the plugin start with "<id>:".
- version is semver; api is a semver requirement for the host API (now 1.0.0).
- entry is a .lua file inside the package; other modules are loaded with require ("name").
- icon is optional as a file: a missing icon is simply not shown.
- dependencies: [{id = "...", version = "^1.0", repository = "https://..."}]. Only declared dependencies can be called
  as services.

### Package files

- plugin.aam, *.lua and everything under assets/. Other files are ignored.
- Limits: 256 files, 2 MB per file, 8 MB per package.
