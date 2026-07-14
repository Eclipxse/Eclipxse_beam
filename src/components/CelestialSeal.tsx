export function CelestialSeal() {
  return (
    <svg className="celestial-seal" viewBox="0 0 520 300" aria-hidden="true">
      <defs>
        <linearGradient id="seal-metal" x1="20" y1="20" x2="500" y2="280">
          <stop stopColor="#f0dfbd" />
          <stop offset="0.43" stopColor="#b87b91" />
          <stop offset="0.78" stopColor="#725079" />
          <stop offset="1" stopColor="#d1ad6d" />
        </linearGradient>
        <radialGradient id="seal-aura">
          <stop stopColor="#a86684" stopOpacity=".24" />
          <stop offset="1" stopColor="#a86684" stopOpacity="0" />
        </radialGradient>
      </defs>
      <ellipse cx="260" cy="145" rx="126" ry="126" fill="url(#seal-aura)" />
      <g fill="none" stroke="url(#seal-metal)" strokeLinecap="round">
        <ellipse cx="260" cy="60" rx="79" ry="24" strokeWidth="2" />
        <ellipse cx="260" cy="60" rx="64" ry="16" strokeWidth="1" opacity=".55" />
        <path d="M198 116c-62-34-126-16-173 42 55-25 102-13 139 25-48-10-83 11-109 51 51-25 97-19 139 19" strokeWidth="2.4" />
        <path d="M322 116c62-34 126-16 173 42-55-25-102-13-139 25 48-10 83 11 109 51-51-25-97-19-139 19" strokeWidth="2.4" />
        <path d="M175 142c-43-3-82 14-116 50M345 142c43-3 82 14 116 50" strokeWidth="1" opacity=".55" />
        <circle cx="260" cy="161" r="71" strokeWidth="2" />
        <circle cx="260" cy="161" r="57" strokeWidth=".8" opacity=".55" />
        <path d="M250 104c-33 8-53 39-46 70 7 33 39 53 71 45-21-7-35-27-34-49 1-23 16-42 37-48-9-10-18-16-28-18Z" fill="url(#seal-metal)" stroke="none" />
        <path d="m304 120 8 22 22 8-22 8-8 22-8-22-22-8 22-8Z" fill="#f5ead8" stroke="none" />
        <path d="M260 232v51M241 260h38" strokeWidth="2" />
        <path d="m260 235 6 12 12 6-12 6-6 12-6-12-12-6 12-6Z" fill="#d1ad6d" stroke="none" />
      </g>
    </svg>
  );
}
