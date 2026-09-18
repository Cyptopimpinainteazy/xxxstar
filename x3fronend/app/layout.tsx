import type { Metadata, Viewport } from "next";
import { Inter, JetBrains_Mono } from "next/font/google";
import "./globals.css";

const inter = Inter({ subsets: ["latin"], variable: "--font-sans", display: "swap" });
const mono = JetBrains_Mono({ subsets: ["latin"], variable: "--font-mono", display: "swap", weight: ["400", "500", "700"] });

const TITLE = "X3 Atomic Star — Multi-VM L1 for Atomic Cross-Chain Settlement";
const DESCRIPTION =
  "A Substrate L1 unifying X3Native, EVM, and SVM execution under one atomic settlement model. v0.4 Internal Testnet Candidate — status reported directly from the repository's own readiness registry, not marketing copy.";

export const metadata: Metadata = {
  metadataBase: new URL("https://www.x3star.net"),
  title: TITLE,
  description: DESCRIPTION,
  openGraph: {
    title: TITLE,
    description: DESCRIPTION,
    url: "https://www.x3star.net",
    siteName: "X3 Atomic Star",
    type: "website",
  },
  twitter: {
    card: "summary_large_image",
    title: TITLE,
    description: DESCRIPTION,
  },
};

export const viewport: Viewport = {
  themeColor: "#07080c",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={`${inter.variable} ${mono.variable}`}>
      <body>{children}</body>
    </html>
  );
}
