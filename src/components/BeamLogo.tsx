interface BeamLogoProps {
  compact?: boolean;
}

export function BeamLogo({ compact = false }: BeamLogoProps) {
  return (
    <div className="brand" aria-label="Eclipxse Beam">
      <svg className="brand__mark" viewBox="0 0 64 64" aria-hidden="true">
        <defs>
          <radialGradient id="brand-night" cx="50%" cy="35%" r="70%">
            <stop stopColor="#321b2c" />
            <stop offset="1" stopColor="#09070b" />
          </radialGradient>
          <linearGradient id="brand-beam" x1="10" y1="10" x2="54" y2="54">
            <stop stopColor="#f2e1bc" />
            <stop offset="0.48" stopColor="#b87591" />
            <stop offset="1" stopColor="#d2ad6a" />
          </linearGradient>
        </defs>
        <rect width="64" height="64" rx="9" fill="url(#brand-night)" />
        <rect x="1" y="1" width="62" height="62" rx="8" fill="none" stroke="#d2ad6a" strokeOpacity=".38" />
        <ellipse cx="32" cy="14" rx="12" ry="4" fill="none" stroke="#d2ad6a" strokeWidth="1.2" />
        <path
          d="M37.7 15.5A19.5 19.5 0 1 0 50 43.7 16.5 16.5 0 1 1 37.7 15.5Z"
          fill="url(#brand-beam)"
        />
        <path
          d="m41 25 2.1 5.8L49 33l-5.9 2.1L41 41l-2.1-5.9L33 33l5.9-2.2Z"
          fill="#f7edda"
        />
        <path d="M32 47v9M28 52h8" fill="none" stroke="#d2ad6a" strokeLinecap="round" strokeWidth="1.2" />
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
