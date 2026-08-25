import { Container } from "@/components/Container";
import { IconArrowRight, IconClock, IconEyeOff, IconLock, IconShield } from "@/components/icons";
import { SECURITY_MODEL_URL } from "@/lib/site";

const CONTROLS = [
  {
    icon: IconLock,
    title: "Session tokens, not paths",
    body: "Every share URL carries a ~127-bit random token. The HTTP server never accepts a filesystem path from a client — only opaque file ids that resolve to paths you added.",
  },
  {
    icon: IconClock,
    title: "Fresh session per launch",
    body: "Restarting DropLAN invalidates every link handed out before. Shared files are never restored on startup.",
  },
  {
    icon: IconShield,
    title: "Instant revocation",
    body: "Removing a file or stopping sharing kills its URL immediately, including transfers already in progress.",
  },
  {
    icon: IconEyeOff,
    title: "Optional PIN",
    body: "A 6-digit code can gate the share page, with a deliberate delay on each wrong guess.",
  },
];

export function Security() {
  return (
    <section id="security" className="border-t border-line py-20 sm:py-28">
      <Container>
        <div className="max-w-xl">
          <h2 className="text-3xl font-semibold tracking-tight sm:text-4xl">Security</h2>
          <p className="mt-4 text-[15px] leading-relaxed text-muted">
            The assumption is that a private LAN is <em>private</em>, not <em>trusted</em> — a
            café network or an office Wi-Fi has other people on it. A wrong token or a guessed
            path gets the same generic &ldquo;not found&rdquo; response as a page that never
            existed, so there&rsquo;s no way to probe from outside.
          </p>
        </div>

        <div className="mt-12 grid gap-6 sm:grid-cols-2">
          {CONTROLS.map(({ icon: Icon, title, body }) => (
            <div key={title} className="rounded-2xl border border-line bg-bg-raised p-6">
              <Icon className="size-5 text-accent" />
              <h3 className="mt-3.5 text-[15px] font-semibold">{title}</h3>
              <p className="mt-1.5 text-sm leading-relaxed text-muted">{body}</p>
            </div>
          ))}
        </div>

        <a
          href={SECURITY_MODEL_URL}
          target="_blank"
          rel="noreferrer"
          className="mt-8 inline-flex items-center gap-1.5 text-sm font-medium text-accent hover:underline"
        >
          Read the full security model in the README
          <IconArrowRight className="size-3.5" />
        </a>
      </Container>
    </section>
  );
}
