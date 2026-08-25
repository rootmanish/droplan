import { Container } from "@/components/Container";
import { IconGithub } from "@/components/icons";
import { REPO_URL } from "@/lib/site";

export function Hero() {
  return (
    <section id="top" className="pt-14 pb-20 sm:pt-20 sm:pb-28">
      <Container className="grid items-center gap-14 lg:grid-cols-[1.05fr_1fr] lg:gap-10">
        <div className="animate-rise-in">
          <p className="mb-5 inline-flex items-center rounded-full border border-line bg-bg-raised px-3 py-1 text-xs font-medium text-muted">
            Free &amp; open source · MIT licensed
          </p>

          <h1 className="text-[2.6rem] leading-[1.05] font-semibold tracking-tight text-balance sm:text-6xl">
            Drop files.
            <br />
            Share over <span className="text-cyan">LAN</span>.
          </h1>

          <p className="mt-5 max-w-md text-lg leading-relaxed text-muted">
            Share files from your computer with any device on the same network. Scan a QR code
            and download directly in the browser.
          </p>

          <ul className="mt-6 flex flex-wrap gap-x-5 gap-y-2 text-sm font-medium text-fg">
            <li className="flex items-center gap-1.5">No cloud</li>
            <li className="flex items-center gap-1.5">No account</li>
            <li className="flex items-center gap-1.5">No internet required</li>
          </ul>

          <div className="mt-8 flex flex-wrap items-center gap-3">
            <a
              href="#download"
              className="inline-flex items-center gap-2 rounded-xl bg-accent px-5 py-3 text-[15px] font-semibold text-accent-fg shadow-sm transition-opacity hover:opacity-90"
            >
              Download DropLAN
            </a>
            <a
              href={REPO_URL}
              target="_blank"
              rel="noreferrer"
              className="inline-flex items-center gap-2 rounded-xl border border-line px-5 py-3 text-[15px] font-semibold text-fg transition-colors hover:bg-bg-raised"
            >
              <IconGithub className="size-4" />
              View on GitHub
            </a>
          </div>
        </div>

        <div className="relative mx-auto w-full max-w-sm lg:max-w-none">
          <div className="relative rounded-2xl border border-line bg-bg-raised p-2 shadow-[0_1px_2px_rgba(16,20,30,0.06),0_16px_40px_-16px_rgba(16,20,30,0.25)]">
            <div className="flex items-center gap-1.5 px-2 pt-1 pb-2">
              <span className="size-2.5 rounded-full bg-line" />
              <span className="size-2.5 rounded-full bg-line" />
              <span className="size-2.5 rounded-full bg-line" />
            </div>
            <img
              src="/images/app-light.png"
              width={560}
              height={1120}
              alt="The DropLAN desktop app, showing the share link, QR code, and files being shared"
              className="w-full rounded-lg dark:hidden"
              loading="eager"
              fetchPriority="high"
            />
            <img
              src="/images/app-dark.png"
              width={560}
              height={1120}
              alt="The DropLAN desktop app, showing the share link, QR code, and files being shared"
              className="hidden w-full rounded-lg dark:block"
              loading="eager"
              fetchPriority="high"
            />
          </div>

          <div className="absolute -right-4 -bottom-8 w-[42%] rounded-[1.4rem] border border-line bg-bg-raised p-1.5 shadow-[0_1px_2px_rgba(16,20,30,0.06),0_20px_40px_-14px_rgba(16,20,30,0.35)] sm:-right-10">
            <img
              src="/images/phone-light.png"
              width={520}
              height={478}
              alt="The same files, open in a phone's browser after scanning the QR code"
              className="w-full rounded-[1rem] dark:hidden"
              loading="eager"
            />
            <img
              src="/images/phone-dark.png"
              width={520}
              height={478}
              alt="The same files, open in a phone's browser after scanning the QR code"
              className="hidden w-full rounded-[1rem] dark:block"
              loading="eager"
            />
          </div>
        </div>
      </Container>
    </section>
  );
}
