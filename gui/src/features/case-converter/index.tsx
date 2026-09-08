import { useState } from "react";

import { convertCase, type CaseMode } from "../../lib/tauri";
import { CASE_MODES, parseProperNounInput, showProperNouns } from "./caseModes";

const SAMPLE_TEXT = "hello world. this is a test.";
const PROPER_NOUN_PLACEHOLDER = "John\nNew York\nNASA";

export default function CaseConverterTool() {
  const [input, setInput] = useState(SAMPLE_TEXT);
  const [properNounsText, setProperNounsText] = useState("");
  const [output, setOutput] = useState("");
  const [activeMode, setActiveMode] = useState<CaseMode | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copyMessage, setCopyMessage] = useState<string | null>(null);

  const properNounsVisible = showProperNouns(activeMode);

  async function convert(mode: CaseMode) {
    try {
      const properNouns = parseProperNounInput(properNounsText);
      const converted = await convertCase(input, mode, properNouns);
      setOutput(converted);
      setActiveMode(mode);
      setError(null);
      setCopyMessage(null);
    } catch (err) {
      setError(String(err));
    }
  }

  async function copyOutput() {
    if (!navigator.clipboard) {
      setCopyMessage("Clipboard API is not available in this WebView.");
      return;
    }

    try {
      await navigator.clipboard.writeText(output);
      setCopyMessage("Converted text copied to clipboard.");
    } catch (err) {
      setCopyMessage(`Copy failed: ${String(err)}`);
    }
  }

  return (
    <div className="stack-vertical">
      <section className="tool-box markdown-box">
        <div className="tool-box-header">
          <label htmlFor="case-input">Input text</label>
        </div>
        <textarea
          id="case-input"
          className="markdown-editor"
          value={input}
          onChange={(event) => setInput(event.target.value)}
          spellCheck={false}
        />
        <div className="toolbar case-mode-toolbar">
          {CASE_MODES.map((definition) => (
            <button
              key={definition.mode}
              type="button"
              className={definition.mode === activeMode ? "primary" : ""}
              title={definition.description}
              aria-pressed={definition.mode === activeMode}
              onClick={() => convert(definition.mode)}
            >
              {definition.label}
            </button>
          ))}
        </div>
      </section>

      <section className="tool-box markdown-box">
        <div className="tool-box-header">
          <label htmlFor="case-output">Output text</label>
        </div>
        <textarea
          id="case-output"
          className="markdown-editor"
          value={output}
          readOnly
          spellCheck={false}
        />
        <div className="toolbar conversion-toolbar">
          <button type="button" className="small-button" onClick={copyOutput}>
            Copy output
          </button>
          {copyMessage && <span className="hint clipboard-message">{copyMessage}</span>}
        </div>
      </section>

      {properNounsVisible && (
        <section className="tool-box case-nouns-box">
          <div className="tool-box-header">
            <label htmlFor="case-proper-nouns">
              Proper nouns (used by Sentence case and Title Case)
            </label>
          </div>
          <textarea
            id="case-proper-nouns"
            className="markdown-editor"
            value={properNounsText}
            onChange={(event) => setProperNounsText(event.target.value)}
            placeholder={PROPER_NOUN_PLACEHOLDER}
            spellCheck={false}
          />
          <div className="toolbar conversion-toolbar">
            <span className="hint">
              One proper noun or phrase per line. Capitalization is copied to the output;
              click the active mode button again to apply changes.
            </span>
          </div>
        </section>
      )}

      {error && <div className="error">{error}</div>}
    </div>
  );
}
