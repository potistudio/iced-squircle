# iced-squircle

> Squircle (superellipse-cornered) surfaces for [iced](https://github.com/iced-rs/iced).

iced's `border.radius` only draws circular-arc corners. This crate draws a superellipse path on a canvas and places it behind your content.

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

Specify `radius` like iced's `border.radius`: the corner is sized so it looks as deep as a circular arc of that radius. The curve starts `radius * 2.25` from each corner, clamped to half the width or height (whichever is smaller), so small surfaces get shallower corners.

Use `iced_squircle::path(rect, radius)` to build the same `Path` for your own canvas. It rebuilds the path on every call, so draw it inside a `canvas::Cache` if the canvas is redrawn every frame. When stroking a border, inset `rect` by half the stroke width so the stroke is not clipped.

## Requirements

- Rust 1.99.0
- iced 0.14

## License

[MIT](LICENSE)
