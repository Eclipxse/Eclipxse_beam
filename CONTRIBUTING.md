# Contributing to Eclipxse Beam

Thanks for helping make private file sharing friendlier.

## Before coding

- Search existing issues and pull requests first.
- Open an issue before a substantial feature or protocol change.
- Keep privacy claims precise: signaling metadata and file payloads are different things.
- Never commit access tokens, test files containing personal data, or analytics that identify users.

## Development workflow

1. Fork the repository and create a focused branch from `main`.
2. Install dependencies with `npm install`.
3. Run the app with `npm run dev`.
4. Add or update tests for behavioral changes.
5. Run `npm run check` before opening a pull request.

Use clear commits such as `Add transfer acceptance prompt` or `Fix duplicate file selection`.

## Pull requests

Explain what changed, why it matters, and how you tested it. Include screenshots for visible interface changes. Keep unrelated cleanup in a separate pull request.

By contributing, you agree that your contribution will be licensed under the MIT License.
