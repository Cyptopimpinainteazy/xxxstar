/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "export",
  reactStrictMode: true,
  images: { unoptimized: true },
  typescript: { ignoreBuildErrors: false },
};

module.exports = nextConfig;
