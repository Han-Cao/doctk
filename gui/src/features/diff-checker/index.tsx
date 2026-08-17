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
      <div className="pane-grid">
        <div className="pane">
          <label>Left / original</label>
          <textarea rows={12} value={left} onChange={(e) => setLeft(e.target.value)} spellCheck={false} />
          <button onClick={() => openFile(setLeft)}>Open file…</button>
        </div>
        <div className="pane">
          <label>Right / changed</label>
          <textarea rows={12} value={right} onChange={(e) => setRight(e.target.value)} spellCheck={false} />
          <button onClick={() => openFile(setRight)}>Open file…</button>
        </div>
      </div>

      <div className="toolbar" style={{ margin: "12px 0" }}>
        <button className={view === "side-by-side" ? "primary" : ""} onClick={() => setView("side-by-side")}>
          Side by side
        </button>
        <button className={view === "track-changes" ? "primary" : ""} onClick={() => setView("track-changes")}>
          Track changes
        </button>
        <button onClick={() => { setLeft(right); setRight(left); }}>Swap</button>
      </div>

      {error && <div className="error">{error}</div>}

      {view === "side-by-side" ? (
        <SideBySideView diff={sideBySide} />
      ) : (
        <TrackChangesView diff={trackChanges} />
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

function TrackChangesView({ diff }: { diff: TrackChangesDiff | null }) {
  if (!diff) return <p className="hint">Computing diff…</p>;
  return (
    <div className="track-changes">
      {diff.segments.map((segment, index) => {
        if (segment.kind === "Deleted") return <del key={index}>{segment.text}</del>;
        if (segment.kind === "Inserted") return <ins key={index}>{segment.text}</ins>;
        return <span key={index}>{segment.text}</span>;
      })}
    </div>
  );
}

function GutterLineNumber({ n, side }: { n: number | null; side: string }) {
  return <span style={{ color: "#9ca3af", minWidth: 36, display: "inline-block" }}>{n ?? " "}</span>;
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
    const start = leftSide ? range.left_start : range.right_start;
    const len = leftSide ? range.left_len : range.right_len;
    if (!len || start < cursor || start + len > text.length) return;
    parts.push(<span key={`plain-${index}`}>{text.slice(cursor, start)}</span>);
    parts.push(<mark key={`mark-${index}`}>{text.slice(start, start + len)}</mark>);
    cursor = start + len;
  });
  parts.push(<span key="tail">{text.slice(cursor)}</span>);
  return <>{parts}</>;
}
