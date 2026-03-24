Output assets/demo.gif

Set Shell zsh
Set FontSize 14
Set FontFamily "FiraCode Nerd Font Mono"
Set Width 1200
Set Height 700
Set Theme "Catppuccin Mocha"
Set TypingSpeed 80ms
Set PlaybackSpeed 0.8

# cd into demo dir and launch trek silently
Hide
Type "cd /tmp/trek-demo && /tmp/trek-bin"
Enter
Sleep 4s
Show

# Now inside trek at /tmp/trek-demo
# Navigate down once to src/ (order: docs, src, tests, ...)
Down
Sleep 600ms

# Enter src/
Right
Sleep 2s

# Browse source files — preview shows content
Down
Sleep 600ms
Down
Sleep 600ms
Down
Sleep 800ms

# Go back to root
Left
Sleep 1s

# Toggle hidden files to reveal .github, .gitignore
Type "."
Sleep 2s

# Toggle hidden back off
Type "."
Sleep 800ms

# Fuzzy search demo
Type "/"
Sleep 500ms
Type "ui"
Sleep 1.5s
Escape
Sleep 600ms

# Go to top and quit
Type "g"
Sleep 500ms
Type "q"
Sleep 500ms
