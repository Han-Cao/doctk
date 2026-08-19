import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  checkPdfFiles,
  type ColorMode,
  type PdfFileReport,
  type PaperSize,
} from "../../lib/tauri";

const MM_PER_INCH = 25.4;
const PT_PER_INCH = 72.0;
const mmToPt = (mm: number) => (mm / MM_PER_INCH) * PT_PER_INCH;
const ptToMm = (pt: number) => (pt / PT_PER_INCH) * MM_PER_INCH;

type Unit = "mm" | "cm" | "in" | "pt";

interface PaperPreset {
  label: string;
  widthMm: number;
  heightMm: number;
}

const PRESETS: Record<string, PaperPreset> = {
  a4: { label: "A4", widthMm: 210, heightMm: 297 },
  a3: { label: "A3", widthMm: 297, heightMm: 420 },
  letter: { label: "Letter", widthMm: 215.9, heightMm: 279.4 },
  legal: { label: "Legal", widthMm: 215.9, heightMm: 355.6 },
  tabloid: { label: "Tabloid", widthMm: 279.4, heightMm: 431.8 },
};

function unitToMm(value: number, unit: Unit): number {
  switch (unit) {
    case "cm":
      return value * 10;
    case "in":
      return value * MM_PER_INCH;
    case "pt":
      return (value / PT_PER_INCH) * MM_PER_INCH;
    case "mm":
      return value;
  }
}

function mmToUnit(mm: number, unit: Unit): number {
  switch (unit) {
    case "cm":
      return mm / 10;
    case "in":
      return mm / MM_PER_INCH;
    case "pt":
      return (mm / MM_PER_INCH) * PT_PER_INCH;
    case "mm":
      return mm;
  }
}

function formatNumber(value: number): string {
  return Number(value.toFixed(3)).toString();
}

export default function PdfCheckerTool() {
  const [files, setFiles] = useState<string[]>([]);
  const [preset, setPreset] = useState("a4");
  const [width, setWidth] = useState("210");
  const [height, setHeight] = useState("297");
  const [unit, setUnit] = useState<Unit>("mm");
  const [tolerance, setTolerance] = useState("0.5");
  const [ignoreOrientation, setIgnoreOrientation] = useState(true);
  const [reports, setReports] = useState<PdfFileReport[] | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function browseFiles() {
    try {
      const selected = await open({
        multiple: true,
        filters: [{ name: "PDF / AI", extensions: ["pdf", "ai"] }],
      });
      if (!selected) return;
      const next = Array.isArray(selected) ? selected : [selected];
      setFiles((current) => Array.from(new Set([...current, ...next])));
      setReports(null);
    } catch (err) {
      setError(String(err));
    }
  }

  function applyPreset(presetId: string, targetUnit: Unit) {
    const info = PRESETS[presetId];
    if (!info) return;
    setPreset(presetId);
    setWidth(formatNumber(mmToUnit(info.widthMm, targetUnit)));
    setHeight(formatNumber(mmToUnit(info.heightMm, targetUnit)));
  }

  function handlePresetChange(nextPreset: string) {
    if (nextPreset === "custom") {
      setPreset("custom");
      return;
    }
    applyPreset(nextPreset, unit);
  }

  function handleUnitChange(nextUnit: Unit) {
    const currentWidth = parseFloat(width);
    const currentHeight = parseFloat(height);
    if (Number.isFinite(currentWidth) && Number.isFinite(currentHeight)) {
      const widthMm = unitToMm(currentWidth, unit);
      const heightMm = unitToMm(currentHeight, unit);
      setWidth(formatNumber(mmToUnit(widthMm, nextUnit)));
      setHeight(formatNumber(mmToUnit(heightMm, nextUnit)));
    } else if (preset !== "custom") {
      applyPreset(preset, nextUnit);
    }
    setUnit(nextUnit);
  }

  function handleWidthChange(value: string) {
    setWidth(value);
    if (preset !== "custom") {
      setPreset("custom");
    }
  }

  function handleHeightChange(value: string) {
    setHeight(value);
    if (preset !== "custom") {
      setPreset("custom");
    }
  }

  async function runCheck() {
    if (!files.length) {
      setError("Select at least one PDF or AI file.");
      return;
    }
    const widthValue = parseFloat(width);
    const heightValue = parseFloat(height);
    const toleranceValue = parseFloat(tolerance);
    if (!Number.isFinite(widthValue) || widthValue <= 0 || !Number.isFinite(heightValue) || heightValue <= 0) {
      setError("Width and height must be positive numbers.");
      return;
    }
    if (!Number.isFinite(toleranceValue) || toleranceValue < 0) {
      setError("Tolerance must be a non-negative number.");
      return;
    }
    try {
      setRunning(true);
      setError(null);
      const paper = buildPaper(widthValue, heightValue);
      const reports = await checkPdfFiles(
        files,
        paper,
        mmToPt(toleranceValue),
        ignoreOrientation,
      );
      setReports(reports);
    } catch (err) {
      setError(String(err));
    } finally {
      setRunning(false);
    }
  }

  function buildPaper(widthValue: number, heightValue: number): PaperSize {
    const widthMm = unitToMm(widthValue, unit);
    const heightMm = unitToMm(heightValue, unit);
    const paperPreset = PRESETS[preset];
    return {
      name: paperPreset ? paperPreset.label : "Custom",
      width_pt: mmToPt(widthMm),
      height_pt: mmToPt(heightMm),
    };
  }

  const totalPages = reports?.flatMap((r) => r.pages).length ?? 0;
  const failedPages = reports?.flatMap((r) => r.pages).filter((p) => !p.fits).length ?? 0;

  return (
    <div>
      <div className="pane-grid">
        <div className="pane">
          <label>PDF / AI files</label>
          <div className="dropzone pdf-dropzone">
            <p>Drop PDF/AI files here.</p>
            {files.length > 0 && (
              <ul className="file-list">
                {files.map((file) => (
                  <li key={file} className="file-row">
                    <span className="file-path">{file}</span>
                    <button
                      className="icon-button danger"
                      title="Remove file"
                      onClick={() => {
                        setFiles((current) => current.filter((f) => f !== file));
                        setReports(null);
                      }}
                    >
                      ✕
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
          <button className="primary" onClick={browseFiles}>
            Browse…
          </button>
        </div>

        <div className="pane">
          <label>Paper size settings</label>
          <div className="toolbar">
            <select value={preset} onChange={(e) => handlePresetChange(e.target.value)}>
              <option value="a4">A4</option>
              <option value="a3">A3</option>
              <option value="letter">Letter</option>
              <option value="legal">Legal</option>
              <option value="tabloid">Tabloid</option>
              <option value="custom">Custom</option>
            </select>
          </div>
          <div className="toolbar" style={{ marginTop: 8 }}>
            <input
              type="number"
              placeholder="Width"
              value={width}
              onChange={(e) => handleWidthChange(e.target.value)}
            />
            <input
              type="number"
              placeholder="Height"
              value={height}
              onChange={(e) => handleHeightChange(e.target.value)}
            />
            <select value={unit} onChange={(e) => handleUnitChange(e.target.value as Unit)}>
              <option value="mm">mm</option>
              <option value="cm">cm</option>
              <option value="in">in</option>
              <option value="pt">pt</option>
            </select>
          </div>
          <label style={{ marginTop: 8 }}>Tolerance (mm)</label>
          <input type="number" step="0.1" value={tolerance} onChange={(e) => setTolerance(e.target.value)} />
          <label style={{ marginTop: 8 }}>
            <input
              type="checkbox"
              checked={ignoreOrientation}
              onChange={(e) => setIgnoreOrientation(e.target.checked)}
            />
            Allow landscape pages to fit portrait paper
          </label>
          <div style={{ marginTop: 12 }}>
            <button className="primary" onClick={runCheck} disabled={running}>
              {running ? "Checking…" : "Run check"}
            </button>
          </div>
        </div>
      </div>

      {error && <div className="error" style={{ marginTop: 12 }}>{error}</div>}

      {reports && (
        <div style={{ marginTop: 16 }}>
          <p className="hint">
            {reports.length} file(s), {totalPages} page(s), {failedPages} fail(s)
          </p>
          <table className="results-table">
            <thead>
              <tr>
                <th>File</th>
                <th>Page</th>
                <th>Color mode</th>
                <th>Width (mm)</th>
                <th>Height (mm)</th>
                <th>Fit</th>
                <th>Reason</th>
              </tr>
            </thead>
            <tbody>
              {reports.flatMap((report) =>
                report.error
                  ? [
                      <tr key={report.path}>
                        <td>{report.path}</td>
                        <td colSpan={6} className="fail">ERROR: {report.error}</td>
                      </tr>,
                    ]
                  : report.pages.map((page) => (
                      <tr key={`${report.path}:${page.page_number}`}>
                        <td>{report.path}</td>
                        <td>{page.page_number}</td>
                        <td>
                          <ColorBadge mode={page.color_mode} />
                        </td>
                        <td>{ptToMm(page.width_pt).toFixed(2)}</td>
                        <td>{ptToMm(page.height_pt).toFixed(2)}</td>
                        <td className={page.fits ? "pass" : "fail"}>{page.fits ? "PASS" : "FAIL"}</td>
                        <td>{page.fit_reason ?? ""}</td>
                      </tr>
                    )),
              )}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function ColorBadge({ mode }: { mode: ColorMode }) {
  const colors: Record<ColorMode, string> = {
    Unknown: "#9ca3af",
    Gray: "#6b7280",
    Rgb: "#2563eb",
    Cmyk: "#d97706",
    Mixed: "#7c3aed",
  };
  return <span style={{ color: colors[mode], fontWeight: 600 }}>{mode}</span>;
}
