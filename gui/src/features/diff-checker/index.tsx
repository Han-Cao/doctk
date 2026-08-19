import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { readTextFile } from "@tauri-apps/plugin-fs";
import {
  diffSideBySide,
  diffTrackChanges,
  type SideBySideDiff,
  type TrackChangeSegment,
  type TrackChangesDiff,
} from "../../lib/tauri";

const LEFT_SAMPLE = "The quick brown fox\njumps over the lazy dog\n";
const RIGHT_SAMPLE = "The quick red fox\njumps over the lazy cat\n";

type CopyMode = "raw" | "original" | "changed";

export default function DiffCheckerTool() {
  const [left, setLeft] = useState(LEFT_SAMPLE);
  const [right, setRight] = useState(RIGHT_SAMPLE);
  const [view, setView] = useState<"side-by-side" | "track-changes">("side-by-side");
  const [copyMode, setCopyMode] = useState<CopyMode>("changed");
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
        <div className="toolbar-group">
          <button className={view === "side-by-side" ? "primary" : ""} onClick={() => setView("side-by-side")}>
            Side by side
          </button>
          <button className={view === "track-changes" ? "primary" : ""} onClick={() => setView("track-changes")}>
            Track changes
          </button>
        </div>
        {view === "track-changes" && (
          <div className="toolbar-group copy-mode-group">
            <label htmlFor="copy-mode-select">Copy:</label>
            <select id="copy-mode-select" value={copyMode} onChange={(e) => setCopyMode(e.target.value as CopyMode)}>
              <option value="raw">Raw</option>
              <option value="original">Original</option>
              <option value="changed">Changed</option>
            </select>
          </div>
        )}
      </div>

      {error && <div className="error">{error}</div>}

      {view === "side-by-side" ? (
        <SideBySideView diff={sideBySide} />
      ) : (
        <TrackChangesView
          diff={trackChanges}
          originalText={left}
          changedText={right}
          copyMode={copyMode}
        />
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
  copyMode,
}: {
  diff: TrackChangesDiff | null;
  originalText: string;
  changedText: string;
  copyMode: CopyMode;
}) {
  const trackChangesRef = useRef<HTMLDivElement>(null);

  function selectedOriginalChanged(): { original: string; changed: string } | null {
    if (!diff) return null;
    const container = trackChangesRef.current;
    if (!container) return null;
    const offsets = getSelectedOffsets(container);
    if (!offsets) return null;
    return mapSelectedText(diff.segments, offsets.start, offsets.end);
  }

  function handleCopy(event: React.ClipboardEvent<HTMLDivElement>) {
    if (copyMode === "raw") return; // keep browser default raw copy

    const selected = selectedOriginalChanged();
    if (selected) {
      event.preventDefault();
      const text = copyMode === "original" ? selected.original : selected.changed;
      event.clipboardData.setData("text/plain", text.length > 0 ? text : copyMode === "original" ? originalText : changedText);
    } else {
      event.preventDefault();
      event.clipboardData.setData("text/plain", copyMode === "original" ? originalText : changedText);
    }
  }

  if (!diff) return <p className="hint">Computing diff…</p>;
  return (
    <div>
      <div className="track-changes" ref={trackChangesRef} onCopy={handleCopy}>
        {diff.segments.map((segment, index) => {
          if (segment.kind === "Deleted") return <del key={index}>{segment.text}</del>;
          if (segment.kind === "Inserted") return <ins key={index}>{segment.text}</ins>;
          return <span key={index}>{segment.text}</span>;
        })}
      </div>
    </div>
  );
}

function getTextNodeAbsoluteOffset(container: Node, node: Node): number | null {
  if (node.nodeType !== Node.TEXT_NODE) return null;
  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT);
  let offset = 0;
  while (walker.nextNode()) {
    const current = walker.currentNode;
    if (current === node) return offset;
    offset += current.textContent?.length ?? 0;
  }
  return null;
}

function getSelectedOffsets(container: HTMLElement): { start: number; end: number } | null {
  const selection = window.getSelection();
  if (!selection || selection.rangeCount === 0 || selection.isCollapsed) return null;
  const range = selection.getRangeAt(0);
  if (!container.contains(range.startContainer) || !container.contains(range.endContainer)) return null;

  const startBase = getTextNodeAbsoluteOffset(container, range.startContainer);
  const endBase = getTextNodeAbsoluteOffset(container, range.endContainer);
  if (startBase === null || endBase === null) return null;

  const start = startBase + range.startOffset;
  const end = endBase + range.endOffset;
  if (start >= end) return null;
  return { start, end };
}

function mapSelectedText(
  segments: TrackChangeSegment[],
  selectedStart: number,
  selectedEnd: number,
): { original: string; changed: string } {
  let cursor = 0;
  let original = "";
  let changed = "";
  for (const segment of segments) {
    const segmentStart = cursor;
    const segmentEnd = cursor + segment.text.length;
    cursor = segmentEnd;

    const overlapStart = Math.max(segmentStart, selectedStart);
    const overlapEnd = Math.min(segmentEnd, selectedEnd);
    if (overlapStart >= overlapEnd) continue;

    const selectedText = segment.text.slice(overlapStart - segmentStart, overlapEnd - segmentStart);
    if (segment.kind !== "Inserted") {
      original += selectedText;
    }
    if (segment.kind !== "Deleted") {
      changed += selectedText;
    }
  }
  return { original, changed };
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
