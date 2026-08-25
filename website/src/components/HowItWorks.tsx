import type { ReactNode } from "react";

import { Container } from "@/components/Container";
import { IconMonitor, IconPhone, IconWifi } from "@/components/icons";

const STEPS = [
  {
    n: "1",
    title: "Drop",
    body: "Drop files into DropLAN, or choose them with the file picker — including whole folders. Nothing is copied elsewhere; DropLAN reads them from where they already live.",
  },
  {
    n: "2",
    title: "Scan",
    body: "DropLAN starts an embedded HTTP server on your machine and shows a local address plus a QR code for it.",
  },
  {
    n: "3",
    title: "Download",
    body: "Open the link from another device on the same Wi-Fi or Ethernet network and download straight in the browser. Nothing to install.",
  },
];

export function HowItWorks() {
  return (
    <section id="how-it-works" className="border-t border-line py-20 sm:py-28">
      <Container>
        <div className="max-w-xl">
          <h2 className="text-3xl font-semibold tracking-tight sm:text-4xl">How it works</h2>
          <p className="mt-3 text-lg text-muted">Three steps, no setup.</p>
        </div>

        <div className="mt-12 grid gap-10 sm:grid-cols-3 sm:gap-8">
          {STEPS.map((step) => (
            <div key={step.n}>
              <span className="flex size-9 items-center justify-center rounded-full bg-accent-soft text-sm font-semibold text-accent">
                {step.n}
              </span>
              <h3 className="mt-4 text-lg font-semibold">{step.title}</h3>
              <p className="mt-1.5 text-[15px] leading-relaxed text-muted">{step.body}</p>
            </div>
          ))}
        </div>

        <div className="mt-16 flex flex-col items-center gap-3 rounded-2xl border border-line bg-bg-raised px-6 py-10 sm:flex-row sm:justify-center sm:gap-6">
          <FlowNode icon={<IconMonitor className="size-5" />} label="Your computer" sub="Running DropLAN" />
          <FlowArrow label="Wi-Fi / LAN" icon={<IconWifi className="size-4" />} />
          <div className="flex flex-wrap items-center justify-center gap-6">
            <FlowNode icon={<IconPhone className="size-5" />} label="Phone" sub="Browser" small />
            <FlowNode icon={<IconMonitor className="size-5" />} label="Laptop" sub="Browser" small />
            <FlowNode icon={<IconPhone className="size-5" />} label="Tablet" sub="Browser" small />
          </div>
        </div>
      </Container>
    </section>
  );
}

function FlowNode({
  icon,
  label,
  sub,
  small = false,
}: {
  icon: ReactNode;
  label: string;
  sub: string;
  small?: boolean;
}) {
  return (
    <div className="flex flex-col items-center gap-2 text-center">
      <span
        className={`flex items-center justify-center rounded-xl border border-line bg-bg text-fg ${
          small ? "size-11" : "size-14"
        }`}
      >
        {icon}
      </span>
      <div>
        <p className="text-sm font-medium">{label}</p>
        <p className="text-xs text-muted">{sub}</p>
      </div>
    </div>
  );
}

function FlowArrow({ icon, label }: { icon: ReactNode; label: string }) {
  return (
    <div className="flex flex-col items-center gap-1.5 px-2">
      <div className="flex items-center gap-1.5 text-muted">
        <span className="h-px w-8 bg-line sm:w-12" />
        {icon}
        <span className="h-px w-8 bg-line sm:w-12" />
      </div>
      <span className="text-xs text-muted">{label}</span>
    </div>
  );
}
