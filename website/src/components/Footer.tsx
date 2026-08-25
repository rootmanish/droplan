import { Container } from "@/components/Container";
import { Logo } from "@/components/Logo";
import { ISSUES_URL, LICENSE_URL, RELEASES_URL, REPO_URL, SITE_URL } from "@/lib/site";

const LINKS = [
  { href: REPO_URL, label: "GitHub" },
  { href: RELEASES_URL, label: "Releases" },
  { href: ISSUES_URL, label: "Issues" },
  { href: LICENSE_URL, label: "License" },
];

export function Footer() {
  return (
    <footer className="border-t border-line py-12">
      <Container className="flex flex-col items-center gap-6 text-center sm:flex-row sm:items-start sm:justify-between sm:text-left">
        <div>
          <Logo className="size-6" />
          <p className="mt-2 text-sm text-muted">Drop files. Share over LAN.</p>
          <p className="mt-4 text-xs text-muted">Built with Rust + Tauri.</p>
        </div>

        <div className="flex flex-col items-center gap-3 sm:items-end">
          <ul className="flex flex-wrap justify-center gap-x-5 gap-y-2 text-sm text-muted sm:justify-end">
            {LINKS.map((link) => (
              <li key={link.label}>
                <a href={link.href} target="_blank" rel="noreferrer" className="hover:text-fg">
                  {link.label}
                </a>
              </li>
            ))}
          </ul>
          <p className="font-mono text-xs text-muted">{SITE_URL.replace("https://", "")}</p>
        </div>
      </Container>
    </footer>
  );
}
