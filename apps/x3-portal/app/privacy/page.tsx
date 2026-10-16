import React from 'react';

export default function PrivacyPage() {
  return (
    <div className="min-h-screen pt-20 bg-black">
      {/* Hero */}
      <section className="py-16 relative overflow-hidden border-b border-[#1a1a1a]">
        <div className="absolute inset-0 bg-gradient-to-br from-gray-950/50 via-black to-gray-950/30" />
        <div className="relative z-10 max-w-4xl mx-auto px-4 sm:px-6 lg:px-8">
          <h1 className="text-4xl md:text-5xl font-bold text-white mb-4">Privacy Policy</h1>
          <p className="text-gray-500">Last updated: December 2024</p>
        </div>
      </section>

      <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="prose prose-invert max-w-none">
          <section className="mb-12">
            <h2 className="text-2xl font-bold text-white mb-4">1. Introduction</h2>
            <p className="text-gray-400 mb-4">
              X3 Atlas Sphere (&quot;we,&quot; &quot;our,&quot; or &quot;us&quot;) is committed to protecting your privacy. 
              This Privacy Policy explains how we collect, use, disclose, and safeguard your information 
              when you use our blockchain network, website, and related services.
            </p>
          </section>

          <section className="mb-12">
            <h2 className="text-2xl font-bold text-white mb-4">2. Information We Collect</h2>
            <h3 className="text-lg font-semibold text-white mb-2">2.1 On-Chain Data</h3>
            <p className="text-gray-400 mb-4">
              All transactions on X3 Atlas Sphere are recorded on a public blockchain. This includes:
            </p>
            <ul className="list-disc list-inside text-gray-400 mb-4 ml-4">
              <li>Wallet addresses</li>
              <li>Transaction hashes and amounts</li>
              <li>Smart contract interactions</li>
              <li>Comit transaction data</li>
            </ul>
            
            <h3 className="text-lg font-semibold text-white mb-2">2.2 Website Data</h3>
            <p className="text-gray-400 mb-4">
              When you visit our website, we may collect:
            </p>
            <ul className="list-disc list-inside text-gray-400 mb-4 ml-4">
              <li>IP address and browser information</li>
              <li>Pages visited and time spent</li>
              <li>Referral sources</li>
              <li>Device and operating system information</li>
            </ul>
          </section>

          <section className="mb-12">
            <h2 className="text-2xl font-bold text-white mb-4">3. How We Use Your Information</h2>
            <p className="text-gray-400 mb-4">We use collected information to:</p>
            <ul className="list-disc list-inside text-gray-400 mb-4 ml-4">
              <li>Operate and maintain our blockchain network</li>
              <li>Improve our services and user experience</li>
              <li>Analyze usage patterns and trends</li>
              <li>Communicate important updates</li>
              <li>Prevent fraud and ensure security</li>
            </ul>
          </section>

          <section className="mb-12">
            <h2 className="text-2xl font-bold text-white mb-4">4. Data Sharing</h2>
            <p className="text-gray-400 mb-4">
              We do not sell your personal information. We may share data with:
            </p>
            <ul className="list-disc list-inside text-gray-400 mb-4 ml-4">
              <li>Service providers who assist our operations</li>
              <li>Legal authorities when required by law</li>
              <li>Partners with your explicit consent</li>
            </ul>
          </section>

          <section className="mb-12">
            <h2 className="text-2xl font-bold text-white mb-4">5. Your Rights</h2>
            <p className="text-gray-400 mb-4">
              You have the right to:
            </p>
            <ul className="list-disc list-inside text-gray-400 mb-4 ml-4">
              <li>Access your personal data</li>
              <li>Request correction of inaccurate data</li>
              <li>Request deletion (where technically feasible)</li>
              <li>Opt-out of marketing communications</li>
            </ul>
            <p className="text-gray-400">
              Note: On-chain data cannot be modified or deleted due to the immutable nature of blockchain.
            </p>
          </section>

          <section className="mb-12">
            <h2 className="text-2xl font-bold text-white mb-4">6. Contact Us</h2>
            <p className="text-gray-400">
              For privacy-related inquiries, please contact us at:
              <br />
              <a href="mailto:privacy@atlas-sphere.io" className="text-orange-400 hover:text-orange-300">
                privacy@atlas-sphere.io
              </a>
            </p>
          </section>
        </div>
      </div>
    </div>
  );
}
