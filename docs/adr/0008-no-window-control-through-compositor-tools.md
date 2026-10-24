# No window control through compositor tools

heimspiel never moves, focuses or resizes windows through compositor tools such as `hyprctl` or
`kdotool`; it controls games only through processes and process groups. It has to behave the same on
Hyprland, KDE and GNOME, and `hyprctl dispatch` already changed syntax with Hyprland 0.56.
Compositor-specific fixes (ES-DE back to fullscreen on Hyprland) ship as opt-in snippets that the user
adds to their own compositor config.
