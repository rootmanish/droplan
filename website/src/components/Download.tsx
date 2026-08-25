import { IconArrowRight, IconDownload } from "@/components/icons";
import { Container } from "@/components/Container";
import { BUILD_FROM_SOURCE_URL, RELEASES_URL } from "@/lib/site";

const PLATFORMS = [
  { name: "macOS", formats: ".dmg", detail: "Apple Silicon and Intel" },
  { name: "Windows", formats: ".exe · .msi", detail: "NSIS installer and MSI" },
  { name: "Linux", formats: ".AppImage · .deb", detail: "x86_64" },
];

export function Download() {
  return (
    <section id="download" className="border-t border-line py-20 sm:py-28">
      <Container>
        <div className="max-w-xl">
          <h2 className="text-3xl font-semibold tracking-tight sm:text-4xl">Download DropLAN</h2>
          <p className="mt-4 text-[15px] leading-relaxed text-muted">
            Installers for macOS, Windows, and Linux are built automatically and attached to{" "}
            <a href={RELEASES_URL} target="_blank" rel="noreferrer" className="text-accent hover:underline">
              GitHub Releases
            </a>{" "}
            whenever a new version is tagged. No release has been published yet — the page above
            will list one as soon as it is, or you can build DropLAN from source today.
          </p>
        </div>

        <div className="mt-10 grid gap-5 sm:grid-cols-3">
          {PLATFORMS.map((platform) => (
            <a
              key={platform.name}
              href={RELEASES_URL}
              target="_blank"
              rel="noreferrer"
              className="group flex flex-col rounded-2xl border border-line bg-bg-raised p-6 transition-colors hover:border-accent/40"
            >
              <IconDownload className="size-5 text-accent" />
              <p className="mt-4 text-lg font-semibold">{platform.name}</p>
              <p className="mt-1 font-mono text-sm text-muted">{platform.formats}</p>
              <p className="mt-0.5 text-xs text-muted">{platform.detail}</p>
              <span className="mt-5 inline-flex items-center gap-1.5 text-sm font-medium text-accent">
                View releases
                <IconArrowRight className="size-3.5 transition-transform group-hover:translate-x-0.5" />
              </span>
            </a>
          ))}
        </div>

        <p className="mt-8 text-sm text-muted">
          Prefer to build it yourself?{" "}
          <a href={BUILD_FROM_SOURCE_URL} target="_blank" rel="noreferrer" className="text-accent hover:underline">
            Build DropLAN from source
          </a>
          .
        </p>
      </Container>
    </section>
  );
}
