# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/INiNiDS/mateshaide/releases/tag/v0.1.0) - 2026-11-08

### Added

- *(eraser)* enhance eraser functionality with improved logging and interaction tracking
- *(ink)* add ink event handling for strokes and eraser functionality
- *(eraser)* enhance eraser functionality with ink tracking and improved interaction
- *(autosave)* implement auto-save functionality for canvas with restore capability
- *(help)* add help viewer with documentation for MateshaIDE and plugin development
- *(panel)* implement panel management and rendering functionality
- *(graph)* add graph overlay and service for graph creation
- *(graphs)* add graphing functionality with support for explicit and parametric equations
- *(caching)* implement caching for font generation and enhance text layout handling
- *(fonts)* add font management system with loading and registration functionality
- update text-editor
- *(eraser)* implement eraser tool with toggle functionality and input bindings
- *(math)* enhance layout of big operators with improved spacing and baseline calculations
- *(erase)* enhance erase functionality to support text blocks and plugin nodes
- *(math-ui)* add Math UI Builder plugin for constructing mathematical formulas and managing text blocks
- *(ui)* implement CardSlot struct and refactor plugin card drawing logic
- *(input)* refactor input handling and improve event processing logic
- *(render)* enhance rendering pipeline with background shape handling and UI improvements
- *(ui, images)* add UI image rendering functionality and integrate Halloween decorations
- *(theme)* implement dynamic theme switching and enhance UI appearance
- *(deploy, vendor)* add Lua runtime and Wasmoon dependencies for browser support
- *(color)* add color palette and functionality for shape and text color selection
- *(ui, render)* enhance plugin confirmation dialog and improve scroll handling
- *(deploy, manifest, preview)* enhance build process with profiling support and normalize manifest fields
- *(shapes)* add shape manipulation functionality with toggle for rectangle and circle
- *(text-editor)* implement core functionality for text manipulation and node management
- *(deploy)* update output directory for WASM build and local preview
- *(lua)* add Lua bootstrap and JSON handling for plugin integration
- *(lua)* integrate Lua backend with helper functions for text processing
- *(plugins)* add plugin management and enhance document handling
- *(dev)* implement test interaction tools and enhance debug overlays
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

- remove unnecessary deployment steps from release workflow
- update release workflow to enhance checkout and rust installation steps
- *(plugin)* fix event dispatching by passing references to event objects
- *(renderer)* correct canvas retrieval in WASM target and update App reference
- *(canvas)* increase pan friction to stop inertial glide sooner

### Other

- improve surface size calculation for wasm and update auto-save asset path
- *(deploy)* update wasm-pack build command to use release profile
- *(eraser)* simplify state management by consolidating press data structure
- rename padding field to meta and restructure tool states in plugin page
- runed cargo fmt
- runned cargo clippy --fix
- *(README)* update installation instructions and enhance feature descriptions
- *(main)* improve code formatting and readability in main.lua
- *(editor)* enhance text editor functionality with improved cursor handling, resizing, and command integration
- *(render)* simplify clipping logic and enhance shape handling
- *(cursor)* simplify cursor movement logic and enhance key handling
- *(manager, runtime, ui)* update argument passing to use slices for consistency and improved performance
- *(runtime)* update argument passing to use references for improved performance
- update compilation options to use PipelineCompilationOptions and standardize event handling arguments
- *(plugin)* pass references to event objects in plugin event handling
- *(runtime)* modularize backend initialization and enhance command handling with budget constraints
- update function signatures to accept slices instead of owned vectors
- runed cargo clippy --fix
- runed cargo fmt
- *(runtime)* introduce constants for limits and enhance validation logic in contributions
- *(package)* introduce constants for limits and simplify file handling logic
- *(manifest)* introduce constants for limits and enhance validation logic
- *(helpers)* improve layout handling and constants for better readability and maintainability
- *(input)* simplify event processing and state tracking in InputManager
- *(renderer)* streamline redraw function by utilizing FrameInput struct for improved parameter handling
- *(renderer)* simplify redraw function by introducing FrameInput struct for better parameter management
- update video in readme
- *(README)* update shape drawing and plugin functionality details
- *(manifest, package, platform, text_renderer, ui)* improve code readability and consistency
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
