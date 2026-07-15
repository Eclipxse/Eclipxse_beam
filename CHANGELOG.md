# Changelog

All notable Eclipxse Beam releases are documented here.

## Native v0.1.1 / Web v0.3.1 — 2026-07-16

This patch restores the proven WebRTC pairing flow behind the premium Slint interface.

### Fixed

- Replaced the native QR's router-dependent `http://192.168.x.x` target with the official HTTPS Beam companion
- Added native Rust PeerJS signaling and encrypted WebRTC data channels
- Added the same STUN and TURN connectivity fallback used by PeerJS so restrictive Wi-Fi networks do not block pairing
- Added a raw, framed transfer protocol shared by the Slint desktop and browser companion
- Restored phone-to-PC uploads and PC-to-phone downloads without requiring a shared Wi-Fi network or inbound firewall rule
- Updated the Slint connection language to describe WebRTC rather than same-Wi-Fi HTTP

### Validation

- Browser tests and production build
- Native unit, integration, formatting, and clippy checks
- Live automated PeerJS/WebRTC handshake and phone-to-PC byte-for-byte transfer verification

## Native v0.1.0 — 2026-07-16

The first functional native Windows release.

### Added

- Native Slint + Rust Windows interface with a warm, borderless Raycast-inspired design
- Real Windows file picker and dynamic multi-file queue
- Fresh token-protected local session and QR code on every launch
- Responsive phone companion served directly from the desktop application
- PC-to-phone streaming downloads
- Phone-to-PC streaming uploads saved under `Downloads\Eclipxse Beam`
- Live phone presence, direction, byte progress, completion, and error states
- Automatic disconnect expiry and transfer-screen navigation
- Filename sanitization, reserved-name protection, collision-safe destinations, and an 8 GiB upload limit
- Automated native integration tests and tagged Windows release workflow

### Security model

Native v0.1.0 uses a long random token over local HTTP and is intended for trusted same-Wi-Fi networks. It is not the same transport as the encrypted WebRTC web edition.

### Known limitations

- Windows x64 only
- Same reachable local network required
- No TLS, folder transfer, resume, installer, automatic updates, or code signing yet

## Web v0.3.0

- Browser-to-browser WebRTC file transfers with PeerJS signaling
- Temporary pairing links and QR codes
- Multi-file queues, chunking, backpressure, progress, and browser downloads
- Responsive installable PWA and GitHub Pages deployment
