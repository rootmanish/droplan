interface LogoProps {
  className?: string;
  withWordmark?: boolean;
}

/**
 * The DropLAN mark, inlined as JSX rather than an <img> so it stays crisp at
 * any size and needs no network request. Geometry matches
 * assets/brand/icon.svg — a file dropping onto this machine, fanning out to
 * two peers on the network.
 */
export function Logo({ className = "size-7", withWordmark = true }: LogoProps) {
  return (
    <span className="inline-flex items-center gap-2.5">
      <svg viewBox="0 0 1024 1024" className={className} aria-hidden={withWordmark}>
        <defs>
          <linearGradient id="logo-bg" x1="0" y1="0" x2="0.65" y2="1">
            <stop offset="0" stopColor="#4A7CFF" />
            <stop offset="0.55" stopColor="#2B4FC9" />
            <stop offset="1" stopColor="#15224A" />
          </linearGradient>
        </defs>
        <rect width="1024" height="1024" rx="228" fill="url(#logo-bg)" />
        <g fill="none" stroke="#fff" strokeWidth="70" strokeLinecap="round" strokeLinejoin="round">
          <path d="M512 214 V 536" />
          <path d="M382 424 L512 554 L642 424" />
        </g>
        <g stroke="#fff" strokeWidth="26" strokeLinecap="round" strokeOpacity="0.5">
          <path d="M512 742 L262 622" />
          <path d="M512 742 L762 622" />
        </g>
        <circle cx="262" cy="622" r="46" fill="#fff" />
        <circle cx="762" cy="622" r="46" fill="#fff" />
        <circle cx="512" cy="742" r="62" fill="#5CE1FF" />
      </svg>
      {withWordmark && (
        <span className="text-lg font-semibold tracking-tight text-fg">
          Drop<span className="text-cyan">LAN</span>
        </span>
      )}
    </span>
  );
}
