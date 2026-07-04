import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { platform } from "@tauri-apps/plugin-os";
import Sidebar from "./components/Sidebar";
import Transcript, { type TranscriptEntry } from "./components/Transcript";
import SettingsModal, { type Settings } from "./components/SettingsModal";

export default function App() {
  const [projectPath, setProjectPath] = useState<string | null>(null);
  const [entries, setEntries] = useState<TranscriptEntry[]>([]);
  const [input, setInput] = useState("");
  const [busy, setBusy] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [isMobile, setIsMobile] = useState(false);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    invoke<Settings>("get_settings").then(setSettings);
    try {
      const p = platform();
      setIsMobile(p === "android" || p === "ios");
    } catch {
      setIsMobile(false);
    }
  }, []);

  useEffect(() => {
    const unlisten = listen<{ kind?: string }>("frox-agent-event", (event) => {
      const { payload } = event as unknown as { payload: any };
      handleAgentEvent(payload);
    });
    return () => {
      unlisten.then((f) => f());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [entries]);

  function handleAgentEvent(payload: any) {
    // events come through with { kind, payload } shape set on the Rust side,
    // but tauri wraps the whole struct as the event payload itself.
    const kind = payload.kind;
    const data = payload.payload;
    if (kind === "assistant_text") {
      setEntries((e) => [...e, { type: "assistant", text: data.text }]);
    } else if (kind === "tool_call") {
      setEntries((e) => [...e, { type: "tool_call", name: data.name, args: data.args }]);
    } else if (kind === "tool_result") {
      setEntries((e) => [...e, { type: "tool_result", name: data.name, result: data.result }]);
    } else if (kind === "error") {
      setEntries((e) => [...e, { type: "error", text: data.message }]);
      setBusy(false);
    } else if (kind === "done") {
      setBusy(false);
    }
  }

  async function openProject() {
    try {
      const path = await invoke<string>("open_project");
      setProjectPath(path);
      setEntries([]);
    } catch {
      // user cancelled the dialog — not an error worth surfacing
    }
  }

  async function send() {
    if (!input.trim() || busy) return;
    const userText = input;
    setEntries((e) => [...e, { type: "user", text: userText }]);
    setInput("");
    setBusy(true);
    try {
      await invoke("send_message", { message: userText });
    } catch (err: any) {
      setEntries((e) => [...e, { type: "error", text: String(err) }]);
      setBusy(false);
    }
  }

  return (
    <div style={{ display: "flex", height: "100vh" }}>
      <Sidebar
        projectPath={projectPath}
        onOpenProject={openProject}
        onOpenSettings={() => setSettingsOpen(true)}
        isMobile={isMobile}
      />

      <div style={{ flex: 1, display: "flex", flexDirection: "column", minWidth: 0 }}>
        <header
          style={{
            padding: "12px 20px",
            borderBottom: "1px solid var(--hairline)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <span className="display" style={{ fontSize: 15, fontWeight: 600 }}>
            Frox <span className="grad-text">Morph Code</span>
          </span>
          {projectPath && (
            <span className="mono" style={{ fontSize: 12, color: "var(--muted)" }}>
              {projectPath}
            </span>
          )}
        </header>

        <div style={{ flex: 1, overflowY: "auto", padding: "20px" }}>
          {entries.length === 0 ? (
            <div style={{ color: "var(--muted)", marginTop: 60, textAlign: "center" }}>
              {projectPath
                ? "Ask Frox to read, write, or run something in this project."
                : "Chat with Frox. Open a project folder (desktop only) to let it read, write, and run code."}
            </div>
          ) : (
            entries.map((entry, i) => <Transcript key={i} entry={entry} />)
          )}
          <div ref={bottomRef} />
        </div>

        <div style={{ padding: "16px 20px", borderTop: "1px solid var(--hairline)" }}>
          <div style={{ display: "flex", gap: 8 }}>
            <textarea
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  send();
                }
              }}
              placeholder="Message Frox..."
              disabled={busy}
              rows={2}
              style={{
                flex: 1,
                background: "var(--surface)",
                color: "var(--ink)",
                border: "1px solid var(--hairline)",
                borderRadius: 10,
                padding: "10px 12px",
                resize: "none",
                fontFamily: "inherit",
                fontSize: 14,
              }}
            />
            <button
              onClick={send}
              disabled={busy || !input.trim()}
              className="grad-bg"
              style={{
                border: "none",
                borderRadius: 10,
                padding: "0 20px",
                fontWeight: 600,
                opacity: busy || !input.trim() ? 0.5 : 1,
              }}
            >
              {busy ? "Working…" : "Send"}
            </button>
          </div>
        </div>
      </div>

      {settingsOpen && settings && (
        <SettingsModal
          settings={settings}
          onClose={() => setSettingsOpen(false)}
          onSave={async (s) => {
            await invoke("save_settings", { settings: s });
            setSettings(s);
            setSettingsOpen(false);
          }}
        />
      )}
    </div>
  );
}
