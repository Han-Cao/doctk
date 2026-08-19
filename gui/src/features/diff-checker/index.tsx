import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { readTextFile } from "@tauri-apps/plugin-fs";
import {
  diffSideBySide,
  diffTrackChanges,
  type SideBySideDiff,
  type TrackChangesDiff,
} from "../../lib/tauri";

const LEFT_SAMPLE = "The quick brown fox\njumps over the lazy dog\n";
const RIGHT_SAMPLE = "The quick red fox\njumps over the lazy cat\n";

export default function DiffCheckerTool() {
  const [left, setLeft] = useState(LEFT_SAMPLE);
  const [right, setRight] = useState(RIGHT_SAMPLE);
  const [view, setView] = useState<"side-by-side" | "track-changes">("side-by-side");
  const [sideBySide, setSideBySide] = useState<SideBySideDiff | null>(null);
  const [trackChanges, setTrackChanges] = useState<TrackChangesDiff | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function openFile(setText: (value: string) => void) {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "Text", extensions: ["txt", "md", "tsv", "csv"] }],
      });
      if (typeof selected === "string") {
        setText(await readTextFile(selected));
      }
    } catch (err) {
      setError(String(err));
    }
  }

  useEffect(() => {
    const timer = setTimeout(async () => {
      try {
        setSideBySide(await diffSideBySide(left, right));
        setTrackChanges(await diffTrackChanges(left, right));
        setError(null);
      } catch (err) {
        setError(String(err));
      }
    }, 250);
    return () => clearTimeout(timer);
  }, [left, right]);

  return (
    <div>
      <div className="diff-input-grid">
        <div className="pane">
          <label>Original</label>
          <div className="diff-open-row left">
            <button className="small-button" onClick={() => openFile(setLeft)}>
              Open file…
            </button>
          </div>
          <textarea
            className="diff-textarea"
            value={left}
            onChange={(e) => setLeft(e.target.value)}
            spellCheck={false}
          />
        </div>

        <div className="diff-swap-column">
          <div className="diff-swap-spacer" />
          <div className="diff-open-row center">
            <button
              className="small-button"
              title="Swap Original and Changed"
              aria-label="Swap Original and Changed"
              onClick={() => {
                setLeft(right);
                setRight(left);
              }}
            >
              ⇄
            </button>
          </div>
        </div>

        <div className="pane">
          <label>Changed</label>
          <div className="diff-open-row right">
            <button className="small-button" onClick={() => openFile(setRight)}>
              Open file…
            </button>
          </div>
          <textarea
            className="diff-textarea"
            value={right}
            onChange={(e) => setRight(e.target.value)}
            spellCheck={false}
          />
        </div>
      </div>

      <div className="toolbar diff-view-toolbar">
        <button className={view === "side-by-side" ? "primary" : ""} onClick={() => setView("side-by-side")}>
          Side by side
        </button>
        <button className={view === "track-changes" ? "primary" : ""} onClick={() => setView("track-changes")}>
          Track changes
        </button>
      </div>

      {error && <div className="error">{error}</div>}

      {view === "side-by-side" ? (
        <SideBySideView diff={sideBySide} />
      ) : (
        <TrackChangesView diff={trackChanges} originalText={left} changedText={right} />
      )}
    </div>
  );
}

function SideBySideView({ diff }: { diff: SideBySideDiff | null }) {
  if (!diff) return <p className="hint">Computing diff…</p>;
  return (
    <div style={{ maxHeight: 480, overflow: "auto" }}>
      {diff.lines.map((line, index) => (
        <div key={index} className={`diff-line ${line.kind.toLowerCase()}`}>
          <span>
            <GutterLineNumber n={line.line_number_left} side="-" />
            <HighlightedText text={line.left_text} ranges={line.word_diff} leftSide />
          </span>
          <span>
            <GutterLineNumber n={line.line_number_right} side="+" />
            <HighlightedText text={line.right_text} ranges={line.word_diff} leftSide={false} />
          </span>
        </div>
      ))}
    </div>
  );
}

function TrackChangesView({
  diff,
  originalText,
  changedText,
}: {
  diff: TrackChangesDiff | null;
  originalText: string;
  changedText: string;
}) {
  const [copyMessage, setCopyMessage] = useState<string | null>(null);

  async function copyText(text: string, label: string) {
    const clipboard = navigator.clipboard;
    if (!clipboard) {
      setCopyMessage("Clipboard API is not available in this WebView.");
      return;
    }
    try {
      await clipboard.writeText(text);
      setCopyMessage(`${label} copied to clipboard.`);
    } catch (err) {
      setCopyMessage(`Copy failed: ${String(err)}`);
    }
  }

  if (!diff) return <p className="hint">Computing diff…</p>;
  return (
    <div>
      <div className="toolbar" style={{ marginBottom: 8 }}>
        <button className="small-button" onClick={() => copyText(originalText, "Original")}>
          Copy original
        </button>
        <button className="small-button" onClick={() => copyText(changedText, "Changed")}>
          Copy changed
        </button>
      </div>
      {copyMessage && <p className="hint clipboard-message">{copyMessage}</p>}
      <div className="track-changes">
        {diff.segments.map((segment, index) => {
          if (segment.kind === "Deleted") return <del key={index}>{segment.text}</del>;
          if (segment.kind === "Inserted") return <ins key={index}>{segment.text}</ins>;
          return <span key={index}>{segment.text}</span>;
        })}
      </div>
    </div>
  );
}

function GutterLineNumber({ n, side }: { n: number | null; side: string }) {
  return <span style={{ color: "#9ca3af", minWidth: 36, display: "inline-block" }}>{n ?? " "}</span>;
}

function byteIndexToJsIndex(text: string, byteIndex: number): number {
  if (byteIndex <= 0) return 0;
  let bytes = 0;
  let i = 0;
  while (i < text.length && bytes < byteIndex) {
    const codePoint = text.codePointAt(i);
    if (codePoint === undefined) break;
    const codePointBytes = codePoint <= 0x7f ? 1 : codePoint <= 0x7ff ? 2 : codePoint <= 0xffff ? 3 : 4;
    if (bytes + codePointBytes > byteIndex) break;
    bytes += codePointBytes;
    i += codePoint > 0xffff ? 2 : 1;
  }
  return i;
}

function HighlightedText(props: {
  text: string;
  ranges: import("../../lib/tauri").WordRange[];
  leftSide: boolean;
}) {
  const { text, ranges, leftSide } = props;
  if (!ranges.length) return <>{text}</>;
  const parts: React.ReactNode[] = [];
  let cursor = 0;
  ranges.forEach((range, index) => {
    const byteStart = leftSide ? range.left_start : range.right_start;
    const byteLen = leftSide ? range.left_len : range.right_len;
    if (!byteLen) return;
    const byteEnd = byteStart + byteLen;
    const start = byteIndexToJsIndex(text, byteStart);
    const end = byteIndexToJsIndex(text, byteEnd);
    const len = end - start;
    if (len <= 0 || start < cursor || end > text.length) return;
    parts.push(<span key={`plain-${index}`}>{text.slice(cursor, start)}</span>);
    parts.push(<mark key={`mark-${index}`}>{text.slice(start, end)}</mark>);
    cursor = end;
  });
  parts.push(<span key="tail">{text.slice(cursor)}</span>);
  return <>{parts}</>;
}
