//! An interactive gallery of squircle surfaces.
//!
//! Run it with `cargo run --example gallery`, then use the sliders and toggles to compare exponents, radii,
//! border widths, and shadows.

use iced::widget::{column, container, row, slider, text, toggler};
use iced::{Color, Element, Fill, Shadow, Theme, Vector, border};
use iced_squircle::{Surface, squircle};

fn main() -> iced::Result {
	iced::application(Gallery::default, Gallery::update, Gallery::view)
		.title("iced-squircle gallery")
		.theme(Gallery::theme)
		.window_size((960.0, 640.0))
		.run()
}

struct Gallery {
	radius: f32,
	exponent: f32,
	border_width: f32,
	shadow: bool,
	dark: bool,
}

impl Default for Gallery {
	fn default() -> Self {
		Self {
			radius: 24.0,
			exponent: iced_squircle::DEFAULT_EXPONENT,
			border_width: 1.0,
			shadow: true,
			dark: false,
		}
	}
}

#[derive(Debug, Clone, Copy)]
enum Message {
	Radius(f32),
	Exponent(f32),
	BorderWidth(f32),
	Shadow(bool),
	Dark(bool),
}

impl Gallery {
	fn update(&mut self, message: Message) {
		match message {
			Message::Radius(radius) => self.radius = radius,
			Message::Exponent(exponent) => self.exponent = exponent,
			Message::BorderWidth(width) => self.border_width = width,
			Message::Shadow(shadow) => self.shadow = shadow,
			Message::Dark(dark) => self.dark = dark,
		}
	}

	fn theme(&self) -> Theme {
		if self.dark { Theme::Dark } else { Theme::Light }
	}

	fn view(&self) -> Element<'_, Message> {
		let palette = *self.theme().extended_palette();
		let base = Surface::default()
			.radius(self.radius)
			.exponent(self.exponent)
			.fill(palette.background.weak.color)
			.border(palette.background.strong.color, self.border_width);
		let base = if self.shadow {
			base.shadow(Shadow {
				color: Color::from_rgba(0.0, 0.0, 0.0, 0.25),
				offset: Vector::new(0.0, 4.0),
				blur_radius: 12.0,
			})
		} else {
			base
		};

		let card = |label: &'static str, surface: Surface| {
			squircle(
				container(text(label))
					.padding(16)
					.width(Fill)
					.height(140)
					.center_x(Fill)
					.center_y(140),
				surface,
			)
		};

		let cards = row![
			card("All corners", base),
			card("Top only", base.radius(border::radius(0.0).top(self.radius))),
			card("Pill", base.radius(f32::INFINITY)),
			card(
				"Accent",
				base.fill(palette.primary.base.color)
					.border(palette.primary.strong.color, self.border_width)
			),
		]
		.spacing(24);

		let compare = row![
			card("Circular arc (2.0)", base.exponent(2.0)),
			card("Squircle (5.0)", base.exponent(5.0)),
			card("Tight (12.0)", base.exponent(12.0)),
		]
		.spacing(24);

		let controls = column![
			labeled(
				format!("radius: {:.0}", self.radius),
				slider(0.0..=80.0, self.radius, Message::Radius)
			),
			labeled(
				format!("exponent: {:.1}", self.exponent),
				slider(1.0..=12.0, self.exponent, Message::Exponent).step(0.1_f32)
			),
			labeled(
				format!("border width: {:.1}", self.border_width),
				slider(0.0..=12.0, self.border_width, Message::BorderWidth).step(0.5_f32)
			),
			row![
				toggler(self.shadow).label("shadow").on_toggle(Message::Shadow),
				toggler(self.dark).label("dark theme").on_toggle(Message::Dark),
			]
			.spacing(24),
		]
		.spacing(12);

		container(column![controls, cards, text("Exponent comparison"), compare].spacing(24))
			.padding(32)
			.into()
	}
}

fn labeled<'a>(label: String, control: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
	row![text(label).width(160), control.into()].spacing(12).into()
}
