// Copyright 2026 David Pettersson
// SPDX-License-Identifier: Apache-2.0

//! Drawing and layout primitives shared by the diagram kinds. Nothing here knows
//! about UML: callers translate their models into these plain input types.

pub mod align;
pub mod ends;
pub mod flow;
pub mod geometry;
pub mod glyphs;
pub mod grow;
pub mod labels;
pub mod layered;
pub mod lettering;
pub mod metrics;
pub mod router;
pub mod style;
pub mod svg;
pub mod text;
