# The desktop library view lives in Spieli

heimspiel does not build its own mouse-and-keyboard library view. That view is a separate project,
Spieli (github.com/aronjanosch/spieli), which reads ES-DE's data the way the Steam library sits next to
Big Picture; the heimspiel setup wizard installs and links it. ES-DE is therefore the only Output heimspiel
must keep complete.

## Considered Options

- **Own view in the Tauri app**: the original plan; dropped so heimspiel stays focused on Sources and
  Provisioning.
- **Pegasus, Playnite, LaunchBox, Retrom**: rejected; no release since 2024, Windows only, Linux not before
  2027, or no API to add games and launches only via emulator profiles.
