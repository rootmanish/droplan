/**
 * A handful of hand-drawn line icons, sized for this page only.
 *
 * Deliberately not a dependency: the whole set is smaller than the smallest
 * icon-font subset would be, and every glyph here is bespoke to what this
 * page actually needs.
 */
type IconProps = { className?: string };

const base = "none";

export function IconDownload({ className = "size-4" }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill={base} className={className} aria-hidden>
      <path
        d="M12 3v12m0 0 4.5-4.5M12 15 7.5 10.5M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function IconGithub({ className = "size-4" }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className={className} aria-hidden>
      <path d="M12 2C6.48 2 2 6.58 2 12.2c0 4.5 2.87 8.32 6.84 9.67.5.1.68-.22.68-.49 0-.24-.01-1.04-.01-1.88-2.78.62-3.37-1.19-3.37-1.19-.45-1.18-1.11-1.49-1.11-1.49-.9-.63.07-.62.07-.62 1 .07 1.53 1.05 1.53 1.05.9 1.56 2.34 1.11 2.91.85.09-.66.35-1.11.63-1.37-2.22-.26-4.56-1.14-4.56-5.06 0-1.12.39-2.03 1.03-2.75-.1-.26-.45-1.32.1-2.76 0 0 .84-.28 2.75 1.05a9.3 9.3 0 0 1 5 0c1.91-1.33 2.75-1.05 2.75-1.05.55 1.44.2 2.5.1 2.76.64.72 1.03 1.63 1.03 2.75 0 3.93-2.34 4.79-4.57 5.05.36.32.68.94.68 1.9 0 1.37-.01 2.47-.01 2.81 0 .27.18.6.69.49A10.02 10.02 0 0 0 22 12.2C22 6.58 17.52 2 12 2Z" />
    </svg>
  );
}

export function IconCheck({ className = "size-4" }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill={base} className={className} aria-hidden>
      <path
        d="m5 12.5 4.5 4.5L19 7"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function IconChevronDown({ className = "size-4" }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill={base} className={className} aria-hidden>
      <path
        d="m6 9 6 6 6-6"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function IconWifi({ className = "size-4" }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill={base} className={className} aria-hidden>
      <path
        d="M3 8.5a15 15 0 0 1 18 0M6.2 12a10.6 10.6 0 0 1 11.6 0M9.5 15.5a6 6 0 0 1 5 0"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <circle cx="12" cy="19" r="1.4" fill="currentColor" />
    </svg>
  );
}

export function IconShield({ className = "size-4" }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill={base} className={className} aria-hidden>
      <path
        d="M12 3.5 5 6v6c0 4.4 3 7.6 7 8.5 4-.9 7-4.1 7-8.5V6l-7-2.5Z"
        stroke="currentColor"
        strokeWidth="1.7"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function IconLayers({ className = "size-4" }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill={base} className={className} aria-hidden>
      <path
        d="m12 3 8 4.2-8 4.2-8-4.2L12 3ZM4 12l8 4.2 8-4.2M4 15.8 12 20l8-4.2"
        stroke="currentColor"
        strokeWidth="1.7"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function IconMonitor({ className = "size-4" }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill={base} className={className} aria-hidden>
      <rect x="3" y="4.5" width="18" height="12" rx="1.6" stroke="currentColor" strokeWidth="1.7" />
      <path d="M8.5 20h7M12 16.5V20" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
    </svg>
  );
}

export function IconPhone({ className = "size-4" }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill={base} className={className} aria-hidden>
      <rect x="6.5" y="2.5" width="11" height="19" rx="2.2" stroke="currentColor" strokeWidth="1.7" />
      <path d="M11 19h2" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
    </svg>
  );
}

export function IconLock({ className = "size-4" }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill={base} className={className} aria-hidden>
      <rect x="5" y="10.5" width="14" height="9" rx="1.8" stroke="currentColor" strokeWidth="1.7" />
      <path d="M8 10.5V7.8a4 4 0 0 1 8 0v2.7" stroke="currentColor" strokeWidth="1.7" />
    </svg>
  );
}

export function IconClock({ className = "size-4" }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill={base} className={className} aria-hidden>
      <circle cx="12" cy="12" r="8.3" stroke="currentColor" strokeWidth="1.7" />
      <path d="M12 7.5V12l3 2" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" />
    </svg>
  );
}

export function IconEyeOff({ className = "size-4" }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill={base} className={className} aria-hidden>
      <path
        d="M3 3l18 18M10.6 10.7a2.6 2.6 0 0 0 3.6 3.6M6.4 6.6C4.3 8 2.8 10 2 12c1.6 3.6 5.4 7 10 7 1.6 0 3.1-.4 4.4-1.1M9.9 5.2A10.6 10.6 0 0 1 12 5c4.6 0 8.4 3.4 10 7-.6 1.3-1.4 2.5-2.5 3.6"
        stroke="currentColor"
        strokeWidth="1.7"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function IconGauge({ className = "size-4" }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill={base} className={className} aria-hidden>
      <path
        d="M4 15a8 8 0 1 1 16 0M12 15l3.2-4.6"
        stroke="currentColor"
        strokeWidth="1.7"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function IconArrowRight({ className = "size-4" }: IconProps) {
  return (
    <svg viewBox="0 0 24 24" fill={base} className={className} aria-hidden>
      <path
        d="M4 12h16m0 0-5.5-5.5M20 12l-5.5 5.5"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
