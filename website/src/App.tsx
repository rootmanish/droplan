import { Navbar } from "@/components/Navbar";
import { Hero } from "@/components/Hero";
import { HowItWorks } from "@/components/HowItWorks";
import { ServerModel } from "@/components/ServerModel";
import { Privacy } from "@/components/Privacy";
import { Security } from "@/components/Security";
import { LargeFiles } from "@/components/LargeFiles";
import { Platforms } from "@/components/Platforms";
import { Download } from "@/components/Download";
import { OpenSource } from "@/components/OpenSource";
import { FAQ } from "@/components/FAQ";
import { Footer } from "@/components/Footer";

export default function App() {
  return (
    <div className="flex min-h-dvh flex-col">
      <Navbar />
      <main className="flex-1">
        <Hero />
        <HowItWorks />
        <ServerModel />
        <Privacy />
        <Security />
        <LargeFiles />
        <Platforms />
        <Download />
        <OpenSource />
        <FAQ />
      </main>
      <Footer />
    </div>
  );
}
