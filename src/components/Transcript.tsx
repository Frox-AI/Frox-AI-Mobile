export type TranscriptEntry =
  | { type: "user"; text: string }
  | { type: "assistant"; text: string }
  | { type: "tool_call"; name: string; args: any }
  | { type: "tool_result"; name: string; result: string }
  | { type: "error"; text: string };

export default function Transcript({ entry }: { entry: TranscriptEntry }) {
  if (entry.type === "user") {
    return (
      <div style={{ display: "flex", justifyContent: "flex-end", marginBottom: 12 }}>
        <div style={bubbleInner("var(--surface2)")}>{entry.text}</div>
      </div>
    );
  }

  if (entry.type === "assistant") {
    return (
      <div style={{ display: "flex", justifyContent: "flex-start", marginBottom: 12 }}>
        <div style={bubbleInner("var(--surface)")}>{entry.text}</div>
      </div>
    );
  }

  if (entry.type === "tool_call") {
    return (
      <div style={toolStyle()}>
        <div className="mono" style={{ fontSize: 12, color: "var(--cyan)", marginBottom: 4 }}>
          → {entry.name}
        </div>
        <pre className="mono" style={preStyle}>
          {JSON.stringify(entry.args, null, 2)}
        </pre>
      </div>
    );
  }

  if (entry.type === "tool_result") {
    return (
      <div style={toolStyle()}>
        <div className="mono" style={{ fontSize: 12, color: "var(--muted)", marginBottom: 4 }}>
          ← {entry.name} result
        </div>
        <pre className="mono" style={preStyle}>
          {entry.result}
        </pre>
      </div>
    );
  }

  // error
  return (
    <div style={{ ...toolStyle(), borderColor: "var(--magenta)" }}>
      <div className="mono" style={{ fontSize: 12, color: "var(--magenta)" }}>
        {entry.text}
      </div>
    </div>
  );
}

function bubbleInner(bg: string): React.CSSProperties {
  return {
    background: bg,
    borderRadius: 12,
    padding: "10px 14px",
    maxWidth: "80%",
    whiteSpace: "pre-wrap",
  };
}

function toolStyle(): React.CSSProperties {
  return {
    background: "var(--void)",
    border: "1px solid var(--hairline)",
    borderRadius: 10,
    padding: "10px 12px",
    marginBottom: 12,
    maxWidth: "90%",
  };
}

const preStyle: React.CSSProperties = {
  margin: 0,
  fontSize: 12,
  whiteSpace: "pre-wrap",
  wordBreak: "break-word",
  maxHeight: 240,
  overflowY: "auto",
  color: "var(--ink)",
};
