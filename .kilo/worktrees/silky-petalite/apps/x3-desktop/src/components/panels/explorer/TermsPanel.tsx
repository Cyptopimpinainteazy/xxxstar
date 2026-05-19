export default function TermsPanel() {
  return (
    <div className="overflow-y-auto h-full bg-[#0a0a0f] text-white p-6">
      <div className="max-w-3xl mx-auto space-y-8">
        <div>
          <h1 className="text-2xl font-bold">Terms of Service</h1>
          <p className="text-slate-500 text-sm mt-1">Last updated: January 2026</p>
        </div>

        <section className="space-y-3">
          <h2 className="text-lg font-semibold text-slate-200">1. Acceptance of Terms</h2>
          <p className="text-sm text-slate-400 leading-relaxed">
            By accessing or using the X3 Chain network, decentralized applications, website, or any related
            services (collectively, the &ldquo;Services&rdquo;), you agree to be bound by these Terms of Service.
            If you do not agree to these terms, do not use the Services.
          </p>
        </section>

        <section className="space-y-3">
          <h2 className="text-lg font-semibold text-slate-200">2. Description of Service</h2>
          <p className="text-sm text-slate-400 leading-relaxed">
            X3 Chain provides a decentralized blockchain network with support for smart contracts, cross-chain
            bridging, decentralized finance (DeFi) protocols, GPU compute swarm, and related developer tools. The
            Services are provided on an &ldquo;as-is&rdquo; and &ldquo;as-available&rdquo; basis, and we make no
            guarantees of uptime, performance, or fitness for a particular purpose.
          </p>
        </section>

        <section className="space-y-3">
          <h2 className="text-lg font-semibold text-slate-200">3. User Responsibilities</h2>
          <ul className="list-disc list-inside text-sm text-slate-400 leading-relaxed space-y-1">
            <li>You are responsible for securing your private keys and wallet credentials.</li>
            <li>You will not use the Services for any unlawful purpose or in violation of applicable regulations.</li>
            <li>You acknowledge that blockchain transactions are irreversible once finalized.</li>
            <li>You will not attempt to disrupt, exploit, or compromise the network or its participants.</li>
            <li>You are solely responsible for understanding the risks associated with digital assets.</li>
          </ul>
        </section>

        <section className="space-y-3">
          <h2 className="text-lg font-semibold text-slate-200">4. Risks</h2>
          <p className="text-sm text-slate-400 leading-relaxed">
            The use of blockchain technology and digital assets involves significant risks, including but not limited
            to: market volatility, smart contract bugs or exploits, regulatory changes, loss of private keys
            resulting in permanent loss of funds, and network congestion. You acknowledge and accept these risks.
          </p>
        </section>

        <section className="space-y-3">
          <h2 className="text-lg font-semibold text-slate-200">5. Limitation of Liability</h2>
          <p className="text-sm text-slate-400 leading-relaxed">
            To the maximum extent permitted by law, X3 Chain and its contributors, developers, and affiliates
            shall not be liable for any indirect, incidental, special, consequential, or punitive damages arising
            from your use of the Services, including loss of funds, data, or profits.
          </p>
        </section>

        <section className="space-y-3">
          <h2 className="text-lg font-semibold text-slate-200">6. Intellectual Property</h2>
          <p className="text-sm text-slate-400 leading-relaxed">
            The X3 Chain protocol is open-source software licensed under applicable open-source licenses.
            Trademarks, logos, and branding remain the property of X3 Chain and may not be used without
            prior written permission.
          </p>
        </section>

        <section className="space-y-3">
          <h2 className="text-lg font-semibold text-slate-200">7. Modifications</h2>
          <p className="text-sm text-slate-400 leading-relaxed">
            We reserve the right to modify these Terms of Service at any time. Changes will be posted on this page
            with an updated revision date. Your continued use of the Services after changes constitutes acceptance
            of the new terms.
          </p>
        </section>

        <section className="space-y-3">
          <h2 className="text-lg font-semibold text-slate-200">8. Governing Law</h2>
          <p className="text-sm text-slate-400 leading-relaxed">
            These Terms shall be governed by and construed in accordance with the laws of the jurisdiction in which
            X3 Chain operates, without regard to conflict of law principles.
          </p>
        </section>

        <section className="space-y-3">
          <h2 className="text-lg font-semibold text-slate-200">9. Contact</h2>
          <p className="text-sm text-slate-400 leading-relaxed">
            For questions regarding these Terms, please contact us at:
          </p>
          <p className="text-sm text-orange-400 font-medium">legal@x3-chain.io</p>
        </section>

        <div className="border-t border-slate-800 pt-4 text-xs text-slate-600 text-center">
          &copy; 2026 X3 Chain. All rights reserved.
        </div>
      </div>
    </div>
  );
}
