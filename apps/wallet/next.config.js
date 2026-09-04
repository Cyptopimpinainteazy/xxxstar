/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  transpilePackages: ['@x3-chain/shared', '@x3-chain/ts-sdk'],

  images: {
    domains: ["assets.x3-chain.io"],
    formats: ["image/avif", "image/webp"],
  },

  env: {
    NEXT_PUBLIC_CHAIN_RPC: process.env.NEXT_PUBLIC_CHAIN_RPC || "wss://ws.x3star.net/ws",
    NEXT_PUBLIC_EVM_RPC: process.env.NEXT_PUBLIC_EVM_RPC || "https://rpc.x3star.net/rpc",
    NEXT_PUBLIC_CHAIN_ID: process.env.NEXT_PUBLIC_CHAIN_ID || "x3-mainnet",
    NEXT_PUBLIC_POLKADEX_API: process.env.NEXT_PUBLIC_POLKADEX_API || "https://api.polkadex.trade",
  },
  typescript: {
    ignoreBuildErrors: true,
  },
};

module.exports = nextConfig;
