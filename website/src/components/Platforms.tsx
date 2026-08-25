import { Container } from "@/components/Container";
import { IconMonitor, IconPhone } from "@/components/icons";

const DESKTOP = ["macOS", "Windows", "Linux"];
const RECEIVING = ["iPhone", "Android", "iPad", "Windows", "macOS", "Linux"];

export function Platforms() {
  return (
    <section className="border-t border-line py-20 sm:py-28">
      <Container>
        <div className="max-w-xl">
          <h2 className="text-3xl font-semibold tracking-tight sm:text-4xl">
            One app. Every major desktop platform.
          </h2>
          <p className="mt-4 text-[15px] leading-relaxed text-muted">
            DropLAN runs as a native desktop app on macOS, Windows, and Linux. The receiving
            device only needs a browser — nothing to install.
          </p>
        </div>

        <div className="mt-10 grid gap-6 sm:grid-cols-2">
          <div className="rounded-2xl border border-line bg-bg-raised p-6">
            <p className="text-xs font-semibold tracking-wide text-muted uppercase">
              Runs DropLAN
            </p>
            <div className="mt-4 flex flex-wrap gap-3">
              {DESKTOP.map((name) => (
                <span
                  key={name}
                  className="inline-flex items-center gap-2 rounded-lg border border-line bg-bg px-3.5 py-2 text-sm font-medium"
                >
                  <IconMonitor className="size-4 text-muted" />
                  {name}
                </span>
              ))}
            </div>
          </div>

          <div className="rounded-2xl border border-line bg-bg-raised p-6">
            <p className="text-xs font-semibold tracking-wide text-muted uppercase">
              Receives in any browser
            </p>
            <div className="mt-4 flex flex-wrap gap-3">
              {RECEIVING.map((name) => (
                <span
                  key={name}
                  className="inline-flex items-center gap-2 rounded-lg border border-line bg-bg px-3.5 py-2 text-sm font-medium"
                >
                  <IconPhone className="size-4 text-muted" />
                  {name}
                </span>
              ))}
            </div>
          </div>
        </div>
      </Container>
    </section>
  );
}
