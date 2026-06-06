/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  // Ensure local monorepo packages are transpiled so their imports
  // (e.g., clsx, framer-motion) are resolved correctly by Next.js.
  transpilePackages: ['@x3-chain/shared', '@x3-chain/ts-sdk'],

  // Image optimization
  images: {
    domains: ["assets.x3-chain.io"],
    formats: ["image/avif", "image/webp"],
  },

  // Environment variables exposed to browser
  env: {
    NEXT_PUBLIC_CHAIN_RPC:
      process.env.NEXT_PUBLIC_CHAIN_RPC || "wss://ws.x3star.net/ws",
    NEXT_PUBLIC_EVM_RPC:
      process.env.NEXT_PUBLIC_EVM_RPC || "https://rpc.x3star.net/rpc",
    NEXT_PUBLIC_CHAIN_ID: process.env.NEXT_PUBLIC_CHAIN_ID || "x3-mainnet",
  },
};

module.exports = nextConfig;
