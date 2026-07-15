# Security policy

## Supported version

The latest version on `main` receives security fixes. Eclipxse Beam is currently pre-1.0 software, so its protocol and interfaces may change.

## Reporting a vulnerability

Please do not open a public issue for a vulnerability involving peer impersonation, unauthorized file access, signaling abuse, unsafe downloads, or exposed secrets.

Use GitHub's private vulnerability reporting for this repository. Include reproduction steps, affected browsers, impact, and a suggested fix when possible. You should receive an acknowledgement within seven days.

## Security models

### Web edition

- File payloads use WebRTC's encrypted transport between connected peers.
- The default PeerJS signaling infrastructure can observe connection metadata, but it does not receive file payloads from Beam.
- A person who obtains a valid temporary pairing code may attempt to connect. Only share pairing information with the intended device.
- Received files are untrusted input. Users should apply the same caution they would to any downloaded file.

### Native Windows edition

- Native v0.1.0 serves a phone companion over local HTTP and is intended only for trusted private networks.
- Access requires the full random session URL embedded in the QR code; the token changes whenever Beam restarts.
- The phone can request only files explicitly selected during the current process.
- Incoming filenames are stripped of paths, unsafe Windows characters, and reserved device names before saving.
- Completed uploads are written under `Downloads\Eclipxse Beam` with collision-safe names.
- Local HTTP is not encrypted against another device capable of observing traffic on the same network.
- Close the native application after transferring to terminate its session and local server.
