# Changelog

All notable changes to this project are documented in this file. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-11-21

### Added

- `Surface::exponent` and `path_with_exponent()` set the superellipse exponent; `DEFAULT_EXPONENT` (`5.0`) is the exponent 0.1 always used.
- `Outline { color, width }` gives `Surface::border` a configurable width. The border runs along the inside of the outline with the same thickness around the corners and is painted over the fill.
- `Surface` implements `Default` and `PartialEq`, and has the builder methods `radius`, `exponent`, `fill`, `border`, and `shadow`.
- `squircle()` is generic over the theme and the renderer, so it works with any theme that implements `container::Catalog` and any renderer that implements iced's geometry `Renderer`.
- The crate is published on crates.io.
- An interactive example: `cargo run --example gallery`.
- Continuous integration for formatting, lints, tests, documentation, and the minimum supported Rust version.

### Changed

- **Breaking:** `Surface::radius` is an `iced::border::Radius`, so each corner is rounded independently. `path()` accepts an `f32` or a `Radius`.
- **Breaking:** `radius` is the length each corner consumes from its two adjacent edges, as for iced's `border.radius`, instead of the radius of a circular arc that cuts the corner to the same depth. Multiply 0.1 values by `2.25` to keep their shape.
- **Breaking:** `Surface::border` is an `Option<Outline>` instead of an `Option<Color>`. The border is drawn inside the full outline instead of along an outline inset by 0.5px, so setting a border no longer shrinks the fill.
- **Breaking:** `f32::INFINITY` in `radius` draws a pill, as it does for iced's `border.radius`, instead of a sharp corner that panics in debug builds. Only `NaN` still panics in debug builds.
- **Breaking:** `squircle()` always wraps its result in a `container`, so adding or removing a shadow no longer changes the widget tree and no longer resets the state of the widgets inside the content.
- The crate depends on `iced_widget` with only its `canvas` feature instead of `iced` with its default features, so applications keep control over iced's renderers, executor, and platform features. The types are the same ones `iced` re-exports.
- The minimum supported Rust version is 1.88, the same as iced 0.14.
- Below an exponent of `2.0`, the shadow's corner radius grows so the shadow stays inside the flatter corner, as long as the renderer does not clamp the grown radius to half of the shorter side.

### Fixed

- Changing a `Surface` while its size stays the same now redraws the canvas.
- A `NaN` in a `Surface` no longer clears the canvas cache on every frame.
- `squircle()` with a shadow keeps `FillPortion` content at its portion instead of turning it into `Fill`.
- A negative radius no longer distorts the shadow's corner; the shadow keeps the sharp corner the outline draws.
- A canvas with no area draws nothing instead of an inverted rectangle.
- Content with a `Fixed(0.0)` width or height is no longer dropped by the stack together with the surface.

## [0.1.0]

- Initial release: `Surface`, `squircle()`, and `path()` with a fixed exponent of `5.0`.

[1.0.0]: https://github.com/potistudio/iced-squircle/compare/v0.1.0...v1.0.0
[0.1.0]: https://github.com/potistudio/iced-squircle/releases/tag/v0.1.0
