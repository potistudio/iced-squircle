//! Squircle (superellipse-cornered) surfaces for iced.
//!
//! Because iced's `border.radius` only draws circular-arc corners, this crate renders a
//! superellipse path on a `canvas` and layers it beneath your content.
//!
//! ```no_run
//! use iced::widget::{container, text};
//! use iced::{Color, Element, border};
//! use iced_squircle::{DEFAULT_EXPONENT, Surface, squircle};
//!
//! fn card() -> Surface {
//!     Surface {
//!         // Round the top corners only.
//!         radius: border::radius(0.0).top(12.0),
//!         exponent: DEFAULT_EXPONENT,
//!         fill: Color::WHITE,
//!         border: Some(Color::BLACK),
//!         shadow: None,
//!     }
//! }
//!
//! fn view<'a, Message: 'a>() -> Element<'a, Message> {
//!     squircle(container(text("Hello")).padding(16), card())
//! }
//! ```

use std::cell::Cell;
use std::f32::consts::FRAC_PI_2;

use iced::border::{self, Radius};
use iced::widget::canvas::{self, Cache, Geometry, Path, Stroke};
use iced::widget::{Stack, container};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Shadow, Theme, mouse};

/// Default superellipse exponent (`5.0`), producing the classic squircle. Used by [`path`] and
/// typically passed to [`Surface::exponent`]; `2.0` is a circular arc, and larger values approach a
/// right angle.
pub const DEFAULT_EXPONENT: f32 = 5.0;
/// Number of line segments per corner.
const STEPS: usize = 24;

/// Appearance of a squircle surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Surface {
	/// Corner roundness, per corner: the length that corner consumes from each of its two adjacent
	/// edges. `12.0` makes the curve start 12px from the corner point on both edges, matching the
	/// corner extent iced's `border.radius` uses (at an exponent of `2.0` the corner is the same
	/// circular arc). The value does not depend on [`exponent`](Surface::exponent).
	///
	/// Each corner is clamped to half of the shorter side. A value at or below `0.0` stays sharp, and
	/// a non-finite value is treated as `0.0` (and panics in debug builds).
	///
	/// Build it from a base radius and override the corners that differ:
	/// `border::radius(12.0)` rounds all four, `border::radius(0.0).top(12.0)` only the top two,
	/// and `border::radius(12.0).bottom(0.0)` only the top two as well. Helpers in [`iced::border`]
	/// such as `border::top(12.0)` are shorthands for the same values.
	pub radius: Radius,
	/// Superellipse exponent: how tightly the curve turns toward its corner point. `2.0` is a circular
	/// arc, and larger values approach a right angle; [`DEFAULT_EXPONENT`] (`5.0`) is the classic
	/// squircle.
	///
	/// The exponent changes the curve's shape without changing the corner's footprint. The distance
	/// from the corner point to the curve along the diagonal is `0.414` times the radius at `2.0` (a
	/// circular arc) and `0.183` times at `5.0`, so a higher exponent keeps the corner closer to a
	/// right angle.
	///
	/// Values below `1.0` are raised to `1.0`, where the corner becomes a straight chamfer, and
	/// `NaN` falls back to [`DEFAULT_EXPONENT`]. Positive infinity (`f32::INFINITY`) is allowed:
	/// it collapses each corner to a right angle and draws a plain rectangle.
	pub exponent: f32,
	pub fill: Color,
	pub border: Option<Color>,
	pub shadow: Option<Shadow>,
}

impl Surface {
	/// Builds this surface's outline for `rect`. [`squircle`] draws this path beneath your content,
	/// first insetting `rect` by 0.5px to keep the stroke inside when [`border`](Surface::border) is
	/// `Some`; this method uses `rect` as given.
	pub fn path(&self, rect: Rectangle) -> Path {
		path_with_exponent(rect, self.radius, self.exponent)
	}
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
	// The path draws a non-finite corner as a sharp one, so `container` must receive the same value
	// instead of a NaN it cannot round.
	let radius = Radius {
		top_left: finite_or_zero(surface.radius.top_left),
		top_right: finite_or_zero(surface.radius.top_right),
		bottom_right: finite_or_zero(surface.radius.bottom_right),
		bottom_left: finite_or_zero(surface.radius.bottom_left),
	};
	// A canvas cannot blur, so `container` draws the shadow. It rounds corners with circular arcs of
	// the same per-corner radius, which cut deeper than the superellipse at every exponent above
	// `2.0` and therefore stay inside it; the blur and the offset spread the shadow outward.
	container(layered)
		.style(move |_| container::Style {
			border: border::rounded(radius),
			shadow,
			..container::Style::default()
		})
		.into()
}

struct Shape(Surface);

/// Cached geometry plus the surface it was drawn from, so a changed surface redraws at the same size.
#[derive(Default)]
struct ShapeState {
	cache: Cache,
	surface: Cell<Option<Surface>>,
}

impl<Message> canvas::Program<Message> for Shape {
	type State = ShapeState;

	fn draw(
		&self,
		state: &ShapeState,
		renderer: &Renderer,
		_theme: &Theme,
		bounds: Rectangle,
		_cursor: mouse::Cursor,
	) -> Vec<Geometry> {
		let surface = self.0;
		// A `Cache` only redraws when its size changes, so clear it whenever the surface itself does.
		if state.surface.get() != Some(surface) {
			state.cache.clear();
			state.surface.set(Some(surface));
		}
		vec![state.cache.draw(renderer, bounds.size(), |frame| {
			// Strokes are centered on the path, so inset by half the stroke width to stay inside the bounds.
			let inset = if surface.border.is_some() { 0.5 } else { 0.0 };
			let rect = Rectangle {
				x: inset,
				y: inset,
				width: bounds.width - inset * 2.0,
				height: bounds.height - inset * 2.0,
			};
			let path = surface.path(rect);
			frame.fill(&path, surface.fill);
			if let Some(border) = surface.border {
				frame.stroke(&path, Stroke::default().with_color(border).with_width(1.0));
			}
		})]
	}
}

/// Builds a squircle [`Path`] for `rect` with roundness `radius` (an `f32` for all four corners or
/// a [`Radius`]; see [`Surface::radius`]) and exponent [`DEFAULT_EXPONENT`]. See
/// [`path_with_exponent`] to use a different exponent.
///
/// The path is rebuilt on every call, so draw it inside a [`canvas::Cache`] if the canvas is redrawn
/// every frame.
pub fn path(rect: Rectangle, radius: impl Into<Radius>) -> Path {
	path_with_exponent(rect, radius, DEFAULT_EXPONENT)
}

/// Builds a superellipse [`Path`] for `rect` with roundness `radius` (see [`Surface::radius`]) and
/// exponent `exponent` (see [`Surface::exponent`]).
///
/// `radius` accepts either one `f32` for all four corners (`12.0`) or a [`Radius`] for per-corner
/// values (`iced::border::radius(0.0).top(12.0)`; see [`Surface::radius`]).
///
/// Intended for drawing the outline yourself in a custom `canvas`.
///
/// Each corner consumes `radius` from each of the two edges it joins: its curve starts `radius`
/// away from the corner point on both adjacent edges, clamped to half the shorter side, and a
/// corner at `radius <= 0` stays sharp. With `n = exponent`, the distance from the corner point to
/// the curve along the diagonal is `radius * √2 * (1 - 2^(-1/n))`, which is `0.414` times the radius
/// at `n = 2.0` (a circular arc) and `0.183` times at the default exponent `5.0`.
///
/// `exponent` follows [`Surface::exponent`]: values below `1.0` are raised to `1.0`, and `NaN`
/// falls back to [`DEFAULT_EXPONENT`]. Any non-finite corner value in `radius` is treated as `0`
/// (and panics in debug builds).
///
/// The path is rebuilt on every call, so draw it inside a [`canvas::Cache`] if the canvas is redrawn
/// every frame.
pub fn path_with_exponent(rect: Rectangle, radius: impl Into<Radius>, exponent: f32) -> Path {
	let points = outline(rect, radius.into(), exponent);
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
fn outline(rect: Rectangle, radius: Radius, exponent: f32) -> Vec<Point> {
	let exponent = clamp_exponent(exponent);
	// Corner order matches the traversal: top-right, bottom-right, bottom-left, top-left.
	let extents = [
		corner_extent(rect, radius.top_right),
		corner_extent(rect, radius.bottom_right),
		corner_extent(rect, radius.bottom_left),
		corner_extent(rect, radius.top_left),
	];
	if extents.iter().all(|&r| r == 0.0) {
		return Vec::new();
	}
	// Each corner mirrors and scales the unit quarter curve by its own extent. Since the curve runs
	// from the x-axis to the y-axis, reverse top-right and bottom-left to traverse clockwise.
	let corners = [
		(Point::new(rect.x + rect.width, rect.y), (1.0, -1.0), true),
		(Point::new(rect.x + rect.width, rect.y + rect.height), (1.0, 1.0), false),
		(Point::new(rect.x, rect.y + rect.height), (-1.0, 1.0), true),
		(Point::new(rect.x, rect.y), (-1.0, -1.0), false),
	];

	let curve = quarter(exponent);
	let mut points = Vec::with_capacity(corners.len() * curve.len());
	for ((corner, (sx, sy), reversed), r) in corners.into_iter().zip(extents) {
		for step in 0..curve.len() {
			let (x, y) = curve[if reversed { curve.len() - 1 - step } else { step }];
			// Offset the unit curve by `r` on both axes so its endpoints land on the edges adjacent to
			// `corner`. A zero extent collapses the curve onto the corner, which stays sharp.
			let point = Point::new(corner.x + sx * r * (x - 1.0), corner.y + sy * r * (y - 1.0));
			// Avoid duplicating points when an edge has zero length (pill shape) or when a sharp
			// corner collapses every step onto the same point.
			if points.last() != Some(&point) {
				points.push(point);
			}
		}
	}
	// A square pill ends where it starts, so drop the repeated point instead of closing with a
	// zero-length segment.
	if points.len() > 1 && points.first() == points.last() {
		points.pop();
	}
	points
}

/// Length of each adjacent edge consumed by the corner rounded with `radius`.
fn corner_extent(rect: Rectangle, radius: f32) -> f32 {
	debug_assert!(radius.is_finite(), "squircle radius must be finite, got {radius}");
	let radius = finite_or_zero(radius);
	let limit = (rect.width / 2.0).min(rect.height / 2.0);
	// A non-positive `rect` reaches this function through the public `path()` functions, and a
	// negative limit would mirror the curve across the corner point instead of leaving it sharp.
	if radius <= 0.0 || limit <= 0.0 {
		return 0.0;
	}
	radius.min(limit)
}

/// Zeroes a non-finite radius, which both the outline and the shadow draw as a sharp corner.
fn finite_or_zero(radius: f32) -> f32 {
	if radius.is_finite() { radius } else { 0.0 }
}

/// Restricts `exponent` to the range the outline can draw: at least `1.0`, below which the curve
/// turns concave and stops looking like a corner, and [`DEFAULT_EXPONENT`] for `NaN`.
fn clamp_exponent(exponent: f32) -> f32 {
	if exponent.is_nan() {
		DEFAULT_EXPONENT
	} else {
		exponent.max(1.0)
	}
}

/// Points along the first-quadrant unit superellipse `x^n + y^n = 1`, from `(1, 0)` to `(0, 1)`.
///
/// Sampled as `(cos^(2/n), sin^(2/n))` for `θ ∈ [0, π/2]`. Values are clamped at `0.0` because
/// `cos(π/2)` is slightly negative in `f32`.
fn quarter(exponent: f32) -> [(f32, f32); STEPS + 1] {
	let power = 2.0 / exponent;
	std::array::from_fn(|step| {
		let (sin, cos) = (FRAC_PI_2 * step as f32 / STEPS as f32).sin_cos();
		(cos.max(0.0).powf(power), sin.max(0.0).powf(power))
	})
}

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
		let curve = quarter(DEFAULT_EXPONENT);
		assert_eq!(curve[0], (1.0, 0.0));
		assert_eq!(curve[STEPS], (0.0, 1.0));
		assert!(curve.iter().all(|&(x, y)| x >= 0.0 && y >= 0.0));
	}

	#[test]
	fn corners_are_mirror_symmetric() {
		// Centered at the origin, reflections are exact sign flips with symmetric rounding error,
		// so we can compare points for exact equality.
		let rect = Rectangle {
			x: -100.0,
			y: -50.0,
			width: 200.0,
			height: 100.0,
		};
		let points = outline(rect, 8.0.into(), DEFAULT_EXPONENT);
		let n = STEPS + 1;
		assert_eq!(points.len(), n * 4);
		for (i, p) in points[..n].iter().enumerate() {
			// Top-right mirrors vertically to bottom-right and horizontally to top-left (both reversed).
			assert_eq!(points[2 * n - 1 - i], Point::new(p.x, -p.y));
			assert_eq!(points[4 * n - 1 - i], Point::new(-p.x, p.y));
			// Bottom-left is point-symmetric and traversed in the same direction.
			assert_eq!(points[2 * n + i], Point::new(-p.x, -p.y));
		}
	}

	#[test]
	fn radius_is_clamped_to_half_the_shorter_side() {
		let points = outline(RECT, 1000.0.into(), DEFAULT_EXPONENT);
		let top = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
		let bottom = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
		assert_eq!((top, bottom), (RECT.y, RECT.y + RECT.height));
		// The left and right edges have zero length, so adjacent corner endpoints must not duplicate.
		assert!(points.windows(2).all(|w| w[0] != w[1]));
	}

	#[test]
	fn top_only_keeps_the_bottom_corners_sharp() {
		let points = outline(RECT, iced::border::top(8.0), DEFAULT_EXPONENT);
		let (left, top) = (RECT.x, RECT.y);
		let (right, bottom) = (RECT.x + RECT.width, RECT.y + RECT.height);
		// The top corners are cut back, so their sharp points leave the outline.
		assert!(!points.contains(&Point::new(left, top)));
		assert!(!points.contains(&Point::new(right, top)));
		// The bottom corners stay sharp and are joined by one straight edge.
		assert!(points.contains(&Point::new(left, bottom)));
		assert!(points.contains(&Point::new(right, bottom)));
		assert!(edges(&points).contains(&[Point::new(right, bottom), Point::new(left, bottom)]));
		assert_eq!(points.len(), 2 * (STEPS + 1) + 2);
	}

	#[test]
	fn bottom_only_keeps_the_top_corners_sharp() {
		let points = outline(RECT, iced::border::bottom(8.0), DEFAULT_EXPONENT);
		let (left, top) = (RECT.x, RECT.y);
		let (right, bottom) = (RECT.x + RECT.width, RECT.y + RECT.height);
		assert!(!points.contains(&Point::new(left, bottom)));
		assert!(!points.contains(&Point::new(right, bottom)));
		assert!(points.contains(&Point::new(left, top)));
		assert!(points.contains(&Point::new(right, top)));
		assert!(edges(&points).contains(&[Point::new(left, top), Point::new(right, top)]));
		assert_eq!(points.len(), 2 * (STEPS + 1) + 2);
	}

	#[test]
	fn each_corner_uses_its_own_extent() {
		let radius = Radius {
			top_left: 8.0,
			top_right: 0.0,
			bottom_right: 4.0,
			bottom_left: 0.0,
		};
		let points = outline(RECT, radius, DEFAULT_EXPONENT);
		let (left, top) = (RECT.x, RECT.y);
		let (right, bottom) = (RECT.x + RECT.width, RECT.y + RECT.height);
		// Zero-radius corners stay sharp.
		assert!(points.contains(&Point::new(right, top)));
		assert!(points.contains(&Point::new(left, bottom)));
		// Rounded corners start at their own extent along each adjacent edge.
		assert!(points.contains(&Point::new(left, top + 8.0)));
		assert!(points.contains(&Point::new(right, bottom - 4.0)));
	}

	#[test]
	fn each_corner_is_clamped_individually() {
		let radius = Radius {
			top_left: 1000.0,
			..Radius::default()
		};
		let points = outline(RECT, radius, DEFAULT_EXPONENT);
		let extent = RECT.height / 2.0;
		// The top-left curve stops at half the shorter side; the other corners stay sharp.
		assert!(points.contains(&Point::new(RECT.x + extent, RECT.y)));
		assert!(points.contains(&Point::new(RECT.x, RECT.y + extent)));
		assert_eq!(points.len(), STEPS + 1 + 3);
	}

	#[test]
	fn zero_radius_has_no_corners() {
		assert!(outline(RECT, 0.0.into(), DEFAULT_EXPONENT).is_empty());
		assert!(outline(RECT, (-3.0).into(), DEFAULT_EXPONENT).is_empty());
		assert!(outline(RECT, Radius::default(), DEFAULT_EXPONENT).is_empty());
	}

	#[test]
	#[cfg_attr(debug_assertions, should_panic(expected = "finite"))]
	fn nan_radius_is_treated_as_zero() {
		assert!(outline(RECT, f32::NAN.into(), DEFAULT_EXPONENT).is_empty());
	}

	#[test]
	fn radius_is_the_corner_extent_at_every_exponent() {
		let square = Rectangle {
			x: 0.0,
			y: 0.0,
			width: 100.0,
			height: 100.0,
		};
		// A corner starts `radius` from its corner point on both adjacent edges, whatever the
		// exponent is.
		for exponent in [1.0, 2.0, DEFAULT_EXPONENT, 8.0] {
			let points = outline(square, 10.0.into(), exponent);
			assert!(
				points.iter().any(|p| (p.x - 90.0).abs() < 1e-3 && p.y.abs() < 1e-3),
				"exponent {exponent}"
			);
			assert!(
				points
					.iter()
					.any(|p| (p.x - 100.0).abs() < 1e-3 && (p.y - 10.0).abs() < 1e-3),
				"exponent {exponent}"
			);
		}
	}

	#[test]
	fn corner_depth_matches_the_documented_table() {
		let square = Rectangle {
			x: 0.0,
			y: 0.0,
			width: 100.0,
			height: 100.0,
		};
		let radius = 10.0;
		let corner = Point::new(100.0, 0.0);
		// Distance from the corner point to the top-right curve along the diagonal, in units of
		// `radius`. A projection onto the diagonal bounds each sampled point from below.
		let depth = |exponent: f32| {
			let points = outline(square, radius.into(), exponent);
			points[..=STEPS]
				.iter()
				.map(|p| (corner.x - p.x + p.y) / std::f32::consts::SQRT_2 / radius)
				.fold(f32::INFINITY, f32::min)
		};
		// The table in the README lists these ratios for the same exponents.
		for (exponent, ratio) in [(2.0, 0.414), (3.0, 0.292), (5.0, 0.183), (8.0, 0.117)] {
			assert!((depth(exponent) - ratio).abs() < 1e-3, "exponent {exponent}");
		}
	}

	#[test]
	fn non_positive_rect_has_no_corners() {
		// A caller-supplied rect can be inverted or flat; rounding it must not mirror the corner.
		assert!(outline(Rectangle { width: -0.5, ..RECT }, 8.0.into(), DEFAULT_EXPONENT).is_empty());
		assert!(outline(Rectangle { height: 0.0, ..RECT }, 8.0.into(), DEFAULT_EXPONENT).is_empty());
	}

	#[test]
	fn square_pill_closes_without_a_repeated_point() {
		let square = Rectangle {
			x: 0.0,
			y: 0.0,
			width: 100.0,
			height: 100.0,
		};
		let points = outline(square, 1000.0.into(), DEFAULT_EXPONENT);
		assert_ne!(points.first(), points.last());
		assert!(points.windows(2).all(|w| w[0] != w[1]));
	}

	#[test]
	fn huge_finite_exponent_stays_inside_the_rect() {
		// `2^(-1/n)` rounds to `1.0` above about `n = 1e7`, collapsing the curve onto the corner
		// point; the outline must still cover the whole rect with finite coordinates.
		let points = outline(RECT, 8.0.into(), 2e7);
		assert!(points.iter().all(|p| p.x.is_finite() && p.y.is_finite()));
		let span = |values: Vec<f32>| {
			let low = values.iter().copied().fold(f32::INFINITY, f32::min);
			let high = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
			high - low
		};
		let width = span(points.iter().map(|p| p.x).collect());
		let height = span(points.iter().map(|p| p.y).collect());
		assert_eq!((width, height), (RECT.width, RECT.height));
	}

	#[test]
	fn nan_exponent_falls_back_to_the_default() {
		assert_eq!(
			outline(RECT, 8.0.into(), f32::NAN),
			outline(RECT, 8.0.into(), DEFAULT_EXPONENT)
		);
	}

	#[test]
	fn exponent_below_one_chamfers_the_corner() {
		assert_eq!(outline(RECT, 8.0.into(), 0.5), outline(RECT, 8.0.into(), 1.0));
	}

	#[test]
	fn zero_radius_survives_an_infinite_exponent() {
		// `0 * inf` must not reach the clamp, which would round a corner meant to stay sharp.
		assert!(outline(RECT, 0.0.into(), f32::INFINITY).is_empty());
	}

	#[test]
	fn infinite_exponent_draws_a_plain_rectangle() {
		// Every step of the curve collapses onto its corner point, leaving only the four corners.
		// This pins the outline, not the `Path::rectangle` fallback: the path is a 4-vertex polygon.
		let points = outline(RECT, 8.0.into(), f32::INFINITY);
		assert_eq!(
			points,
			vec![
				Point::new(RECT.x + RECT.width, RECT.y),
				Point::new(RECT.x + RECT.width, RECT.y + RECT.height),
				Point::new(RECT.x, RECT.y + RECT.height),
				Point::new(RECT.x, RECT.y),
			]
		);
	}

	/// Segments of the closed outline, including the one that wraps back to the first point.
	fn edges(points: &[Point]) -> Vec<[Point; 2]> {
		let mut edges: Vec<_> = points.windows(2).map(|w| [w[0], w[1]]).collect();
		if let (Some(first), Some(last)) = (points.first(), points.last()) {
			edges.push([*last, *first]);
		}
		edges
	}
}
