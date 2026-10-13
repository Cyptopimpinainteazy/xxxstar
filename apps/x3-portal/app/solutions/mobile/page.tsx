'use client';

import React from 'react';
import Link from 'next/link';
import {
  Smartphone,
  Fingerprint,
  Wifi,
  QrCode,
  Bell,
  Zap,
  Shield,
  Download,
  CheckCircle2,
  Code,
} from 'lucide-react';
import { HexagonCluster } from '../../../components/frontend/ui/Logo';

const mobileFeatures = [
  {
    name: 'Native SDKs',
    description: 'Swift and Kotlin SDKs for iOS and Android',
    icon: <Code className="w-6 h-6" />,
  },
  {
    name: 'Biometric Auth',
    description: 'Face ID, Touch ID, and fingerprint support',
    icon: <Fingerprint className="w-6 h-6" />,
  },
  {
    name: 'Offline Signing',
    description: 'Sign transactions without network connectivity',
    icon: <Wifi className="w-6 h-6" />,
  },
  {
    name: 'Push Notifications',
    description: 'Real-time alerts for transactions and events',
    icon: <Bell className="w-6 h-6" />,
  },
];

const sdkOptions = [
  {
    platform: 'iOS',
    language: 'Swift',
    installCmd: 'pod install X3StarSDK',
    features: ['SwiftUI components', 'Keychain integration', 'Face ID/Touch ID'],
  },
  {
    platform: 'Android',
    language: 'Kotlin',
    installCmd: "implementation 'io.x3star:sdk:1.0.0'",
    features: ['Jetpack Compose', 'Biometric API', 'KeyStore integration'],
  },
  {
    platform: 'React Native',
    language: 'TypeScript',
    installCmd: 'npm install @x3star/react-native',
    features: ['Cross-platform', 'Expo support', 'Native modules'],
  },
  {
    platform: 'Flutter',
    language: 'Dart',
    installCmd: 'flutter pub add x3star_sdk',
    features: ['Widget library', 'Platform channels', 'Hot reload'],
  },
];

const codeExample = `// iOS Swift SDK Example
import X3StarSDK

class WalletManager {
    let sdk = X3Star(network: .mainnet)
    
    func connectWallet() async throws {
        // Authenticate with biometrics
        let auth = try await sdk.authenticate(
            method: .biometric
        )
        
        // Get wallet address
        let wallet = try await sdk.getWallet()
        print("Connected: \\(wallet.address)")
    }
    
    func sendTransaction() async throws {
        let tx = Transaction(
            to: "0x742d35Cc6634C0532925a3b844Bc...",
            amount: "10.0",
            token: .usdc
        )
        
        // Sign with biometric auth
        let signed = try await sdk.signTransaction(
            tx, 
            auth: .biometric
        )
        
        // Submit to network
        let hash = try await sdk.submit(signed)
        print("TX Hash: \\(hash)")
    }
}`;

const apps = [
  { name: 'Atlas Mobile', platform: 'iOS & Android', users: '50K+', rating: '4.8' },
  { name: 'Sphere Pay', platform: 'iOS & Android', users: '30K+', rating: '4.7' },
  { name: 'X3 Games', platform: 'iOS', users: '25K+', rating: '4.6' },
];

export default function MobilePage() {
  return (
    <div className="min-h-screen pt-20">
      {/* Hero */}
      <section className="py-20 relative overflow-hidden">
        <div className="absolute inset-0 mesh-gradient opacity-30" />
        <div className="absolute right-0 top-1/4 w-96 h-96 opacity-30">
          <HexagonCluster className="w-full h-full" />
        </div>
        
        <div className="relative z-10 container-wide">
          <div className="max-w-3xl">
            <Link href="/solutions" className="text-gray-400 hover:text-white mb-4 inline-flex items-center">
              ← Back to Solutions
            </Link>
            <div className="badge badge-info mt-4 mb-4">Mobile</div>
            <h1 className="text-4xl md:text-5xl font-bold text-white mb-6">
              Mobile-First Blockchain
            </h1>
            <p className="text-xl text-gray-400 mb-8">
              Build native mobile apps with blockchain capabilities. Native SDKs for 
              iOS, Android, React Native, and Flutter.
            </p>
            <div className="flex flex-wrap gap-4">
              <Link href="/developers/docs" className="btn-primary">
                <Download className="w-4 h-4 mr-2" />
                Get SDK
              </Link>
              <Link href="/developers/cookbook" className="btn-secondary">
                View Examples
              </Link>
            </div>
          </div>
        </div>
      </section>

      {/* Features */}
      <section className="py-12 border-t border-[#1a1a1a]">
        <div className="container-wide">
          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6">
            {mobileFeatures.map((feature, index) => (
              <div key={index} className="flex items-start">
                <div className="p-2 rounded-lg bg-orange-500/20 text-orange-400 mr-4">
                  {feature.icon}
                </div>
                <div>
                  <h3 className="font-semibold text-white mb-1">{feature.name}</h3>
                  <p className="text-sm text-gray-400">{feature.description}</p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* SDK Options */}
      <section className="py-16">
        <div className="container-wide">
          <h2 className="text-2xl font-bold text-white mb-8">SDK Options</h2>
          
          <div className="grid md:grid-cols-2 gap-6">
            {sdkOptions.map((sdk, index) => (
              <div key={index} className="glass-card p-6">
                <div className="flex items-center justify-between mb-4">
                  <div>
                    <h3 className="text-xl font-semibold text-white">{sdk.platform}</h3>
                    <span className="text-sm text-gray-400">{sdk.language}</span>
                  </div>
                  <span className="badge badge-default">{sdk.language}</span>
                </div>
                <div className="p-3 rounded-lg bg-[#0a0a0a] border border-[#1a1a1a] mb-4">
                  <code className="text-sm text-cyan-400">{sdk.installCmd}</code>
                </div>
                <div className="flex flex-wrap gap-2">
                  {sdk.features.map((feature, i) => (
                    <span key={i} className="text-xs px-2 py-1 rounded bg-[#0a0a0a] text-gray-400">
                      {feature}
                    </span>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Code Example */}
      <section className="py-16 bg-[#050505]">
        <div className="container-wide">
          <div className="grid lg:grid-cols-2 gap-12 items-center">
            <div>
              <h2 className="text-2xl font-bold text-white mb-4">Native Experience</h2>
              <p className="text-gray-400 mb-6">
                Our mobile SDKs provide a seamless native experience with biometric 
                authentication, secure key storage, and offline capabilities.
              </p>
              <ul className="space-y-3">
                {[
                  'Secure enclave key storage',
                  'Biometric transaction signing',
                  'Background sync and notifications',
                  'Deep linking for dApp integration',
                ].map((item, i) => (
                  <li key={i} className="flex items-center text-gray-400">
                    <CheckCircle2 className="w-5 h-5 text-emerald-500 mr-3" />
                    {item}
                  </li>
                ))}
              </ul>
            </div>
            <div className="glass-card p-0 overflow-hidden">
              <div className="p-3 border-b border-[#1a1a1a] flex items-center justify-between">
                <span className="text-sm text-gray-400">WalletManager.swift</span>
                <button className="text-xs text-gray-500 hover:text-white">Copy</button>
              </div>
              <pre className="p-4 overflow-x-auto text-sm">
                <code className="text-gray-400">{codeExample}</code>
              </pre>
            </div>
          </div>
        </div>
      </section>

      {/* Mobile Preview */}
      <section className="py-16">
        <div className="container-wide">
          <div className="grid lg:grid-cols-2 gap-12 items-center">
            <div className="glass-card p-8 flex justify-center">
              <div className="w-64 h-[500px] rounded-[40px] bg-[#0a0a0a] border-4 border-[#1a1a1a] p-3 relative">
                <div className="absolute top-6 left-1/2 -translate-x-1/2 w-20 h-6 bg-black rounded-full" />
                <div className="w-full h-full rounded-[32px] bg-gradient-to-br from-black to-[#0a0a0a] flex flex-col items-center justify-center">
                  <div className="text-5xl mb-4">📱</div>
                  <p className="text-white font-medium">X3 Mobile</p>
                  <p className="text-sm text-gray-400">Coming Soon</p>
                </div>
              </div>
            </div>
            <div>
              <h2 className="text-2xl font-bold text-white mb-4">Apps Built on X3 STAR</h2>
              <p className="text-gray-400 mb-6">
                Join the growing ecosystem of mobile apps building on X3 STAR.
              </p>
              <div className="space-y-4">
                {apps.map((app, index) => (
                  <div key={index} className="glass-card p-4 flex items-center justify-between">
                    <div>
                      <h3 className="font-medium text-white">{app.name}</h3>
                      <p className="text-sm text-gray-400">{app.platform}</p>
                    </div>
                    <div className="text-right">
                      <p className="text-emerald-400">{app.users}</p>
                      <p className="text-sm text-gray-400">⭐ {app.rating}</p>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section className="py-16 border-t border-[#1a1a1a]">
        <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 text-center">
          <Smartphone className="w-12 h-12 text-orange-400 mx-auto mb-6" />
          <h2 className="text-3xl font-bold text-white mb-4">
            Build Your Mobile App
          </h2>
          <p className="text-gray-400 mb-8">
            Get started with our mobile SDK and build the next great blockchain app.
          </p>
          <div className="flex flex-wrap justify-center gap-4">
            <Link href="/developers/docs" className="btn-primary">
              Mobile SDK Docs
            </Link>
            <Link href="/community/grants" className="btn-secondary">
              Apply for Grant
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
