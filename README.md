# iced-squircle

> Squircle (superellipse-cornered) surfaces for [iced](https://github.com/iced-rs/iced).

This crate paints a superellipse (`|x|^n + |y|^n = 1`, with `n = 5` unless you change it) outline into a `canvas` to draw smooth rounded corners. Whereas a circular arc has constant curvature that drops abruptly to zero at a straight edge, a superellipse leaves each edge tangentially with zero curvature. It tightens to its sharpest point on the diagonal and loosens again into the next edge, so the corner has no visible seam where it starts or ends.

`iced-squircle` only supplies the background shape. Your content is placed on top of it unchanged, so give the content a transparent background unless you want it to hide the surface.

## Requirements

- Rust 1.100.0
- iced 0.14.0

> [!NOTE]
> This crate enables iced's `canvas` feature.

## Usage

```toml
[dependencies]
iced-squircle = { git = "https://github.com/potistudio/iced-squircle", tag = "v0.1.0" }
```

```rust
use iced::widget::{container, text};
use iced::{Color, Element, border};
use iced_squircle::{DEFAULT_EXPONENT, Surface, squircle};

fn card() -> Surface {
  Surface {
    radius: border::radius(12.0),
    exponent: DEFAULT_EXPONENT,
    fill: Color::WHITE,
    border: Some(Color::BLACK),
    shadow: None,
  }
}

fn view<'a, Message: 'a>() -> Element<'a, Message> {
  squircle(container(text("Hello")).padding(16), card())
}
```

## Rounding only some corners

`radius` is an [`iced::border::Radius`](https://docs.rs/iced/0.14.0/iced/border/struct.Radius.html): one value per corner, in the same order as iced's `border.radius`. Start from a base radius for all four corners, then override only the corners that differ:

```rust
radius: border::radius(12.0),               // all four corners
radius: border::radius(0.0).top(12.0),      // top two only
radius: border::radius(0.0).bottom(12.0),   // bottom two only
radius: border::radius(0.0).top_left(12.0), // top-left only
radius: border::radius(12.0).top_left(0.0), // all but the top-left
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

A corner left at `0.0` stays a sharp right angle, and the outline runs a straight edge from it to the next corner.

## How `radius` is measured

Each per-corner value is **not** the size of that corner. It is the radius of a circular arc that cuts the corner to the same depth, whatever the exponent is — until the clamp to half the shorter side binds, as described below.

Measured along the diagonal, a circular arc of radius `r` cuts `r * (√2 - 1)` (`0.414 r`) off the corner, while a superellipse of exponent `n` and corner extent `R` cuts `R * √2 * (1 - 2^(-1/n))` (`0.183 R` when `n = 5`). Matching the two depths fixes the extent:

```text
R = r * (1 - 1/√2) / (1 - 2^(-1/n))
```

That is `1.0 r` at `n = 2`, `2.26 r` at the default `n = 5`, and it keeps growing as the corner squares off. Each corner's curve therefore starts `radius * 2.26` away from the corner at the default exponent — farther out than a circular arc — yet reaches the same depth along the diagonal.

That extent is then clamped to half of the shorter side, which gives three cases per corner:

| Per-corner `radius`                         | Result for that corner                                        |
| ------------------------------------------- | ------------------------------------------------------------- |
| `<= 0` or non-finite                        | Sharp right angle; a non-finite value panics in debug builds. |
| `0 < radius * R/r < min(width, height) / 2` | Extent is `radius * R/r`.                                     |
| `radius * R/r >= min(width, height) / 2`    | Extent is clamped to `min(width, height) / 2`.                |

Every corner is clamped to half of the *shorter* side, so adjacent extents never overlap. At the limit they meet, producing a pill (or a closed superellipse for a square). Corners with different extents are joined by a straight edge, which is shorter than the side whenever either corner is rounded.

## The exponent

`exponent` is the `n` in `|x|^n + |y|^n = 1`. It decides how tightly the curve turns toward its corner point:

| `exponent` | Corner                                                        |
| ---------- | ------------------------------------------------------------- |
| `1`        | Straight chamfer between the two edges.                       |
| `2`        | Circular arc — the corner `border.radius` draws.              |
| `5`        | The default (`DEFAULT_EXPONENT`): the classic squircle shape. |
| toward `∞` | Approaches a plain right angle.                               |

Values below `1` are raised to `1`, and `NaN` falls back to `DEFAULT_EXPONENT`, so no input can produce a broken outline. A positive infinite exponent (`f32::INFINITY`) is allowed and produces a plain rectangle.

Because `radius` keeps its meaning at every exponent, raising the exponent makes each rounded corner consume more of its two edges while keeping the same depth:

| `exponent` | Corner extent per unit of `radius` |
| ---------- | ---------------------------------- |
| `2`        | `1.00`                             |
| `3`        | `1.42`                             |
| `4`        | `1.84`                             |
| `5`        | `2.26`                             |
| `6`        | `2.68`                             |
| `8`        | `3.53`                             |

Keep that in mind when a corner must not swallow much of the widget: the clamp to half the shorter side arrives sooner at a higher exponent.

## How `squircle()` builds the element

`squircle(content, surface)` returns an `Element` that layers a full-size `canvas` under `content` with `Stack::push_under`. The canvas fills the bounds that `content` resolves to, and `push_under` keeps it out of the intrinsic sizing, so the surface is exactly the size of `content` and adds no padding, no margin, and no gap.

Both axes of the stack receive the same `Length` as `content`, so `Fill`, `Shrink`, and `Fixed` content all keep their own sizing strategy.

The canvas caches its geometry and redraws only when its size or the `Surface` changes, so an unchanged surface avoids rebuilding the path on every frame.

The fields of `Surface` behave as follows:

| Field      | Behavior                                                                                                                                                                                                     |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `radius`   | Corner roundness, per corner, as described above.                                                                                                                                                            |
| `exponent` | Corner tightness, as described above. Usually `DEFAULT_EXPONENT`; together with `radius`, it sets how far each corner extends along its edges.                                                               |
| `fill`     | Always painted. There is no "no background" mode, so use `Color::TRANSPARENT` to draw the border alone.                                                                                                      |
| `border`   | Optional 1px stroke. When `Some`, the path is inset by 0.5px on all sides so the stroke stays inside the bounds, which also pulls the fill in by 0.5px. A thicker border needs `path()` and your own canvas. |
| `shadow`   | Optional drop shadow (see below).                                                                                                                                                                            |

A canvas cannot blur, so a shadow cannot follow the superellipse. When `shadow` is `Some`, the result is wrapped in a padding-free `container`, and the shadow is left to iced, which casts it from a rounded rectangle whose circular corners use the same per-corner radii and therefore the same depth as the superellipse. The blur hides the difference, but the shape of the shadow is approximated rather than exact. This wrapper is the only thing `shadow` changes: you get a `container` instead of the bare `Stack`, at the same size.

One thing `squircle()` does not do is clip. The surface is only a background, so content that extends all the way into a corner — a background color, an image, or its own border — will paint over the curve with square corners. Keep such content inside the padding.

## Drawing the path yourself

`Surface::path(rect)` returns the outline `squircle()` paints as a `canvas::Path` for the same `rect` (when `border` is `Some`, `squircle()` first insets it by 0.5px), so you can draw it in your own `canvas::Program` — for example, with a different fill, a gradient, a thicker stroke, or other geometry in the same frame. `path(rect, radius)` returns the same outline at `DEFAULT_EXPONENT`, and `path_with_exponent(rect, radius, exponent)` lets you choose the exponent. In both functions, `radius` accepts either a `Radius` (like `Surface::radius`) or a single `f32` for all four corners.

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
- When stroking, inset `rect` by half the stroke width; otherwise, the stroke is clipped at the bounds.
- The outline is a polygon with 24 line segments per rounded corner; a corner left at `radius <= 0` is a single sharp point.
- A non-finite `radius` panics via `debug_assert!` in debug builds and is treated as `0` in release builds.

## License

This project is licensed under the [MIT License](LICENSE).
