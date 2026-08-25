import { Container } from "@/components/Container";
import { IconGauge, IconLayers } from "@/components/icons";

export function LargeFiles() {
  return (
    <section className="border-t border-line py-20 sm:py-28">
      <Container>
        <div className="grid items-center gap-12 lg:grid-cols-2 lg:gap-16">
          <div>
            <IconLayers className="size-6 text-accent" />
            <h2 className="mt-4 text-3xl font-semibold tracking-tight sm:text-4xl">
              Built for large files.
            </h2>
            <p className="mt-4 max-w-md text-[15px] leading-relaxed text-muted">
              DropLAN streams files directly from disk instead of copying them into the app
              first. A 50 GB video doesn&rsquo;t become another 50 GB copy — DropLAN reads it from
              where it already lives and streams it over your network.
            </p>
          </div>

          <div className="rounded-2xl border border-line bg-bg-raised p-6">
            <div className="flex items-center gap-2.5 text-sm font-semibold">
              <IconGauge className="size-4 text-accent" />
              HTTP range requests
            </div>
            <p className="mt-2.5 text-[15px] leading-relaxed text-muted">
              Downloads support standard HTTP range requests, so a video can be seeked or resumed
              in the browser instead of restarting from the beginning.
            </p>
            <div className="mt-5 h-2 overflow-hidden rounded-full bg-line">
              <div className="h-full w-[63%] rounded-full bg-accent" />
            </div>
            <p className="mt-2 text-xs text-muted">Resuming a paused download from 63%</p>
          </div>
        </div>
      </Container>
    </section>
  );
}
