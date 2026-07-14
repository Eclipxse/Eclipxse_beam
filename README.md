<div align="center">
  <img src="public/favicon.svg" width="92" alt="Eclipxse Beam logo" />
  <h1>Eclipxse Beam</h1>
  <p><strong>Send through the veil.</strong></p>
  <p>A gothic-angelic, private way to send files directly between devices.</p>

  [![CI](https://github.com/Eclipxse/Eclipxse_beam/actions/workflows/ci.yml/badge.svg)](https://github.com/Eclipxse/Eclipxse_beam/actions/workflows/ci.yml)
  [![Deploy](https://github.com/Eclipxse/Eclipxse_beam/actions/workflows/deploy.yml/badge.svg)](https://github.com/Eclipxse/Eclipxse_beam/actions/workflows/deploy.yml)
  [![License: MIT](https://img.shields.io/badge/license-MIT-8d71ff.svg)](LICENSE)
  [![TypeScript](https://img.shields.io/badge/TypeScript-strict-55e4ff.svg)](https://www.typescriptlang.org/)

  [Try the live app](https://eclipxse.github.io/Eclipxse_beam/) · [Report a bug](https://github.com/Eclipxse/Eclipxse_beam/issues/new?template=bug_report.yml) · [Request a feature](https://github.com/Eclipxse/Eclipxse_beam/issues/new?template=feature_request.yml)
</div>

---

## Why Beam?

Most file-sharing tools ask you to upload first and download second. Beam connects two browsers with an encrypted WebRTC data channel and sends the file directly between them.

- **No accounts** — open the app and pair your devices.
- **No cloud file storage** — file payloads never pass through the signaling server.
- **Any file type** — photos, videos, archives, documents, and more.
- **Clear progress** — see each outgoing and incoming transfer in real time.
- **Installable** — use it as a lightweight progressive web app.
- **Open source** — inspect the code, improve it, or self-host it.

## How it works

1. The receiving device opens Beam and shows its temporary pairing code or QR code.
2. The sending device scans the QR code or pastes the pairing code.
3. Once WebRTC connects the browsers, selected files travel directly through the encrypted peer-to-peer channel.

> [!IMPORTANT]
> Beam's default PeerJS signaling service helps the two browsers discover each other. Signaling metadata passes through that service, but file contents do not. WebRTC encrypts the peer-to-peer transport. For complete infrastructure control, self-host the app and a compatible PeerServer.

## Features in v0.1

- Temporary device pairing codes
- Shareable pairing links and QR codes
- Friendly editable device names
- Multi-file drag and drop
- Chunked file transfers with backpressure
- Live progress for sending and receiving
- In-browser downloads for received files
- Responsive mobile and desktop interface
- Offline shell caching and installable PWA metadata
- Automated tests, builds, and GitHub Pages deployment

## Run it locally

Requirements: Node.js 24+ and npm 11+.

```bash
git clone https://github.com/Eclipxse/Eclipxse_beam.git
cd Eclipxse_beam
npm install
npm run dev
```

Open the displayed local URL in two browser windows or devices. Keep both tabs open during a transfer.

## Windows desktop app

Eclipxse Beam includes a native Windows edition with an original gothic-angelic interface, custom celestial icon, secure Electron window, and two distribution formats:

- **Installer** — guided setup with Start Menu and optional desktop shortcuts.
- **Portable** — a single EXE that runs without installation.

Build both executables locally:

```bash
npm install
npm run desktop:build
```

The finished files are written to `release/`. You can also run the **Build Windows EXE** workflow from the repository's Actions tab and download the `Eclipxse-Beam-Windows` artifact.

> [!NOTE]
> Community builds are not code-signed. Windows SmartScreen may show an “Unknown publisher” warning until the project uses a trusted Windows signing certificate. The product metadata still identifies the application as Eclipxse Beam by Eclipxse.

### Validate a change

```bash
npm run check
```

This runs the test suite, performs strict TypeScript checking, and creates the production build.

## Architecture

```text
Sender browser ── signaling metadata ── PeerJS signaling service
       │                                      │
       └──── encrypted WebRTC data channel ───┘ Receiver browser
                    file payload only
```

The UI is a static React application. It needs no application database and can be hosted on GitHub Pages. Files are divided into 64 KiB chunks, transferred over a reliable data channel, rebuilt as a browser Blob, and exposed to the receiver as a local download.

## Current limitations

- Both devices need network access to reach the default signaling service.
- Browsers must remain open until the transfer completes.
- Received files are assembled in memory, so extremely large transfers are limited by available browser memory.
- Pairing codes are temporary and should only be shared with the intended receiver.

## Roadmap

- [ ] Optional self-hosted signaling configuration
- [ ] Transfer acceptance prompt and trusted devices
- [ ] Folder transfers
- [ ] Pause, resume, retry, and receiver acknowledgements
- [ ] Local-network discovery where browser support allows it
- [ ] Native desktop and mobile wrappers
- [ ] End-to-end browser transfer tests
- [ ] Internationalization

## Contributing

Thoughtful contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md), open an issue for substantial changes, and keep pull requests focused. Security concerns should follow [SECURITY.md](SECURITY.md) rather than a public issue.

## License

Released under the [MIT License](LICENSE).

<div align="center">
  <sub>Designed and built with moonlight by Eclipxse 🌙⚡</sub>
</div>
