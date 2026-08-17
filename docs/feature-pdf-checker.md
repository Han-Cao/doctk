# Feature Plan: PDF Checker

**Tool id:** `pdf_checker`
**Tool name:** PDF Checker
**Status:** planned
**Framework:** see `docs/tool-framework.md`

## 1. Overview

Check Acrobat PDF(s) and AI file(s) for page size and color mode, and verify whether each page fits within a specific paper size. Presets (A4, A3, Letter, Legal, Tabloid) and user-specified custom sizes are supported.

## 2. Scope

### In scope
- PDF page size detection (points, with mm/in display).
- Color mode classification: grayscale, RGB, CMYK, mixed, or unknown.
- Fit check against presets and custom sizes, with configurable tolerance.
- Orientation-insensitive fit option (page may be landscape or portrait).
- Multi-page PDFs with per-page results.
- PDF-compatible `.ai` files (those containing a PDF stream).
- CLI table and JSON output; GUI drag-and-drop, settings, results table, export.

### Out of scope (v1)
- Non-PDF-compatible `.ai` files (clear error is returned).
- Encrypted PDFs (clear error is returned; no password entry in v1).
- Full rasterization-based color analysis (color detection is based on PDF resource and content-stream inspection).
- Checking bleed/trim boxes; only `MediaBox` with `CropBox` fallback is used.
- Repairing or converting files.

## 3. Core Design (`doctk-core/src/features/pdf_checker.rs`)

### 3.1 Data model

```rust
pub struct PaperSize {
    pub name: String,        // "A4", "Letter", "Custom"
    pub width_pt: f64,
    pub height_pt: f64,
}

pub enum ColorMode { Unknown, Gray, Rgb, Cmyk, Mixed }

pub struct PdfPageReport {
    pub page_number: usize,
    pub width_pt: f64,
    pub height_pt: f64,
    pub color_mode: ColorMode,
    pub color_spaces: Vec<String>,    // e.g. ["DeviceRGB", "ICCBased", "DeviceCMYK"]
    pub fits: bool,
    pub fit_reason: Option<String>,
}

pub struct PdfFileReport {
    pub path: String,
    pub is_encrypted: bool,
    pub pages: Vec<PdfPageReport>,
    pub error: Option<String>,
}
```

### 3.2 Public API

```rust
pub const MANIFEST: ToolManifest;

pub fn default_paper(name: &str) -> Option<PaperSize>;
pub fn custom_paper(width: f64, height: f64, unit: Unit) -> PaperSize;
pub fn inspect_pdf(path: &Path, paper: &PaperSize, tolerance_pt: f64, ignore_orientation: bool) -> PdfFileReport;
pub fn inspect_pdfs(paths: &[PathBuf], paper: &PaperSize, tolerance_pt: f64, ignore_orientation: bool) -> Vec<PdfFileReport>;
```

`Unit` and preset constants live in shared `doctk-core/src/units.rs` because future print-related tools will reuse them.

### 3.3 Page size detection

1. For each page, read `MediaBox`; fallback to `CropBox`.
2. If both are missing, report a page-level error.
3. Normalize coordinates:
   - `width = abs(x2 - x1)`
   - `height = abs(y2 - y1)`
   - Swapped coordinates are normalized.
4. Report sizes in points; CLI and GUI convert to mm/inches for display.

### 3.4 Fit check

```text
fits = width  <= paper.width  + tolerance && height <= paper.height + tolerance   (portrait match)
    OR width  <= paper.height + tolerance && height <= paper.width  + tolerance   (landscape match, when ignore_orientation)
```

- Default tolerance: `0.5 mm` (converted to points).
- `ignore_orientation=true` accepts a landscape page on a portrait paper size.
- `fit_reason` examples:
  - `page 210.2 x 297.3 mm exceeds A4 by 0.3 mm`
  - `page 279.4 x 215.9 mm fits Letter (landscape)`

### 3.5 Color mode detection

For each page, inspect two sources:

1. **Page resources**:
   - `Resources -> ColorSpace`: classify entries:
     - `DeviceGray`, `G` → Gray
     - `DeviceRGB`, `RGB` → RGB
     - `DeviceCMYK`, `CMYK` → CMYK
     - `ICCBased` → read embedded ICC profile or alternate color space where possible
     - `Indexed`, `Separation`, `Pattern` → resolve base/alternate space where possible
   - `Resources -> XObject -> Image`: read image dictionary `ColorSpace`, `Filter`, `BitsPerComponent`.
2. **Page content stream operators**:
   - `g`, `G` (gray stroke/fill)
   - `rg`, `RG` (RGB stroke/fill)
   - `k`, `K` (CMYK stroke/fill)
   - `cs`, `CS`, `scn`, `SCN` (color space selection)

Classification per page:

| Detected families | ColorMode |
|---|---|
| None | Unknown |
| Only gray | Gray |
| Only RGB | Rgb |
| Only CMYK | Cmyk |
| More than one | Mixed |

Limitations (documented in README):
- Detection is based on resource/operator inspection, not rasterization.
- Inline images may be missed.
- Complex transparency groups may under-report color spaces.

### 3.6 AI file support

- `.ai` files that begin with `%PDF-` are parsed as PDFs.
- `.ai` files without a PDF stream return `PdfError::NotPdfCompatible`.
- GUI file picker and drag-drop accept `.pdf` and `.ai`.

## 4. CLI Design (`doctk-cli/src/commands/pdf_checker.rs`)

```bash
doctk pdf check FILES... \
    [--paper a4|a3|letter|legal|tabloid] \
    [--width VALUE --height VALUE --unit mm|cm|in|pt] \
    [--tolerance VALUE --tolerance-unit mm|cm|in|pt] \
    [--ignore-orientation] \
    [--json] \
    [--list]
```

- Default output: human-readable summary table (`file | pages | min size | max size | color modes | fit`).
- `--list`: one row per page with page number, size, color mode, fit result, reason.
- `--json`: structured report (`Vec<PdfFileReport>`) for scripting.
- Exit codes:
  - `0`: all pages fit.
  - `3`: at least one page does not fit.
  - `1`: processing error (unreadable, encrypted, not PDF-compatible, invalid custom size).

## 5. GUI Design (`gui/src/features/pdf-checker/`)

### 5.1 Layout

```text
[ Drop PDF/AI files here or click to browse ]       [ Settings                  ]
[ file list with remove buttons                ]     [ Preset: A4 [dropdown]    ]
                                                    [ Custom: W [  ] H [  ] mm  ]
                                                    [ Tolerance: [0.5] mm        ]
                                                    [ [x] Allow landscape       ]
                                                    [ ( Run check )             ]
----------------------------------------------------------------------------------
[ Summary bar: 3 files, 12 pages, 10 pass, 2 fail                                 ]
[ Results table: file | page | size (mm) | color mode | fit | reason              ]
[ ( Export CSV / Export JSON )                                                     ]
```

### 5.2 Behavior

- Drag-and-drop `.pdf`/`.ai` files from the OS using Tauri file-drop events.
- File picker via Tauri dialog plugin.
- Settings changes invalidate current results; click **Run check** (or auto-run after drop, if enabled) to recompute.
- Results table:
  - Pass/fail icon per page.
  - Size displayed in mm, with pt in tooltip.
  - Color mode as a color-coded badge: Gray, RGB, CMYK, Mixed, Unknown.
  - `fit_reason` shown when a page fails; empty when it passes.
- Summary bar shows file/page/pass/fail counts.
- Export report to CSV or JSON via Tauri save dialog.
- Batch processing is async with a progress indicator; results use a generation counter to discard stale results.

### 5.3 Tauri commands

```rust
#[tauri::command] pub async fn pdf_checker_check_files(
    paths: Vec<String>,
    paper: PaperSize,
    tolerance_pt: f64,
    ignore_orientation: bool,
) -> Result<Vec<PdfFileReport>, String>;
```

- Heavy PDF parsing runs in `tauri::async_runtime::spawn_blocking`.
- `PaperSize` is JSON-serializable; the frontend builds it from preset id or custom values.

## 6. Edge Cases

| Case | Expected behavior |
|---|---|
| A4 portrait page vs A4 | Pass |
| A4 landscape page vs A4 with `--ignore-orientation` | Pass (landscape) |
| A4 landscape page vs A4 without `--ignore-orientation` | Fail with orientation reason |
| Letter page vs A4 | Fail (Letter is wider than A4) |
| Custom size in mm/cm/in/pt | Converted to points; check with tolerance |
| Multi-page PDF with mixed sizes | Per-page fit results |
| RGB / CMYK / Gray / Mixed PDF | Correct color mode badge per page |
| Encrypted PDF | `is_encrypted=true`, error message, exit 1 |
| Non-PDF-compatible `.ai` | Clear error, exit 1 |
| Corrupt file | Error with file path, other files still processed |
| Missing MediaBox/CropBox | Page-level error |

## 7. Testing

### Core (unit + fixtures)
- A4 portrait/landscape vs A4 preset, with and without orientation ignore.
- Letter vs A4.
- Custom size unit conversions (`mm`, `cm`, `in`, `pt`) and tolerance.
- Multi-page PDF with mixed sizes (minimal generated PDF fixtures).
- Color mode classification for generated RGB/CMYK/Gray PDF fixtures.
- Mixed color spaces → `Mixed`.
- Encrypted PDF fixture → `is_encrypted=true`.
- Non-PDF `.ai` fixture → `NotPdfCompatible`.
- Corrupt file → error.

### CLI (`assert_cmd`)
- Summary output for a passing and failing PDF.
- `--json` outputs valid JSON matching `PdfFileReport` schema.
- `--list` per-page output.
- Exit codes `0`, `1`, `3`.

### GUI (Vitest)
- Result table row rendering logic (pass/fail icon, color badge).
- Settings form validation (custom size, tolerance).
- Stale-result generation counter.

## 8. Future Extensions

- Support password-protected PDFs.
- Check bleed/trim/art boxes.
- Full rasterization-based color audit for complex PDFs.
- Batch export to PDF report.
- Integration with print preflight profiles.
- Support non-PDF-compatible AI files by parsing Illustrator binary format.
