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

export default function PdfCheckerTool() {
  const [files, setFiles] = useState<string[]>([]);
  const [preset, setPreset] = useState("a4");
  const [customWidth, setCustomWidth] = useState("");
  const [customHeight, setCustomHeight] = useState("");
  const [unit, setUnit] = useState("mm");
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

  async function runCheck() {
    if (!files.length) {
      setError("Select at least one PDF or AI file.");
      return;
    }
    try {
      setRunning(true);
      setError(null);
      const paper = buildPaper();
      const reports = await checkPdfFiles(files, paper, mmToPt(parseFloat(tolerance || "0")), ignoreOrientation);
      setReports(reports);
    } catch (err) {
      setError(String(err));
    } finally {
      setRunning(false);
    }
  }

  function buildPaper(): PaperSize {
    if (customWidth && customHeight) {
      const factor = unit === "cm" ? mmToPt(10) : unit === "in" ? PT_PER_INCH : unit === "pt" ? 1 : mmToPt(1);
      return {
        name: "Custom",
        width_pt: parseFloat(customWidth) * factor,
        height_pt: parseFloat(customHeight) * factor,
      };
    }
    const presets: Record<string, PaperSize> = {
      a4: { name: "A4", width_pt: mmToPt(210), height_pt: mmToPt(297) },
      a3: { name: "A3", width_pt: mmToPt(297), height_pt: mmToPt(420) },
      letter: { name: "Letter", width_pt: 612, height_pt: 792 },
      legal: { name: "Legal", width_pt: 612, height_pt: 1008 },
      tabloid: { name: "Tabloid", width_pt: 792, height_pt: 1224 },
    };
    return presets[preset] ?? presets.a4;
  }

  const totalPages = reports?.flatMap((r) => r.pages).length ?? 0;
  const failedPages = reports?.flatMap((r) => r.pages).filter((p) => !p.fits).length ?? 0;

  return (
    <div>
      <div className="pane-grid">
        <div className="pane">
          <label>PDF / AI files</label>
          <div className="dropzone">
            <p>Drop PDF/AI files here or click to browse.</p>
            <button className="primary" onClick={browseFiles}>Browse…</button>
          </div>
          {files.length > 0 && (
            <ul>
              {files.map((file) => (
                <li key={file}>
                  <span style={{ marginRight: 8 }}>{file}</span>
                  <button
                    className="danger"
                    onClick={() => {
                      setFiles((current) => current.filter((f) => f !== file));
                      setReports(null);
                    }}
                  >
                    Remove
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="pane">
          <label>Paper size settings</label>
          <div className="toolbar">
            <select value={preset} onChange={(e) => { setPreset(e.target.value); setCustomWidth(""); setCustomHeight(""); }}>
              <option value="a4">A4</option>
              <option value="a3">A3</option>
              <option value="letter">Letter</option>
              <option value="legal">Legal</option>
              <option value="tabloid">Tabloid</option>
              <option value="custom">Custom</option>
            </select>
          </div>
          {preset === "custom" && (
            <div className="toolbar" style={{ marginTop: 8 }}>
              <input type="number" placeholder="Width" value={customWidth} onChange={(e) => setCustomWidth(e.target.value)} />
              <input type="number" placeholder="Height" value={customHeight} onChange={(e) => setCustomHeight(e.target.value)} />
              <select value={unit} onChange={(e) => setUnit(e.target.value)}>
                <option value="mm">mm</option>
                <option value="cm">cm</option>
                <option value="in">in</option>
                <option value="pt">pt</option>
              </select>
            </div>
          )}
          <label style={{ marginTop: 8 }}>Tolerance (mm)</label>
          <input type="number" step="0.1" value={tolerance} onChange={(e) => setTolerance(e.target.value)} />
          <label style={{ marginTop: 8 }}>
            <input type="checkbox" checked={ignoreOrientation} onChange={(e) => setIgnoreOrientation(e.target.checked)} />
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
                <th>Width (mm)</th>
                <th>Height (mm)</th>
                <th>Color</th>
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
                        <td>{ptToMm(page.width_pt).toFixed(2)}</td>
                        <td>{ptToMm(page.height_pt).toFixed(2)}</td>
                        <td><ColorBadge mode={page.color_mode} /></td>
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
