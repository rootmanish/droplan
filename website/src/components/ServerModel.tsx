import type { ReactNode } from "react";

import { Container } from "@/components/Container";
import { IconArrowRight, IconMonitor, IconPhone, IconWifi } from "@/components/icons";

export function ServerModel() {
  return (
    <section id="features" className="border-t border-line py-20 sm:py-28">
      <Container>
        <div className="grid gap-12 lg:grid-cols-2 lg:gap-16">
          <div>
            <h2 className="text-3xl font-semibold tracking-tight sm:text-4xl">
              Your computer is the server.
            </h2>
            <p className="mt-4 max-w-md text-[15px] leading-relaxed text-muted">
              Most file-sharing tools upload your files to someone else&rsquo;s server first, then
              the other person downloads them from there. DropLAN skips that hop: it starts an
              HTTP server on your own machine and serves the files directly to devices on your
              local network.
            </p>
            <p className="mt-4 max-w-md text-[15px] leading-relaxed text-muted">
              Nothing is copied, queued, or held anywhere in between — the transfer exists only
              between your computer and the device you shared it with.
            </p>
          </div>

          <div className="flex flex-col gap-5">
            <CompareRow
              label="Traditional sharing"
              nodes={["Computer", "Internet", "Cloud", "Internet", "Device"]}
              muted
            />
            <CompareRow
              label="DropLAN"
              nodes={["Computer", "Local network", "Device"]}
              icons={[<IconMonitor className="size-4" key="m" />, <IconWifi className="size-4" key="w" />, <IconPhone className="size-4" key="p" />]}
            />
          </div>
        </div>
      </Container>
    </section>
  );
}

function CompareRow({
  label,
  nodes,
  icons,
  muted = false,
}: {
  label: string;
  nodes: string[];
  icons?: ReactNode[];
  muted?: boolean;
}) {
  return (
    <div
      className={`rounded-2xl border p-5 ${
        muted ? "border-line/70 bg-bg" : "border-accent/30 bg-accent-soft"
      }`}
    >
      <p className={`mb-4 text-xs font-semibold tracking-wide uppercase ${muted ? "text-muted" : "text-accent"}`}>
        {label}
      </p>
      <div className="flex flex-wrap items-center gap-x-2 gap-y-3">
        {nodes.map((node, i) => (
          <span key={node + i} className="flex items-center gap-2">
            <span
              className={`inline-flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-sm font-medium ${
                muted ? "bg-bg-raised text-muted" : "bg-bg-raised text-fg"
              }`}
            >
              {icons?.[i]}
              {node}
            </span>
            {i < nodes.length - 1 && (
              <IconArrowRight className={`size-3.5 ${muted ? "text-muted/60" : "text-accent"}`} />
            )}
          </span>
        ))}
      </div>
    </div>
  );
}
