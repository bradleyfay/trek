# Trek

Terminal file browser that runs as a persistent pane inside [cmux](https://github.com/bradleyfay/cmux). Built for developers working alongside AI coding assistants who need a live view of the project: files appearing, git status changing, contents written.

## Primary Jobs

- Preview file contents (read-only, right pane)
- Navigate directory structure
- Monitor git status (modified, staged, untracked, deleted)
- Search file contents across the project (ripgrep)
- Open files in the appropriate external tool

## File Routing

Routing is user-configurable via `~/.config/trek/opener.conf` (or `$XDG_CONFIG_HOME/trek/opener.conf`). Rules use `ext <exts>` or `glob <pattern>` matchers; `{}` is the file path placeholder; first match wins. See the README "Opening files" section for config format and examples.

When no config file exists, built-in defaults apply:

| File type | Opens in |
|---|---|
| Markdown (`.md`) | cmux markdown viewer |
| HTML (`.html`, `.htm`) | System browser |
| Images | System viewer or cmux image preview |
| Code / text | `$EDITOR` in a new cmux surface |
| PDFs | System viewer |
| Directories | Navigate in-place |

## Design Rules

- Optimize for inspection and orientation. Complex file operations belong in the AI assistant.
- All UI stays within Trek's pane. No full-screen overlays.
- Every keybinding must appear in the help overlay (`?`) and command palette.
- Mouse is a first-class input method.

## Scope

| In scope | Out of scope |
|---|---|
| File tree navigation | Text editing |
| File preview (read-only) | LSP / code intelligence |
| Git status overlays | Git operations beyond status |
| Content search (ripgrep) | Terminal emulation |
| Watch mode (auto-refresh) | Bulk rename / complex file manipulation |
| File operations (copy, move, delete, mkdir) | Managing cmux layout |
| Opening files in external tools | Plugin system |
| Mouse-resizable panes | Remote file systems |
| Command palette | Application launching |
| Shell integration (`cd` on exit) | |

## See Also

`AGENTS.md` — build workflow, testing, commits, releases.
