# Trek

**A terminal-first visual file browser built for AI-native developers.**

Trek gives you a persistent, three-pane window into your project — designed to live in a cmux pane alongside Claude Code, Codex, or whatever AI assistant is working in your codebase. When an AI agent is actively modifying files, Trek keeps you oriented: you can see what changed, preview what was written, and understand the project structure as it evolves.

Trek does not try to be a text editor, a shell, or a replacement for your AI assistant. Its job is transparency and navigation — helping you stay in the loop without interrupting the AI's flow.

---

## What Trek Is

Trek presents your filesystem across three panes:

- **Left** — the parent directory, so you always know where you are
- **Center** — the current directory, where you navigate
- **Right** — a live preview of the selected file or directory

When you open a file, Trek routes it to the appropriate tool — your editor for code, a viewer for images and PDFs, a browser for HTML. It handles the navigation; everything else goes to the best tool for the job.

---

## Key Features

- **Watch mode** — the file tree refreshes automatically when the filesystem changes; see what the AI created in real time
- **Git status overlays** — modified, staged, untracked, and deleted files are marked inline in the tree as the AI works
- **File preview** — inspect what was written without opening an editor; the right pane updates as you navigate
- **Content search** — ripgrep-powered full-text search across the project (`Ctrl+F`)
- **Fuzzy file search** — locate any file in the current directory instantly (`/`)
- **Mouse-resizable panes** — drag dividers to reconfigure the layout; mouse and keyboard are both first-class
- **Archive browsing** — navigate into `.zip`, `.tar.gz`, and other archives as virtual directories
- **Command palette** — press `:` to see every available action with its keybinding, searchable by name
- **Shell integration** — the `m` function launches Trek and `cd`s your shell to the directory you exit from

---

## Install

```sh
brew install bradleyfay/trek/trek
```

Then set up shell integration so your shell follows Trek when you quit:

```sh
trek --install-shell
source ~/.zshrc   # or ~/.bashrc
```

After that, use `m` anywhere in your terminal to open Trek. When you quit, your shell session moves to whatever directory you were browsing.

---

## Where to Go Next

- [Installation](getting-started/installation.md) — Homebrew, building from source, and shell integration details
- [Quick Start](getting-started/quick-start.md) — learn the basics in five minutes
- [All Keybindings](reference/keybindings.md) — complete keyboard reference
- [Command Palette](reference/command-palette.md) — every action, searchable from inside Trek
