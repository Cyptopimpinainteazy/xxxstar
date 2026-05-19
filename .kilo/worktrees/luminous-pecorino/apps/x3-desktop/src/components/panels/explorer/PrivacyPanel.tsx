export default function PrivacyPanel() {
  return (
    <div className="overflow-y-auto h-full bg-[#0a0a0f] text-white p-6">
      <div className="max-w-3xl mx-auto space-y-8">
        <div>
          <h1 className="text-2xl font-bold">Privacy Policy</h1>
          <p className="text-slate-500 text-sm mt-1">Last updated: January 2026</p>
        </div>

        <section className="space-y-3">
          <h2 className="text-lg font-semibold text-slate-200">1. Introduction</h2>
          <p className="text-sm text-slate-400 leading-relaxed">
            X3 Chain (&ldquo;we&rdquo;, &ldquo;us&rdquo;, or &ldquo;our&rdquo;) is committed to protecting your
            privacy. This Privacy Policy explains how we collect, use, disclose, and safeguard your information when
            you use our blockchain network, applications, and website (collectively, the &ldquo;Services&rdquo;).
          </p>
        </section>

        <section className="space-y-3">
          <h2 className="text-lg font-semibold text-slate-200">2. Information We Collect</h2>

          <h3 className="text-md font-medium text-slate-300">On-Chain Data</h3>
          <p className="text-sm text-slate-400 leading-relaxed">
            Transactions on the X3 Chain blockchain are public and transparent by design. This includes wallet
            addresses, transaction amounts, timestamps, and smart contract interactions. We do not control on-chain
            data once it is finalized, and it cannot be deleted due to the immutable nature of blockchain technology.
          </p>

          <h3 className="text-md font-medium text-slate-300">Website &amp; Application Data</h3>
          <p className="text-sm text-slate-400 leading-relaxed">
            When you interact with our website or applications, we may collect: device information (browser type,
            operating system), usage data (pages visited, features used), IP addresses (anonymized where possible),
            and optional account information you provide (email, display name). We use cookies and similar
            technologies for analytics and improving user experience. You may disable cookies in your browser settings.
          </p>
        </section>

        <section className="space-y-3">
          <h2 className="text-lg font-semibold text-slate-200">3. How We Use Your Information</h2>
          <ul className="list-disc list-inside text-sm text-slate-400 leading-relaxed space-y-1">
            <li>To provide, maintain, and improve our Services</li>
            <li>To communicate with you about updates, security alerts, and support</li>
            <li>To detect and prevent fraud, abuse, and security incidents</li>
            <li>To comply with legal obligations and enforce our terms</li>
            <li>To conduct research and analytics to improve our network</li>
          </ul>
        </section>

        <section className="space-y-3">
          <h2 className="text-lg font-semibold text-slate-200">4. Data Sharing</h2>
          <p className="text-sm text-slate-400 leading-relaxed">
            We do not sell your personal information. We may share data with: service providers who assist in
            operating our Services (under strict confidentiality agreements), law enforcement when required by law,
            and other parties with your explicit consent. On-chain data is publicly accessible and not subject to
            this section.
          </p>
        </section>

        <section className="space-y-3">
          <h2 className="text-lg font-semibold text-slate-200">5. Your Rights</h2>
          <p className="text-sm text-slate-400 leading-relaxed">
            Depending on your jurisdiction, you may have the right to: access, correct, or delete your personal data;
            opt out of certain data processing activities; request data portability; and withdraw consent. Note that
            on-chain data cannot be modified or deleted due to the nature of blockchain technology. To exercise your
            rights, contact us at the address below.
          </p>
        </section>

        <section className="space-y-3">
          <h2 className="text-lg font-semibold text-slate-200">6. Contact</h2>
          <p className="text-sm text-slate-400 leading-relaxed">
            For questions or concerns about this Privacy Policy, please contact us at:
          </p>
          <p className="text-sm text-orange-400 font-medium">privacy@x3-chain.io</p>
        </section>

        <div className="border-t border-slate-800 pt-4 text-xs text-slate-600 text-center">
          &copy; 2026 X3 Chain. All rights reserved.
        </div>
      </div>
    </div>
  );
}
