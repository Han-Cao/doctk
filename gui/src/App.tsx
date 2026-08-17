import { Suspense, useState } from "react";
import { TOOL_DEFINITIONS } from "./toolRegistry";
import { ToolLayout } from "./shell/ToolLayout";

export default function App() {
  const [activeId, setActiveId] = useState(TOOL_DEFINITIONS[0].id);
  const activeTool = TOOL_DEFINITIONS.find((tool) => tool.id === activeId) ?? TOOL_DEFINITIONS[0];

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="sidebar-title">doctk</div>
        <nav>
          {TOOL_DEFINITIONS.map((tool) => (
            <button
              key={tool.id}
              className={`tool-nav ${tool.id === activeId ? "active" : ""}`}
              onClick={() => setActiveId(tool.id)}
              title={tool.description}
            >
              <span className="tool-icon">{tool.icon}</span>
              <span>{tool.name}</span>
            </button>
          ))}
        </nav>
      </aside>
      <main className="main-area">
        <Suspense fallback={<div className="loading">Loading tool…</div>}>
          <ToolLayout title={activeTool.name} description={activeTool.description}>
            <activeTool.component />
          </ToolLayout>
        </Suspense>
      </main>
    </div>
  );
}
