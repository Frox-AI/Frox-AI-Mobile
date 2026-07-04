import { useState } from "react";

export interface Settings {
  base_url: string;
  api_key: string;
  model: string;
}

interface Props {
  settings: Settings;
  onClose: () => void;
  onSave: (s: Settings) => void;
}

export default function SettingsModal({ settings, onClose, onSave }: Props) {
  const [form, setForm] = useState<Settings>(settings);

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0,0,0,0.5)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 10,
      }}
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          background: "var(--surface)",
          border: "1px solid var(--hairline)",
          borderRadius: 14,
          padding: 24,
          width: 420,
        }}
      >
        <h2 className="display" style={{ margin: "0 0 16px", fontSize: 16 }}>
          Model settings
        </h2>

        <Field
          label="API base URL"
          value={form.base_url}
          onChange={(v) => setForm({ ...form, base_url: v })}
          placeholder="http://localhost:8000/v1"
        />
        <Field
          label="API key"
          value={form.api_key}
          onChange={(v) => setForm({ ...form, api_key: v })}
          placeholder="sk-..."
          type="password"
        />
        <Field
          label="Model name"
          value={form.model}
          onChange={(v) => setForm({ ...form, model: v })}
          placeholder="frox-morph-code"
        />

        <p style={{ fontSize: 12, color: "var(--muted)", marginTop: 4 }}>
          This should point to an OpenAI-compatible /chat/completions endpoint serving your
          fine-tuned Frox model, with tool/function-calling support enabled.
        </p>

        <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 20 }}>
          <button
            onClick={onClose}
            style={{
              background: "transparent",
              color: "var(--muted)",
              border: "1px solid var(--hairline)",
              borderRadius: 8,
              padding: "8px 16px",
            }}
          >
            Cancel
          </button>
          <button
            onClick={() => onSave(form)}
            className="grad-bg"
            style={{ border: "none", borderRadius: 8, padding: "8px 16px", fontWeight: 600 }}
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
}

function Field({
  label,
  value,
  onChange,
  placeholder,
  type = "text",
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  type?: string;
}) {
  return (
    <div style={{ marginBottom: 12 }}>
      <label style={{ display: "block", fontSize: 12, color: "var(--muted)", marginBottom: 4 }}>
        {label}
      </label>
      <input
        type={type}
        value={value}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
        style={{
          width: "100%",
          background: "var(--void)",
          color: "var(--ink)",
          border: "1px solid var(--hairline)",
          borderRadius: 8,
          padding: "8px 10px",
          fontSize: 13,
        }}
      />
    </div>
  );
}
