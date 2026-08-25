import { Container } from "@/components/Container";
import { IconCheck, IconEyeOff } from "@/components/icons";

const POINTS = [
  "No cloud uploads",
  "No accounts or sign-in",
  "No external servers",
  "No database",
  "No tracking or analytics",
  "No internet required for sharing",
];

export function Privacy() {
  return (
    <section className="border-t border-line py-20 sm:py-28">
      <Container>
        <div className="grid gap-12 lg:grid-cols-2 lg:gap-16">
          <div>
            <h2 className="text-3xl font-semibold tracking-tight sm:text-4xl">
              Your files stay with you.
            </h2>
            <p className="mt-4 max-w-md text-[15px] leading-relaxed text-muted">
              DropLAN has no backend service, no database, and no external API to send anything
              to. What you share only ever exists on your computer and, once downloaded, on the
              device that requested it.
            </p>

            <ul className="mt-7 grid grid-cols-1 gap-3 sm:grid-cols-2">
              {POINTS.map((point) => (
                <li key={point} className="flex items-center gap-2.5 text-[15px]">
                  <IconCheck className="size-4 shrink-0 text-accent" />
                  {point}
                </li>
              ))}
            </ul>
          </div>

          <div className="flex items-start gap-3.5 rounded-2xl border border-line bg-bg-raised p-6">
            <IconEyeOff className="mt-0.5 size-5 shrink-0 text-muted" />
            <div>
              <p className="text-sm font-semibold">Worth knowing: LAN transfers use plain HTTP</p>
              <p className="mt-2 text-[15px] leading-relaxed text-muted">
                DropLAN serves files over HTTP, not HTTPS. A browser can&rsquo;t get a trusted
                certificate for a private IP address, and a self-signed one would just train
                people to click through security warnings — so traffic on your local network is
                unencrypted. Treat it the way you&rsquo;d treat any other unencrypted local
                network.
              </p>
            </div>
          </div>
        </div>
      </Container>
    </section>
  );
}
