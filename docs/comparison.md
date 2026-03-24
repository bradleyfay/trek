# Trek vs. yazi vs. ranger

Trek, yazi, and ranger all present a three-column file browser in the terminal. If you've used any of them, Trek will feel familiar in layout. But they are built around different mental models and serve different use cases.

This page is not an argument that Trek is better. It's meant to help you understand what distinguishes each tool so you can pick the right one.

---

## At a glance

|  | Trek | yazi | ranger |
|---|---|---|---|
| Primary language | Rust | Rust | Python |
| Mental model | Project panel (VSCode sidebar) | Full file manager | Full file manager |
| Vim fluency required | No | Somewhat | Yes |
| Mouse support | First-class | Limited | Limited |
| Command palette | Yes | No | No |
| Plugin/theme system | No | Yes | Yes |
| cmux integration | Native | None | None |
| Async operations | Yes | Yes | No |
| Image preview | No | Yes (sixel/kitty) | Via scope.sh |
| Shell `cd` on exit | Yes | Yes | Yes |
| Built-in editor | No | No | No |
| Config language | TOML | TOML | Python |

---

## ranger

Ranger has been around since 2009. It's a stable, mature tool with a substantial configuration surface and a community of users who know it deeply.

**Where ranger excels:**
- Deep vim integration — if you think in vim motions, ranger's keybindings will feel natural immediately
- Highly scriptable — the Python-based config lets you wire up complex behaviors
- Large community, many examples of `scope.sh` configs for custom preview handlers
- Runs anywhere Python does; no compilation step

**Where it differs from Trek:**
- Ranger assumes vim fluency. The bindings are not labeled or explained in the UI — you're expected to have already learned them or to read the man page. Trek shows every action in the command palette and help overlay, and treats mouse input as equal to keyboard.
- Ranger is synchronous. File operations and previews happen on the main thread, which can cause the UI to stall on large directories or slow network mounts. Trek and yazi are both async.
- Ranger is a general-purpose file manager. It doesn't have a concept of "project scope" — search is filesystem-wide unless you configure it otherwise. Trek's search is scoped to the current project by default.
- There is no command palette in ranger. Discoverability comes from documentation, not from the tool itself.

---

## yazi

Yazi is a modern, async Rust rewrite built for speed and extensibility. It's the most actively developed of the three and has the richest feature set.

**Where yazi excels:**
- Native image preview with sixel and kitty graphics protocols — if your terminal supports it, yazi shows actual images inline
- A full plugin and theme ecosystem built on Lua
- Async throughout — every I/O operation is non-blocking, and the UI stays responsive under load
- Active development with frequent releases

**Where it differs from Trek:**
- Yazi is a file manager; Trek is a project browser. Yazi is designed to navigate your entire filesystem. Trek is designed to live inside a cmux pane and stay scoped to a project.
- Mouse support in yazi is present but not a design priority. Dragging pane dividers, clicking to navigate, and scrolling the preview pane are all first-class in Trek because Trek targets people who may not have memorized keyboard shortcuts.
- Yazi has no command palette. Actions are discovered through documentation or `:help`. Trek's command palette (`:`}) lists every available action in the current context, searchable by name.
- Yazi's plugin system is powerful but adds surface area. Trek has no plugin system — it does a smaller set of things and routes everything else to the appropriate external tool.
- Yazi does not integrate with cmux. Trek is designed specifically to run as a persistent cmux pane and can route files to other cmux surfaces.

---

## When to use each

**Use ranger if:**
- You are fluent in vim and want a file manager that matches that mental model
- You need deep Python-based scripting and custom preview handlers
- You're on a system where Rust tooling is unavailable or cumbersome

**Use yazi if:**
- You want the most feature-complete terminal file manager available
- Image previews matter to you and your terminal supports sixel or kitty
- You want a plugin and theme ecosystem
- Raw performance and async I/O under load are priorities

**Use Trek if:**
- You work primarily inside cmux and want a persistent project panel
- You want an experience closer to VSCode's file sidebar than to a file manager
- Mouse and keyboard should be equally usable without memorizing bindings
- You prefer a tool that stays scoped to the current project and routes files to the right tool rather than trying to handle everything itself
- Discoverability matters — you want every action findable from inside the app, not from a man page
