# iced-squircle

> Squircle (superellipse-cornered) surfaces for [iced](https://github.com/iced-rs/iced).

This crate paints a superellipse (`|x|^5 + |y|^5 = 1`) outline into a `canvas` to draw smooth rounded corners. Whereas a circular arc has constant curvature that drops abruptly to zero at a straight edge, a superellipse leaves each edge tangentially with zero curvature. It tightens to its sharpest point on the diagonal and loosens again into the next edge, so the corner has no visible seam where it starts or ends.

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
use iced::{Color, Element};
use iced_squircle::{Surface, squircle};

const CARD: Surface = Surface {
  radius: 12.0,
  fill: Color::WHITE,
  border: Some(Color::BLACK),
  shadow: None,
};

fn view<'a, Message: 'a>() -> Element<'a, Message> {
  squircle(container(text("Hello")).padding(16), CARD)
}
```

## How `radius` is measured

`radius` is **not** the size of the corner. It is the radius of a circular arc that cuts the corner to the same depth.

Measured along the diagonal, a circular arc of radius `r` cuts `0.414 r` off the corner, while an exponent-5 superellipse cuts `0.183 R` off a corner of extent `R`. An exact match would need `R = 2.26 r`; the crate uses `2.25`, which lands within 1% of the same depth. The curve therefore starts `radius * 2.25` away from each corner — farther out than a circular arc — yet reaches the same depth at the diagonal.

That extent is then clamped to half of the shorter side, which gives three cases:

| `radius`                                     | Result                                                                                                          |
| -------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `<= 0`                                       | Plain rectangle (no rounded corners).                                                                           |
| `0 < radius * 2.25 < min(width, height) / 2` | Corner extent is `radius * 2.25`.                                                                               |
| `radius * 2.25 >= min(width, height) / 2`    | Corner extent is clamped to `min(width, height) / 2`, producing a pill (or a closed superellipse for a square). |

The exponent is fixed at `5` and is not configurable. Likewise, `radius` applies uniformly to all four corners, with no per-corner settings.

## How `squircle()` builds the element

`squircle(content, surface)` returns an `Element` that layers a full-size `canvas` under `content` with `Stack::push_under`. The canvas fills the bounds that `content` resolves to, and `push_under` keeps it out of the intrinsic sizing, so the surface is exactly the size of `content` and adds no padding, no margin, and no gap.

Both axes of the stack receive the same `Length` as `content`, so `Fill`, `Shrink`, and `Fixed` content all keep their own sizing strategy.

The fields of `Surface` behave as follows:

| Field    | Behavior                                                                                                                                                                                                     |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `radius` | Corner shape, as described above.                                                                                                                                                                            |
| `fill`   | Always painted. There is no "no background" mode, so use `Color::TRANSPARENT` to draw the border alone.                                                                                                      |
| `border` | Optional 1px stroke. When `Some`, the path is inset by 0.5px on all sides so the stroke stays inside the bounds, which also pulls the fill in by 0.5px. A thicker border needs `path()` and your own canvas. |
| `shadow` | Optional drop shadow (see below).                                                                                                                                                                            |

A canvas cannot blur, so a shadow cannot follow the superellipse. When `shadow` is `Some`, the result is wrapped in a padding-free `container`, and the shadow is left to iced, which casts it from a rounded rectangle whose circular corners have the same depth as the superellipse. The blur hides the difference, but the shape of the shadow is approximated rather than exact. This wrapper is the only thing `shadow` changes: you get a `container` instead of the bare `Stack`, at the same size.

One thing `squircle()` does not do is clip. The surface is only a background, so content that extends all the way into a corner — a background color, an image, or its own border — will paint over the curve with square corners. Keep such content inside the padding.

## Drawing the path yourself

`path(rect, radius)` returns the same outline as a `canvas::Path`, so you can draw it in your own `canvas::Program` — for example, with a different fill, a gradient, a thicker stroke, or other geometry in the same frame.

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
- Inset `rect` by half the stroke width; otherwise, the stroke is clipped at the bounds.
- The outline is a polygon with 24 line segments per corner.
- A non-finite `radius` panics via `debug_assert!` in debug builds and is treated as `0` in release builds.

## License

This project is licensed under the [MIT License](LICENSE).
