import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "X3 Chain Wallet | X3 Trading Dashboard",
  description: "Integrated crypto wallet with advanced trading platform powered by Polkadex",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="bg-dark text-white">
        {children}
      </body>
    </html>
  );
}
