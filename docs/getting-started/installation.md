# Installation

## Homebrew (recommended)

The fastest way to install Trek on macOS is via Homebrew:

```sh
brew install bradleyfay/trek/trek
```

This installs the `trek` binary and keeps it up to date with `brew upgrade`.

---

## Build from Source

Trek requires **Rust 1.80 or later**. If you do not have Rust installed, get it from [rustup.rs](https://rustup.rs).

```sh
git clone https://github.com/bradleyfay/trek.git
cd trek
cargo build --release
```

The compiled binary will be at `target/release/trek`. Copy it somewhere on your `$PATH`:

```sh
cp target/release/trek ~/.local/bin/trek
```

---

## Shell Integration

Trek can integrate with your shell so that quitting Trek changes your shell session's working directory to wherever you were browsing. This is one of the most useful features for day-to-day use.

Run the installer once:

```sh
trek --install-shell
```

Then reload your shell config:

```sh
source ~/.zshrc   # zsh
# or
source ~/.bashrc  # bash
```

### What the `m` function does

After installation, a shell function named `m` is added to your shell config. When you run `m`:

1. Trek launches in the current directory
2. You navigate normally
3. When you quit (`q`), your shell session `cd`s to the directory Trek had open at exit

This means Trek acts as a navigation tool for your shell, not just a file viewer. You can use `m` as a replacement for `cd` when you are not sure exactly where you want to go.

!!! note
    The underlying `trek` binary still works independently. `m` is a convenience wrapper — you are not required to use it.

---

## Verify the Installation

Run Trek directly to confirm it is working:

```sh
trek
```

Trek should open in the current directory with a three-pane layout. Press `q` to quit.
