import { Container } from "@/components/Container";
import { IconGithub } from "@/components/icons";
import { LICENSE_URL, REPO_URL } from "@/lib/site";

const STACK = ["Rust", "Tauri", "React", "TypeScript", "Axum"];

export function OpenSource() {
  return (
    <section className="border-t border-line py-20 sm:py-28">
      <Container className="flex flex-col items-center text-center">
        <h2 className="text-3xl font-semibold tracking-tight sm:text-4xl">
          Open source, by design.
        </h2>
        <p className="mt-4 max-w-md text-[15px] leading-relaxed text-muted">
          DropLAN is{" "}
          <a href={LICENSE_URL} target="_blank" rel="noreferrer" className="text-accent hover:underline">
            MIT licensed
          </a>
          . Read the code, audit the security model, or send a pull request.
        </p>

        <a
          href={REPO_URL}
          target="_blank"
          rel="noreferrer"
          className="mt-7 inline-flex items-center gap-2 rounded-xl bg-accent px-5 py-3 text-[15px] font-semibold text-accent-fg transition-opacity hover:opacity-90"
        >
          <IconGithub className="size-4" />
          View source on GitHub
        </a>

        <div className="mt-10 flex flex-wrap items-center justify-center gap-x-5 gap-y-2 text-sm text-muted">
          {STACK.map((tech) => (
            <span key={tech}>{tech}</span>
          ))}
        </div>
      </Container>
    </section>
  );
}
