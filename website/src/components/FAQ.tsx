import { Container } from "@/components/Container";
import { IconChevronDown } from "@/components/icons";

const FAQS = [
  {
    q: "Does DropLAN upload my files to the cloud?",
    a: "No. Files stay on your computer. DropLAN only serves them to other devices on the same local network — there is no external server in between.",
  },
  {
    q: "Does the receiving device need DropLAN installed?",
    a: "No. It just needs a modern browser. DropLAN itself only runs on the computer sharing the files.",
  },
  {
    q: "Does DropLAN require internet access?",
    a: "No, not for local sharing. Your computer and the receiving device just need to be able to reach each other on the same Wi-Fi or Ethernet network.",
  },
  {
    q: "Does it work between different operating systems?",
    a: "Yes. The sending computer can run macOS, Windows, or Linux, and any device with a browser — phone, tablet, or another computer, regardless of its OS — can receive.",
  },
  {
    q: "Can I share large files?",
    a: "Yes. Files are streamed from disk rather than loaded into memory, and downloads support HTTP range requests, so a large file can be seeked or resumed instead of restarting.",
  },
  {
    q: "Are transfers encrypted?",
    a: "No — DropLAN currently uses plain HTTP on your local network. A trusted certificate isn't practical for a private IP address, so treat your LAN like you would any other unencrypted local network.",
  },
  {
    q: "Is DropLAN free?",
    a: "Yes. It's free and open source under the MIT license.",
  },
];

export function FAQ() {
  return (
    <section id="faq" className="border-t border-line py-20 sm:py-28">
      <Container className="max-w-2xl">
        <h2 className="text-3xl font-semibold tracking-tight sm:text-4xl">
          Frequently asked questions
        </h2>

        <div className="mt-10 divide-y divide-line border-t border-b border-line">
          {FAQS.map(({ q, a }) => (
            <details key={q} className="group py-5">
              <summary className="flex cursor-pointer list-none items-center justify-between gap-4 text-[15px] font-medium">
                {q}
                <IconChevronDown className="size-4 shrink-0 text-muted transition-transform group-open:rotate-180" />
              </summary>
              <p className="mt-3 text-[15px] leading-relaxed text-muted">{a}</p>
            </details>
          ))}
        </div>
      </Container>
    </section>
  );
}
