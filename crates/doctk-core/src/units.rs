//! Shared unit-conversion helpers and paper-size presets.
//!
//! The PDF checker is the first consumer, but any future print-related tool
//! should reuse these definitions rather than redefining them.

use serde::{Deserialize, Serialize};

pub const MM_PER_INCH: f64 = 25.4;
pub const PT_PER_INCH: f64 = 72.0;
pub const CM_PER_INCH: f64 = 2.54;

/// Physical length unit accepted in CLI and GUI inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    Mm,
    Cm,
    In,
    Pt,
}

impl Unit {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "mm" => Some(Self::Mm),
            "cm" => Some(Self::Cm),
            "in" | "inch" | "inches" => Some(Self::In),
            "pt" | "point" | "points" => Some(Self::Pt),
            _ => None,
        }
    }

    pub fn to_points(self, value: f64) -> f64 {
        match self {
            Self::Mm => mm_to_pt(value),
            Self::Cm => cm_to_pt(value),
            Self::In => in_to_pt(value),
            Self::Pt => value,
        }
    }

    pub fn from_points(self, value_pt: f64) -> f64 {
        match self {
            Self::Mm => pt_to_mm(value_pt),
            Self::Cm => pt_to_cm(value_pt),
            Self::In => pt_to_in(value_pt),
            Self::Pt => value_pt,
        }
    }
}

pub fn mm_to_pt(mm: f64) -> f64 {
    mm / MM_PER_INCH * PT_PER_INCH
}

pub fn pt_to_mm(pt: f64) -> f64 {
    pt / PT_PER_INCH * MM_PER_INCH
}

pub fn cm_to_pt(cm: f64) -> f64 {
    cm / CM_PER_INCH * PT_PER_INCH
}

pub fn pt_to_cm(pt: f64) -> f64 {
    pt / PT_PER_INCH * CM_PER_INCH
}

pub fn in_to_pt(inches: f64) -> f64 {
    inches * PT_PER_INCH
}

pub fn pt_to_in(pt: f64) -> f64 {
    pt / PT_PER_INCH
}

/// A named paper size, expressed in PDF points.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaperSize {
    pub name: String,
    pub width_pt: f64,
    pub height_pt: f64,
}

impl PaperSize {
    pub fn new(name: impl Into<String>, width_pt: f64, height_pt: f64) -> Self {
        Self {
            name: name.into(),
            width_pt,
            height_pt,
        }
    }
}

/// Standard paper-size presets.
///
/// Defined as a function because the preset names are heap-allocated `String`s
/// and a const slice would be overly restrictive.
pub fn paper_preset(name: &str) -> Option<PaperSize> {
    match name.to_ascii_lowercase().as_str() {
        "a4" => Some(PaperSize::new("A4", mm_to_pt(210.0), mm_to_pt(297.0))),
        "a3" => Some(PaperSize::new("A3", mm_to_pt(297.0), mm_to_pt(420.0))),
        "letter" => Some(PaperSize::new("Letter", 612.0, 792.0)),
        "legal" => Some(PaperSize::new("Legal", 612.0, 1008.0)),
        "tabloid" => Some(PaperSize::new("Tabloid", 792.0, 1224.0)),
        _ => None,
    }
}

/// Default fit-check tolerance in points (0.5 mm).
pub fn default_tolerance_pt() -> f64 {
    mm_to_pt(0.5)
}
