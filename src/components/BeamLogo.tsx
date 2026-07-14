interface BeamLogoProps {
  compact?: boolean;
}

export function BeamLogo({ compact = false }: BeamLogoProps) {
  return (
    <div className="brand" aria-label="Eclipxse Beam">
      <svg className="brand__mark" viewBox="0 0 64 64" aria-hidden="true">
        <defs>
          <linearGradient id="brand-beam" x1="10" y1="10" x2="54" y2="54">
            <stop stopColor="#c6b6ff" />
            <stop offset="0.5" stopColor="#7c5cff" />
            <stop offset="1" stopColor="#51e5ff" />
          </linearGradient>
        </defs>
        <rect width="64" height="64" rx="18" fill="#111024" />
        <path
          d="M39.7 9.5A23.7 23.7 0 1 0 54 43.7 20 20 0 1 1 39.7 9.5Z"
          fill="url(#brand-beam)"
        />
        <path
          d="m39.5 23.5 2.2 6.1 6.1 2.2-6.1 2.2-2.2 6.1-2.2-6.1-6.1-2.2 6.1-2.2 2.2-6.1Z"
          fill="#fff"
        />
      </svg>
      {!compact && (
        <span className="brand__wordmark">
          <strong>Eclipxse</strong>
          <span>Beam</span>
        </span>
      )}
    </div>
  );
}
