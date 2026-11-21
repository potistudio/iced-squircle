//! Squircle (superellipse-cornered) surfaces for iced.
//!
//! Because iced's `border.radius` only draws circular-arc corners, this crate renders a
//! superellipse path on a `canvas` and layers it beneath your content.
//!
//! ```no_run
//! use iced::widget::{container, text};
//! use iced::{Color, Element, border};
//! use iced_squircle::{Surface, squircle};
//!
//! fn card() -> Surface {
//!     Surface::default()
//!         // Round the top corners only.
//!         .radius(border::radius(0.0).top(12.0))
//!         .fill(Color::WHITE)
//!         .border(Color::BLACK, 1.0)
//! }
//!
//! fn view<'a, Message: 'a>() -> Element<'a, Message> {
//!     squircle(container(text("Hello")).padding(16), card())
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::cell::Cell;
use std::f32::consts::{FRAC_PI_2, SQRT_2};

use iced_widget::canvas::{self, Cache, Geometry, Path};
use iced_widget::core::border::{self, Radius};
use iced_widget::core::{Color, Element, Length, Point, Rectangle, Shadow, Vector, mouse};
use iced_widget::graphics::geometry;
use iced_widget::{Stack, container};

/// Default superellipse exponent (`5.0`), producing the classic squircle. Used by [`path`] and by
/// [`Surface::default`]; `2.0` is a circular arc, and larger values approach a right angle.
pub const DEFAULT_EXPONENT: f32 = 5.0;
/// Number of line segments per corner.
const STEPS: usize = 24;

/// Appearance of a squircle surface.
///
/// Start from [`Surface::default`] (sharp, transparent, no border, no shadow) and set what you need,
/// either with the builder methods or with struct update syntax:
///
/// ```
/// use iced::{Color, border};
/// use iced_squircle::{DEFAULT_EXPONENT, Outline, Surface};
///
/// let built = Surface::default()
///     .radius(12.0)
///     .fill(Color::WHITE)
///     .border(Color::BLACK, 2.0);
///
/// let literal = Surface {
///     radius: border::radius(12.0),
///     fill: Color::WHITE,
///     border: Some(Outline::new(Color::BLACK, 2.0)),
///     ..Surface::default()
/// };
///
/// assert_eq!(built, literal);
/// assert_eq!(built.exponent, DEFAULT_EXPONENT);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Surface {
	/// Corner roundness, per corner: the length that corner consumes from each of its two adjacent
	/// edges. `12.0` makes the curve start 12px from the corner point on both edges, matching the
	/// corner extent iced's `border.radius` uses (at an exponent of `2.0` the corner is the same
	/// circular arc). The value does not depend on [`exponent`](Surface::exponent).
	///
	/// Each corner is clamped to half of the shorter side, so `f32::INFINITY` rounds a corner as far
	/// as the clamp allows, and `f32::INFINITY` on all four corners draws a pill, as it does for
	/// iced's own `border.radius`. A value at or below `0.0` stays sharp, and `NaN` is treated as
	/// `0.0` (and panics in debug builds).
	///
	/// Build it from a base radius and override the corners that differ:
	/// `border::radius(12.0)` rounds all four, `border::radius(0.0).top(12.0)` only the top two,
	/// and `border::radius(12.0).bottom(0.0)` only the top two as well. Helpers in [`iced::border`](border)
	/// such as `border::top(12.0)` are shorthands for the same values.
	pub radius: Radius,
	/// Superellipse exponent: how tightly the curve turns toward its corner point. `2.0` is a circular
	/// arc, and larger values approach a right angle; [`DEFAULT_EXPONENT`] (`5.0`) is the classic
	/// squircle.
	///
	/// The exponent changes the curve's shape without changing the corner's extent. The distance
	/// from the corner point to the curve along the diagonal is `0.414` times the extent at `2.0` (a
	/// circular arc) and `0.183` times at `5.0`, so a higher exponent keeps the corner closer to a
	/// right angle.
	///
	/// Values below `1.0` are raised to `1.0`, where the corner becomes a straight chamfer, and
	/// `NaN` falls back to [`DEFAULT_EXPONENT`]. Positive infinity (`f32::INFINITY`) is allowed:
	/// it collapses each corner to a right angle and draws a plain rectangle.
	pub exponent: f32,
	/// Color painted inside the outline, beneath the border. Use [`Color::TRANSPARENT`] to draw the
	/// border alone.
	pub fill: Color,
	/// Optional border, drawn inside the outline over the fill. See [`Outline`].
	pub border: Option<Outline>,
	/// Optional drop shadow. A canvas cannot blur, so iced draws it as a rounded rectangle that
	/// approximates the outline; see [`squircle`] for how closely it follows the corners.
	pub shadow: Option<Shadow>,
}

impl Default for Surface {
	/// A sharp, transparent surface with no border and no shadow, at [`DEFAULT_EXPONENT`].
	fn default() -> Self {
		Self {
			radius: Radius::default(),
			exponent: DEFAULT_EXPONENT,
			fill: Color::TRANSPARENT,
			border: None,
			shadow: None,
		}
	}
}

impl Surface {
	/// Sets [`radius`](Surface::radius) from one `f32` for all four corners or from a [`Radius`].
	#[must_use]
	pub fn radius(mut self, radius: impl Into<Radius>) -> Self {
		self.radius = radius.into();
		self
	}

	/// Sets [`exponent`](Surface::exponent).
	#[must_use]
	pub fn exponent(mut self, exponent: f32) -> Self {
		self.exponent = exponent;
		self
	}

	/// Sets [`fill`](Surface::fill).
	#[must_use]
	pub fn fill(mut self, fill: impl Into<Color>) -> Self {
		self.fill = fill.into();
		self
	}

	/// Sets [`border`](Surface::border) to a border of `color` that is `width` pixels wide.
	#[must_use]
	pub fn border(mut self, color: impl Into<Color>, width: f32) -> Self {
		self.border = Some(Outline::new(color.into(), width));
		self
	}

	/// Sets [`shadow`](Surface::shadow).
	#[must_use]
	pub fn shadow(mut self, shadow: impl Into<Shadow>) -> Self {
		self.shadow = Some(shadow.into());
		self
	}

	/// Builds this surface's outline for `rect`. For a canvas-local `rect` at the origin with the
	/// canvas's size, `Rectangle::with_size(bounds.size())`, this is the path [`squircle`] fills: the
	/// border is drawn inside it, so the outline is the outer edge of the surface whether or not
	/// [`border`](Surface::border) is set.
	///
	/// A `rect` without area has no outline, and the result is `Path::rectangle` for `rect` as given;
	/// [`squircle`] draws nothing at that size.
	pub fn path(&self, rect: Rectangle) -> Path {
		path_with_exponent(rect, self.radius, self.exponent)
	}
}

/// Border of a [`Surface`].
///
/// The border runs along the inside of the outline: its outer edge is the outline itself, and its
/// inner edge lies `width` pixels inward, measured perpendicular to the outline at every point, so
/// the border keeps the same thickness around the corners. It is painted over the fill, so a
/// translucent border shows the fill through it, as with a CSS border over its background.
///
/// A `width` at or below `0.0`, or `NaN`, draws no border; a `width` of half the shorter side or
/// more covers the whole surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outline {
	/// Color of the border.
	pub color: Color,
	/// Thickness of the border in logical pixels.
	pub width: f32,
}

impl Outline {
	/// Creates a border of `color` that is `width` pixels wide.
	pub const fn new(color: Color, width: f32) -> Self {
		Self { color, width }
	}
}

/// Layers a squircle surface beneath `content`, preserving `content`'s size.
///
/// The result is a `container` wrapping a [`Stack`] whose bottom layer is a canvas that fills the
/// stack. Both take `content`'s width and height [`Length`], so `Fill`, `FillPortion`, `Shrink`,
/// and `Fixed` content keep their own sizing strategy, and the surface adds no padding. The widget
/// tree has the same shape whether or not [`Surface::shadow`] is set, so toggling the shadow keeps
/// the state of `content`'s widgets.
///
/// The surface does not clip `content`: anything `content` paints into a corner covers the curve.
///
/// # Shadow
///
/// A canvas cannot blur, so the `container` casts [`Surface::shadow`] from a rounded rectangle with
/// circular corners, sized so that each arc stays inside the superellipse it approximates. At an
/// exponent of `2.0` or more the arc keeps the corner's radius, which cuts at least as deep as the
/// superellipse; below `2.0` the arc's radius grows until it cuts as deep as the flatter curve, up
/// to `1.71` times the corner's radius at `1.0`. The renderer clamps the arc's radius to half of the
/// shorter side, and once that clamp applies the arc no longer reaches as deep as the corner: at an
/// exponent of `1.0` with every corner at half of the shorter side, the shadow's corner sticks out
/// of the outline by `0.29` times half of the shorter side along the diagonal. Below `2.0`, keep
/// each corner's radius at or below half of the shorter side divided by that growth factor (`0.59`
/// times half of the shorter side at `1.0`) for a shadow that stays inside the outline.
///
/// The shadow is meant for an opaque [`fill`](Surface::fill). Through a translucent fill, the
/// shadow shows in the corners between the arc and the superellipse, and with the `tiny-skia`
/// renderer it shows beneath the whole surface.
pub fn squircle<'a, Message, Theme, Renderer>(
	content: impl Into<Element<'a, Message, Theme, Renderer>>,
	surface: Surface,
) -> Element<'a, Message, Theme, Renderer>
where
	Message: 'a,
	Theme: container::Catalog + 'a,
	<Theme as container::Catalog>::Class<'a>: From<container::StyleFn<'a, Theme>>,
	Renderer: geometry::Renderer + 'static,
{
	let content = content.into();
	let size = content.as_widget().size_hint();
	let shape = canvas::Canvas::new(Shape(surface))
		.width(Length::Fill)
		.height(Length::Fill);
	// `from_vec` keeps `content` even when one of its lengths is `Fixed(0.0)`, which `Stack::push`
	// would drop together with the canvas beneath it.
	let layered = Stack::from_vec(vec![content])
		.width(size.width)
		.height(size.height)
		.push_under(shape);

	// The container is always present so that adding or removing a shadow does not change the
	// widget tree, which would reset the state of every widget inside `content`. Without a shadow
	// it has no background, border, or shadow, so it draws nothing.
	let shadow = surface.shadow.unwrap_or_default();
	let radius = shadow_radius(surface.radius, surface.exponent);
	container(layered)
		.width(size.width)
		.height(size.height)
		.style(move |_| container::Style {
			border: border::rounded(radius),
			shadow,
			..container::Style::default()
		})
		.into()
}

/// Corner radii for the shadow's rounded rectangle, chosen so that each circular arc stays inside
/// the superellipse corner it approximates.
///
/// An arc of radius `r` cuts `(√2 - 1) r` off its corner along the diagonal, and the superellipse
/// cuts [`corner_depth`] times its radius. At an exponent of `2.0` or more the superellipse cuts no
/// deeper, so the arc keeps the radius; below `2.0` the radius is scaled up until the cuts match.
fn shadow_radius(radius: Radius, exponent: f32) -> Radius {
	let scale = (corner_depth(clamp_exponent(exponent)) / (SQRT_2 - 1.0)).max(1.0);
	// `container` rounds negative or NaN radii into a distorted shape instead of a sharp corner, so
	// pass `0.0` for every corner the outline keeps sharp. Infinity passes through, since the
	// renderers clamp it to half of the shorter side as `corner_extent` does.
	let corner = |radius: f32| if radius > 0.0 { radius * scale } else { 0.0 };
	Radius {
		top_left: corner(radius.top_left),
		top_right: corner(radius.top_right),
		bottom_right: corner(radius.bottom_right),
		bottom_left: corner(radius.bottom_left),
	}
}

struct Shape(Surface);

/// Cached geometry plus the key of the surface it was drawn from, so a changed surface redraws at
/// the same size.
struct ShapeState<Renderer: geometry::Renderer> {
	cache: Cache<Renderer>,
	key: Cell<Option<CacheKey>>,
}

impl<Renderer: geometry::Renderer> Default for ShapeState<Renderer> {
	fn default() -> Self {
		Self {
			cache: Cache::default(),
			key: Cell::new(None),
		}
	}
}

/// Bit patterns of the [`Surface`] fields the canvas draws. Comparing bits instead of values keeps
/// a `NaN` field equal to itself, so it does not clear the cache on every frame.
type CacheKey = ([u32; 9], Option<[u32; 5]>);

fn cache_key(surface: &Surface) -> CacheKey {
	let color = |color: Color| [color.r, color.g, color.b, color.a].map(f32::to_bits);
	let radius = surface.radius;
	let [r, g, b, a] = color(surface.fill);
	let shape = [
		radius.top_left,
		radius.top_right,
		radius.bottom_right,
		radius.bottom_left,
		surface.exponent,
	]
	.map(f32::to_bits);
	let border = surface.border.map(|border| {
		let [r, g, b, a] = color(border.color);
		[r, g, b, a, border.width.to_bits()]
	});
	([shape[0], shape[1], shape[2], shape[3], shape[4], r, g, b, a], border)
}

impl<Message, Theme, Renderer> canvas::Program<Message, Theme, Renderer> for Shape
where
	// The canvas keeps its `State` in the widget tree, which requires `'static`.
	Renderer: geometry::Renderer + 'static,
{
	type State = ShapeState<Renderer>;

	fn draw(
		&self,
		state: &ShapeState<Renderer>,
		renderer: &Renderer,
		_theme: &Theme,
		bounds: Rectangle,
		_cursor: mouse::Cursor,
	) -> Vec<Geometry<Renderer>> {
		if !(bounds.width > 0.0 && bounds.height > 0.0) {
			return Vec::new();
		}
		let surface = self.0;
		// A `Cache` only redraws when its size changes, so clear it whenever the surface itself does.
		let key = cache_key(&surface);
		if state.key.get() != Some(key) {
			state.cache.clear();
			state.key.set(Some(key));
		}
		vec![state.cache.draw(renderer, bounds.size(), |frame| {
			draw_surface(frame, &surface, Rectangle::with_size(bounds.size()));
		})]
	}
}

/// Fills the outline of `surface` for `rect`, then paints its border over the fill.
fn draw_surface<Renderer: geometry::Renderer>(frame: &mut canvas::Frame<Renderer>, surface: &Surface, rect: Rectangle) {
	let outer = outline(rect, surface.radius, surface.exponent);
	if outer.len() < 3 {
		return;
	}
	if surface.fill.a > 0.0 {
		frame.fill(&polygon(&outer), surface.fill);
	}
	let Some(border) = surface.border else {
		return;
	};
	if !(border.width > 0.0 && border.color.a > 0.0) {
		return;
	}
	// The border is the ring between the outline and its erosion. Even-odd filling cuts the inner
	// polygon out regardless of its orientation, and an empty erosion leaves the whole outline.
	let inner = erode(&outer, border.width);
	let ring = Path::new(|p| {
		add_polygon(p, &outer);
		if inner.len() >= 3 {
			add_polygon(p, &inner);
		}
	});
	frame.fill(
		&ring,
		canvas::Fill {
			rule: canvas::fill::Rule::EvenOdd,
			..canvas::Fill::from(border.color)
		},
	);
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
/// away from the corner point on both adjacent edges, clamped to half the shorter side (this
/// clamped length is the corner's extent), and a corner at `radius <= 0` stays sharp. With `n = exponent`, the distance from the corner point to
/// the curve along the diagonal is `extent * √2 * (1 - 2^(-1/n))`, which is `0.414` times the extent
/// at `n = 2.0` (a circular arc) and `0.183` times at the default exponent `5.0`.
///
/// `exponent` follows [`Surface::exponent`]: values below `1.0` are raised to `1.0`, and `NaN`
/// falls back to [`DEFAULT_EXPONENT`]. `radius` follows [`Surface::radius`]: `f32::INFINITY`
/// rounds a corner as far as the clamp allows, and a `NaN` corner is treated as `0` (and panics in
/// debug builds).
///
/// The path is rebuilt on every call, so draw it inside a [`canvas::Cache`] if the canvas is redrawn
/// every frame.
pub fn path_with_exponent(rect: Rectangle, radius: impl Into<Radius>, exponent: f32) -> Path {
	let points = outline(rect, radius.into(), exponent);
	if points.len() >= 3 {
		polygon(&points)
	} else {
		// A rect without area collapses to fewer than three points, which do not form a polygon.
		Path::rectangle(rect.position(), rect.size())
	}
}

/// Closed polygon through `points`.
fn polygon(points: &[Point]) -> Path {
	Path::new(|p| add_polygon(p, points))
}

/// Adds a closed subpath through `points`, which must not be empty.
fn add_polygon(builder: &mut canvas::path::Builder, points: &[Point]) {
	let (first, rest) = points.split_first().expect("a polygon needs at least one point");
	builder.move_to(*first);
	for point in rest {
		builder.line_to(*point);
	}
	builder.close();
}

/// Outline points for [`path`], ordered clockwise on screen starting at the top-right corner. A
/// sharp corner contributes its corner point alone, so a rect without rounded corners yields its
/// four corners, and a rect without area yields fewer than three points.
fn outline(rect: Rectangle, radius: Radius, exponent: f32) -> Vec<Point> {
	let exponent = clamp_exponent(exponent);
	// Corner order matches the traversal: top-right, bottom-right, bottom-left, top-left.
	let extents = [
		corner_extent(rect, radius.top_right),
		corner_extent(rect, radius.bottom_right),
		corner_extent(rect, radius.bottom_left),
		corner_extent(rect, radius.top_left),
	];
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
	debug_assert!(!radius.is_nan(), "squircle radius must not be NaN");
	let limit = (rect.width / 2.0).min(rect.height / 2.0);
	// A non-positive `rect` reaches this function through the public `path()` functions, and a
	// negative limit would mirror the curve across the corner point instead of leaving it sharp.
	// The negated comparison also keeps a `NaN` radius sharp.
	if !(radius > 0.0 && limit > 0.0) {
		return 0.0;
	}
	radius.min(limit)
}

/// Distance from the corner point to a superellipse corner of exponent `exponent` along the
/// diagonal, in units of its radius: `√2 * (1 - 2^(-1/n))`.
fn corner_depth(exponent: f32) -> f32 {
	SQRT_2 * (1.0 - 2f32.powf(-1.0 / exponent))
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

/// Erodes the convex polygon `points` by `distance`: the result bounds the points that lie at least
/// `distance` inside every edge, or is empty when no such point exists.
///
/// A convex polygon is the intersection of the half-planes of its edges, so its erosion is the
/// intersection of those half-planes moved inward by `distance`. Clipping the polygon by each moved
/// half-plane in turn builds that intersection without the self-intersections that offsetting each
/// vertex along its normal would produce where `distance` exceeds the curve's radius of curvature.
fn erode(points: &[Point], distance: f32) -> Vec<Point> {
	// A positive signed area means the vertices run counterclockwise in a y-up frame, where the
	// left-hand normal of each edge points inward.
	let orientation = if signed_area(points) >= 0.0 { 1.0 } else { -1.0 };

	let mut eroded = points.to_vec();
	for (&a, &b) in points.iter().zip(points.iter().cycle().skip(1)) {
		let edge = b - a;
		let length = edge.x.hypot(edge.y);
		if length == 0.0 {
			continue;
		}
		let normal = Vector::new(-edge.y, edge.x) * (orientation / length);
		eroded = clip(&eroded, |p| normal.x * (p.x - a.x) + normal.y * (p.y - a.y) - distance);
		if eroded.is_empty() {
			break;
		}
	}
	eroded.dedup();
	if eroded.len() > 1 && eroded.first() == eroded.last() {
		eroded.pop();
	}
	// An erosion of exactly half the shorter side leaves a line along the middle, which has no
	// inside to fill. The threshold is relative so that surfaces smaller than a pixel keep theirs.
	if signed_area(&eroded).abs() <= signed_area(points).abs() * 1e-6 {
		eroded.clear();
	}
	eroded
}

/// Twice the signed area of the closed polygon `points` (shoelace formula).
fn signed_area(points: &[Point]) -> f32 {
	points
		.iter()
		.zip(points.iter().cycle().skip(1))
		.map(|(a, b)| a.x * b.y - b.x * a.y)
		.sum()
}

/// Clips the closed polygon `points` to the half-plane where `side` is non-negative
/// (Sutherland–Hodgman).
fn clip(points: &[Point], side: impl Fn(Point) -> f32) -> Vec<Point> {
	let mut clipped = Vec::with_capacity(points.len() + 1);
	for (&current, &next) in points.iter().zip(points.iter().cycle().skip(1)) {
		let (here, there) = (side(current), side(next));
		if here >= 0.0 {
			clipped.push(current);
		}
		if (here >= 0.0) != (there >= 0.0) {
			let t = here / (here - there);
			clipped.push(current + (next - current) * t);
		}
	}
	clipped
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

	const SQUARE: Rectangle = Rectangle {
		x: 0.0,
		y: 0.0,
		width: 100.0,
		height: 100.0,
	};

	/// The four corners of `rect`, in outline order.
	fn corners(rect: Rectangle) -> Vec<Point> {
		vec![
			Point::new(rect.x + rect.width, rect.y),
			Point::new(rect.x + rect.width, rect.y + rect.height),
			Point::new(rect.x, rect.y + rect.height),
			Point::new(rect.x, rect.y),
		]
	}

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
	fn infinite_radius_draws_a_pill() {
		assert_eq!(
			outline(RECT, f32::INFINITY.into(), DEFAULT_EXPONENT),
			outline(RECT, (RECT.height / 2.0).into(), DEFAULT_EXPONENT)
		);
	}

	#[test]
	fn top_only_keeps_the_bottom_corners_sharp() {
		let points = outline(RECT, border::top(8.0), DEFAULT_EXPONENT);
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
		let points = outline(RECT, border::bottom(8.0), DEFAULT_EXPONENT);
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
	fn zero_radius_keeps_only_the_corner_points() {
		assert_eq!(outline(RECT, 0.0.into(), DEFAULT_EXPONENT), corners(RECT));
		assert_eq!(outline(RECT, (-3.0).into(), DEFAULT_EXPONENT), corners(RECT));
		assert_eq!(outline(RECT, f32::NEG_INFINITY.into(), DEFAULT_EXPONENT), corners(RECT));
		assert_eq!(outline(RECT, Radius::default(), DEFAULT_EXPONENT), corners(RECT));
	}

	#[test]
	#[cfg_attr(debug_assertions, should_panic(expected = "NaN"))]
	fn nan_radius_is_treated_as_zero() {
		assert_eq!(outline(RECT, f32::NAN.into(), DEFAULT_EXPONENT), corners(RECT));
	}

	#[test]
	fn radius_is_the_corner_extent_at_every_exponent() {
		// A corner starts `radius` from its corner point on both adjacent edges, whatever the
		// exponent is.
		for exponent in [1.0, 2.0, DEFAULT_EXPONENT, 8.0] {
			let points = outline(SQUARE, 10.0.into(), exponent);
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
		let radius = 10.0;
		let corner = Point::new(100.0, 0.0);
		// Distance from the corner point to the top-right curve along the diagonal, in units of
		// `radius`. A projection onto the diagonal bounds each sampled point from below.
		let depth = |exponent: f32| {
			let points = outline(SQUARE, radius.into(), exponent);
			points[..=STEPS]
				.iter()
				.map(|p| (corner.x - p.x + p.y) / SQRT_2 / radius)
				.fold(f32::INFINITY, f32::min)
		};
		// The table in the README lists these ratios for the same exponents.
		for (exponent, ratio) in [(2.0, 0.414), (3.0, 0.292), (5.0, 0.183), (8.0, 0.117)] {
			assert!((depth(exponent) - ratio).abs() < 1e-3, "exponent {exponent}");
			assert!((corner_depth(exponent) - ratio).abs() < 1e-3, "exponent {exponent}");
		}
	}

	#[test]
	fn rect_without_area_is_not_a_polygon() {
		// A flat rect collapses to two points; rounding it must not mirror the corner.
		assert!(outline(Rectangle { height: 0.0, ..RECT }, 8.0.into(), DEFAULT_EXPONENT).len() < 3);
		assert!(
			outline(
				Rectangle {
					width: 0.0,
					height: 0.0,
					..RECT
				},
				8.0.into(),
				DEFAULT_EXPONENT
			)
			.len()
				< 3
		);
		// An inverted rect keeps its corner points instead of rounding outward.
		let inverted = Rectangle { width: -0.5, ..RECT };
		assert_eq!(outline(inverted, 8.0.into(), DEFAULT_EXPONENT), corners(inverted));
	}

	#[test]
	fn square_pill_closes_without_a_repeated_point() {
		let points = outline(SQUARE, 1000.0.into(), DEFAULT_EXPONENT);
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
		assert_eq!(outline(RECT, 0.0.into(), f32::INFINITY), corners(RECT));
	}

	#[test]
	fn infinite_exponent_draws_a_plain_rectangle() {
		// Every step of the curve collapses onto its corner point, leaving only the four corners.
		assert_eq!(outline(RECT, 8.0.into(), f32::INFINITY), corners(RECT));
	}

	#[test]
	fn eroding_a_rectangle_insets_it() {
		let inner = erode(&corners(RECT), 3.0);
		let inset = Rectangle {
			x: RECT.x + 3.0,
			y: RECT.y + 3.0,
			width: RECT.width - 6.0,
			height: RECT.height - 6.0,
		};
		assert_eq!(inner, corners(inset));
		// The orientation of the input does not matter.
		let mut reversed = corners(RECT);
		reversed.reverse();
		let mut expected = corners(inset);
		expected.reverse();
		assert_eq!(erode(&reversed, 3.0), expected);
	}

	#[test]
	fn eroded_outline_keeps_the_border_width() {
		let width = 6.0;
		for exponent in [1.0, 2.0, DEFAULT_EXPONENT, 8.0] {
			let outer = outline(SQUARE, 30.0.into(), exponent);
			let inner = erode(&outer, width);
			assert!(inner.len() >= 3, "exponent {exponent}");
			// Every inner vertex is at least `width` from every outer edge, and the closest edge is
			// exactly `width` away, so the border never thins or thickens.
			for p in &inner {
				let distance = edges(&outer)
					.iter()
					.map(|&[a, b]| segment_distance(*p, a, b))
					.fold(f32::INFINITY, f32::min);
				assert!((distance - width).abs() < 1e-3, "exponent {exponent}: {distance}");
			}
		}
	}

	#[test]
	fn erosion_past_the_center_is_empty() {
		assert!(erode(&outline(RECT, 20.0.into(), DEFAULT_EXPONENT), RECT.height / 2.0).is_empty());
		assert!(erode(&outline(RECT, 20.0.into(), DEFAULT_EXPONENT), f32::INFINITY).is_empty());
	}

	#[test]
	fn shadow_arc_stays_inside_the_superellipse() {
		let radius = 10.0;
		for exponent in [1.0, 1.25, 1.5, 1.75, 2.0, 3.0, DEFAULT_EXPONENT, 8.0, f32::INFINITY] {
			let r = shadow_radius(radius.into(), exponent).top_left;
			// Sample the arc in corner coordinates: the corner point at the origin, the edges along
			// the axes, and the superellipse filling `((r0 - x) / r0)^n + ((r0 - y) / r0)^n <= 1` in
			// the corner's square, which holds the corner's part of the shape.
			for step in 0..=100 {
				let angle = FRAC_PI_2 * step as f32 / 100.0;
				let (x, y) = (r - r * angle.cos(), r - r * angle.sin());
				if x >= radius || y >= radius {
					continue;
				}
				let n = clamp_exponent(exponent).min(1e6);
				let inside = ((radius - x) / radius).powf(n) + ((radius - y) / radius).powf(n);
				assert!(inside <= 1.0 + 1e-4, "exponent {exponent}: {inside}");
			}
		}
	}

	#[test]
	fn shadow_radius_keeps_sharp_corners_sharp() {
		let radius = Radius {
			top_left: -3.0,
			top_right: f32::NAN,
			bottom_right: f32::NEG_INFINITY,
			bottom_left: f32::INFINITY,
		};
		let shadow = shadow_radius(radius, DEFAULT_EXPONENT);
		assert_eq!(
			(
				shadow.top_left,
				shadow.top_right,
				shadow.bottom_right,
				shadow.bottom_left
			),
			(0.0, 0.0, 0.0, f32::INFINITY)
		);
	}

	#[test]
	fn nan_surface_keeps_its_cache_key() {
		let surface = Surface::default()
			.radius(12.0)
			.exponent(f32::NAN)
			.border(Color::BLACK, f32::NAN);
		assert_eq!(cache_key(&surface), cache_key(&surface));
		assert_ne!(cache_key(&surface), cache_key(&surface.border(Color::BLACK, 2.0)));
		// The shadow is drawn by the container, not the canvas.
		assert_eq!(cache_key(&surface), cache_key(&surface.shadow(Shadow::default())));
	}

	/// Layout tests use iced's `()` renderer, which iced only provides with debug assertions.
	#[cfg(debug_assertions)]
	mod layout_tests {
		use super::*;
		use iced_widget::core::widget::Tree;
		use iced_widget::core::{Size, layout};
		use iced_widget::{Row, Space};

		/// A squircle around `content` for the `()` renderer, which lays out without a backend.
		fn element<'a>(
			content: impl Into<Element<'a, (), iced_widget::Theme, ()>>,
			surface: Surface,
		) -> Element<'a, (), iced_widget::Theme, ()> {
			squircle(content, surface)
		}

		/// Lays out `element` within `max` and returns its node.
		fn layout(element: &mut Element<'_, (), iced_widget::Theme, ()>, max: Size) -> layout::Node {
			let mut tree = Tree::new(&*element);
			element
				.as_widget_mut()
				.layout(&mut tree, &(), &layout::Limits::new(Size::ZERO, max))
		}

		#[test]
		fn squircle_keeps_the_content_length() {
			let lengths = [
				Length::Fill,
				Length::FillPortion(3),
				Length::Shrink,
				Length::Fixed(40.0),
			];
			for width in lengths {
				for shadow in [None, Some(Shadow::default())] {
					let surface = Surface {
						shadow,
						..Surface::default()
					};
					let element = element(Space::new().width(width).height(Length::Shrink), surface);
					let size = element.as_widget().size();
					assert_eq!(
						(size.width, size.height),
						(width, Length::Shrink),
						"{width:?}, {shadow:?}"
					);
				}
			}
		}

		#[test]
		fn fill_portion_content_keeps_its_portion_in_a_row() {
			for shadow in [None, Some(Shadow::default())] {
				let surface = Surface {
					shadow,
					..Surface::default()
				};
				let card = element(Space::new().width(Length::FillPortion(3)).height(20.0), surface);
				let mut row: Element<'_, (), iced_widget::Theme, ()> = Row::new()
					.push(card)
					.push(Space::new().width(Length::FillPortion(1)))
					.into();
				let node = layout(&mut row, Size::new(400.0, 60.0));
				let card = &node.children()[0];
				assert_eq!(card.size(), Size::new(300.0, 20.0), "{shadow:?}");
				// The container holds the stack, whose canvas covers the same bounds as the content.
				let stack = &card.children()[0];
				let [canvas, content] = stack.children() else {
					panic!("the stack must hold the canvas and the content");
				};
				assert_eq!(canvas.size(), card.size());
				assert_eq!(content.size(), card.size());
			}
		}

		#[test]
		fn content_with_a_zero_length_is_kept() {
			// `Stack::push` drops children with a `Fixed(0.0)` length; the content must survive.
			let mut element = element(Space::new().width(50.0).height(0.0), Surface::default());
			let node = layout(&mut element, Size::new(400.0, 60.0));
			assert_eq!(node.size(), Size::new(50.0, 0.0));
			assert_eq!(node.children()[0].children().len(), 2);
		}
	}

	#[test]
	fn builder_sets_every_field() {
		let shadow = Shadow {
			color: Color::BLACK,
			offset: Vector::new(0.0, 2.0),
			blur_radius: 8.0,
		};
		let surface = Surface::default()
			.radius(border::top(12.0))
			.exponent(4.0)
			.fill(Color::WHITE)
			.border(Color::BLACK, 2.0)
			.shadow(shadow);
		assert_eq!(
			surface,
			Surface {
				radius: border::top(12.0),
				exponent: 4.0,
				fill: Color::WHITE,
				border: Some(Outline::new(Color::BLACK, 2.0)),
				shadow: Some(shadow),
			}
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

	/// Distance from `p` to the segment from `a` to `b`.
	fn segment_distance(p: Point, a: Point, b: Point) -> f32 {
		let edge = b - a;
		let t = (((p.x - a.x) * edge.x + (p.y - a.y) * edge.y) / (edge.x * edge.x + edge.y * edge.y)).clamp(0.0, 1.0);
		p.distance(a + edge * t)
	}
}
