# iced-squircle

> Squircle (superellipse-cornered) surfaces for [iced](https://github.com/iced-rs/iced).

This crate paints a superellipse outline (`|x|^n + |y|^n = 1`, with `n = 5` unless you change it) into a `canvas` to draw smooth rounded corners. Whereas a circular arc has constant curvature that drops abruptly to zero at a straight edge, a superellipse leaves each edge tangentially with zero curvature. It tightens to its sharpest point on the diagonal and loosens again into the next edge, so the corner has no visible seam where it starts or ends.

`iced-squircle` only supplies the background shape. Your content is placed on top of it unchanged, so give the content a transparent background unless you want it to hide the surface.

## Requirements

- Rust 1.88 or later
- iced 0.14

> **Note:** This crate depends on `iced_widget` with its `canvas` feature and enables no other iced feature, so your application keeps control over iced's renderers, executor, and platform features. `iced` and `iced_widget` re-export the same underlying types, so you pass `iced::Color`, `iced::border::Radius`, and `iced::Element` as usual.

## Usage

```toml
[dependencies]
iced = "0.14"
iced-squircle = "1.0"
```

```rust
use iced::widget::{container, text};
use iced::{Color, Element};
use iced_squircle::{Surface, squircle};

fn card() -> Surface {
  Surface::default()
    .radius(12.0)
    .fill(Color::WHITE)
    .border(Color::BLACK, 1.0)
}

fn view<'a, Message: 'a>() -> Element<'a, Message> {
  squircle(container(text("Hello")).padding(16), card())
}
```

`Surface::default()` is a sharp, transparent surface at `DEFAULT_EXPONENT` with no border and no shadow. Set what you need with the builder methods (`radius`, `exponent`, `fill`, `border`, `shadow`), or with struct update syntax:

```rust
use iced::border;
use iced_squircle::Outline;

Surface {
  radius: border::radius(12.0),
  fill: Color::WHITE,
  border: Some(Outline::new(Color::BLACK, 1.0)),
  ..Surface::default()
}
```

`squircle()` works with any theme that implements `container::Catalog`, including your own, and with any renderer that implements iced's geometry `Renderer`.

Run the interactive gallery to try the options:

```sh
cargo run --example gallery
```

## Rounding only some corners

`radius` is an [`iced::border::Radius`](https://docs.rs/iced/0.14.0/iced/border/struct.Radius.html): one value per corner, the same type iced's `border.radius` uses. Start from a base radius for all four corners, then override only the corners that differ:

```rust
.radius(border::radius(12.0))               // all four corners
.radius(border::radius(0.0).top(12.0))      // top two only
.radius(border::radius(0.0).bottom(12.0))   // bottom two only
.radius(border::radius(0.0).top_left(12.0)) // top-left only
.radius(border::radius(12.0).top_left(0.0)) // all but the top-left
```

Setters name the corners they cover:

| Setter         | Corners set                  |
| -------------- | ---------------------------- |
| `top`          | top-left and top-right       |
| `bottom`       | bottom-left and bottom-right |
| `left`         | top-left and bottom-left     |
| `right`        | top-right and bottom-right   |
| `top_left`     | top-left                     |
| `top_right`    | top-right                    |
| `bottom_left`  | bottom-left                  |
| `bottom_right` | bottom-right                 |

iced also provides `border::top(r)`, `border::bottom(r)`, `border::left(r)`, `border::right(r)`, `border::top_left(r)`, `border::top_right(r)`, `border::bottom_left(r)`, and `border::bottom_right(r)` as shorthands for a base of `border::radius(0.0)` plus one setter. They produce the same values, so this crate draws them identically; the examples above use one form throughout.

A corner left at `0.0` stays a sharp right angle, and the outline runs a straight edge from its corner point to where the next corner begins.

## How `radius` is measured

Each per-corner value is the length that corner consumes from each of its two adjacent edges: the curve starts `radius` away from the corner point on both edges. That is the quantity `border.radius` sets for iced's own corners, and at `exponent: 2.0` the corner is the same circular arc. The value is independent of the exponent, so changing the exponent never moves where a corner begins.

At exponents above `2.0`, a superellipse stays closer to its corner point than a circular arc of the same radius. Measured along the diagonal, an arc of radius `r` sits `r * (√2 - 1)` (`0.414 r`) from the corner point, while a superellipse of exponent `n` and the same radius `r` (before clamping) sits `r * √2 * (1 - 2^(-1/n))` from it:

| `exponent` | Diagonal cut per unit of `radius` |
| ---------- | --------------------------------- |
| `2`        | `0.414`                           |
| `3`        | `0.292`                           |
| `5`        | `0.183`                           |
| `8`        | `0.117`                           |

Each corner is then clamped to half of the shorter side, which gives these cases per corner:

| Per-corner `radius`                   | Result for that corner                                                                           |
| ------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `<= 0` (including `-∞`)               | Sharp right angle.                                                                               |
| `NaN`                                 | Sharp right angle; panics in debug builds.                                                       |
| `0 < radius < min(width, height) / 2` | Extent is `radius`.                                                                              |
| `radius >= min(width, height) / 2`    | Extent is clamped to `min(width, height) / 2`.                                                   |
| `+∞`                                  | Extent is clamped to `min(width, height) / 2`; `f32::INFINITY` on all four corners draws a pill. |

Every corner is clamped to half of the *shorter* side, so adjacent extents never overlap. At the limit they meet, producing a pill (or a closed superellipse for a square), as `f32::INFINITY` does for iced's own `border.radius`. Adjacent rounded corners are joined by a straight edge along the part of the side their extents leave uncovered, which is shorter than the side whenever either corner is rounded.

## The exponent

`exponent` is the `n` in `|x|^n + |y|^n = 1`. It decides how tightly the curve turns toward its corner point:

| `exponent` | Corner                                                        |
| ---------- | ------------------------------------------------------------- |
| `1`        | Straight chamfer between the two edges.                       |
| `2`        | Circular arc — the corner `border.radius` draws.              |
| `5`        | The default (`DEFAULT_EXPONENT`): the classic squircle shape. |
| toward `∞` | Approaches a plain right angle.                               |

Values below `1` are raised to `1`, and `NaN` falls back to `DEFAULT_EXPONENT`, so no input can produce a broken outline. Positive infinity (`f32::INFINITY`) is allowed and produces a plain rectangle.

The exponent changes the curve's shape only. A corner keeps the extent `radius` gives it and moves closer to its corner point as the exponent rises, so the diagonal cut listed above shrinks; the clamp threshold stays at half of the shorter side.

## The border

`border` is an optional `Outline { color, width }`, set with `.border(color, width)` or `Some(Outline::new(color, width))`. The border runs along the inside of the outline:

- Its outer edge is the outline itself, so the surface keeps the same size and shape with or without a border.
- Its inner edge lies `width` pixels inward, measured perpendicular to the outline at every point, so the border keeps the same thickness around the corners.
- It is painted over the fill, so a translucent border shows the fill through it, as a CSS border shows its background.

A `width` at or below `0`, or `NaN`, draws no border. A `width` of half the shorter side or more covers the whole surface with the border color.

## How `squircle()` builds the element

`squircle(content, surface)` returns an `Element` that layers a full-size `canvas` under `content` with `Stack::push_under`, inside a padding-free `container`. The canvas fills the bounds that `content` resolves to, and `push_under` keeps it out of the intrinsic sizing, so the surface is exactly the size of `content` and adds no padding, no margin, and no gap.

The stack and the container both receive the same width and height `Length` as `content`, so `Fill`, `FillPortion`, `Shrink`, and `Fixed` content all keep their own sizing strategy.

The widget tree has the same shape whether or not `shadow` is set: the container is always there and draws nothing without a shadow. Toggling the shadow at runtime, for example to lift a card on hover, therefore keeps the state of the widgets inside `content`, such as a focused text input.

The canvas caches its geometry and redraws only when its size or the drawn part of the `Surface` (`radius`, `exponent`, `fill`, `border`) changes, so an unchanged surface avoids rebuilding the path on every frame.

The fields of `Surface` behave as follows:

| Field      | Behavior                                                                                                                                 |
| ---------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `radius`   | Corner roundness, per corner, as the length consumed from each adjacent edge.                                                            |
| `exponent` | Corner tightness. Usually `DEFAULT_EXPONENT`; it changes the curve's shape only and leaves the corner extent set by `radius` unchanged.  |
| `fill`     | Painted inside the outline. Use `Color::TRANSPARENT` to draw the border alone.                                                           |
| `border`   | Optional border of any width, drawn inside the outline over the fill (see above).                                                        |
| `shadow`   | Optional drop shadow (see below).                                                                                                        |

### The shadow

A canvas cannot blur, so a shadow cannot follow the superellipse exactly. The container casts it instead, from a rounded rectangle with circular corners sized to stay inside the outline:

- At an exponent of `2.0` or more, each arc keeps the corner's `radius`. An arc of radius `r` cuts `0.414 r` off its corner, at least as deep as the superellipse (`0.183 r` at the default), so the shadow stays inside the surface and only its blur and offset spread outward.
- Below `2.0`, the superellipse cuts deeper than an arc of the same radius, so the arc's radius grows by a factor of up to `1.71` (to `1.71 r` for a chamfer at `exponent: 1.0`) until it cuts as deep.
- The renderer clamps the arc's radius to half of the shorter side. Once that clamp applies, which below `2.0` happens before the corner itself reaches the limit, the arc no longer cuts as deep as the superellipse, and the shadow's corner sticks out of the outline. The worst case is a chamfer with every corner at half of the shorter side `h`: the shadow sticks out by `0.29 h` along the diagonal, which a blur softens but does not hide. To keep the shadow inside, keep each corner's radius at or below `h` divided by that growth factor, for example `0.59 h` for a chamfer.

The shadow is meant for an opaque `fill`. Through a translucent fill, the shadow shows in the corners between the arc and the superellipse, and with the `tiny-skia` renderer it shows beneath the whole surface.

### Clipping

`squircle()` does not clip. The surface is only a background, so content that extends all the way into a corner — a background color, an image, or its own border — will paint over the curve with square corners. Keep such content inset with your own padding.

## Drawing the path yourself

`Surface::path(rect)` returns the outline `squircle()` fills for `rect` as a `canvas::Path`, so you can draw it in your own `canvas::Program` — for example, with a gradient or other geometry in the same frame. `path(rect, radius)` returns the outline at `DEFAULT_EXPONENT`, and `path_with_exponent(rect, radius, exponent)` lets you choose the exponent. In both functions, `radius` accepts either a `Radius` (like `Surface::radius`) or a single `f32` for all four corners.

```rust
// Inside canvas::Program::draw for a 200x100 frame, inset by 0.5px for a 1px stroke
let path = iced_squircle::path(
  Rectangle { x: 0.5, y: 0.5, width: 199.0, height: 99.0 },
  12.0,
);

frame.stroke(&path, Stroke::default().with_width(1.0));
```

Keep these in mind:

- The path is rebuilt on every call, so draw it inside a `canvas::Cache` if the canvas is redrawn every frame.
- A canvas stroke is centered on the path, so inset `rect` by half the stroke width; otherwise, the stroke is clipped at the bounds. If you use `squircle()`, `Surface::border` avoids manual insetting by drawing a uniform-width border inside the outline.
- The outline is a polygon with 24 line segments per rounded corner; a corner left at `radius <= 0` is a single sharp point.
- A `NaN` `radius` panics via `debug_assert!` in debug builds and is treated as `0` in release builds.

## Upgrading from 0.1

See [CHANGELOG.md](CHANGELOG.md) for every change. In short:

- `radius` is an `iced::border::Radius` with one value per corner. A plain `f32` still converts with `.radius(12.0)` or `12.0.into()`.
- `radius` is the corner extent itself, no longer the radius of a circular arc that cuts the corner to the same depth. `radius: 12.0` now consumes 12px of each adjacent edge, where the same value covered 27px in 0.1. To keep the 0.1 shape, multiply the value by `2.25`.
- The new `exponent` field sets the superellipse exponent; 0.1 always used `5.0`, which is `DEFAULT_EXPONENT`.
- `border: Some(color)` becomes `border: Some(Outline::new(color, 1.0))`, or `.border(color, 1.0)`. The border is drawn inside the full outline instead of along an outline inset by 0.5px, so the fill no longer shrinks when a border is set.
- `f32::INFINITY` in `radius` draws a pill instead of a sharp corner.
- Build a `Surface` with `Surface::default()` and the builder methods, or with `..Surface::default()` in a struct literal, so later fields do not break your code.

## License

This project is licensed under the [MIT License](LICENSE).
