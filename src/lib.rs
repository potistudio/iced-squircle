//! Squircle (superellipse-cornered) surfaces for iced.
//!
//! Because iced's `border.radius` only draws circular-arc corners, this crate renders a
//! superellipse path on a `canvas` and layers it beneath your content.
//!
//! ```no_run
//! use iced::widget::{container, text};
//! use iced::{Color, Element};
//! use iced_squircle::{Surface, squircle};
//!
//! const CARD: Surface = Surface {
//!     radius: 12.0,
//!     fill: Color::WHITE,
//!     border: Some(Color::BLACK),
//!     shadow: None,
//! };
//!
//! fn view<'a, Message: 'a>() -> Element<'a, Message> {
//!     squircle(container(text("Hello")).padding(16), CARD)
//! }
//! ```

use std::f32::consts::FRAC_PI_2;
use std::sync::LazyLock;

use iced::widget::canvas::{self, Cache, Geometry, Path, Stroke};
use iced::widget::{Stack, container};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Shadow, Theme, mouse};

/// Superellipse corner exponent. `2.0` is a circle; larger values approach a rectangle.
const EXPONENT: f32 = 5.0;
/// Ratio of superellipse corner extent to the radius of a circular arc of the same depth.
///
/// Along the diagonal, a circular arc cuts `0.414 r` off the corner while an exponent-5
/// superellipse cuts `0.183 R`, so `R = 2.25 r` matches the depth within 1%.
const EXTENT: f32 = 2.25;
/// Number of line segments per corner.
const STEPS: usize = 24;

/// Appearance of a squircle surface.
#[derive(Debug, Clone, Copy)]
pub struct Surface {
	/// Corner roundness. Sized so that the diagonal cut matches a circular arc of this radius,
	/// allowing values comparable to `border.radius` (though the curve starts farther from the
	/// corner than a circular arc). See [`path`] for the actual corner extent.
	pub radius: f32,
	pub fill: Color,
	pub border: Option<Color>,
	pub shadow: Option<Shadow>,
}

/// Layers a squircle surface beneath `content`, preserving `content`'s size.
pub fn squircle<'a, Message: 'a>(content: impl Into<Element<'a, Message>>, surface: Surface) -> Element<'a, Message> {
	let content = content.into();
	let size = content.as_widget().size_hint();
	let shape = canvas::Canvas::new(Shape(surface)).width(Fill).height(Fill);
	let layered = Stack::with_children([content])
		.width(size.width)
		.height(size.height)
		.push_under(shape);

	let Some(shadow) = surface.shadow else {
		return layered.into();
	};
	// canvas ではぼかせないので、影は同じ丸みの円弧の角の影で代用する (ぼけるので形の差はほぼ見えない)。
	container(layered)
		.style(move |_| container::Style {
			border: iced::border::rounded(surface.radius),
			shadow,
			..container::Style::default()
		})
		.into()
}

struct Shape(Surface);

impl<Message> canvas::Program<Message> for Shape {
	type State = Cache;

	fn draw(
		&self,
		cache: &Cache,
		renderer: &Renderer,
		_theme: &Theme,
		bounds: Rectangle,
		_cursor: mouse::Cursor,
	) -> Vec<Geometry> {
		let surface = self.0;
		vec![cache.draw(renderer, bounds.size(), |frame| {
			// 枠線は線の中心がパス上に来るので、半分だけ内側に寄せてはみ出さないようにする。
			let inset = if surface.border.is_some() { 0.5 } else { 0.0 };
			let rect = Rectangle {
				x: inset,
				y: inset,
				width: bounds.width - inset * 2.0,
				height: bounds.height - inset * 2.0,
			};
			let path = path(rect, surface.radius);
			frame.fill(&path, surface.fill);
			if let Some(border) = surface.border {
				frame.stroke(&path, Stroke::default().with_color(border).with_width(1.0));
			}
		})]
	}
}

/// Builds a squircle [`Path`] for `rect` with roundness `radius` (see [`Surface::radius`]).
///
/// Intended for drawing the outline yourself in a custom `canvas`.
///
/// `radius` is not the corner extent itself, but the radius of a circular arc of the same visual
/// depth; the curve actually starts `radius * 2.25` from each corner (clamped to half the shorter
/// side). Non-finite values of `radius` are treated as `0` (and panic in debug builds).
///
/// The path is rebuilt on every call, so draw it inside a [`canvas::Cache`] if redrawn every frame.
pub fn path(rect: Rectangle, radius: f32) -> Path {
	let points = outline(rect, radius);
	if let [first, rest @ ..] = points.as_slice()
		&& !rest.is_empty()
	{
		Path::new(|p| {
			p.move_to(*first);
			for point in rest {
				p.line_to(*point);
			}
			p.close();
		})
	} else {
		Path::rectangle(rect.position(), rect.size())
	}
}

/// Outline points for [`path`], ordered clockwise starting at the top-right corner, or empty if
/// there are no rounded corners.
fn outline(rect: Rectangle, radius: f32) -> Vec<Point> {
	debug_assert!(radius.is_finite(), "squircle radius must be finite, got {radius}");
	let radius = if radius.is_finite() { radius } else { 0.0 };
	let r = (radius * EXTENT).min(rect.width / 2.0).min(rect.height / 2.0).max(0.0);
	if r == 0.0 {
		return Vec::new();
	}
	let (left, top) = (rect.x + r, rect.y + r);
	let (right, bottom) = (rect.x + rect.width - r, rect.y + rect.height - r);
	// 各角は単位 1/4 曲線を符号で向きを変えて `r` 倍したもの。曲線は x 軸側から y 軸側へ進むので、
	// 時計回りにするには右上と左下では逆に辿る。
	let corners = [
		(Point::new(right, top), (1.0, -1.0), true),
		(Point::new(right, bottom), (1.0, 1.0), false),
		(Point::new(left, bottom), (-1.0, 1.0), true),
		(Point::new(left, top), (-1.0, -1.0), false),
	];

	let quarter = &*QUARTER;
	let mut points = Vec::with_capacity(corners.len() * quarter.len());
	for (center, (sx, sy), reversed) in corners {
		for step in 0..quarter.len() {
			let (x, y) = quarter[if reversed { quarter.len() - 1 - step } else { step }];
			let point = Point::new(center.x + sx * r * x, center.y + sy * r * y);
			// 辺の長さが 0 のとき (ピル形) は隣の角の端と重なるので、同じ点を打たない。
			if points.last() != Some(&point) {
				points.push(point);
			}
		}
	}
	points
}

/// Points along the first-quadrant unit superellipse `x^n + y^n = 1`, from `(1, 0)` to `(0, 1)`.
///
/// Sampled as `(cos^(2/n), sin^(2/n))` for `θ ∈ [0, π/2]`. Values are clamped at `0.0` because
/// `cos(π/2)` is slightly negative in `f32`.
static QUARTER: LazyLock<[(f32, f32); STEPS + 1]> = LazyLock::new(|| {
	let power = 2.0 / EXPONENT;
	std::array::from_fn(|step| {
		let (sin, cos) = (FRAC_PI_2 * step as f32 / STEPS as f32).sin_cos();
		(cos.max(0.0).powf(power), sin.max(0.0).powf(power))
	})
});

#[cfg(test)]
mod tests {
	use super::*;

	const RECT: Rectangle = Rectangle {
		x: 10.0,
		y: 20.0,
		width: 200.0,
		height: 100.0,
	};

	#[test]
	fn quarter_ends_exactly_on_the_axes() {
		assert_eq!(QUARTER[0], (1.0, 0.0));
		assert_eq!(QUARTER[STEPS], (0.0, 1.0));
		assert!(QUARTER.iter().all(|&(x, y)| x >= 0.0 && y >= 0.0));
	}

	#[test]
	fn corners_are_mirror_symmetric() {
		// 原点中心なら鏡映しは符号の反転になり、丸め誤差も対称に乗るので厳密に比べられる。
		let rect = Rectangle {
			x: -100.0,
			y: -50.0,
			width: 200.0,
			height: 100.0,
		};
		let points = outline(rect, 8.0);
		let n = STEPS + 1;
		assert_eq!(points.len(), n * 4);
		for (i, p) in points[..n].iter().enumerate() {
			// 右上 ↔ 右下は上下に、右上 ↔ 左上は左右に鏡映し (どちらも辿る向きは逆)。
			assert_eq!(points[2 * n - 1 - i], Point::new(p.x, -p.y));
			assert_eq!(points[4 * n - 1 - i], Point::new(-p.x, p.y));
			// 左下は点対称で、辿る向きは同じ。
			assert_eq!(points[2 * n + i], Point::new(-p.x, -p.y));
		}
	}

	#[test]
	fn radius_is_clamped_to_half_the_shorter_side() {
		let points = outline(RECT, 1000.0);
		let top = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
		let bottom = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
		assert_eq!((top, bottom), (RECT.y, RECT.y + RECT.height));
		// 左右の辺は長さ 0 なので、角どうしの継ぎ目の点が重複しない。
		assert!(points.windows(2).all(|w| w[0] != w[1]));
	}

	#[test]
	fn zero_radius_has_no_corners() {
		assert!(outline(RECT, 0.0).is_empty());
		assert!(outline(RECT, -3.0).is_empty());
	}

	#[test]
	#[cfg_attr(debug_assertions, should_panic(expected = "finite"))]
	fn nan_radius_is_treated_as_zero() {
		assert!(outline(RECT, f32::NAN).is_empty());
	}
}
