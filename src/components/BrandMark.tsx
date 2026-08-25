import { cn } from "@/lib/utils";

/**
 * The DropLAN mark.
 *
 * Inlined rather than loaded from a file so it renders on the very first
 * paint, and so it needs no request at all — the desktop shell should not
 * depend on asset fetching to show its own identity.
 */
export function BrandMark({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 32 32"
      className={cn("size-5 shrink-0", className)}
      role="img"
      aria-label="DropLAN"
    >
      <rect width="32" height="32" rx="7.1" fill="#2B4FC9" />
      <g
        fill="none"
        stroke="#fff"
        strokeWidth="2.2"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <path d="M16 6.7V16.8" />
        <path d="M11.9 13.3 16 17.3 20.1 13.3" />
      </g>
      <g stroke="#fff" strokeWidth="0.9" strokeLinecap="round" strokeOpacity=".5">
        <path d="M16 23.2 8.2 19.4" />
        <path d="M16 23.2 23.8 19.4" />
      </g>
      <circle cx="8.2" cy="19.4" r="1.45" fill="#fff" />
      <circle cx="23.8" cy="19.4" r="1.45" fill="#fff" />
      <circle cx="16" cy="23.2" r="1.95" fill="#5CE1FF" />
    </svg>
  );
}
