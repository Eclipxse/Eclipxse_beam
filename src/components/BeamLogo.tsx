interface BeamLogoProps {
  compact?: boolean;
}

export function BeamLogo({ compact = false }: BeamLogoProps) {
  return (
    <div className="brand" aria-label="Eclipxse Beam">
      <svg className="brand__mark" viewBox="0 0 64 64" aria-hidden="true">
        <defs>
          <linearGradient id="beam-mark-bg" x1="8" y1="5" x2="58" y2="61">
            <stop stopColor="#171924" />
            <stop offset="1" stopColor="#0c0d13" />
          </linearGradient>
          <linearGradient id="beam-mark-accent" x1="15" y1="14" x2="49" y2="51">
            <stop stopColor="#f4f0ff" />
            <stop offset="0.42" stopColor="#b7a6ff" />
            <stop offset="1" stopColor="#7d65ec" />
          </linearGradient>
        </defs>
        <rect x="2" y="2" width="60" height="60" rx="17" fill="url(#beam-mark-bg)" stroke="#ffffff" strokeOpacity=".1" />
        <path d="M37.5 16.2A18.3 18.3 0 1 0 49 42.8a15.6 15.6 0 1 1-11.5-26.6Z" fill="url(#beam-mark-accent)" />
        <ellipse cx="32" cy="32" rx="23" ry="9" fill="none" stroke="#a994ff" strokeOpacity=".5" strokeWidth="1.3" transform="rotate(-24 32 32)" />
        <path d="m44.5 17.5 1.4 3.8 3.8 1.4-3.8 1.4-1.4 3.8-1.4-3.8-3.8-1.4 3.8-1.4Z" fill="#fff" />
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
