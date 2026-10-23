//! PDF / AI page-size and color-mode checker feature.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use lopdf::{Dictionary, Document, Object, ObjectId};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::registry::ToolManifest;
use crate::units::{paper_preset, PaperSize, Unit};

pub const MANIFEST: ToolManifest = ToolManifest {
    id: "pdf_checker",
    name: "PDF Checker",
    description: "Check PDF/AI page size and color mode against paper-size presets.",
    keywords: &["pdf", "ai", "print", "page size", "color mode", "preflight"],
    version: "0.1.0",
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorMode {
    Unknown,
    Gray,
    Rgb,
    Cmyk,
    Mixed,
}

impl ColorMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ColorMode::Unknown => "unknown",
            ColorMode::Gray => "gray",
            ColorMode::Rgb => "rgb",
            ColorMode::Cmyk => "cmyk",
            ColorMode::Mixed => "mixed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PdfPageReport {
    pub page_number: usize,
    pub width_pt: f64,
    pub height_pt: f64,
    pub color_mode: ColorMode,
    pub color_spaces: Vec<String>,
    pub fits: bool,
    pub fit_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PdfFileReport {
    pub path: String,
    pub is_encrypted: bool,
    pub pages: Vec<PdfPageReport>,
    pub error: Option<String>,
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum PdfError {
    #[error("cannot load `{path}`: {detail}")]
    Load { path: String, detail: String },

    #[error("PDF is encrypted")]
    Encrypted,

    #[error("not a PDF-compatible AI file: {path}")]
    NotPdfCompatible { path: String },

    #[error("page {page} is missing both MediaBox and CropBox")]
    MissingPageBox { page: usize },

    #[error("page {page} has invalid page box values")]
    InvalidPageBox { page: usize },

    #[error("unknown paper preset `{0}` (expected a4, a3, letter, legal, or tabloid)")]
    UnknownPaperPreset(String),

    #[error("unknown unit `{0}` (expected mm, cm, in, or pt)")]
    InvalidUnit(String),
}

/// Inspect a single PDF/AI file and return a per-page report.
pub fn inspect_pdf(
    path: &Path,
    paper: &PaperSize,
    tolerance_pt: f64,
    ignore_orientation: bool,
) -> PdfFileReport {
    let path_str = path.to_string_lossy().to_string();

    let loaded = std::fs::read(path)
        .map_err(lopdf::Error::from)
        .and_then(|bytes| load_pdf_bytes(&bytes));
    let doc = match loaded {
        Ok(doc) => doc,
        Err(err) => {
            let detail = err.to_string();
            return PdfFileReport {
                path: path_str.clone(),
                is_encrypted: false,
                pages: Vec::new(),
                error: Some(if is_ai_path(path) && !detail.contains("PDF") {
                    PdfError::NotPdfCompatible {
                        path: path.to_string_lossy().to_string(),
                    }
                    .to_string()
                } else {
                    PdfError::Load {
                        path: path_str,
                        detail,
                    }
                    .to_string()
                }),
            };
        }
    };

    if doc.is_encrypted() {
        return PdfFileReport {
            path: path_str,
            is_encrypted: true,
            pages: Vec::new(),
            error: Some(PdfError::Encrypted.to_string()),
        };
    }

    let mut pages = Vec::new();
    for (page_number, page_id) in doc.get_pages() {
        pages.push(build_page_report(
            &doc,
            page_number as usize,
            page_id,
            paper,
            tolerance_pt,
            ignore_orientation,
        ));
    }

    PdfFileReport {
        path: path_str,
        is_encrypted: false,
        pages,
        error: None,
    }
}

/// Inspect multiple files sequentially.
pub fn inspect_pdfs(
    paths: &[PathBuf],
    paper: &PaperSize,
    tolerance_pt: f64,
    ignore_orientation: bool,
) -> Vec<PdfFileReport> {
    paths
        .iter()
        .map(|path| inspect_pdf(path, paper, tolerance_pt, ignore_orientation))
        .collect()
}

/// Keep strict parsing as the default, tolerating only a whitespace-early xref pointer.
fn load_pdf_bytes(bytes: &[u8]) -> lopdf::Result<Document> {
    match Document::load_mem(bytes) {
        // lopdf 0.35 exposes Error::Parse but keeps its ParseError enum private.
        Err(err) if matches!(&err, lopdf::Error::Parse(detail) if detail.to_string() == "invalid file trailer") =>
        {
            if let Some(corrected) = correct_startxref(bytes) {
                if let Ok(doc) = Document::load_mem(&corrected) {
                    return Ok(doc);
                }
            }
            Err(err)
        }
        result => result,
    }
}

fn is_pdf_whitespace(byte: u8) -> bool {
    matches!(byte, 0 | b'\t' | b'\n' | 12 | b'\r' | b' ')
}

/// Correct only the terminal pointer; never shift object, stream, or /Prev offsets.
fn correct_startxref(bytes: &[u8]) -> Option<Vec<u8>> {
    // lopdf interprets offsets relative to the first PDF header, even with a prefix.
    let header = bytes.windows(5).position(|token| token == b"%PDF-")?;
    let pdf = &bytes[header..];
    let eof = pdf.windows(5).rposition(|token| token == b"%%EOF")?;
    if !pdf[eof + 5..].iter().copied().all(is_pdf_whitespace) {
        return None;
    }
    let marker = pdf[..eof]
        .windows(9)
        .rposition(|token| token == b"startxref")?;
    if marker == 0 || !matches!(pdf[marker - 1], b'\r' | b'\n') {
        return None;
    }
    let mut number_start = marker + 9;
    if !matches!(pdf.get(number_start), Some(b'\r' | b'\n')) {
        return None;
    }
    while number_start < eof && is_pdf_whitespace(pdf[number_start]) {
        number_start += 1;
    }
    let mut number_end = number_start;
    let mut offset = 0usize;
    while number_end < eof && pdf[number_end].is_ascii_digit() {
        offset = offset
            .checked_mul(10)?
            .checked_add((pdf[number_end] - b'0') as usize)?;
        number_end += 1;
    }
    if number_start == number_end
        || number_end == eof
        || !pdf[number_end..eof].iter().copied().all(is_pdf_whitespace)
    {
        return None;
    }

    // No arbitrary search for xref: only skip the whitespace at the declared offset.
    let target = pdf.get(offset..marker)?;
    let whitespace = target.iter().take_while(|&&b| is_pdf_whitespace(b)).count();
    if whitespace == 0 {
        return None;
    }
    let table = &target[whitespace..];
    if !(table.starts_with(b"xref\r") || table.starts_with(b"xref\n")) {
        return None;
    }
    let corrected_offset = offset.checked_add(whitespace)?;
    let mut corrected = bytes[..header + number_start].to_vec();
    corrected.extend_from_slice(corrected_offset.to_string().as_bytes());
    corrected.extend_from_slice(&bytes[header + number_end..]);
    Some(corrected)
}

fn is_ai_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ai"))
}

fn build_page_report(
    doc: &Document,
    page_number: usize,
    page_id: ObjectId,
    paper: &PaperSize,
    tolerance_pt: f64,
    ignore_orientation: bool,
) -> PdfPageReport {
    let mut report = PdfPageReport {
        page_number,
        width_pt: 0.0,
        height_pt: 0.0,
        color_mode: ColorMode::Unknown,
        color_spaces: Vec::new(),
        fits: false,
        fit_reason: None,
    };

    // Page size.
    match page_size_pt(doc, page_id) {
        Ok((width_pt, height_pt)) => {
            report.width_pt = width_pt;
            report.height_pt = height_pt;
            let (fits, reason) =
                check_fit(width_pt, height_pt, paper, tolerance_pt, ignore_orientation);
            report.fits = fits;
            report.fit_reason = reason;
        }
        Err(err) => {
            report.fit_reason = Some(err.to_string());
            return report;
        }
    }

    // Color mode.
    let (color_mode, color_spaces) = detect_color_mode(doc, page_id);
    report.color_mode = color_mode;
    report.color_spaces = color_spaces;

    report
}

fn page_size_pt(doc: &Document, page_id: ObjectId) -> Result<(f64, f64), PdfError> {
    let page = doc
        .get_dictionary(page_id)
        .map_err(|_| PdfError::MissingPageBox {
            page: page_id.0 as usize,
        })?;

    let box_result =
        page_box_pt(doc, page, b"MediaBox").or_else(|| page_box_pt(doc, page, b"CropBox"));

    let Some((width, height)) = box_result else {
        return Err(PdfError::MissingPageBox {
            page: page_id.0 as usize,
        });
    };

    if width <= 0.0 || height <= 0.0 || !width.is_finite() || !height.is_finite() {
        return Err(PdfError::InvalidPageBox {
            page: page_id.0 as usize,
        });
    }

    Ok((width, height))
}

fn page_box_pt(doc: &Document, page: &Dictionary, key: &[u8]) -> Option<(f64, f64)> {
    let obj = page.get(key).ok()?;
    let arr = match resolve_object(doc, obj) {
        Some(Object::Array(arr)) => arr,
        _ => return None,
    };
    if arr.len() < 4 {
        return None;
    }
    let x1 = arr[0].as_float().ok()? as f64;
    let y1 = arr[1].as_float().ok()? as f64;
    let x2 = arr[2].as_float().ok()? as f64;
    let y2 = arr[3].as_float().ok()? as f64;
    Some(((x2 - x1).abs(), (y2 - y1).abs()))
}

fn resolve_object<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a Object> {
    match obj {
        Object::Reference(id) => doc.get_object(*id).ok(),
        other => Some(other),
    }
}

fn check_fit(
    width_pt: f64,
    height_pt: f64,
    paper: &PaperSize,
    tolerance_pt: f64,
    ignore_orientation: bool,
) -> (bool, Option<String>) {
    let pw = paper.width_pt;
    let ph = paper.height_pt;

    let portrait = width_pt <= pw + tolerance_pt && height_pt <= ph + tolerance_pt;
    let landscape =
        ignore_orientation && width_pt <= ph + tolerance_pt && height_pt <= pw + tolerance_pt;

    if portrait || landscape {
        (true, None)
    } else {
        let orientation = if ignore_orientation {
            ""
        } else {
            " (landscape not allowed)"
        };
        let reason = format!(
            "page {:.1} x {:.1} pt does not fit {} ({:.1} x {:.1} pt){}",
            width_pt, height_pt, paper.name, pw, ph, orientation
        );
        (false, Some(reason))
    }
}

// ---------------------------------------------------------------------------
// Color-mode detection
// ---------------------------------------------------------------------------

fn detect_color_mode(doc: &Document, page_id: ObjectId) -> (ColorMode, Vec<String>) {
    let mut found: BTreeSet<String> = BTreeSet::new();
    let mut gray = false;
    let mut rgb = false;
    let mut cmyk = false;

    // Direct page resources.
    if let Ok(page) = doc.get_dictionary(page_id) {
        if let Ok(resources) = page.get(b"Resources") {
            scan_resources_object(doc, resources, &mut gray, &mut rgb, &mut cmyk, &mut found);
        }
    }

    // Inherited resources from page-tree ancestors.
    if let Ok((_, resource_ids)) = doc.get_page_resources(page_id) {
        for resource_id in resource_ids {
            if let Ok(resources) = doc.get_dictionary(resource_id) {
                scan_resources_dictionary(
                    doc, resources, &mut gray, &mut rgb, &mut cmyk, &mut found,
                );
            }
        }
    }

    // Content-stream color operators.
    if let Ok(content) = doc.get_page_content(page_id) {
        scan_content_operators(&content, &mut gray, &mut rgb, &mut cmyk);
    }

    let color_mode = match (gray, rgb, cmyk) {
        (false, false, false) => ColorMode::Unknown,
        (true, false, false) => ColorMode::Gray,
        (false, true, false) => ColorMode::Rgb,
        (false, false, true) => ColorMode::Cmyk,
        _ => ColorMode::Mixed,
    };

    (color_mode, found.into_iter().collect())
}

fn scan_resources_object(
    doc: &Document,
    obj: &Object,
    gray: &mut bool,
    rgb: &mut bool,
    cmyk: &mut bool,
    found: &mut BTreeSet<String>,
) {
    match obj {
        Object::Dictionary(dict) => scan_resources_dictionary(doc, dict, gray, rgb, cmyk, found),
        Object::Reference(id) => {
            if let Ok(dict) = doc.get_dictionary(*id) {
                scan_resources_dictionary(doc, dict, gray, rgb, cmyk, found);
            }
        }
        _ => {}
    }
}

fn scan_resources_dictionary(
    doc: &Document,
    resources: &Dictionary,
    gray: &mut bool,
    rgb: &mut bool,
    cmyk: &mut bool,
    found: &mut BTreeSet<String>,
) {
    // ColorSpace dictionary.
    if let Ok(cs) = resources.get(b"ColorSpace") {
        match cs {
            Object::Dictionary(dict) => {
                for (_, value) in dict.iter() {
                    classify_color_space(doc, value, gray, rgb, cmyk, found);
                }
            }
            Object::Reference(id) => {
                if let Ok(dict) = doc.get_dictionary(*id) {
                    for (_, value) in dict.iter() {
                        classify_color_space(doc, value, gray, rgb, cmyk, found);
                    }
                }
            }
            _ => {}
        }
    }

    // XObject -> image color spaces.
    if let Ok(xobjects) = resources.get(b"XObject") {
        let xobject_dict = match xobjects {
            Object::Dictionary(dict) => Some(dict),
            Object::Reference(id) => doc.get_dictionary(*id).ok(),
            _ => None,
        };
        if let Some(dict) = xobject_dict {
            for (_, value) in dict.iter() {
                classify_image_xobject(doc, value, gray, rgb, cmyk, found);
            }
        }
    }
}

fn classify_image_xobject(
    doc: &Document,
    obj: &Object,
    gray: &mut bool,
    rgb: &mut bool,
    cmyk: &mut bool,
    found: &mut BTreeSet<String>,
) {
    let stream = match obj {
        Object::Stream(stream) => Some(stream),
        Object::Reference(id) => doc.get_object(*id).ok().and_then(|o| match o {
            Object::Stream(stream) => Some(stream),
            _ => None,
        }),
        _ => None,
    };
    let Some(stream) = stream else { return };
    if stream
        .dict
        .get(b"Subtype")
        .ok()
        .and_then(|o| o.as_name().ok())
        != Some(b"Image".as_slice())
    {
        return;
    }
    if let Ok(cs) = stream.dict.get(b"ColorSpace") {
        classify_color_space(doc, cs, gray, rgb, cmyk, found);
    }
}

fn classify_color_space(
    doc: &Document,
    obj: &Object,
    gray: &mut bool,
    rgb: &mut bool,
    cmyk: &mut bool,
    found: &mut BTreeSet<String>,
) {
    match obj {
        Object::Name(name) => {
            classify_color_family_name(name, gray, rgb, cmyk, found);
        }
        Object::Array(arr) => {
            // e.g. [/Indexed /DeviceRGB ...], [/Separation /All /DeviceCMYK ...]
            if let Some(first) = arr.first().and_then(|o| o.as_name().ok()) {
                let family = first.to_ascii_uppercase();
                if family == b"INDEXED" || family == b"SEPARATION" || family == b"PATTERN" {
                    if let Some(base) = arr.get(1) {
                        classify_color_space(doc, base, gray, rgb, cmyk, found);
                    }
                } else if let Some(name) = arr.first().and_then(|o| o.as_name().ok()) {
                    classify_color_family_name(name, gray, rgb, cmyk, found);
                }
            }
        }
        Object::Reference(id) => {
            if let Ok(target) = doc.get_object(*id) {
                classify_color_space(doc, target, gray, rgb, cmyk, found);
            }
        }
        Object::Stream(stream) => {
            classify_iccbased_or_dict(doc, &stream.dict, gray, rgb, cmyk, found)
        }
        Object::Dictionary(dict) => classify_iccbased_or_dict(doc, dict, gray, rgb, cmyk, found),
        _ => {}
    }
}

fn classify_iccbased_or_dict(
    doc: &Document,
    dict: &Dictionary,
    gray: &mut bool,
    rgb: &mut bool,
    cmyk: &mut bool,
    found: &mut BTreeSet<String>,
) {
    // ICCBased profiles have an /N key: 1=Gray, 3=RGB, 4=CMYK.
    if let Ok(n) = dict.get(b"N").and_then(Object::as_i64) {
        match n {
            1 => *gray = true,
            3 => *rgb = true,
            4 => *cmyk = true,
            _ => {}
        }
        found.insert("ICCBased".to_string());
    } else if let Ok(alt) = dict.get(b"Alternate") {
        classify_color_space(doc, alt, gray, rgb, cmyk, found);
    }
}

fn classify_color_family_name(
    name: &[u8],
    gray: &mut bool,
    rgb: &mut bool,
    cmyk: &mut bool,
    found: &mut BTreeSet<String>,
) {
    let upper = name.to_ascii_uppercase();
    if upper == b"DEVICEGRAY" || upper == b"G" || upper == b"CALGRAY" {
        *gray = true;
        found.insert(String::from_utf8_lossy(name).to_string());
    } else if upper == b"DEVICERGB" || upper == b"RGB" || upper == b"CALRGB" {
        *rgb = true;
        found.insert(String::from_utf8_lossy(name).to_string());
    } else if upper == b"DEVICECMYK" || upper == b"CMYK" {
        *cmyk = true;
        found.insert(String::from_utf8_lossy(name).to_string());
    } else if upper == b"ICCBASED" {
        found.insert("ICCBased".to_string());
    } else {
        found.insert(String::from_utf8_lossy(name).to_string());
    }
}

fn scan_content_operators(content: &[u8], gray: &mut bool, rgb: &mut bool, cmyk: &mut bool) {
    let text = String::from_utf8_lossy(content);
    for token in text.split_ascii_whitespace() {
        match token {
            "g" | "G" => *gray = true,
            "rg" | "RG" => *rgb = true,
            "k" | "K" => *cmyk = true,
            _ => {}
        }
    }
}

/// Convenience wrapper used by CLI/GUI to build a paper size from arguments.
pub fn resolve_paper(
    preset: Option<&str>,
    width: Option<f64>,
    height: Option<f64>,
    unit: Option<&str>,
) -> Result<PaperSize, PdfError> {
    if let (Some(width), Some(height)) = (width, height) {
        let unit = unit
            .and_then(Unit::parse)
            .ok_or_else(|| PdfError::InvalidUnit(unit.unwrap_or("").to_string()))?;
        return Ok(PaperSize::new(
            "Custom",
            unit.to_points(width),
            unit.to_points(height),
        ));
    }
    let preset = preset.unwrap_or("a4");
    paper_preset(preset).ok_or_else(|| PdfError::UnknownPaperPreset(preset.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestPdf(PathBuf);

    impl TestPdf {
        fn new(bytes: &[u8], extension: &str) -> Self {
            static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "doctk-pdf-{}-{}.{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed),
                extension,
            ));
            std::fs::write(&path, bytes).unwrap();
            Self(path)
        }

        fn inspect(&self) -> PdfFileReport {
            inspect_pdf(&self.0, &paper_preset("letter").unwrap(), 0.0, false)
        }
    }

    impl Drop for TestPdf {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    // Minimal, public-content-only PDFs with optional incremental /Prev chains.
    fn test_pdf(eol: &str, early_xref: bool, chained: bool) -> Vec<u8> {
        let mut pdf = format!("%PDF-1.4{eol}");
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /ColorSpace << /CS0 /DeviceRGB >> >> >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /ColorSpace << /CS0 /DeviceCMYK >> >> >>",
        ];
        let mut offsets = Vec::new();
        for (i, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.push_str(&format!("{} 0 obj{eol}{object}{eol}endobj{eol}", i + 1));
        }
        let first_xref = pdf.len();
        let entry_end = if eol == "\n" { " \n" } else { "\r\n" };
        pdf.push_str(&format!("xref{eol}0 5{eol}0000000000 65535 f{entry_end}"));
        for offset in offsets {
            pdf.push_str(&format!("{offset:010} 00000 n{entry_end}"));
        }
        let declared = if early_xref && !chained {
            first_xref - eol.len()
        } else {
            first_xref
        };
        pdf.push_str(&format!(
            "trailer{eol}<< /Size 5 /Root 1 0 R >>{eol}startxref{eol}{declared}{eol}%%EOF{eol}"
        ));
        if chained {
            let declared = pdf.len() - if early_xref { eol.len() } else { 0 };
            pdf.push_str(&format!(
                "xref{eol}0 1{eol}0000000000 65535 f{entry_end}trailer{eol}<< /Size 5 /Root 1 0 R /Prev {first_xref} >>{eol}startxref{eol}{declared}{eol}%%EOF{eol}"
            ));
        }
        pdf.into_bytes()
    }

    #[test]
    fn recovers_startxref_on_adjacent_line_break() {
        for eol in ["\n", "\r\n"] {
            for chained in [false, true] {
                let bytes = test_pdf(eol, true, chained);
                assert!(matches!(
                    Document::load_mem(&bytes),
                    Err(lopdf::Error::Parse(err)) if err.to_string() == "invalid file trailer"
                ));
                let file = TestPdf::new(&bytes, "pdf");
                let report = file.inspect();
                assert!(
                    report.error.is_none(),
                    "{eol:?}, chained={chained}: {:?}",
                    report.error
                );
                assert_eq!(report.pages.len(), 2);
                for page in &report.pages {
                    assert_eq!((page.width_pt, page.height_pt), (612.0, 792.0));
                    assert!(page.fits);
                }
                assert_eq!(report.pages[0].color_mode, ColorMode::Rgb);
                assert_eq!(report.pages[1].color_mode, ColorMode::Cmyk);
                assert_eq!(std::fs::read(&file.0).unwrap(), bytes);
            }
        }
    }

    fn replace_startxref(bytes: &[u8], value: &str) -> Vec<u8> {
        let marker = bytes.windows(9).rposition(|w| w == b"startxref").unwrap();
        let mut start = marker + 9;
        while is_pdf_whitespace(bytes[start]) {
            start += 1;
        }
        let mut end = start;
        while bytes[end].is_ascii_digit() {
            end += 1;
        }
        let mut changed = bytes[..start].to_vec();
        changed.extend_from_slice(value.as_bytes());
        changed.extend_from_slice(&bytes[end..]);
        changed
    }

    #[test]
    fn valid_classic_and_chained_pdfs_stay_on_strict_path() {
        for eol in ["\n", "\r\n"] {
            for chained in [false, true] {
                let bytes = test_pdf(eol, false, chained);
                assert!(correct_startxref(&bytes).is_none());
                let strict = Document::load_mem(&bytes).unwrap();
                let loaded = load_pdf_bytes(&bytes).unwrap();
                assert_eq!(loaded.get_pages(), strict.get_pages());
                assert_eq!(TestPdf::new(&bytes, "pdf").inspect().pages.len(), 2);
            }
        }
    }

    #[test]
    fn valid_xref_stream_pdf_stays_on_strict_path() {
        let mut doc = Document::load_mem(&test_pdf("\n", false, false)).unwrap();
        doc.version = "1.5".into();
        doc.reference_table.cross_reference_type = lopdf::xref::XrefType::CrossReferenceStream;
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).unwrap();
        assert!(correct_startxref(&bytes).is_none());
        assert_eq!(load_pdf_bytes(&bytes).unwrap().get_pages().len(), 2);
        let report = TestPdf::new(&bytes, "pdf").inspect();
        assert!(report.error.is_none());
        assert_eq!(report.pages.len(), 2);
        assert_eq!(report.pages[0].color_mode, ColorMode::Rgb);
        assert_eq!(report.pages[1].color_mode, ColorMode::Cmyk);
    }

    #[test]
    fn recovery_preserves_bytes_before_terminal_pointer_and_pdf_prefix() {
        let mut bytes = b"preceding bytes\n".to_vec();
        bytes.extend(test_pdf("\r\n", true, true));
        let original = bytes.clone();
        let corrected = correct_startxref(&bytes).unwrap();
        let marker = bytes.windows(9).rposition(|w| w == b"startxref").unwrap();
        assert_eq!(corrected[..marker + 9], bytes[..marker + 9]);
        assert!(corrected.ends_with(b"\r\n%%EOF\r\n"));
        assert_eq!(load_pdf_bytes(&bytes).unwrap().get_pages().len(), 2);
        assert_eq!(bytes, original);
    }

    #[test]
    fn recovery_allows_terminal_pointer_to_grow_in_digit_count() {
        let mut bytes = b"%PDF-1.4\n".to_vec();
        bytes.resize(99, b' ');
        bytes.extend_from_slice(b"\nxref\ntrailer\nstartxref\n99\n%%EOF\n");
        let corrected = correct_startxref(&bytes).unwrap();
        assert_eq!(&corrected[..100], &bytes[..100]);
        assert!(corrected.ends_with(b"startxref\n100\n%%EOF\n"));
    }

    #[test]
    fn invalid_offsets_and_truncated_files_are_not_recovered() {
        let valid = test_pdf("\n", false, false);
        for value in ["0", "-1", "999999", "1844674407370955161600000", "oops", ""] {
            let bytes = replace_startxref(&valid, value);
            assert!(correct_startxref(&bytes).is_none(), "{value}");
            assert!(load_pdf_bytes(&bytes).is_err(), "{value}");
        }
        let early = test_pdf("\n", true, false);
        for bytes in [&b""[..], &b"%PDF-1.4\n"[..], &early[..early.len() - 10]] {
            assert!(correct_startxref(bytes).is_none());
            assert!(load_pdf_bytes(bytes).is_err());
        }
        let mut trailing_garbage = early;
        trailing_garbage.extend_from_slice(b"not whitespace");
        assert!(correct_startxref(&trailing_garbage).is_none());
    }

    #[test]
    fn recovery_does_not_scan_through_non_whitespace_or_xref_like_tokens() {
        for bytes in [
            b"%PDF-1.4\nJUNKxref\ntrailer\nstartxref\n9\n%%EOF\n".as_slice(),
            b"%PDF-1.4\n\nxrefFake\ntrailer\nstartxref\n9\n%%EOF\n".as_slice(),
        ] {
            assert!(correct_startxref(bytes).is_none());
            assert!(load_pdf_bytes(bytes).is_err());
        }
    }

    #[test]
    fn failed_retry_keeps_original_error() {
        let mut bytes = test_pdf("\n", true, true);
        let xref = bytes.windows(8).rposition(|w| w == b"xref\n0 1").unwrap();
        bytes[xref + 5] = b'X';
        let original_error = Document::load_mem(&bytes).unwrap_err().to_string();
        assert!(correct_startxref(&bytes).is_some());
        assert_eq!(
            load_pdf_bytes(&bytes).unwrap_err().to_string(),
            original_error
        );
    }

    #[test]
    fn encrypted_marker_and_ai_errors_keep_existing_behavior() {
        let mut doc = Document::load_mem(&test_pdf("\n", false, false)).unwrap();
        // Only an encryption dictionary marker is needed for our rejection check.
        let encryption = doc.add_object(lopdf::dictionary! { "Filter" => "Standard" });
        doc.trailer.set("Encrypt", encryption);
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).unwrap();
        let encrypted = TestPdf::new(&bytes, "pdf").inspect();
        assert!(encrypted.is_encrypted);
        assert_eq!(encrypted.error.as_deref(), Some("PDF is encrypted"));
        assert!(encrypted.pages.is_empty());

        let ai = TestPdf::new(&test_pdf("\r\n", true, true), "ai").inspect();
        assert!(ai.error.is_none());
        assert_eq!(ai.pages.len(), 2);
        let not_pdf = TestPdf::new(b"%!PS-Adobe-3.0\n", "ai").inspect();
        assert!(not_pdf
            .error
            .unwrap()
            .contains("not a PDF-compatible AI file"));
    }

    #[test]
    fn file_and_batch_errors_remain_per_file() {
        let corrupt = TestPdf::new(b"not a PDF", "pdf");
        let valid = TestPdf::new(&test_pdf("\n", true, true), "pdf");
        let missing = TestPdf::new(b"", "pdf");
        std::fs::remove_file(&missing.0).unwrap();
        let reports = inspect_pdfs(
            &[corrupt.0.clone(), missing.0.clone(), valid.0.clone()],
            &paper_preset("letter").unwrap(),
            0.0,
            false,
        );
        assert_eq!(reports.len(), 3);
        for report in &reports[..2] {
            assert!(report.error.as_ref().unwrap().contains(&report.path));
            assert!(report.pages.is_empty());
        }
        assert!(reports[2].error.is_none());
        assert_eq!(reports[2].pages.len(), 2);
    }

    #[test]
    fn a4_page_fits_a4() {
        let a4 = paper_preset("a4").unwrap();
        let (fits, reason) = check_fit(a4.width_pt, a4.height_pt, &a4, 0.0, false);
        assert!(fits);
        assert!(reason.is_none());
    }

    #[test]
    fn landscape_a4_requires_ignore_orientation() {
        let a4 = paper_preset("a4").unwrap();
        let (fits, _) = check_fit(a4.height_pt, a4.width_pt, &a4, 0.0, false);
        assert!(!fits);
        let (fits2, _) = check_fit(a4.height_pt, a4.width_pt, &a4, 0.0, true);
        assert!(fits2);
    }

    #[test]
    fn letter_does_not_fit_a4() {
        let a4 = paper_preset("a4").unwrap();
        let letter = paper_preset("letter").unwrap();
        let (fits, _) = check_fit(letter.width_pt, letter.height_pt, &a4, 0.0, true);
        assert!(!fits);
    }

    #[test]
    fn unit_conversions_are_reversible() {
        assert!((Unit::Mm.to_points(210.0) - 595.2755905511812).abs() < 0.001);
        assert!((Unit::In.from_points(Unit::In.to_points(8.5)) - 8.5).abs() < 1e-9);
    }

    #[test]
    fn resolve_paper_custom() {
        let paper = resolve_paper(None, Some(210.0), Some(297.0), Some("mm")).unwrap();
        assert_eq!(paper.name, "Custom");
        assert!((paper.width_pt - 595.2755905511812).abs() < 0.001);
    }
}
