# No RetroDECK; emulators from system packages with EmuDeck configs loaded at runtime

heimspiel does not build on RetroDECK. Emulators come from Flatpak on Linux and from winget or GitHub
releases into ES-DE's `Emulators\` folder on Windows. Their configuration comes from EmuDeck
(dragoonDorise/EmuDeck and EmuDeck/emudeck-we), fetched at runtime and adapted, never copied into this
repository because it is GPL-3.0.

## Considered Options

- **RetroDECK**: rejected; Linux only, and its sandbox makes launching host programs such as Ports and
  Steam games hard.

## Consequences

- Flatpak is not installed on every target (Omarchy lacks it). Whether AppImage becomes an equal path
  for emulators is still open.
- heimspiel locates emulators itself and writes explicit paths, because ES-DE's find rules miss
  AppImages stored without extension.
