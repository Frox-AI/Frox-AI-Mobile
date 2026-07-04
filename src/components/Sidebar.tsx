interface Props {
  projectPath: string | null;
  onOpenProject: () => void;
  onOpenSettings: () => void;
  isMobile: boolean;
}

export default function Sidebar({ projectPath, onOpenProject, onOpenSettings, isMobile }: Props) {
  return (
    <div
      style={{
        width: 240,
        background: "var(--surface)",
        borderRight: "1px solid var(--hairline)",
        display: "flex",
        flexDirection: "column",
        padding: 16,
      }}
    >
      <div className="display" style={{ fontSize: 18, fontWeight: 700, marginBottom: 20 }}>
        frox
      </div>

      {!isMobile && (
        <>
          <button
            onClick={onOpenProject}
            style={{
              background: "var(--surface2)",
              color: "var(--ink)",
              border: "1px solid var(--hairline)",
              borderRadius: 8,
              padding: "8px 12px",
              fontSize: 13,
              textAlign: "left",
              marginBottom: 8,
            }}
          >
            {projectPath ? "Change project folder" : "Open project folder"}
          </button>

          {projectPath && (
            <div
              className="mono"
              style={{
                fontSize: 11,
                color: "var(--muted)",
                wordBreak: "break-all",
                marginBottom: 20,
              }}
            >
              {projectPath}
            </div>
          )}
        </>
      )}

      {isMobile && (
        <div style={{ fontSize: 12, color: "var(--muted)", marginBottom: 20 }}>
          File editing and running commands are desktop-only. This is chat mode.
        </div>
      )}

      <div style={{ flex: 1 }} />

      <button
        onClick={onOpenSettings}
        style={{
          background: "transparent",
          color: "var(--muted)",
          border: "1px solid var(--hairline)",
          borderRadius: 8,
          padding: "8px 12px",
          fontSize: 13,
          textAlign: "left",
        }}
      >
        Model settings
      </button>
    </div>
  );
}
