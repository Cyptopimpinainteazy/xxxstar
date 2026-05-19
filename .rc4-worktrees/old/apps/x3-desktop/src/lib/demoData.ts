// Demo Data Generator for X3 Desktop TIER 6 & 7
// Generates sample contacts, campaigns, and social network data for testing

export const DEMO_CONTACTS = [
  {
    firstName: 'Alice',
    lastName: 'Johnson',
    email: 'alice.johnson@techcorp.com',
    phone: '+1-555-0101',
    company: 'TechCorp Inc',
    jobTitle: 'VP Engineering',
    source: 'LinkedIn',
    status: 'qualified',
    tags: ['enterprise', 'high-value', 'tech'],
  },
  {
    firstName: 'Bob',
    lastName: 'Smith',
    email: 'bob.smith@startupxyz.io',
    phone: '+1-555-0102',
    company: 'StartupXYZ',
    jobTitle: 'CEO & Founder',
    source: 'Conference',
    status: 'hot',
    tags: ['startup', 'decision-maker'],
  },
  {
    firstName: 'Carol',
    lastName: 'Davis',
    email: 'carol.davis@finance.global',
    phone: '+1-555-0103',
    company: 'Finance Global',
    jobTitle: 'CFO',
    source: 'Referral',
    status: 'qualified',
    tags: ['finance', 'c-level'],
  },
  {
    firstName: 'David',
    lastName: 'Wilson',
    email: 'david.wilson@medtech.bio',
    phone: '+1-555-0104',
    company: 'MedTech Bio',
    jobTitle: 'Product Manager',
    source: 'Website',
    status: 'prospect',
    tags: ['healthcare', 'mid-market'],
  },
  {
    firstName: 'Emma',
    lastName: 'Martinez',
    email: 'emma.martinez@retailco.com',
    phone: '+1-555-0105',
    company: 'RetailCo',
    jobTitle: 'Head of Innovation',
    source: 'Inbound',
    status: 'qualified',
    tags: ['retail', 'enterprise'],
  },
  {
    firstName: 'Frank',
    lastName: 'Zhang',
    email: 'frank.zhang@cloudtech.com',
    phone: '+1-555-0106',
    company: 'CloudTech Solutions',
    jobTitle: 'CTO',
    source: 'Partner',
    status: 'hot',
    tags: ['technology', 'strategic'],
  },
  {
    firstName: 'Grace',
    lastName: 'Kim',
    email: 'grace.kim@marketing.digital',
    phone: '+1-555-0107',
    company: 'Digital Marketing Pro',
    jobTitle: 'Marketing Director',
    source: 'Event',
    status: 'qualified',
    tags: ['marketing', 'agency'],
  },
  {
    firstName: 'Henry',
    lastName: 'Brown',
    email: 'henry.brown@construction.dev',
    phone: '+1-555-0108',
    company: 'Construction Dev',
    jobTitle: 'PM Office Manager',
    source: 'Cold Outreach',
    status: 'prospect',
    tags: ['construction', 'sme'],
  },
  {
    firstName: 'Isabella',
    lastName: 'Garcia',
    email: 'isabella.garcia@esgfirst.org',
    phone: '+1-555-0109',
    company: 'ESG First',
    jobTitle: 'Sustainability Officer',
    source: 'Webinar',
    status: 'qualified',
    tags: ['sustainability', 'enterprise'],
  },
  {
    firstName: 'James',
    lastName: 'Lee',
    email: 'james.lee@logistics.io',
    phone: '+1-555-0110',
    company: 'Logistics IO',
    jobTitle: 'Operations VP',
    source: 'LinkedIn',
    status: 'hot',
    tags: ['logistics', 'high-value'],
  },
];

export const DEMO_EMAIL_TEMPLATES = [
  {
    name: 'First Contact',
    subject: 'Quick intro - {{firstName}}',
    body: `<p>Hi {{firstName}},</p>
<p>I came across {{company}} and was impressed by your work in {{industry}}.</p>
<p>Would love to chat about mutual opportunities.</p>
<p>Best,<br/>Sales Team</p>`,
  },
  {
    name: 'Follow-up (3 days)',
    subject: 'Following up - {{firstName}}',
    body: `<p>Hi {{firstName}},</p>
<p>Just checking in on my previous message. Would love to find 15 mins for a quick call next week?</p>
<p>Let me know what works for you.</p>`,
  },
  {
    name: 'Product Demo',
    subject: 'Let\'s see if we\'re a fit - {{firstName}}',
    body: `<p>Hi {{firstName}},</p>
<p>Based on our conversation, I think you'd find our solution valuable for {{department}}.</p>
<p>Available for demo:</p>
<ul>
  <li>Tuesday 2pm</li>
  <li>Wednesday 10am</li>
  <li>Thursday 3pm</li>
</ul>
<p>What works best?</p>`,
  },
  {
    name: 'Case Study',
    subject: 'How {{competitor}} increased efficiency by 40%',
    body: `<p>Hi {{firstName}},</p>
<p>Thought you'd be interested in this case study from a similar company in {{industry}}.</p>
<p>They achieved 40% efficiency gains in their {{department}} function.</p>
<p><strong><a href="#">Read the full case study</a></strong></p>`,
  },
  {
    name: 'Close',
    subject: 'Final offer - {{firstName}}',
    body: `<p>Hi {{firstName}},</p>
<p>After our conversations, I'm confident we can deliver real value to {{company}}.</p>
<p>Here's what I'm proposing:</p>
<ul>
  <li>30-day pilot for {{department}}</li>
  <li>No-risk guarantee</li>
  <li>${'{{discount}}'}k special pricing</li>
</ul>
<p>Let's move forward?</p>`,
  },
];

export const DEMO_CAMPAIGNS = [
  {
    name: 'Q1 2026 Enterprise Push',
    campaignType: 'email',
    targetContacts: 5,
    description: 'Outbound campaign targeting C-level executives at enterprise companies',
  },
  {
    name: 'Startup Fast-Track',
    campaignType: 'email',
    targetContacts: 3,
    description: 'Special pricing campaign for growing startups with 50-500 employees',
  },
  {
    name: 'Partner Activation',
    campaignType: 'multi-channel',
    targetContacts: 2,
    description: 'Activate partner network with co-marketing materials',
  },
  {
    name: 'Nurture Sequence',
    campaignType: 'drip',
    targetContacts: 10,
    description: 'Automated 5-email nurture sequence for prospects',
  },
];

export const DEMO_SOCIAL_POSTS = [
  {
    content: 'Just launched TIER 6 & 7 of X3 Desktop! 🚀 Check out the new CRM and Social features.',
    mediaHashes: [],
    visibility: 'public',
  },
  {
    content: 'Real-time WebSocket messaging is live! Experience seamless social networking on the blockchain.',
    mediaHashes: [],
    visibility: 'public',
  },
  {
    content: 'ActivityPub federation now enabled - connect with Mastodon, Pixelfed, and the broader Fediverse!',
    mediaHashes: [],
    visibility: 'public',
  },
  {
    content: 'IPFS integration for decentralized media storage. Your content, your rules. 🔐',
    mediaHashes: [],
    visibility: 'public',
  },
  {
    content: 'Production deployment complete! Thanks to the entire team for making this happen. 🎉',
    mediaHashes: [],
    visibility: 'public',
  },
];

// ============================================
// Utility Functions
// ============================================

export function generateDemoCSV(): string {
  const headers = ['first_name', 'last_name', 'email', 'phone', 'company', 'job_title'];
  const rows = DEMO_CONTACTS.map(c => [
    c.firstName,
    c.lastName,
    c.email,
    c.phone,
    c.company,
    c.jobTitle,
  ]);

  const csvContent = [
    headers.join(','),
    ...rows.map(row => row.map(cell => `"${cell}"`).join(',')),
  ].join('\n');

  return csvContent;
}

export function getRandomContact() {
  return DEMO_CONTACTS[Math.floor(Math.random() * DEMO_CONTACTS.length)];
}

export function getRandomCampaign() {
  return DEMO_CAMPAIGNS[Math.floor(Math.random() * DEMO_CAMPAIGNS.length)];
}

export function getRandomTemplate() {
  return DEMO_EMAIL_TEMPLATES[Math.floor(Math.random() * DEMO_EMAIL_TEMPLATES.length)];
}

export function getRandomPost() {
  return DEMO_SOCIAL_POSTS[Math.floor(Math.random() * DEMO_SOCIAL_POSTS.length)];
}

// Generate a sample contact with all fields for testing
export function generateTestContact(overrides?: Partial<typeof DEMO_CONTACTS[0]>) {
  const base = getRandomContact();
  return {
    ...base,
    ...overrides,
  };
}

// Generate multiple test contacts
export function generateTestContacts(count: number) {
  const result = [];
  for (let i = 0; i < count; i++) {
    result.push({
      ...getRandomContact(),
      email: `test-${i}@example.com`, // Make emails unique
    });
  }
  return result;
}

// Generate test campaign with random contacts
export function generateTestCampaign(targetCount: number = 5) {
  return {
    ...getRandomCampaign(),
    targetContacts: targetCount,
  };
}

// Simulate lead scores for demo contacts
export function generateDemoLeadScores() {
  return DEMO_CONTACTS.map((contact, idx) => ({
    contactId: contact.email,
    score: Math.floor(Math.random() * 100),
    grade: ['A', 'B', 'C', 'D', 'F'][Math.floor(Math.random() * 5)],
    metrics: {
      engagement: Math.floor(Math.random() * 10),
      company_size: Math.floor(Math.random() * 5),
      email_interactions: Math.floor(Math.random() * 20),
    },
  }));
}

// Generate pipeline analytics for demo
export function generateDemoPipelineAnalytics() {
  return {
    total_value: 1250000,
    total_deals: 12,
    stage_breakdown: {
      prospect: { count: 4, value: 150000 },
      qualified: { count: 5, value: 450000 },
      demo: { count: 2, value: 300000 },
      proposal: { count: 1, value: 350000 },
    },
    forecast_6_months: {
      month_1: 85000,
      month_2: 120000,
      month_3: 180000,
      month_4: 250000,
      month_5: 320000,
      month_6: 400000,
    },
    win_probability_average: 0.42,
  };
}

// Generate duplicate contact pairs for testing deduplication
export function generateDemoDuplicates() {
  const alice = DEMO_CONTACTS[0];
  const alice2 = { ...alice, email: 'alice.j@techcorp.com' }; // Slight variation

  return [
    {
      id1: alice.email,
      id2: alice2.email,
      similarity_score: 0.92,
      reason: 'Same first/last name, similar email domain',
    },
    {
      id1: DEMO_CONTACTS[1].email,
      id2: DEMO_CONTACTS[5].email,
      similarity_score: 0.45,
      reason: 'Manual match required - verify before merge',
    },
  ];
}
