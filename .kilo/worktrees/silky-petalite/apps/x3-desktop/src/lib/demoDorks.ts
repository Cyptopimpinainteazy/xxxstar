// Demo Data: Google Dorks Searches, Investors & Grants
// Pre-configured searches for common scenarios

export const DEMO_DORKS_QUERIES = [
  // ============================================
  // INVESTOR DISCOVERY QUERIES
  // ============================================
  {
    name: "Seed-Stage VCs - AI & ML",
    category: "investor_emails",
    query: `site:crunchbase.com "seed" OR "seed stage" investor "AI" OR "machine learning" -site:news`,
    description: "Find venture capital firms investing seed capital in AI/ML startups",
    tags: ["seed", "AI", "investors"],
  },
  {
    name: "Angel Investors - Web3",
    category: "investor_emails",
    query: `site:angel.co "invested in" "web3" OR "blockchain" OR "defi" "area of expertise"`,
    description: "Find angel investors with Web3/blockchain experience",
    tags: ["angel", "web3", "investors"],
  },
  {
    name: "Series A VCs - ClimaTech",
    category: "investor_emails",
    query: `site:crunchbase.com "series a" invested "climate" OR "sustainability" OR "green" 2025 2026`,
    description: "Find Series A focused VCs in climate/green tech space",
    tags: ["series_a", "climate", "investors"],
  },
  {
    name: "Corporate Venture Investors",
    category: "investor_emails",
    query: `site:linkedin.com "corporate venture" OR "CVC" investor location:"San Francisco" OR "Silicon Valley"`,
    description: "Find corporate venture capital teams on LinkedIn",
    tags: ["corporate", "cvc", "investors"],
  },

  // ============================================
  // GRANT DISCOVERY QUERIES
  // ============================================
  {
    name: "Federal SBIR Grants - Clean Energy",
    category: "grants",
    query: `site:sbir.gov "clean energy" OR "renewable" OR "carbon" phase 1 OR phase 2 2026`,
    description: "Find SBIR/STTR federal grants in clean energy sector",
    tags: ["sbir", "federal", "clean_energy"],
  },
  {
    name: "NSF Research Grants - Computer Science",
    category: "grants",
    query: `site:nsf.gov "grant program" "computer science" OR "artificial intelligence" application deadline 2026`,
    description: "NSF research grants for CS and AI initiatives",
    tags: ["nsf", "research", "ai"],
  },
  {
    name: "Foundation Grants - Social Impact",
    category: "grants",
    query: `site:grantstation.com OR site:philanthropy.org "social impact" OR "education" grant "$" deadline 2026`,
    description: "Foundation grants for social impact and education",
    tags: ["foundation", "social_impact"],
  },
  {
    name: "Government Grants.gov - All Tech",
    category: "grants",
    query: `site:grants.gov "technology" OR "innovation" grant application deadline 2026 2027`,
    description: "Browse all government grants related to technology",
    tags: ["government", "technology"],
  },

  // ============================================
  // ACCELERATOR & COMPETITION PROGRAMS
  // ============================================
  {
    name: "Y Combinator Cohorts - Current",
    category: "accelerators",
    query: `site:ycombinator.com "apply" OR "demo day" OR "batch" 2026`,
    description: "Y Combinator application info and upcoming batches",
    tags: ["yc", "accelerator"],
  },
  {
    name: "Techstars Programs - Global",
    category: "accelerators",
    query: `site:techstars.com "apply" OR "program" location worldwide founding team`,
    description: "Techstars accelerator programs worldwide",
    tags: ["techstars", "accelerator"],
  },
  {
    name: "University Startup Grants",
    category: "accelerators",
    query: `site:.edu startup grant OR accelerator fund OR entrepreneurship ($"' OR "million"`,
    description: "University-based startup grants and accelerator programs",
    tags: ["university", "startup_grants"],
  },

  // ============================================
  // FOUNDER & MARKET INTELLIGENCE
  // ============================================
  {
    name: "Founder Profiles - AI/ML",
    category: "founder_profiles",
    query: `site:linkedin.com "founder" OR "CEO" "AI" OR "machine learning" -"this account"`,
    description: "Find AI/ML founders on LinkedIn",
    tags: ["founders", "ai"],
  },
  {
    name: "Recent Funding Announcements",
    category: "competitor_tech",
    query: `site:techcrunch.com OR site:venturebeat.com "raises" OR "funding round" 2026 "$" million`,
    description: "Recent startup funding announcements",
    tags: ["funding_news", "market_intel"],
  },
  {
    name: "Unicorn Exits - Last Year",
    category: "competitor_tech",
    query: `site:crunchbase.com "unicorn" ("exit" OR "acquired for" OR "IPO") 2025 2026`,
    description: "Track recent exits and IPOs of unicorn companies",
    tags: ["exits", "ipo"],
  },
];

export const DEMO_INVESTORS = [
  {
    name: "Sequoia Capital",
    firmName: "Sequoia Capital",
    investorType: "venture_capital",
    email: "info@sequoiacap.com",
    website: "https://www.sequoiacap.com",
    location: "Menlo Park, CA",
    focusSectors: ["AI", "Enterprise Software", "FinTech"],
    stagePreference: "seed_to_series_c",
    ticketSizeMin: 500000,
    ticketSizeMax: 150000000,
    yearsInvesting: 50,
    linkedinUrl: "https://www.linkedin.com/company/sequoia-capital",
    twitterHandle: "@sequoia",
    rating: "hot",
  },
  {
    name: "Y Combinator",
    firmName: "Y Combinator",
    investorType: "accelerator",
    email: "partners@ycombinator.com",
    website: "https://www.ycombinator.com",
    location: "San Francisco, CA",
    focusSectors: ["All Sectors"],
    stagePreference: "seed",
    ticketSizeMin: 125000,
    ticketSizeMax: 125000,
    yearsInvesting: 20,
    linkedinUrl: "https://www.linkedin.com/company/y-combinator",
    twitterHandle: "@ycombinator",
    rating: "hot",
  },
  {
    name: "Andreessen Horowitz (a16z)",
    firmName: "Andreessen Horowitz",
    investorType: "venture_capital",
    email: "contact@a16z.com",
    website: "https://a16z.com",
    location: "Menlo Park, CA",
    focusSectors: ["Fintech", "Crypto", "AI"],
    stagePreference: "series_a_plus",
    ticketSizeMin: 10000000,
    ticketSizeMax: 500000000,
    yearsInvesting: 15,
    linkedinUrl: "https://www.linkedin.com/company/andreessen-horowitz",
    twitterHandle: "@a16z",
    rating: "hot",
  },
  {
    name: "500 Global",
    firmName: "500 Global",
    investorType: "accelerator",
    email: "applications@500.co",
    website: "https://www.500.co",
    location: "San Francisco, CA",
    focusSectors: ["All Sectors", "Emerging Markets"],
    stagePreference: "seed",
    ticketSizeMin: 50000,
    ticketSizeMax: 250000,
    yearsInvesting: 12,
    linkedinUrl: "https://www.linkedin.com/company/500-global",
    twitterHandle: "@500global",
    rating: "warm",
  },
  {
    name: "Khosla Ventures",
    firmName: "Khosla Ventures",
    investorType: "venture_capital",
    email: "pitch@khoslaventures.com",
    website: "https://www.khoslaventures.com",
    location: "Menlo Park, CA",
    focusSectors: ["Climate Tech", "Energy", "AI"],
    stagePreference: "all",
    ticketSizeMin: 1000000,
    ticketSizeMax: 200000000,
    yearsInvesting: 20,
    linkedinUrl: "https://www.linkedin.com/company/khosla-ventures",
    twitterHandle: "@khosla",
    rating: "warm",
  },
];

export const DEMO_GRANTS = [
  {
    name: "$250K NSF Small Business Phase I Award",
    provider: "National Science Foundation (NSF)",
    amount: 250000,
    currency: "USD",
    deadline: "2026-04-15",
    eligibility: "For-profit US companies, innovative R&D projects",
    focusAreas: ["AI", "Computer Science", "Biotechnology"],
    locationFocus: "USA",
    url: "https://sbir.nsf.gov/funding-opportunities/sbir-phase-i",
    status: "open",
    tags: ["federal", "sbir", "r&d"],
  },
  {
    name: "$500K SBIR Phase II - Clean Energy",
    provider: "Department of Energy (DOE)",
    amount: 500000,
    currency: "USD",
    deadline: "2026-05-30",
    eligibility: "Small businesses working on clean energy innovation",
    focusAreas: ["Clean Energy", "Green Tech", "Sustainability"],
    locationFocus: "USA",
    url: "https://www.energy.gov/sbir",
    status: "open",
    tags: ["federal", "sbir", "clean_energy"],
  },
  {
    name: "$100K Macro Grant - Climate Innovation",
    provider: "Gates Foundation / Macro Foundation Partners",
    amount: 100000,
    currency: "USD",
    deadline: "2026-03-31",
    eligibility: "Teams working on climate solutions, developing world focus",
    focusAreas: ["Climate", "Agriculture", "Energy"],
    locationFocus: "Global",
    url: "https://www.macrofoundation.org/grants",
    status: "open",
    tags: ["foundation", "climate", "global"],
  },
  {
    name: "Google.org AI for Social Good Grant",
    provider: "Google.org",
    amount: 500000,
    currency: "USD",
    deadline: "2026-06-30",
    eligibility: "Non-profits and social enterprises using AI for impact",
    focusAreas: ["AI", "Social Impact", "Education"],
    locationFocus: "Global",
    url: "https://www.google.org/our-work/ai-for-social-good",
    status: "open",
    tags: ["corporate", "ai", "social_impact"],
  },
  {
    name: "$50K Stripe Climate Grant",
    provider: "Stripe Climate",
    amount: 50000,
    currency: "USD",
    deadline: "2026-12-31",
    eligibility: "Climate tech companies removing carbon from atmosphere",
    focusAreas: ["Climate", "Carbon Removal", "Sustainability"],
    locationFocus: "North America",
    url: "https://climate.stripe.com",
    status: "open",
    tags: ["corporate", "climate", "carbon"],
  },
];

export const DEMO_DORKS_SEARCH_CAMPAIGNS = [
  {
    name: "Fundraise Campaign - Seed Round",
    objective: "find_investors",
    targetKeywords: ["AI", "SaaS", "Series A"],
    locationFocus: "San Francisco Bay Area",
    sectorFocus: ["AI", "Enterprise Software"],
    queriesCount: 12,
    resultsFound: 342,
    contactsCreated: 28,
    status: "active",
  },
  {
    name: "Grant Discovery - ClimaTech",
    objective: "find_grants",
    targetKeywords: ["Climate", "Clean Energy", "Renewable"],
    locationFocus: "USA",
    sectorFocus: ["Climate Tech", "Energy"],
    queriesCount: 8,
    resultsFound: 156,
    contactsCreated: 12,
    status: "active",
  },
  {
    name: "Competitor Analysis - Web3",
    objective: "market_research",
    targetKeywords: ["DeFi", "Blockchain", "Crypto"],
    locationFocus: null,
    sectorFocus: ["Crypto", "DeFi"],
    queriesCount: 15,
    resultsFound: 487,
    contactsCreated: 0,
    status: "completed",
  },
  {
    name: "Partnership Opportunities",
    objective: "partnership_search",
    targetKeywords: ["Integration", "API", "Partnership"],
    locationFocus: "Global",
    sectorFocus: ["SaaS", "B2B", "Enterprise"],
    queriesCount: 9,
    resultsFound: 234,
    contactsCreated: 15,
    status: "active",
  },
];

// Investor Profile Matching for Demo
export const DEMO_INVESTOR_MATCHES = [
  {
    investorId: "inv_seq_001",
    companyName: "Example AI Startup",
    matchScore: 94,
    reason: "Perfect alignment on AI sector, seed to series C stage preference, B2B SaaS focus",
    sectorAlignment: 0.98,
    stageAlignment: 0.92,
    ticketSizeAlignment: 0.96,
    locationAlignment: 0.88,
    contactProbability: 0.87,
  },
  {
    investorId: "inv_yc_001",
    companyName: "Example AI Startup",
    matchScore: 88,
    reason: "Great seed-stage fit, no sector restrictions, high success rate for similar companies",
    sectorAlignment: 1.0,
    stageAlignment: 0.95,
    ticketSizeAlignment: 0.78,
    locationAlignment: 0.92,
    contactProbability: 0.82,
  },
  {
    investorId: "inv_a16z_001",
    companyName: "Example AI Startup",
    matchScore: 72,
    reason: "Strong AI focus but typically invests series A+, larger check sizes preferred",
    sectorAlignment: 0.95,
    stageAlignment: 0.55,
    ticketSizeAlignment: 0.62,
    locationAlignment: 0.90,
    contactProbability: 0.45,
  },
];

// Search Results Template
export const DEMO_SEARCH_RESULTS = [
  {
    title: "Sequoia Capital - AI Investments",
    url: "https://www.sequoiacap.com/ai-fund",
    snippet: "Sequoia Capital's dedicated AI investment team, investing in seed through Series C AI companies...",
    domain: "sequoiacap.com",
    email: "ai-team@sequoiacap.com",
    phone: "+1-650-555-0123",
    type: "investor",
    relevanceScore: 96,
  },
  {
    title: "Y Combinator - Apply Now",
    url: "https://www.ycombinator.com/apply",
    snippet: "Apply to Y Combinator and get $500K in seed funding plus mentorship from top founders...",
    domain: "ycombinator.com",
    email: "partners@ycombinator.com",
    phone: "+1-415-555-0456",
    type: "accelerator",
    relevanceScore: 94,
  },
  {
    title: "NSF SBIR Phase I Grant Program",
    url: "https://sbir.nsf.gov/funding-opportunities",
    snippet: "National Science Foundation Small Business Innovation Research - $250K for Phase I projects...",
    domain: "sbir.nsf.gov",
    email: "sbir@nsf.gov",
    phone: "+1-703-555-0789",
    type: "grant",
    relevanceScore: 88,
  },
];

// Helper to get all demo data
export function getAllDorksDemoData() {
  return {
    queries: DEMO_DORKS_QUERIES,
    investors: DEMO_INVESTORS,
    grants: DEMO_GRANTS,
    campaigns: DEMO_DORKS_SEARCH_CAMPAIGNS,
    investorMatches: DEMO_INVESTOR_MATCHES,
    searchResults: DEMO_SEARCH_RESULTS,
  };
}

// Search by category helper
export function getDorkQueriesByCategory(category: string) {
  return DEMO_DORKS_QUERIES.filter(q => q.category === category);
}

// Find matching investors for company profile
export function findMatchingInvestors(
  companySectors: string[],
  seekingAmount: number,
  stage: string
) {
  return DEMO_INVESTOR_MATCHES.filter(match => {
    // Simple matching logic - in production would use ML
    return match.matchScore > 70;
  });
}

// Get recommended search strategy
export function getRecommendedSearchStrategy(objective: string) {
  const strategies: Record<string, string[]> = {
    find_investors: [
      "Search by sector + stage preference",
      "Find angels on AngelList",
      "Search VCs on Crunchbase",
      "LinkedIn investor profiles",
      "Industry-specific funds",
    ],
    find_grants: [
      "Federal grants (Grants.gov, SBIR, NSF)",
      "Foundation grants (GrantStation, FC)",
      "Corporate grants programs",
      "Award competitions",
      "University partnerships",
    ],
    market_research: [
      "Competitor funding rounds",
      "Recent exits in sector",
      "Hiring trends",
      "Customer signals",
      "Market concentration",
    ],
  };

  return strategies[objective] || [];
}
