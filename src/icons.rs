use std::path::Path;

/// Return a nerd font icon for the given entry.
pub fn icon_for_entry(name: &str, is_dir: bool) -> &'static str {
    if is_dir {
        return icon_for_dir(name);
    }
    // Check exact filename matches first.
    if let Some(icon) = icon_for_filename(name) {
        return icon;
    }
    // Then check by extension.
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    icon_for_extension(&ext)
}

fn icon_for_dir(name: &str) -> &'static str {
    match name.to_lowercase().as_str() {
        ".git" => "\u{e5fb}",           //
        ".github" => "\u{e5fd}",        //
        ".vscode" => "\u{e70c}",        //
        "node_modules" => "\u{e5fa}",   //
        "src" => "\u{f0c9b}",           // 󰲛
        "bin" => "\u{f0219}",           // 󰈙
        "build" | "dist" | "out" | "target" => "\u{f0567}", // 󰕧
        "docs" | "doc" => "\u{f0219}",  // 󰈙
        "test" | "tests" | "__tests__" | "spec" => "\u{f0668}", // 󰙨
        "config" | ".config" => "\u{e5fc}", //
        "assets" | "static" | "public" => "\u{f024b}", // 󰉋
        "lib" => "\u{f121}",            //
        _ => "\u{f07b}",                //  (default folder)
    }
}

fn icon_for_filename(name: &str) -> Option<&'static str> {
    let lower = name.to_lowercase();
    let icon = match lower.as_str() {
        // Dotfiles & configs
        ".gitignore" | ".gitattributes" | ".gitmodules" => "\u{e5fb}", //
        ".editorconfig" => "\u{e652}",       //
        ".env" | ".env.local" | ".env.development" | ".env.production" => "\u{f0462}", // 󰑢
        ".dockerignore" => "\u{f0868}",      // 󰡨
        "dockerfile" | "containerfile" => "\u{f0868}", // 󰡨
        "docker-compose.yml" | "docker-compose.yaml" | "compose.yml" | "compose.yaml" => "\u{f0868}", // 󰡨
        "makefile" | "gnumakefile" => "\u{e673}", //
        "cmakelists.txt" => "\u{e673}",      //
        "rakefile" => "\u{e739}",            //
        "gemfile" | "gemfile.lock" => "\u{e739}", //
        "license" | "licence" | "license.md" | "licence.md" | "license.txt" => "\u{f0219}", // 󰈙
        "readme" | "readme.md" | "readme.txt" | "readme.rst" => "\u{f048a}", // 󰒊
        "cargo.toml" | "cargo.lock" => "\u{e7a8}", //
        "package.json" | "package-lock.json" => "\u{e71e}", //
        "tsconfig.json" => "\u{e628}",       //
        "webpack.config.js" | "webpack.config.ts" => "\u{f072b}", // 󰜫
        "vite.config.js" | "vite.config.ts" => "\u{e6b4}", //
        "rollup.config.js" | "rollup.config.ts" => "\u{f072b}", // 󰜫
        "babel.config.js" | ".babelrc" => "\u{f0a25}", // 󰨥
        "yarn.lock" => "\u{e6a7}",          //
        "pnpm-lock.yaml" => "\u{e71e}",     //
        "go.mod" | "go.sum" => "\u{e626}",  //
        "requirements.txt" | "pipfile" | "pipfile.lock" | "pyproject.toml" | "setup.py" | "setup.cfg" => "\u{e606}", //
        "flake.nix" | "flake.lock" => "\u{f313}",  //
        "justfile" => "\u{e673}",            //
        _ => return None,
    };
    Some(icon)
}

fn icon_for_extension(ext: &str) -> &'static str {
    match ext {
        // Rust
        "rs" => "\u{e7a8}",        //
        // Python
        "py" | "pyi" | "pyw" | "pyx" => "\u{e606}", //
        // JavaScript / TypeScript
        "js" | "mjs" | "cjs" => "\u{e74e}",   //
        "jsx" => "\u{e7ba}",                    //
        "ts" | "mts" | "cts" => "\u{e628}",   //
        "tsx" => "\u{e7ba}",                    //
        // Web
        "html" | "htm" => "\u{e736}",   //
        "css" => "\u{e749}",            //
        "scss" | "sass" => "\u{e603}",  //
        "less" => "\u{e614}",           //
        "vue" => "\u{e6a0}",            //
        "svelte" => "\u{e697}",         //
        // Data / Config
        "json" | "jsonc" | "json5" => "\u{e60b}", //
        "yaml" | "yml" => "\u{e6a8}",  //
        "toml" => "\u{e6b2}",          //
        "xml" => "\u{f05c0}",          // 󰗀
        "csv" => "\u{f0219}",          // 󰈙
        "ini" | "cfg" | "conf" => "\u{e615}", //
        "env" => "\u{f0462}",          // 󰑢
        // Shell
        "sh" | "bash" | "zsh" | "fish" => "\u{e795}", //
        "ps1" | "psm1" | "psd1" => "\u{e70f}", //
        // C / C++
        "c" => "\u{e61e}",             //
        "h" => "\u{e61e}",             //
        "cpp" | "cc" | "cxx" | "c++" => "\u{e61d}", //
        "hpp" | "hh" | "hxx" | "h++" => "\u{e61d}", //
        // Go
        "go" => "\u{e626}",            //
        // Java / JVM
        "java" => "\u{e738}",          //
        "kt" | "kts" => "\u{e634}",    //
        "scala" | "sc" => "\u{e737}",  //
        "groovy" | "gvy" => "\u{e775}", //
        "gradle" => "\u{e660}",        //
        // .NET
        "cs" => "\u{f031b}",           // 󰌛
        "fs" | "fsx" => "\u{e7a7}",    //
        // Ruby
        "rb" | "erb" => "\u{e739}",    //
        // PHP
        "php" => "\u{e608}",           //
        // Swift / Objective-C
        "swift" => "\u{e755}",         //
        "m" | "mm" => "\u{e61e}",      //
        // Lua
        "lua" => "\u{e620}",           //
        // Haskell
        "hs" | "lhs" => "\u{e61f}",    //
        // Elixir / Erlang
        "ex" | "exs" => "\u{e62d}",    //
        "erl" | "hrl" => "\u{e7b1}",   //
        // Clojure
        "clj" | "cljs" | "cljc" | "edn" => "\u{e76a}", //
        // Zig
        "zig" => "\u{e6a9}",           //
        // Nix
        "nix" => "\u{f313}",           //
        // Dart
        "dart" => "\u{e798}",          //
        // R
        "r" | "rmd" => "\u{f07d4}",    // 󰟔
        // SQL
        "sql" => "\u{e706}",           //
        // Markdown
        "md" | "mdx" | "markdown" => "\u{e73e}", //
        // Docs
        "txt" => "\u{f0219}",          // 󰈙
        "pdf" => "\u{f0226}",          // 󰈦
        "doc" | "docx" => "\u{f0219}", // 󰈙
        "xls" | "xlsx" => "\u{f0219}", // 󰈙
        "ppt" | "pptx" => "\u{f0219}", // 󰈙
        // Images
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "svg" | "webp" | "avif" => "\u{f03e}", //
        // Video
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv" => "\u{f03d}", //
        // Audio
        "mp3" | "wav" | "flac" | "ogg" | "aac" | "wma" | "m4a" => "\u{f001}", //
        // Archives
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "zst" => "\u{f1c6}", //
        // Binary / Executable
        "exe" | "dll" | "so" | "dylib" | "bin" | "elf" => "\u{f013}", //
        "wasm" => "\u{e6a1}",          //
        // Lock files
        "lock" => "\u{f023}",          //
        // Log
        "log" => "\u{f0219}",          // 󰈙
        // Terraform
        "tf" | "tfvars" => "\u{e69a}", //
        // Diff / Patch
        "diff" | "patch" => "\u{f0440}", // 󰑀
        // Default
        _ => "\u{f016}",               //  (generic file)
    }
}
