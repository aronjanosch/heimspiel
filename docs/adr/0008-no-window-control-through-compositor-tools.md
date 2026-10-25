# No window control through compositor tools

heimspiel never moves, focuses or resizes windows through compositor tools such as `hyprctl` or
`kdotool`; it controls games only through processes and process groups. It has to behave the same on
Hyprland, KDE and GNOME, and `hyprctl dispatch` already changed syntax with Hyprland 0.56.
Compositor-specific fixes (ES-DE back to fullscreen on Hyprland) ship as opt-in snippets that the user
adds to their own compositor config.

## Consequences

- The Hyprland snippet (`contrib/hyprland/es-de.lua`) makes ES-DE fullscreen again once every window
  that opened on its workspace after it has closed, and floats windowed games there centred. Counting
  windows instead of requiring ES-DE to be alone on its workspace keeps a terminal opened before ES-DE
  from blocking the return, and works for games Steam starts, which do not descend from ES-DE.
- KDE Plasma gets a KWin script with the same rule (`contrib/kde/heimspiel-es-de`); GNOME is expected to
  need nothing. Both untested.
