# Research: PS5 homebrew streaming clients (as of 2026-10-24)

Question set: is there a Moonlight-compatible client for PS5 homebrew; if not, what exists for PS4 and what would a port need;
other streaming-related PS5 homebrew; Artemis-specific features. Sources are GitHub repos, READMEs, source headers and
release metadata read via `gh` on 2026-10-24. Dates are the repo `pushed_at` values unless noted.
"Unverified" means: stated only in a README or inferred, not checked in code or on hardware.

Target setup for the user (from the request, not verified here): PS5 at firmware 12.70 with Relapse + kstuff-lite +
ShadowMountPlus; host Sunshine/Vibepollo (Apollo fork); client Artemis on other devices.

## 1. Is there a Moonlight (or Artemis, or other GameStream/Sunshine client) for PS5 homebrew?

**Short answer: no working one.** Only one PS5 attempt was found by search (`gh search repos "moonlight ps5 homebrew"`,
`"ps5 moonlight client"`, `"gamestream ps5"`, `"sunshine ps5"` and similar; only the two repos below matched).

- **br1cks4t1ts/moonlight-ps5**: https://github.com/br1cks4t1ts/moonlight-ps5
  - Status: README says "Pre-alpha. The app can launch and connect to a Moonlight server, but video output requires further work
    to function in the PS5 environment." (README, "Status")
  - Last push 2026-05-12 (API `pushed_at`); licence GPLv3 per README (GitHub reports NOASSERTION); 4 stars.
  - Target: firmware 10.00 with etaHEN per README. The user's FW 12.70 / Relapse setup is not covered. Unverified whether it
    runs on 12.70 at all.
  - Codec: no codec or HDR statement in the README. Unverified.
  - Controller: not mentioned in the README. Unverified.
  - Bundles `moonlight-common-c`, mbedTLS and Opus; FFmpeg is not included (README, "Dependencies").
  - Pairing: generates an RSA key and certificate on first run; PIN pairing (README, "Authentication").
  - Latency: no data.
  - Conclusion: no usable video path. Do not expect a PS5 client from this repo.
- **aybarsyazici/sunshine-ps5-prototype** (https://github.com/aybarsyazici/sunshine-ps5-prototype): a search hit for "sunshine ps5",
  last push 2026-05-10, 0 stars, no description. Not a client, as far as the search metadata shows. Not read further.

Artemis itself is not a PS5 app (see section 4). The Moonlight protocol library (`moonlight-common-c`) is portable C, which is why
the PS5 attempt uses it.

## 2. If none: PS4 homebrew precedent and what a PS5 port would need

### 2a. Moonlight PS4 (working, PS4 only)

- **JaimeJimenezG/Moonlight-ps4**: https://github.com/JaimeJimenezG/Moonlight-ps4
  - Version 1.1.0 (README "Current version"); last push 2026-08-15 (API); no licence detected by GitHub.
  - Target: jailbroken PS4, GoldHEN, firmware 9.00 validated (README, "Status"). Built on the OpenOrbis toolchain v0.5.4.
  - Status list from README: pairing, applist, launch, H.264 streaming against Sunshine; Opus audio via `sceAudioOut`;
    software decode via FFmpeg and hardware decode via `libSceVideodec2` (`prefer_hw`); DualShock 4 input via `scePad`;
    on-console menu; installable `.pkg`.
  - Not done per README TODO: keyboard and mouse, mDNS discovery, native YCbCr/NV12 presentation.
  - Codec: H.264 only (README, "Defaults": 1920x1080 @ 60, 20 Mbps, H.264). No HEVC, AV1 or HDR listed.
  - Latency: per-frame decode, convert and present times are logged over UDP every second (README, "Defaults").
  - Pairing: PIN shown on the console and written to `/data/moonlight/pin.txt`; entered at `https://<pc>:47990/pin` (README, "Usage").
- Its `PLAN.md` (https://github.com/JaimeJimenezG/Moonlight-ps4/blob/main/PLAN.md) is the most useful design document found. Key points,
  all from that file:
  - It states there is no Moonlight PS4 port before its own work, and that moonlight-ps5 is "blocked on video output".
  - PacBrew (xfangfang/pacbrew, Docker image) ships FFmpeg 5.0, libopus 1.3, mbedTLS, libcurl and SDL2 already built for OpenOrbis.
  - Hardware decode "has no precedent in PS4 homebrew" as of the plan; `libSceVideodec2` link stubs exist and its structures are
    reversed in shadPS4 (https://github.com/shadps4-emu/shadPS4 — not read directly, cited via the plan).
  - Architecture: a custom C client on `moonlight-common-c`, with the decoder behind an interface (`IVideoDecoder`) and a separate
    renderer thread. Do not render inside `submitDecodeUnit`; without `CAPABILITY_DIRECT_SUBMIT` moonlight-common-c creates its own
    VideoDec thread.
  - Latency settings: `max_dec_frame_buffering = 1` and `num_reorder_frames = 0`; request
    `CAPABILITY_SLICES_PER_FRAME(4)` to parallelise decode.
  - Precedent: wiliwili (https://github.com/xfangfang/wiliwili), a PS4 Bilibili client with mpv + FFmpeg, software decode only.

### 2b. What a PS5 port would need (from the PS5 SDK and the PS4 plan; not verified on hardware)

- **Payload SDK**: https://github.com/ps5-payload-dev/sdk (GPL-3.0, last push 2026-10-21). README describes an SDK for payloads on
  exploited PS5s, with ELF loaders including ps5-payload-elfldr, ps5-payload-websrv, bdj-ipv6-hen, elfloader and remote_lua_loader.
  This is a plain C/C++ payload SDK, not a ports ecosystem.
- **SCE stubs present** (`sce_stubs/*.c` in the tree, 2026-10-24): libSceNet, libSceNetCtl, libSceHttp, libSceHttp2, libSceSsl,
  libScePad, libSceAudioOut, libSceVideoOut, libSceUserService, libSceSysmodule, libSceSystemService, libSceRemoteplay, libSceKeyboard,
  libSceImeDialog, libSceNotification, libSceRegMgr, libSceRandom, and others.
  - Networking and audio/pad/video-out are available through these stubs. Unverified whether the stubs match PS5 firmware 12.70.
  - `libSceRemoteplay` is present. This is Sony's own Remote Play library, not a Moonlight path. Unverified what it would give a port.
- **Not present as stubs**: no `libSceVideodec2`, no `libSceAvPlayer`, no video-decoder library. Hardware decode would need
  decrypted sprx files and stub generation (README, "Adding new SCE Libs": link the sprx into `sce_stubs/` and run `make -C sce_stubs stubs`).
  Whether PS5 exposes a usable hardware H.264/HEVC decoder to homebrew is unverified.
- **SDL2 / FFmpeg**: not in the SDK tree. Searches `"sdl2 ps5 payload"`, `"ps5 sdl2"` returned no results. A port would have to
  cross-build FFmpeg and SDL2 itself, or get them from a PS4-style portlib set, which was not found for PS5. Unverified.
- **Build**: clang 18 and lld 18 (SDK README, "Prerequisites").
- **Transport**: the SDK ships FreeBSD headers (`include/freebsd/`), which suggests BSD sockets are available. Inferred, not tested.
- **Host side**: Sunshine latest release v2026.914.233613 (2026-09-15, https://github.com/LizardByte/Sunshine/releases). The
  Moonlight-ps4 client already works against Sunshine, so the host needs no change for a PS5 port.

## 3. Other streaming-related PS5 homebrew

- **Chiaki-ng** (https://github.com/streetpea/chiaki-ng): remote play client for PlayStation consoles (README, description: "Next-Generation of Chiaki").
  Latest release v1.10.0 (2026-04-03); last push 2026-10-09. This is the opposite direction (desktop/phone client into the PS4/PS5);
  it does not stream PC games to the PS5. Mentioned only for completeness.
- No PS5 homebrew Sunshine host, and no PS5 homebrew encoder, was found. Not searched exhaustively.

## 4. Artemis: features and whether a Moonlight port would need them

Repo: https://github.com/ClassicOldSong/moonlight-android (GPL-3.0, 4135 stars, last push 2026-09-09). Latest release v20.2.6
(2025-08-08, `gh api .../releases/latest`). Note: the repo's last release is over a year old while the code is newer; the
README calls the fork "Moonlight Noir" in places.

Features listed in the README (https://github.com/ClassicOldSong/moonlight-android#readme):
- Client for Apollo and Sunshine; virtual display integration with Apollo (feature 28); server command integration with Apollo (feature 29).
- Clipboard sync (feature 30), marked "requires Apollo".
- Multiple mouse modes (normal, multi-touch, touchpad, disabled, local cursor).
- Custom virtual buttons with import/export; optimised virtual gamepad skins; gamepad debug page (vibration and gyro).
- Display-on-top mode; SBS 3D for external displays (AI MiDaS v2 Lite).
- The README says Artemis works "more seamless" with virtual display when paired with Apollo.
- The README also says the author has moved to Apollo and that Artemis and Apollo "will no longer be compatible with OG Sunshine and OG
  Moonlight eventually". Relevant because the user runs Vibepollo/Apollo.

Codec support in Artemis: no codec list in the README (grep for HEVC, AV1, H.264 and HDR returned nothing). Protocol-level codec
flags are in `moonlight-common-c` (https://github.com/moonlight-stream/moonlight-common-c/blob/master/src/Limelight.h): H.264
High and High 4:4:4 8-bit; HEVC Main, Main10, RExt 4:4:4 8/10-bit; AV1 Main8, Main10, High 4:4:4 8/10-bit; masks for 10-bit and 4:4:4;
`CAPABILITY_REFERENCE_FRAME_INVALIDATION_AV1` (0x40) and HEVC/AVC variants. Whether Artemis enables HEVC/AV1/HDR on a given device is
unverified (it depends on the Android decoder, which was not checked).

Port requirements, from the features list:
- Virtual display, server command and clipboard sync are host-side features that need Apollo's API. They are not part of the
  streaming protocol. A PS5 port would not need them for basic streaming. (Inference from the README; the Apollo API was not read.)
- Mouse modes and touchpad are input features; a PS5 port would need DualSense/DS4 input through `scePad` (the PS4 port already does
  DS4 input). Keyboard and mouse are not implemented in moonlight-ps4 (its TODO).

## Summary of unverified points

- PS5 FW 12.70 compatibility of any PS5 Moonlight attempt: unverified.
- PS5 hardware video decoder available to homebrew (`libSceVideodec2` equivalent): unverified; no stub in the SDK.
- SDL2 and FFmpeg for PS5 homebrew: no PS5 build found; unverified.
- Artemis codec and HDR behaviour on Android: unverified (README silent).
- Controller support for a PS5 port: unverified beyond the DS4 path in moonlight-ps4.

## Sources

- https://github.com/br1cks4t1ts/moonlight-ps5 (README; API `pushed_at` 2026-05-12)
- https://github.com/JaimeJimenezG/Moonlight-ps4 (README, pushed 2026-08-15, v1.1.0)
- https://github.com/JaimeJimenezG/Moonlight-ps4/blob/main/PLAN.md
- https://github.com/ps5-payload-dev/sdk (README, `sce_stubs/`, pushed 2026-10-21)
- https://github.com/ClassicOldSong/moonlight-android (README, releases, pushed 2026-09-09)
- https://github.com/moonlight-stream/moonlight-common-c (`src/Limelight.h`, pushed 2026-10-23)
- https://github.com/LizardByte/Sunshine/releases (latest v2026.914.233613)
- https://github.com/streetpea/chiaki-ng (description, releases v1.10.0, pushed 2026-10-09)
- https://github.com/xfangfang/wiliwili and https://github.com/xfangfang/pacbrew (cited via the PS4 plan, not read directly)
