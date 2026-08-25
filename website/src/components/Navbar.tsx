import { useEffect, useState } from "react";

import { Logo } from "@/components/Logo";
import { IconGithub } from "@/components/icons";
import { REPO_URL } from "@/lib/site";

const LINKS = [
  { href: "#features", label: "Features" },
  { href: "#how-it-works", label: "How it works" },
  { href: "#security", label: "Security" },
  { href: "#download", label: "Download" },
];

export function Navbar() {
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 8);
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <header
      className={`sticky top-0 z-40 border-b transition-colors ${
        scrolled ? "border-line bg-bg/85 backdrop-blur-sm" : "border-transparent bg-transparent"
      }`}
    >
      <nav className="mx-auto flex w-full max-w-5xl items-center justify-between gap-4 px-5 py-3.5 sm:px-8">
        <a href="#top" className="shrink-0" aria-label="DropLAN home">
          <Logo className="size-7" />
        </a>

        <ul className="hidden items-center gap-7 text-sm text-muted md:flex">
          {LINKS.map((link) => (
            <li key={link.href}>
              <a href={link.href} className="transition-colors hover:text-fg">
                {link.label}
              </a>
            </li>
          ))}
        </ul>

        <div className="flex items-center gap-2">
          <a
            href={REPO_URL}
            target="_blank"
            rel="noreferrer"
            className="hidden items-center gap-1.5 rounded-lg px-3 py-2 text-sm font-medium text-muted transition-colors hover:text-fg sm:inline-flex"
          >
            <IconGithub className="size-4" />
            GitHub
          </a>
          <a
            href="#download"
            className="inline-flex items-center gap-1.5 rounded-lg bg-accent px-3.5 py-2 text-sm font-semibold text-accent-fg transition-opacity hover:opacity-90"
          >
            Download
          </a>
        </div>
      </nav>
    </header>
  );
}
