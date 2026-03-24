# Trek

**A terminal-first visual file browser. Navigate your project like VSCode's sidebar — without leaving the terminal.**

Trek gives you a three-pane file browsing experience designed to live inside a terminal multiplexer. It does not try to be a text editor or a shell. Its job is to help you find the right file and open it in the right tool.

---

## What Trek Is

Trek presents your filesystem across three panes:

- **Left** — the parent directory, so you always know where you are
- **Center** — the current directory, where you navigate
- **Right** — a live preview of the selected file or directory

When you open a file, Trek routes it to the appropriate tool — your editor for code, a viewer for images and PDFs, a browser for HTML. It handles the navigation; everything else goes to the best tool for the job.

---

## Key Features

- **Mouse-resizable panes** — drag dividers to reconfigure the layout; mouse and keyboard are both first-class
- **Fuzzy file search** — locate any file in the current project with `/` or `Ctrl+F`
- **Content search** — ripgrep-powered full-text search across the project
- **Git status overlays** — modified, staged, untracked, and deleted files are marked inline in the tree
- **Command palette** — press `:` to see every available action with its keybinding, searchable by name
- **Bulk rename** — rename multiple files with a regex pattern and live preview before committing
- **Archive support** — browse into `.zip`, `.tar.gz`, and other archives; create archives from selected files
- **Watch mode** — the file tree refreshes automatically when the filesystem changes
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
