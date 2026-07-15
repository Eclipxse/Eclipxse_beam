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

- Native v0.1.1 uses WebRTC's encrypted transport with the official HTTPS phone companion.
- PeerJS can observe temporary signaling metadata but does not receive Beam file payloads.
- TURN may relay encrypted WebRTC packets when a direct connection cannot be established.
- Access requires the full random pairing identity embedded in the QR link; the identity changes whenever Beam restarts.
- The desktop sends only files explicitly selected during the current process.
- Incoming filenames are stripped of paths, unsafe Windows characters, and reserved device names before saving.
- Completed uploads are written under `Downloads\Eclipxse Beam` with collision-safe names.
- The token-protected local HTTP companion remains available only as a development fallback and is not used by the release QR flow.
- Close the native application after transferring to terminate its pairing identity and active data channel.
