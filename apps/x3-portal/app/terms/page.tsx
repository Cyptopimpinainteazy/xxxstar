import React from 'react';

export default function TermsPage() {
  return (
    <div className="min-h-screen pt-20 bg-black">
      {/* Hero */}
      <section className="py-16 relative overflow-hidden border-b border-[#1a1a1a]">
        <div className="absolute inset-0 bg-gradient-to-br from-gray-950/50 via-black to-gray-950/30" />
        <div className="relative z-10 max-w-4xl mx-auto px-4 sm:px-6 lg:px-8">
          <h1 className="text-4xl md:text-5xl font-bold text-white mb-4">Terms of Service</h1>
          <p className="text-gray-500">Last updated: December 2024</p>
        </div>
      </section>

      <div className="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-12">
        <div className="prose prose-invert max-w-none">
          <section className="mb-12">
            <h2 className="text-2xl font-bold text-white mb-4">1. Acceptance of Terms</h2>
            <p className="text-gray-400 mb-4">
              By accessing or using X3 Atlas Sphere, you agree to be bound by these Terms of Service. 
              If you do not agree to these terms, do not use our services.
            </p>
          </section>

          <section className="mb-12">
            <h2 className="text-2xl font-bold text-white mb-4">2. Description of Service</h2>
            <p className="text-gray-400 mb-4">
              X3 Atlas Sphere is a decentralized blockchain platform that provides:
            </p>
            <ul className="list-disc list-inside text-gray-400 mb-4 ml-4">
              <li>Dual VM (EVM and SVM) execution environment</li>
              <li>Cross-VM atomic transactions (Comits)</li>
              <li>Canonical ledger for unified asset management</li>
              <li>Developer tools and APIs</li>
            </ul>
          </section>

          <section className="mb-12">
            <h2 className="text-2xl font-bold text-white mb-4">3. User Responsibilities</h2>
            <p className="text-gray-400 mb-4">You agree to:</p>
            <ul className="list-disc list-inside text-gray-400 mb-4 ml-4">
              <li>Use the platform in compliance with all applicable laws</li>
              <li>Maintain the security of your private keys and accounts</li>
              <li>Not engage in any activity that disrupts the network</li>
              <li>Not use the platform for illegal or fraudulent activities</li>
              <li>Take responsibility for all transactions made with your account</li>
            </ul>
          </section>

          <section className="mb-12">
            <h2 className="text-2xl font-bold text-white mb-4">4. Risks and Disclaimers</h2>
            <p className="text-gray-400 mb-4">
              You acknowledge and accept that:
            </p>
            <ul className="list-disc list-inside text-gray-400 mb-4 ml-4">
              <li>Blockchain technology involves inherent risks</li>
              <li>Cryptocurrency values can be highly volatile</li>
              <li>Smart contracts may contain bugs or vulnerabilities</li>
              <li>Lost private keys cannot be recovered</li>
              <li>Transactions are irreversible once confirmed</li>
            </ul>
          </section>

          <section className="mb-12">
            <h2 className="text-2xl font-bold text-white mb-4">5. Limitation of Liability</h2>
            <p className="text-gray-400 mb-4">
              TO THE MAXIMUM EXTENT PERMITTED BY LAW, X3 ATLAS SPHERE SHALL NOT BE LIABLE FOR ANY 
              INDIRECT, INCIDENTAL, SPECIAL, CONSEQUENTIAL, OR PUNITIVE DAMAGES, INCLUDING BUT NOT 
              LIMITED TO LOSS OF PROFITS, DATA, OR OTHER INTANGIBLE LOSSES.
            </p>
          </section>

          <section className="mb-12">
            <h2 className="text-2xl font-bold text-white mb-4">6. Intellectual Property</h2>
            <p className="text-gray-400 mb-4">
              The X3 Atlas Sphere protocol is open source under the Apache 2.0 license. 
              The X3 Atlas Sphere name, logo, and branding are trademarks of the Atlas Foundation.
            </p>
          </section>

          <section className="mb-12">
            <h2 className="text-2xl font-bold text-white mb-4">7. Modifications</h2>
            <p className="text-gray-400 mb-4">
              We reserve the right to modify these terms at any time. Continued use of the platform 
              after changes constitutes acceptance of the modified terms.
            </p>
          </section>

          <section className="mb-12">
            <h2 className="text-2xl font-bold text-white mb-4">8. Governing Law</h2>
            <p className="text-gray-400 mb-4">
              These terms shall be governed by and construed in accordance with applicable laws, 
              without regard to conflict of law principles.
            </p>
          </section>

          <section className="mb-12">
            <h2 className="text-2xl font-bold text-white mb-4">9. Contact</h2>
            <p className="text-gray-400">
              For questions about these terms, please contact us at:
              <br />
              <a href="mailto:legal@atlas-sphere.io" className="text-orange-400 hover:text-orange-300">
                legal@atlas-sphere.io
              </a>
            </p>
          </section>
        </div>
      </div>
    </div>
  );
}
