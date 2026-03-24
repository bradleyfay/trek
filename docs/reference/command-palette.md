# Command Palette

Trek v0.53.0

The command palette is Trek's primary discoverability surface. It lists every
action available in the current context, lets you filter by name, and executes
your selection — all without leaving the keyboard.

> **Tip:** If you can't remember a keybinding, `:` is always the answer.

---

## Opening the Palette

Press `:` from anywhere in Trek. The palette opens as an overlay within the
current pane and does not disturb your layout.

The `?` key opens a read-only help overlay instead. Use `:` when you want to
act; use `?` when you want to browse.

---

## Using the Palette

| Input | Effect |
|-------|--------|
| Type any text | Filter actions by name (case-insensitive substring match) |
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `Enter` or `l` | Execute the selected action |
| `Esc` or `:` | Close without executing |

Filtering is live — the list narrows with each character you type. You do not
need to type the full action name. For example, typing `git` will surface the git log and git diff preview actions.

---

## What the Palette Shows

Each entry in the palette displays:

- The action name
- The keybinding that triggers it directly (when one exists)
- A brief description of what the action does

Actions that are not applicable in the current context (for example, "Compare
files" when fewer than two entries are selected) are either hidden or shown as
disabled, depending on the context.

---

## Why Use the Palette

The command palette exists to make Trek usable without prior knowledge of its
keybindings. If you are new to Trek or encounter an unfamiliar file type or
situation, the palette surfaces what you can do right now.

As you use Trek regularly, the keybindings shown next to each action serve as
passive reinforcement — you learn the shortcuts at your own pace rather than
having to study them upfront.

The palette is also useful for infrequently-used actions (such as "Toggle hex
dump preview" or "Open clipboard inspector") where memorizing a dedicated key
is not worth the effort.

---

## See Also

- [Keybinding Reference](keybindings.md) — full list of all keyboard shortcuts
- [Help overlay](keybindings.md#app) — press `?` for a read-only quick reference
