# Eclipxse Beam Native

This folder contains the native Slint + Rust desktop redesign for **Eclipxse Beam**.

The native milestone now combines the borderless, Raycast-inspired interface with a functional same-Wi-Fi transfer backend. Its warm cream, sage, caramel, and espresso palette covers real file selection, phone pairing, uploads, downloads, and live transfer progress.

## Preview

| Send | Receive | Active transfer |
| --- | --- | --- |
| ![Send view](docs/send.png) | ![Receive view](docs/receive.png) | ![Transfer view](docs/transfer.png) |

## Run it

Install the stable Rust toolchain, then run from the repository root:

```bash
npm run native:dev
```

Open a specific interface state:

```bash
cargo run --manifest-path beam-native/Cargo.toml -- --screen receive
cargo run --manifest-path beam-native/Cargo.toml -- --screen transfer
```

## Current scope

- Native frameless Windows shell with working minimize, maximize, close, and drag controls
- Shared warm-earth design tokens and reusable, borderless Slint components
- Native Windows file picker and a real, model-backed file queue
- A fresh token-protected QR session generated whenever Beam starts
- Responsive phone companion served directly by the desktop app
- Phone-to-PC uploads saved under `Downloads/Eclipxse Beam`
- PC-to-phone downloads streamed from the selected source files
- Live device, direction, completion, and byte-progress updates in Slint
- Filename sanitization, collision-safe saves, and an 8 GiB per-file upload limit
- Deterministic screenshot mode for visual regression review

The approved visual direction is preserved in [`docs/raycast-warm-borderless.png`](docs/raycast-warm-borderless.png).

## Phone pairing

1. Put the computer and phone on the same Wi-Fi network.
2. Open **Receive** in the native app and scan its QR code with the phone camera.
3. Use the companion page to download files selected on the computer or upload files back to it.
4. Keep Beam and the phone page open until the transfer reaches **Done**.

Windows may ask for firewall permission the first time the companion server starts. Allow access on private networks so the phone can reach the computer.

The native transport currently uses a private, unguessable session link over local HTTP. It does not yet use the browser edition's Internet-capable WebRTC/PeerJS transport, so it is intended for trusted local networks and does not work across different networks.

## Visual capture

Set `ECLIPXSE_CAPTURE` to save a rendered PNG and close the app automatically:

```powershell
$env:ECLIPXSE_CAPTURE = "beam-native/captures/send.png"
cargo run --manifest-path beam-native/Cargo.toml -- --screen send
```
