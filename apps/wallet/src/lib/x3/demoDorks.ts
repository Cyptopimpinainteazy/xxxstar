export interface GoogleDorksQuery {
  id: string;
  name: string;
  category: string;
  query: string;
  description?: string;
  tags?: string[];
}

export interface GoogleDorksResult {
  title: string;
  url: string;
  snippet: string;
  domain: string;
  email?: string;
  phone?: string;
  type: 'investor' | 'grant' | 'accelerator' | 'founder' | 'competitor';
  relevanceScore: number;
  saved: boolean;
}

export interface InvestorMatch {
  investorId: string;
  matchScore: number;
  sectorAlignment: number;
  stageAlignment: number;
  ticketSizeAlignment: number;
  locationAlignment: number;
  contactProbability: number;
}

// Demo Dorks Queries for AI Sector
export const DEMO_DORKS_QUERIES: GoogleDorksQuery[] = [
  {
    id: 'ai-investors-1',
    name: 'AI Investor Profiles',
    category: 'investor',
    query: 'site:crunchbase.com "artificial intelligence" investor "seed funding"',
    description: 'Find AI-focused investors on Crunchbase',
    tags: ['AI', 'investor', 'crunchbase']
  },
  {
    id: 'ai-investors-2', 
    name: 'AI Angel Investors',
    category: 'investor',
    query: 'site:angel.co "artificial intelligence" angel investor',
    description: 'Find AI angel investors on AngelList',
    tags: ['AI', 'angel', 'angellist']
  },
  {
    id: 'ai-investors-3',
    name: 'AI VC Firms',
    category: 'investor', 
    query: 'site:linkedin.com "artificial intelligence" "venture capital" partner',
    description: 'Find AI-focused VC partners on LinkedIn',
    tags: ['AI', 'vc', 'linkedin']
  },
  {
    id: 'ai-investors-4',
    name: 'AI Corporate VCs',
    category: 'investor',
    query: 'site:techcrunch.com "artificial intelligence" "corporate venture" funding',
    description: 'Find corporate venture arms investing in AI',
    tags: ['AI', 'corporate', 'techcrunch']
  },
  {
    id: 'ai-investors-5',
    name: 'AI Accelerator Programs',
    category: 'accelerator',
    query: 'site:accelerator.com "artificial intelligence" startup program',
    description: 'Find AI accelerator programs',
    tags: ['AI', 'accelerator', 'startup']
  }
];

// Demo Investor Results
export const DEMO_INVESTORS: GoogleDorksResult[] = [
  {
    title: 'Andreessen Horowitz - AI Investments',
    url: 'https://a16z.com/portfolio/artificial-intelligence/',
    snippet: 'Leading AI investments in companies like OpenAI, Anthropic, and Stability AI',
    domain: 'a16z.com',
    email: 'contact@a16z.com',
    phone: '+1-415-670-2000',
    type: 'investor',
    relevanceScore: 95,
    saved: false
  },
  {
    title: 'Sequoia Capital - AI Portfolio',
    url: 'https://www.sequoiacap.com/companies/?industry=artificial-intelligence',
    snippet: 'Investments in AI companies including Anthropic, OpenAI, and Scale AI',
    domain: 'sequoiacap.com',
    email: 'info@sequoiacap.com',
    phone: '+1-415-200-5600',
    type: 'investor',
    relevanceScore: 92,
    saved: false
  },
  {
    title: 'Accel - AI & ML Investments',
    url: 'https://www.accel.com/industries/artificial-intelligence',
    snippet: 'Focus on AI infrastructure, applications, and ML platforms',
    domain: 'accel.com',
    email: 'contact@accel.com',
    phone: '+1-650-859-2600',
    type: 'investor',
    relevanceScore: 88,
    saved: false
  },
  {
    title: 'Lightspeed Venture Partners - AI',
    url: 'https://lsvp.com/portfolio/?industry=artificial-intelligence',
    snippet: 'Early-stage AI investments in infrastructure and applications',
    domain: 'lsvp.com',
    email: 'info@lsvp.com',
    phone: '+1-650-213-5100',
    type: 'investor',
    relevanceScore: 85,
    saved: false
  },
  {
    title: 'Index Ventures - AI Portfolio',
    url: 'https://www.indexventures.com/companies/?industry=artificial-intelligence',
    snippet: 'European and US AI investments including DeepMind and Graphcore',
    domain: 'indexventures.com',
    email: 'contact@indexventures.com',
    phone: '+44-20-7290-1100',
    type: 'investor',
    relevanceScore: 83,
    saved: false
  }
];

// Demo Grant Opportunities
export const DEMO_GRANTS: GoogleDorksResult[] = [
  {
    title: 'NSF SBIR Phase I - AI Research',
    url: 'https://www.nsf.gov/funding/programs.jsp?porg=NSF&org=SBIR',
    snippet: 'Up to $256,000 for AI research and development',
    domain: 'nsf.gov',
    email: 'sbir@nsf.gov',
    phone: '+1-703-292-8240',
    type: 'grant',
    relevanceScore: 90,
    saved: false
  },
  {
    title: 'DARPA AI Research Grants',
    url: 'https://www.darpa.mil/funding/opportunities',
    snippet: 'Defense Advanced Research Projects Agency AI funding opportunities',
    domain: 'darpa.mil',
    email: 'grants@darpa.mil',
    phone: '+1-703-696-7000',
    type: 'grant',
    relevanceScore: 88,
    saved: false
  },
  {
    title: 'Google AI Research Grants',
    url: 'https://ai.google/research-outreach/',
    snippet: 'Funding for AI research projects and PhD fellowships',
    domain: 'ai.google',
    email: 'research@ai.google',
    phone: '+1-650-253-0000',
    type: 'grant',
    relevanceScore: 85,
    saved: false
  },
  {
    title: 'Microsoft AI for Good Grants',
    url: 'https://www.microsoft.com/en-us/ai/ai-for-good',
    snippet: 'Grants for AI projects addressing social and environmental challenges',
    domain: 'microsoft.com',
    email: 'aiforgood@microsoft.com',
    phone: '+1-425-882-8080',
    type: 'grant',
    relevanceScore: 82,
    saved: false
  },
  {
    title: 'EIC Accelerator - AI Startups',
    url: 'https://eic.europa.eu/programmes/eic-accelerator_en',
    snippet: 'Up to €2.5M for innovative AI startups in Europe',
    domain: 'europa.eu',
    email: 'eic@europa.eu',
    phone: '+32-2-299-9555',
    type: 'grant',
    relevanceScore: 80,
    saved: false
  }
];

// Demo Investor Matches
export const DEMO_INVESTOR_MATCHES: InvestorMatch[] = [
  {
    investorId: 'a16z-ai',
    matchScore: 95,
    sectorAlignment: 0.95,
    stageAlignment: 0.90,
    ticketSizeAlignment: 0.85,
    locationAlignment: 0.80,
    contactProbability: 0.75
  },
  {
    investorId: 'sequoia-ai',
    matchScore: 92,
    sectorAlignment: 0.92,
    stageAlignment: 0.88,
    ticketSizeAlignment: 0.90,
    locationAlignment: 0.85,
    contactProbability: 0.70
  },
  {
    investorId: 'accel-ai',
    matchScore: 88,
    sectorAlignment: 0.88,
    stageAlignment: 0.85,
    ticketSizeAlignment: 0.80,
    locationAlignment: 0.90,
    contactProbability: 0.65
  },
  {
    investorId: 'lsvp-ai',
    matchScore: 85,
    sectorAlignment: 0.85,
    stageAlignment: 0.82,
    ticketSizeAlignment: 0.88,
    locationAlignment: 0.75,
    contactProbability: 0.60
  },
  {
    investorId: 'index-ai',
    matchScore: 83,
    sectorAlignment: 0.83,
    stageAlignment: 0.80,
    ticketSizeAlignment: 0.75,
    locationAlignment: 0.95,
    contactProbability: 0.55
  }
];

// Demo Funding Analytics
export const DEMO_FUNDING_ANALYTICS = {
  totalTargetUsd: 1000000,
  totalRaisedUsd: 250000,
  fundingGapUsd: 750000,
  fromVcUsd: 150000,
  fromAngelUsd: 50000,
  fromGrantsUsd: 30000,
  fromCorporateUsd: 20000,
  investorsInPipeline: 15,
  investorsInterested: 8,
  investorsCommitted: 3,
  successProbabilityPercentage: 65,
  monthsToClose: 6,
  estimatedCloseDate: new Date('2025-06-01')
};