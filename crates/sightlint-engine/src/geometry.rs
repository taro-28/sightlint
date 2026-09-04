//! Deterministic geometry queries over validated Artifact IR.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use sightlint_ir::{
    ArtifactIr, Axis, BoxKind, Canvas, HorizontalDirection, Identifier, Node, ObservedRect, Rect,
    Unit, VerticalDirection,
};

/// One rectangle resolved with its coordinate system, unit, and provenance.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedRect<'a> {
    /// Axis-aligned rectangle.
    pub rect: Rect,
    /// Coordinate space containing the rectangle.
    pub coordinate_space_id: &'a Identifier,
    /// Unit of the coordinate space.
    pub unit: Unit,
    /// Evidence supporting the rectangle.
    pub evidence_id: &'a Identifier,
    /// Canvas that defines direction and extent.
    pub canvas: &'a Canvas,
}

/// Deterministic query failure that should normally become `cantTell`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryError {
    /// A node referenced by a validated relation was not found.
    MissingNode(String),
    /// A rectangle references a canvas that was not found.
    MissingCanvas(String),
    /// Two measurements cannot be compared without a coordinate transform.
    IncomparableCoordinateSpaces {
        /// First coordinate-space identifier.
        first: String,
        /// Second coordinate-space identifier.
        second: String,
    },
}

impl fmt::Display for QueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(id) => write!(formatter, "node {id} does not exist"),
            Self::MissingCanvas(id) => write!(formatter, "canvas {id} does not exist"),
            Self::IncomparableCoordinateSpaces { first, second } => write!(
                formatter,
                "coordinate spaces {first} and {second} are not directly comparable"
            ),
        }
    }
}

impl Error for QueryError {}

/// Indexed, read-only view used by deterministic rules.
#[derive(Debug)]
pub struct QueryContext<'a> {
    document: &'a ArtifactIr,
    nodes: BTreeMap<&'a str, &'a Node>,
    canvases: BTreeMap<&'a str, &'a Canvas>,
}

impl<'a> QueryContext<'a> {
    /// Builds indexes over an already validated document.
    pub fn new(document: &'a ArtifactIr) -> Self {
        Self {
            document,
            nodes: document
                .nodes
                .iter()
                .map(|node| (node.id.as_str(), node))
                .collect(),
            canvases: document
                .canvases
                .iter()
                .map(|canvas| (canvas.id.as_str(), canvas))
                .collect(),
        }
    }

    /// Returns the indexed document.
    pub const fn document(&self) -> &'a ArtifactIr {
        self.document
    }

    /// Resolves a node by stable identifier.
    pub fn node(&self, id: &Identifier) -> Result<&'a Node, QueryError> {
        self.nodes
            .get(id.as_str())
            .copied()
            .ok_or_else(|| QueryError::MissingNode(id.to_string()))
    }

    /// Resolves a canvas by stable identifier.
    pub fn canvas(&self, id: &Identifier) -> Result<&'a Canvas, QueryError> {
        self.canvases
            .get(id.as_str())
            .copied()
            .ok_or_else(|| QueryError::MissingCanvas(id.to_string()))
    }

    /// Resolves the requested rectangle for a node.
    pub fn rect(
        &self,
        node_id: &Identifier,
        box_kind: BoxKind,
    ) -> Result<Option<ResolvedRect<'a>>, QueryError> {
        let node = self.node(node_id)?;
        let observed = match box_kind {
            BoxKind::Layout => node.geometry.layout_box.as_ref(),
            BoxKind::Render => node.geometry.render_box.as_ref(),
            BoxKind::Ink => node.geometry.ink_box.as_ref(),
            BoxKind::Hit => node.geometry.hit_box.as_ref(),
        };
        observed.map(|value| self.resolve_rect(value)).transpose()
    }

    fn resolve_rect(&self, observed: &'a ObservedRect) -> Result<ResolvedRect<'a>, QueryError> {
        let canvas = self.canvas(&observed.coordinate_space_id)?;
        Ok(ResolvedRect {
            rect: observed.rect,
            coordinate_space_id: &observed.coordinate_space_id,
            unit: canvas.unit,
            evidence_id: &observed.evidence_id,
            canvas,
        })
    }
}

/// Ensures two measurements share an exact coordinate space and unit.
pub fn ensure_comparable(
    first: ResolvedRect<'_>,
    second: ResolvedRect<'_>,
) -> Result<(), QueryError> {
    if first.coordinate_space_id == second.coordinate_space_id && first.unit == second.unit {
        Ok(())
    } else {
        Err(QueryError::IncomparableCoordinateSpaces {
            first: first.coordinate_space_id.to_string(),
            second: second.coordinate_space_id.to_string(),
        })
    }
}

/// Returns the non-negative horizontal and vertical overlap extents.
pub fn overlap_extents(first: Rect, second: Rect) -> (f64, f64) {
    let horizontal = (right(first).min(right(second)) - first.x.max(second.x)).max(0.0);
    let vertical = (bottom(first).min(bottom(second)) - first.y.max(second.y)).max(0.0);
    (horizontal, vertical)
}

/// Returns the directed gap between two ordered rectangles.
pub fn ordered_gap(
    first: ResolvedRect<'_>,
    second: ResolvedRect<'_>,
    axis: Axis,
) -> Result<f64, QueryError> {
    ensure_comparable(first, second)?;
    let gap = match axis {
        Axis::Horizontal => match first.canvas.horizontal_direction {
            HorizontalDirection::Right => second.rect.x - right(first.rect),
            HorizontalDirection::Left => first.rect.x - right(second.rect),
        },
        Axis::Vertical => match first.canvas.vertical_direction {
            VerticalDirection::Down => second.rect.y - bottom(first.rect),
            VerticalDirection::Up => first.rect.y - bottom(second.rect),
        },
    };
    Ok(normalize_zero(gap))
}

/// Returns whether a rectangle lies inside a zero-origin canvas within tolerance.
pub fn within_canvas(rect: Rect, canvas: &Canvas, tolerance: f64) -> bool {
    rect.x >= -tolerance
        && rect.y >= -tolerance
        && right(rect) <= canvas.size.width + tolerance
        && bottom(rect) <= canvas.size.height + tolerance
}

/// Returns the right edge of a rectangle.
pub fn right(rect: Rect) -> f64 {
    normalize_zero(rect.x + rect.width)
}

/// Returns the bottom edge of a rectangle.
pub fn bottom(rect: Rect) -> f64 {
    normalize_zero(rect.y + rect.height)
}

fn normalize_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}

#[cfg(test)]
mod tests {
    use sightlint_ir::Rect;

    use super::overlap_extents;

    #[test]
    fn overlap_extents_never_return_negative_values() {
        let first = Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        };
        let second = Rect {
            x: 20.0,
            y: 20.0,
            width: 5.0,
            height: 5.0,
        };

        assert_eq!(overlap_extents(first, second), (0.0, 0.0));
    }
}
