import { useState } from "react";
import { markdownToText } from "../../lib/tauri";

const SAMPLE_MD = "# Header\n\nSome **bold** text and a [link](https://example.com).\n\n- item one\n- item two\n";

export default function MarkdownTextTool() {
  const [markdown, setMarkdown] = useState(SAMPLE_MD);
  const [output, setOutput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [copyMessage, setCopyMessage] = useState<string | null>(null);

  async function convert() {
    try {
      setOutput(await markdownToText(markdown));
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
      setCopyMessage("Plain text copied to clipboard.");
    } catch (err) {
      setCopyMessage(`Copy failed: ${String(err)}`);
    }
  }

  return (
    <div className="stack-vertical">
      <section className="tool-box markdown-box">
        <div className="tool-box-header">
          <label>Markdown (plain text)</label>
        </div>
        <textarea
          className="markdown-editor"
          value={markdown}
          onChange={(event) => setMarkdown(event.target.value)}
          spellCheck={false}
        />
        <div className="toolbar conversion-toolbar">
          <button className="primary" onClick={convert}>
            Convert to Text ↓
          </button>
        </div>
      </section>

      <section className="tool-box markdown-box">
        <div className="tool-box-header">
          <label>Plain text output</label>
        </div>
        <textarea className="markdown-editor" value={output} readOnly spellCheck={false} />
        <div className="toolbar conversion-toolbar">
          <button className="small-button" onClick={copyOutput}>
            Copy output
          </button>
          {copyMessage && <span className="hint clipboard-message">{copyMessage}</span>}
        </div>
      </section>

      {error && <div className="error">{error}</div>}
    </div>
  );
}
