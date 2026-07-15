# Eclipxse Beam Native

This folder contains **Eclipxse Obsidian**, the native Slint + Rust desktop redesign for Beam.

The current milestone is an interactive visual prototype. It proves the Windows shell, design system, navigation, receive QR, and transfer states before the WebRTC transfer engine is moved behind the native interface.

## Preview

| Send | Receive | Active transfer |
| --- | --- | --- |
| ![Send view](docs/send.png) | ![Receive view](docs/receive.png) | ![Transfer view](docs/transfer.png) |

## Run it

Install the stable Rust toolchain, then run from the repository root:

```bash
npm run native:dev
```

Open a specific prototype state:

```bash
cargo run --manifest-path beam-native/Cargo.toml -- --screen receive
cargo run --manifest-path beam-native/Cargo.toml -- --screen transfer
```

## Current scope

- Native frameless Windows shell with working minimize, maximize, close, and drag controls
- Shared graphite/violet design tokens and reusable Slint components
- Interactive Send, Receive, and Active Transfer navigation
- Real QR code pointing to the hosted Beam phone companion
- Pause/resume interaction in the transfer prototype
- Deterministic screenshot mode for visual regression review

The file-picker, discovery, and transfer data shown here are intentionally mocked. Backend integration begins after this visual direction is approved.

## Visual capture

Set `ECLIPXSE_CAPTURE` to save a rendered PNG and close the app automatically:

```powershell
$env:ECLIPXSE_CAPTURE = "beam-native/captures/send.png"
cargo run --manifest-path beam-native/Cargo.toml -- --screen send
```
