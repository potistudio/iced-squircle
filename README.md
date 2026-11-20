# iced-squircle

> Squircle (superellipse-cornered) surfaces for [iced](https://github.com/iced-rs/iced).

iced's `border.radius` only draws circular-arc corners. This crate draws a superellipse path on a canvas and lays it under your content.

## Usage

```toml
[dependencies]
iced-squircle = { git = "https://github.com/potistudio/iced-squircle" }
```

```rust
use iced::widget::text;
use iced::{Color, Element};
use iced_squircle::{Surface, squircle};

const CARD: Surface = Surface {
  radius: 12.0,
  fill: Color::WHITE,
  border: Some(Color::BLACK),
  shadow: None,
};

fn view<'a, Message: 'a>() -> Element<'a, Message> {
  squircle(text("Hello"), CARD)
}
```

`radius` is chosen so the corner looks as deep as a circular arc of the same radius. Use `iced_squircle::path` to draw the outline on your own canvas.

## Requirements

- Rust 1.99.0
- iced 0.14

## License

[MIT](LICENSE)
