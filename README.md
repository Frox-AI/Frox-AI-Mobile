# Frox Code

A real native desktop app (Tauri: Rust backend + React frontend) that works like Claude Code /
Codex — open a project folder, chat with it, and it actually reads, writes, and edits files and
runs commands in that folder, powered by your own Frox model.

## How it actually works (the important part)

This isn't a chatbot UI bolted onto a static system prompt. It's a real **agent loop**, implemented
in `src-tauri/src/agent.rs`:

1. User sends a message.
2. The app sends the full conversation + a tool schema (`read_file`, `write_file`, `edit_file`,
   `list_dir`, `run_command`) to your model's `/chat/completions` endpoint (OpenAI-compatible,
   tool-calling format).
3. If the model responds with tool calls, the app executes them for real against the open project
   folder (`src-tauri/src/tools.rs`) and feeds the results back to the model.
4. This repeats (up to 25 steps per message) until the model responds with plain text instead of
   a tool call — meaning it considers the task done.
5. Every step (assistant text, tool call, tool result) streams live to the UI as it happens.

## Requirements for your model

Your Frox model endpoint needs to support **OpenAI-style tool/function calling** — i.e. it must be
able to receive a `tools` array and respond with a `tool_calls` field when it wants to act, not just
describe what it would do in prose. This is a capability the base model and your fine-tune need to
actually support well. If tool calls come back malformed or the model just talks about using tools
instead of emitting real `tool_calls`, the agent loop won't work. Test this early — it's the single
biggest risk to this whole app working the way you want.

## Known limitations (read before giving this to real users)

- **No sandboxing.** `run_command` executes directly on the host machine with the user's own
  permissions — there is no container or VM isolation. Claude Code and Codex both add real
  protection here (approval prompts before running commands, and/or sandboxed execution). Right
  now, a bad or manipulated model response could run a destructive command. Before shipping this
  beyond your own testing, add at minimum a **user-approval step before `run_command` executes**,
  and ideally move execution into a disposable container.
- **File tools are path-scoped, not fully audited.** `read_file`/`write_file`/`edit_file` refuse to
  leave the opened project folder (blocks `../../` traversal), but there's no per-file confirmation
  UI yet — the model can overwrite any file in the project without the user approving each change.
  Claude Code shows a diff and asks before writing; adding that here is a natural next step.
- **No conversation persistence.** History lives in memory and clears when you close the app or
  open a new project. Add a save-to-disk step if you want history across sessions.
- **No streaming tokens.** Responses arrive as a full message per turn, not token-by-token. Adding
  streaming means switching to SSE parsing in `agent.rs` — doable, just not in this first pass.

## Project structure
```
frox-code/
  src/                    React frontend (chat UI, file sidebar, settings)
  src-tauri/
    src/
      main.rs             Entry point
      lib.rs              Tauri commands, app state, settings persistence
      agent.rs            The agent loop — talks to your model, dispatches tool calls
      tools.rs            Tool implementations (file IO, command execution)
    tauri.conf.json        Window/bundle config
    icons/                 Frox app icon (generated from the brand mark)
```

## Setup & running

Prerequisites: Node.js 18+, Rust (via rustup), and platform build tools
(`build-essential` + `libwebkit2gtk-4.1-dev` on Linux; Visual Studio Build Tools on Windows).

```bash
npm install
npm run tauri dev      # runs in dev mode with hot reload
```

Build installers:
```bash
npm run tauri build    # produces .deb / .AppImage on Linux, .exe (NSIS) on Windows
```

On first launch, open **Model settings** in the sidebar and point it at your Frox model's API
(base URL + key + model name) before opening a project.

## Suggested next steps, in priority order
1. Test tool-calling reliability with your actual fine-tuned model — this determines whether the
   core concept works at all.
2. Add a confirmation dialog before `run_command` and before `write_file`/`edit_file` overwrite
   an existing file — this is the biggest safety gap right now.
3. Add a real diff view for edits (show old vs. new before/after applying).
4. Add response streaming for a more responsive feel.
