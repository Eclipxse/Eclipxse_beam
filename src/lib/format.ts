const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'] as const;

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 B';

  const unitIndex = Math.min(
    Math.floor(Math.log(bytes) / Math.log(1024)),
    UNITS.length - 1,
  );
  const value = bytes / 1024 ** unitIndex;
  const decimals = value >= 10 || unitIndex === 0 ? 0 : 1;

  return `${value.toFixed(decimals)} ${UNITS[unitIndex]}`;
}

export function shortCode(code: string): string {
  if (code.length <= 16) return code;
  return `${code.slice(0, 8)}…${code.slice(-6)}`;
}

export function connectionLabel(status: string): string {
  switch (status) {
    case 'starting':
      return 'Starting Beam';
    case 'ready':
      return 'Ready to pair';
    case 'connecting':
      return 'Connecting';
    case 'connected':
      return 'Connected';
    case 'error':
      return 'Needs attention';
    default:
      return 'Offline';
  }
}

export function getFriendlyDeviceName(): string {
  const platform = navigator.userAgent.toLowerCase();

  if (platform.includes('iphone') || platform.includes('ipad')) return 'Apple device';
  if (platform.includes('android')) return 'Android device';
  if (platform.includes('mac')) return 'Mac';
  if (platform.includes('windows')) return 'Windows PC';
  if (platform.includes('linux')) return 'Linux device';
  return 'My device';
}

export function normalizePairingCode(value: string): string {
  const trimmed = value.trim();

  try {
    const url = new URL(trimmed);
    return url.searchParams.get('peer')?.trim() ?? trimmed;
  } catch {
    return trimmed.replace(/\s+/g, '');
  }
}
