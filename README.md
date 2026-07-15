<div align="center">
  <img src="public/favicon.svg" width="96" alt="Eclipxse Beam eclipse logo" />
  <h1>Eclipxse Beam</h1>
  <p><strong>Move files. Leave nothing behind.</strong></p>
  <p>A warm, private file-transfer experience for the browser and Windows.</p>

  [![CI](https://github.com/Eclipxse/Eclipxse_beam/actions/workflows/ci.yml/badge.svg)](https://github.com/Eclipxse/Eclipxse_beam/actions/workflows/ci.yml)
  [![Native Windows](https://github.com/Eclipxse/Eclipxse_beam/actions/workflows/native-release.yml/badge.svg)](https://github.com/Eclipxse/Eclipxse_beam/actions/workflows/native-release.yml)
  [![Deploy](https://github.com/Eclipxse/Eclipxse_beam/actions/workflows/deploy.yml/badge.svg)](https://github.com/Eclipxse/Eclipxse_beam/actions/workflows/deploy.yml)
  [![License: MIT](https://img.shields.io/badge/license-MIT-9D6638.svg)](LICENSE)

  [Download for Windows](https://github.com/Eclipxse/Eclipxse_beam/releases/latest/download/Eclipxse-Beam-Native-Windows-x64.exe)
  · [Open the web app](https://eclipxse.github.io/Eclipxse_beam/)
  · [Release notes](CHANGELOG.md)
</div>

---

## Meet Eclipxse Beam

Beam is an open-source file-transfer project with two deliberately different editions:

| Edition | Best for | Transport | Install required |
| --- | --- | --- | --- |
| **Native Windows** | Fast transfers between a Windows PC and a phone on the same Wi-Fi | Token-protected local HTTP | One portable EXE |
| **Web app** | Transfers between modern browsers, including different networks | Encrypted WebRTC data channel with PeerJS signaling | No |

Neither edition requires an account or uploads your files to Beam-owned cloud storage.

> [!IMPORTANT]
> The native and web transports have different security properties. The web app uses encrypted WebRTC. Native v0.1.0 uses local HTTP with a long random session token and is intended for trusted private networks. See [Security and privacy](#security-and-privacy) before using it on a public network.

## Native Windows preview

### Select real files

![Native Send screen showing the empty file queue and phone pairing action](beam-native/docs/send.png)

The Send workspace opens the native Windows file picker, displays the real queue, calculates its total size, and exposes only those selected files to the current session.

### Pair your phone

![Native Receive screen showing the generated local QR session](beam-native/docs/receive.png)

Every launch creates a new random session and QR code. Scan it with the phone camera while both devices are on the same Wi-Fi network.

### Follow every transfer

![Native Transfer screen showing the live queue and connected phone area](beam-native/docs/transfer.png)

Phone presence, transfer direction, byte progress, completion, and errors are pushed into the native Slint interface. Incoming files are written directly to disk rather than held entirely in memory.

## Download the native app

Download the latest portable executable:

**[Eclipxse Beam Native for Windows x64](https://github.com/Eclipxse/Eclipxse_beam/releases/latest/download/Eclipxse-Beam-Native-Windows-x64.exe)**

The matching SHA-256 checksum is published beside the EXE on the [Releases page](https://github.com/Eclipxse/Eclipxse_beam/releases).

### Windows requirements

- Windows 10 or Windows 11, 64-bit
- A Wi-Fi or trusted local network shared with the phone
- Permission through Windows Firewall on **private networks**
- No Node.js, Rust, account, or installer required for the downloaded EXE

The current community build is not code-signed. Windows SmartScreen may display an **Unknown publisher** warning. Verify the checksum from the release before running it.

## Use the native app

### Send files from the PC to a phone

1. Start `Eclipxse-Beam-Native-Windows-x64.exe`.
2. Select **Choose files from this computer**.
3. Pick one or more files in the native Windows dialog.
4. Open **Receive** or select **Pair phone**.
5. Scan the generated QR code with the phone camera.
6. Download the shared files from the Beam companion page.
7. Keep the desktop app and phone page open until Beam shows **Done**.

### Send files from a phone to the PC

1. Open **Receive** in the Windows app.
2. Scan the QR code from the phone.
3. Select **Choose files to beam** on the companion page.
4. Choose files from the phone.
5. Beam saves completed uploads under:

```text
Downloads\Eclipxse Beam
```

Existing names are preserved safely. If a file already exists, Beam creates a collision-safe name such as `photo (1).jpg`.

## Native features

- Native frameless Windows shell built with Slint and Rust
- Warm Raycast-inspired cream, sage, caramel, and espresso design system
- Real Windows file picker and model-backed multi-file queue
- Fresh, unguessable session token and QR code on every launch
- Responsive mobile companion served by the desktop application
- PC-to-phone streaming downloads
- Phone-to-PC streaming uploads
- Live device presence with automatic disconnect expiry
- Direction, byte progress, completion, and error states
- Automatic transfer-workspace navigation when an upload begins
- Filename sanitization and collision-safe download paths
- 8 GiB per-file upload guard
- Deterministic screenshot mode for visual regression checks

## Web edition

The browser edition remains available at **[eclipxse.github.io/Eclipxse_beam](https://eclipxse.github.io/Eclipxse_beam/)**.

Its workflow is:

1. The receiving browser opens Beam and receives a temporary PeerJS identifier.
2. The sender scans the QR code or opens its pairing link.
3. PeerJS exchanges signaling metadata.
4. File chunks travel over the encrypted WebRTC data channel.
5. The receiver rebuilds the file locally and downloads it from the browser.

The default signaling service can observe connection metadata, but Beam does not intentionally send file payloads through it.

## Architecture

### Native v0.1.0

```text
Windows PC                                           Phone browser
┌─────────────────────────────┐                     ┌─────────────────────────┐
│ Slint interface             │                     │ Beam companion          │
│  • real file queue          │                     │  • download PC files    │
│  • device/progress models   │                     │  • upload phone files   │
└──────────────┬──────────────┘                     └────────────┬────────────┘
               │ Rust events                                      │
┌──────────────▼──────────────┐   tokenized same-Wi-Fi HTTP   ┌────▼────────────┐
│ Axum companion server      ◄├──────────────────────────────►│ Camera/browser  │
│ random port + session token │                                └─────────────────┘
└──────────────┬──────────────┘
               │
       selected source files / Downloads\Eclipxse Beam
```

The server binds to a random local port. File identifiers map to selected paths held inside the process, so the phone cannot request arbitrary filesystem paths. Upload names are sanitized before a collision-safe destination is created.

### Web edition

```text
Sender browser ── signaling metadata ── PeerJS signaling service
       │                                      │
       └────── encrypted WebRTC data channel ─┘ Receiver browser
                         file payload
```

## Security and privacy

### Native edition

- Access requires the full random session URL embedded in the QR code.
- The token changes whenever the app restarts.
- The phone can download only files explicitly selected in the current process.
- Incoming names are stripped of paths, unsafe Windows characters, and reserved device names.
- Native v0.1.0 uses **HTTP, not HTTPS**, on the local network.
- Use it only on a trusted home, work, or private hotspot network.
- Close Beam after the transfer to terminate the session immediately.
- Treat every received file as untrusted input and scan it when appropriate.

### Web edition

- File payloads travel through WebRTC's encrypted transport.
- PeerJS signaling infrastructure may observe connection metadata.
- Anyone with a valid temporary pairing link may attempt to connect.
- Received files are untrusted input.

Report vulnerabilities through GitHub's [private vulnerability reporting](https://github.com/Eclipxse/Eclipxse_beam/security/advisories/new), not a public issue. Read the complete [security policy](SECURITY.md).

## Troubleshooting

### The phone cannot open the QR page

- Confirm the phone and PC are connected to the same Wi-Fi or hotspot.
- Allow Eclipxse Beam through Windows Firewall on private networks.
- Disable a VPN temporarily if it is routing Beam through the wrong adapter.
- Avoid guest Wi-Fi networks that isolate devices from one another.
- Restart Beam to generate a fresh session and try the new QR code.

### Windows shows “Unknown publisher”

The release is not code-signed yet. Download only from this repository and compare the EXE's SHA-256 hash with `SHA256SUMS.txt` on the release.

### A received filename changed

Beam removes unsafe path characters and Windows reserved names. It also adds `(1)`, `(2)`, and so on when a file already exists.

## Build from source

### Native Windows app

Requirements:

- Stable Rust toolchain
- Windows 10 or 11

```powershell
git clone https://github.com/Eclipxse/Eclipxse_beam.git
cd Eclipxse_beam
cargo run --manifest-path beam-native/Cargo.toml
```

Create the optimized EXE:

```powershell
cargo build --release --manifest-path beam-native/Cargo.toml
```

Output:

```text
beam-native\target\release\eclipxse-beam-native.exe
```

### Web app

Requirements: Node.js 24+ and npm 11+.

```bash
npm ci
npm run dev
```

## Validate changes

```powershell
cargo fmt --manifest-path beam-native/Cargo.toml --all -- --check
cargo test --manifest-path beam-native/Cargo.toml
cargo clippy --manifest-path beam-native/Cargo.toml --all-targets -- -D warnings
npm run check
```

The native integration suite starts the companion server, retrieves the mobile page, streams a real selected file, uploads a phone payload, verifies the saved bytes, and checks the resulting transfer state.

## Repository structure

```text
beam-native/
  src/backend.rs       Native pairing and streaming backend
  src/companion.html   Responsive phone companion
  src/main.rs          Slint/backend bridge and Windows shell
  ui/                  Slint screens, components, tokens, and assets
desktop/               Electron wrapper for the web edition
src/                   React/PeerJS web application
.github/workflows/     CI, Pages, Electron, and native release automation
```

## Current limitations

- Native v0.1.0 requires both devices to be on the same reachable local network.
- Native local HTTP is not encrypted against other devices on that network.
- Guest Wi-Fi client isolation can block phone-to-PC access.
- The native build is currently Windows x64 only.
- The native app does not yet support folders, resumable transfers, or code signing.
- The web receiver currently assembles received files in memory.

## Roadmap

- [ ] Internet-capable native WebRTC transport
- [ ] TLS or authenticated encrypted native local transport
- [ ] Signed Windows installer and automatic updates
- [ ] Folder transfers
- [ ] Pause, resume, retry, and receiver acknowledgements
- [ ] Transfer acceptance prompts and trusted-device history
- [ ] macOS and Linux native builds
- [ ] Internationalization and accessibility review

## Contributing

Contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md), keep pull requests focused, include tests, and attach screenshots for visible changes.

## License

Eclipxse Beam is released under the [MIT License](LICENSE).

<div align="center">
  <sub>Designed and built with moonlight by Eclipxse 🌙⚡</sub>
</div>
