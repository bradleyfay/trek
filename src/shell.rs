use anyhow::{bail, Result};

pub fn install_shell_integration() -> Result<()> {
    let shell = std::env::var("SHELL").unwrap_or_default();
    let home = std::env::var("HOME").unwrap_or_default();

    // Validate HOME before constructing any paths from it.
    if home.is_empty() {
        bail!("HOME environment variable is not set; cannot determine shell profile path");
    }
    let home_path = std::path::Path::new(&home);
    if !home_path.is_absolute() {
        bail!(
            "HOME is not an absolute path ({:?}); refusing to write shell profile",
            home
        );
    }

    let (profile, snippet) = if shell.contains("zsh") {
        (
            format!("{home}/.zshrc"),
            r#"
# trek shell integration — added by `trek --install-shell`
m() {
  local tmp=$(mktemp)
  trek --choosedir "$tmp"
  local dir=$(cat "$tmp")
  rm -f "$tmp"
  [[ -n "$dir" ]] && cd "$dir"
}
"#,
        )
    } else if shell.contains("bash") {
        (
            format!("{home}/.bashrc"),
            r#"
# trek shell integration — added by `trek --install-shell`
m() {
  local tmp=$(mktemp)
  trek --choosedir "$tmp"
  local dir=$(cat "$tmp")
  rm -f "$tmp"
  [ -n "$dir" ] && cd "$dir"
}
"#,
        )
    } else if shell.contains("fish") {
        (
            format!("{home}/.config/fish/config.fish"),
            r#"
# trek shell integration — added by `trek --install-shell`
function m
  set tmp (mktemp)
  trek --choosedir $tmp
  set dir (cat $tmp)
  rm -f $tmp
  if test -n "$dir"
    cd $dir
  end
end
"#,
        )
    } else {
        bail!(
            "Unsupported shell: {:?}. Add the wrapper manually — see `trek --help`.",
            shell
        );
    };

    let profile_path = std::path::Path::new(&profile);
    let contents = std::fs::read_to_string(profile_path).unwrap_or_default();

    if contents.contains("trek --choosedir") {
        println!("Already installed in {profile}. Nothing to do.");
        return Ok(());
    }

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(profile_path)?;
    std::io::Write::write_all(&mut file, snippet.as_bytes())?;

    println!("Installed! Added `m` function to {profile}");
    println!("Reload your shell:  source {profile}");
    Ok(())
}
