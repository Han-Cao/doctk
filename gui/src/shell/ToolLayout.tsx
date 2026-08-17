import type { ReactNode } from "react";

export function ToolLayout(props: { title: string; description: string; children: ReactNode }) {
  return (
    <div className="tool-layout">
      <header className="tool-header">
        <h1>{props.title}</h1>
        <p>{props.description}</p>
      </header>
      <div className="tool-content">{props.children}</div>
    </div>
  );
}
