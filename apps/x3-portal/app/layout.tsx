import './globals.css';
import Navigation from '@/components/layout/Navigation';
import Footer from '@/components/layout/Footer';
import { Providers } from '@/components/providers';
import FloatingNeuralTerminal from '@/components/ai/FloatingNeuralTerminal';

export const metadata = {
  title: 'X3 Atlas Sphere - Dual VM Layer-1 Blockchain',
  description: 'Next-generation Layer-1 blockchain with native EVM and SVM interoperability. Build cross-chain applications with atomic execution.',
  keywords: 'blockchain, EVM, SVM, cross-chain, DeFi, Web3, smart contracts, Solana, Ethereum',
  openGraph: {
    title: 'X3 Atlas Sphere - Dual VM Layer-1 Blockchain',
    description: 'Next-generation Layer-1 blockchain with native EVM and SVM interoperability.',
    type: 'website',
    locale: 'en_US',
    siteName: 'X3 Atlas Sphere',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'X3 Atlas Sphere - Dual VM Layer-1 Blockchain',
    description: 'Next-generation Layer-1 blockchain with native EVM and SVM interoperability.',
  },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className="scroll-smooth">
      <body className="bg-slate-950 min-h-screen">
        <Providers>
          <Navigation />
          <main>{children}</main>
          <Footer />
          <FloatingNeuralTerminal />
        </Providers>
      </body>
    </html>
  );
}