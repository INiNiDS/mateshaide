# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/INiNiDS/mateshaide/releases/tag/v0.1.0) - 2026-10-27

### Added

- *(deploy)* update deployment process and add preview script for local testing
- *(input)* add process_event_without_actions method for handling input updates without triggering actions
- *(capture)* enhance capture overlay with improved layout and animation parameters
- *(readme)* add drawing and export instructions to README
- *(demo)* add help text overlay with control instructions for drawing and exporting
- *(capture)* enhance capture overlay with corner glow and preview border functionality
- *(dependencies)* update Cargo.lock and add image crate for enhanced image processing
- *(export)* refactor export functions and add save and copy functionality for images
- *(capture)* implement PNG capture functionality with overlay preview and clipboard support
- *(export)* implement PNG and PDF export functionality with corresponding actions
- *(ci)* add GitHub Actions workflows for quality checks and releases
- *(lints)* enhance Clippy configuration and improve code quality checks
- *(lints)* add Clippy configuration for improved linting and code quality
- *(input)* refactor input initialization and event handling for better organization
- *(text)* refactor TextLayer to improve text layout handling and rendering
- *(text)* add CanvasText struct and TextLayer for rendering text on canvas
- *(draw)* implement stroke functionality for canvas drawing
- *(draw)* implement DrawTool for canvas drawing functionality
- *(canvas, wasm)* enhance canvas resizing and add device pixel ratio support
- *(canvas, wasm)* add canvas element to HTML and configure window attributes for WASM target
- *(renderer, engine)* add size retrieval methods and synchronize window size during redraw
- *(canvas, engine)* replace std::time::Instant with web_time::Instant for WASM target
- *(canvas)* use web-time for WASM target and update dependencies
- *(config)* update package name and dependencies in Cargo file
- *(config)* update package name and dependencies in Cargo files
- *(config)* update package metadata for IDE
- *(app)* implement main application structure and input handling
- *(canvas)* implement undo/redo functionality and shape eraser mode
- *(renderer)* add color swatch overlay for shape preview
- *(test)* add color mode toggling and next picked color functionality
- *(dev)* add test shape spawner and debug shortcuts
- *(render)* draw shapes through instanced vertex buffer
- *(canvas)* store shape instances on canvas
- *(shape)* add instanced shape data and SDF shape registry
- *(color)* add Color type with rgb, hex and vector conversions
- *(input)* implement action-based InputManager with shortcut bindings
- *(canvas)* introduce Canvas with inertial panning and zoom calculations
- Add initial implementation of rendering engine with camera and shader support

### Fixed

- *(renderer)* correct canvas retrieval in WASM target and update App reference
- *(canvas)* increase pan friction to stop inertial glide sooner

### Other

- Merge pull request #2 from INiNiDS/feat/textg
- update project structure and improve capture overlay functionality
- Add video guide section to README
- *(clippy)* used cargo clippy fix
- *(structure)* reorganize module structure and rename files for clarity
- *(README)* add initial project description and features overview
- *(deps)* add thiserror
- *(app)* wire Canvas and InputManager into engine and event loop
- *(render)* decouple camera uniform and simplify fullscreen triangle
- add hashbrown, rustc-hash and define fast hash aliases
- initial commit
