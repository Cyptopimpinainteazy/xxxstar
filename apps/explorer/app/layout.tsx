import type { Metadata } from 'next'

export const metadata: Metadata = {
  title: 'X3 Chain Explorer',
  description: 'X3 Chain Block Explorer',
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  )
}
