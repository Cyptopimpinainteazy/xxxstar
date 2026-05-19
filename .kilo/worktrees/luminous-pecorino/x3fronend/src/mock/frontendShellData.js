export const shellRouteGroups = [
  {
    id: 'core',
    label: 'Core Surfaces',
    summary: 'Safe shells that can absorb stable contracts later without needing live chain access now.',
    routes: [
      {
        id: 'wallet-home',
        label: 'Wallet Home',
        stage: 'shell-only',
        description: 'Portfolio frame, signing boundary copy, balance narratives, and alert choreography.',
        readiness: 'Design-safe now',
        blockedBy: ['Live wallet signing contracts', 'Custody ownership freeze'],
        metrics: [
          { label: 'Draft modules', value: '4' },
          { label: 'Contract coupling', value: 'None' },
          { label: 'Fixture scope', value: 'User-only' }
        ],
        screens: [
          {
            id: 'overview',
            label: 'Overview',
            kicker: 'Account frame',
            headline: 'Show balances and trust boundaries before asking users to sign anything.',
            summary:
              'This view tests how much chain posture, warnings, and account context fit above the fold before the first transactional prompt.',
            cards: [
              { label: 'Portfolio spread', value: '$184.2k', tone: 'warm', detail: 'Mock valuation over four custody classes.' },
              { label: 'Available liquidity', value: '$37.6k', tone: 'neutral', detail: 'Immediate user-controlled balance only.' },
              { label: 'Signing routes', value: '2 / 5', tone: 'alert', detail: 'Only user-signed paths should appear here.' }
            ],
            modules: [
              { name: 'Balance shelf', status: 'ready for shell', detail: 'Token grouping, balance ladder, and fiat framing.' },
              { name: 'Trust banner', status: 'ready for shell', detail: 'Explains what the wallet will never sign on behalf of backend operators.' },
              { name: 'Action rail', status: 'blocked', detail: 'Real send, swap, and bridge actions stay disabled until signer ownership freezes.' }
            ]
          },
          {
            id: 'activity',
            label: 'Activity',
            kicker: 'History shell',
            headline: 'Design the cadence of an activity ledger before event truth is stable.',
            summary:
              'This tests density, status language, and empty-state choreography using local fixture events only.',
            cards: [
              { label: 'Recent items', value: '18', tone: 'neutral', detail: 'Mock events across send, stake, and review states.' },
              { label: 'Status families', value: '6', tone: 'cool', detail: 'Pending, signed, observed, delayed, escalated, closed.' },
              { label: 'Needs model freeze', value: 'Yes', tone: 'alert', detail: 'Final event taxonomies still depend on indexer freeze.' }
            ],
            modules: [
              { name: 'Timeline cards', status: 'ready for shell', detail: 'Visual rhythm and copy can be tuned now.' },
              { name: 'Filters', status: 'ready for shell', detail: 'By asset, by route, by risk.' },
              { name: 'Deep links', status: 'blocked', detail: 'Cannot bind to explorer detail pages yet.' }
            ]
          },
          {
            id: 'risk',
            label: 'Risk & trust',
            kicker: 'Boundary education',
            headline: 'Put signing, pause, and custody boundaries in plain language.',
            summary:
              'This screen exists to pressure-test the copy that separates user keys from backend-controlled protocol keys.',
            cards: [
              { label: 'User-signed actions', value: 'Send / approve', tone: 'cool', detail: 'Only user-controlled keys belong here.' },
              { label: 'Backend-owned actions', value: 'Relayer / treasury', tone: 'warm', detail: 'Must route through custody-service.' },
              { label: 'Misleading copy risk', value: 'High', tone: 'alert', detail: 'Reason durable wallet UX still waits.' }
            ],
            modules: [
              { name: 'Boundary glossary', status: 'ready for shell', detail: 'Lets product and security align language early.' },
              { name: 'Incident states', status: 'shell-only', detail: 'Mock pause and custody outage messaging.' },
              { name: 'Recovery flows', status: 'blocked', detail: 'Need confirmed backend fallbacks and ownership.' }
            ]
          }
        ],
        scenarios: [
          { id: 'calm', label: 'Calm market', state: 'safe now', description: 'Default healthy account posture with no action pressure.' },
          { id: 'stressed', label: 'Signer uncertainty', state: 'copy test', description: 'Stress-tests warnings around provisional signing semantics.' },
          { id: 'paused', label: 'Protocol pause', state: 'education', description: 'Explains what the wallet can still show when protocol actions are blocked.' }
        ],
        journey: [
          { title: 'Enter account', state: 'ready', detail: 'Identity and balances are visible without backend mutation.' },
          { title: 'Evaluate trust banner', state: 'ready', detail: 'Boundary copy can be refined immediately.' },
          { title: 'Attempt action', state: 'blocked', detail: 'Real mutations remain disabled until signing freezes.' }
        ]
      },
      {
        id: 'network-overview',
        label: 'Network Overview',
        stage: 'shell-only',
        description: 'Public-facing chain posture, release language, and module scorecard composition.',
        readiness: 'Design-safe now',
        blockedBy: ['Frozen RPC and sidecar contract set'],
        metrics: [
          { label: 'Draft modules', value: '4' },
          { label: 'Contract coupling', value: 'Low' },
          { label: 'Fixture scope', value: 'Network-wide' }
        ],
        screens: [
          {
            id: 'command',
            label: 'Command deck',
            kicker: 'Public posture',
            headline: 'Let visitors understand the chain narrative before exposing raw protocol state.',
            summary:
              'This tests the hierarchy between readiness, proof systems, validator signals, and chain identity.',
            cards: [
              { label: 'Public readiness rail', value: '5 modules', tone: 'cool', detail: 'Consensus, bridge, wallet, ops, launch.' },
              { label: 'Narrative confidence', value: '72%', tone: 'warm', detail: 'How much copy can stay honest without going technical.' },
              { label: 'Contract dependency', value: 'RPC pack', tone: 'neutral', detail: 'Needs frozen read models, not write flows.' }
            ],
            modules: [
              { name: 'Hero and readiness rail', status: 'ready for shell', detail: 'Works entirely on fixture language.' },
              { name: 'Module scorecards', status: 'shell-only', detail: 'Can later bind to LaunchOps or sidecar summaries.' },
              { name: 'Live telemetry', status: 'blocked', detail: 'Needs explicit consumer contract and ownership.' }
            ]
          },
          {
            id: 'modules',
            label: 'Module scorecards',
            kicker: 'Subsystem map',
            headline: 'Test how much operational truth a public dashboard can carry without collapsing into noise.',
            summary:
              'This screen pressures card density, status legend clarity, and how technical to make module names for different audiences.',
            cards: [
              { label: 'Subsystem cards', value: '8', tone: 'neutral', detail: 'Mocked cards for bridge, wallet, verifier, and GPU paths.' },
              { label: 'Status legend', value: '4 states', tone: 'cool', detail: 'Ready, partial, blocked, downstream.' },
              { label: 'Live data need', value: 'Deferred', tone: 'alert', detail: 'Stable query pack still pending.' }
            ],
            modules: [
              { name: 'Status legend', status: 'ready for shell', detail: 'No chain bindings required.' },
              { name: 'Readiness meter', status: 'ready for shell', detail: 'Can later bind to LaunchOps or sidecar summary.' },
              { name: 'Validator pulse', status: 'blocked', detail: 'Needs explicit consumer contract and ownership.' }
            ]
          },
          {
            id: 'cta',
            label: 'Action band',
            kicker: 'Public next step',
            headline: 'Design the call-to-action layer without committing to a backend promise surface.',
            summary:
              'This view isolates launch-oriented CTAs, role-specific onboarding, and risk language for testnet versus mainnet visitors.',
            cards: [
              { label: 'CTA lanes', value: '3', tone: 'warm', detail: 'Builders, validators, operators.' },
              { label: 'Copy variants', value: '9', tone: 'neutral', detail: 'Role-specific language paths.' },
              { label: 'Backend risk', value: 'Low', tone: 'cool', detail: 'Mostly information architecture.' }
            ],
            modules: [
              { name: 'Role selector', status: 'ready for shell', detail: 'Pure IA problem.' },
              { name: 'Eligibility copy', status: 'shell-only', detail: 'Can be tuned before live program state exists.' },
              { name: 'Live forms', status: 'blocked', detail: 'Need confirmed APIs and operational flow.' }
            ]
          }
        ],
        scenarios: [
          { id: 'public', label: 'Public visitor', state: 'narrative', description: 'Focuses on clarity and credibility over protocol detail.' },
          { id: 'operator', label: 'Operator lens', state: 'technical', description: 'Pushes denser module labeling and more specific readiness copy.' },
          { id: 'launch', label: 'Launch week', state: 'event mode', description: 'Stresses announcement rails and urgency placement.' }
        ],
        journey: [
          { title: 'Read chain posture', state: 'ready', detail: 'Narrative and hierarchy can be tuned now.' },
          { title: 'Inspect subsystems', state: 'ready', detail: 'Module cards can stay fixture-backed.' },
          { title: 'Query live state', state: 'blocked', detail: 'Needs finalized read contracts.' }
        ]
      }
    ]
  },
  {
    id: 'deferred',
    label: 'Deferred Until Freeze',
    summary: 'High-risk flows remain visible for planning, but stay mock-only until the backend contracts stop moving.',
    routes: [
      {
        id: 'bridge-status',
        label: 'Bridge Status',
        stage: 'blocked',
        description: 'Deposit, refund, timeout, and settlement ladder for cross-chain sessions.',
        readiness: 'Blocked',
        blockedBy: ['Bridge and relayer lifecycle freeze', 'Indexer event model freeze'],
        metrics: [
          { label: 'Draft modules', value: '5' },
          { label: 'Contract coupling', value: 'High' },
          { label: 'Fixture scope', value: 'Bridge journey' }
        ],
        screens: [
          {
            id: 'session-board',
            label: 'Session board',
            kicker: 'Bridge queue',
            headline: 'Test how much lifecycle detail fits before the board becomes an operator console.',
            summary:
              'This shell compares public user status against the denser operator context that the real sidecar will eventually need to provide.',
            cards: [
              { label: 'Mock sessions', value: '14', tone: 'neutral', detail: 'Across pending, proving, timed out, and refunded.' },
              { label: 'Lifecycle rungs', value: '7', tone: 'warm', detail: 'From source lock to settlement or refund.' },
              { label: 'Truth source', value: 'Unfrozen', tone: 'alert', detail: 'Still blocked on relayer and event freeze.' }
            ],
            modules: [
              { name: 'Session list', status: 'shell-only', detail: 'IA and status vocabulary can be refined now.' },
              { name: 'Source/target chips', status: 'ready for shell', detail: 'Good place to settle chain icon and naming patterns.' },
              { name: 'Real progress state', status: 'blocked', detail: 'Needs authoritative relayer and settlement lifecycle.' }
            ]
          },
          {
            id: 'lifecycle',
            label: 'Lifecycle ladder',
            kicker: 'State machine',
            headline: 'Model every bridge step visually before the backend chooses the final event families.',
            summary:
              'This tests the legibility of timeouts, duplicate rejection, pause, and refund outcomes without claiming any final state names yet.',
            cards: [
              { label: 'Visible transitions', value: '9', tone: 'cool', detail: 'Designed to compress into a mobile timeline.' },
              { label: 'Refund branches', value: '2', tone: 'warm', detail: 'Timeout and rejection flows remain mock-only.' },
              { label: 'Semantic stability', value: 'Low', tone: 'alert', detail: 'Final lifecycle names still belong to backend.' }
            ],
            modules: [
              { name: 'Step ladder', status: 'shell-only', detail: 'Pure IA and motion testing.' },
              { name: 'Failure states', status: 'shell-only', detail: 'Copy stress-test for refunds and disputes.' },
              { name: 'Progress polling', status: 'blocked', detail: 'Needs sidecar and indexer contracts.' }
            ]
          },
          {
            id: 'incident',
            label: 'Pause and incident',
            kicker: 'Exceptional state',
            headline: 'Make pause, degraded relayer health, and operator intervention legible before implementation.',
            summary:
              'This shell exists to test warning choreography and who sees which level of detail when bridge operations are degraded.',
            cards: [
              { label: 'Incident tiers', value: '3', tone: 'alert', detail: 'Informational, degraded, paused.' },
              { label: 'Banner variants', value: '6', tone: 'neutral', detail: 'Role-based language variants.' },
              { label: 'Backend dependency', value: 'Critical', tone: 'alert', detail: 'Requires authoritative pause semantics.' }
            ],
            modules: [
              { name: 'Incident banner', status: 'ready for shell', detail: 'UI can be refined immediately.' },
              { name: 'User guidance', status: 'shell-only', detail: 'Who should wait, retry, or contact support.' },
              { name: 'Operator controls', status: 'blocked', detail: 'Not a frontend problem until governance pause is frozen.' }
            ]
          }
        ],
        scenarios: [
          { id: 'steady', label: 'Steady flow', state: 'baseline', description: 'Healthy relayer and normal settlement timing.' },
          { id: 'timeout', label: 'Timeout path', state: 'stress', description: 'Tests how refunds and elapsed timers should read.' },
          { id: 'paused', label: 'Pause active', state: 'critical', description: 'Tests what the UI should hide versus keep visible.' }
        ],
        journey: [
          { title: 'Lock source asset', state: 'blocked', detail: 'Needs final bridge contract semantics.' },
          { title: 'Observe proof progression', state: 'blocked', detail: 'Needs relayer lifecycle and event freeze.' },
          { title: 'Complete or refund', state: 'blocked', detail: 'Cannot be made durable before timeout and refund logic settle.' }
        ]
      },
      {
        id: 'explorer',
        label: 'Explorer Feed',
        stage: 'blocked',
        description: 'Block, transaction, account, and event timeline shell for public chain observability.',
        readiness: 'Blocked',
        blockedBy: ['Indexer event model freeze'],
        metrics: [
          { label: 'Draft modules', value: '4' },
          { label: 'Contract coupling', value: 'High' },
          { label: 'Fixture scope', value: 'Event taxonomy' }
        ],
        screens: [
          {
            id: 'blocks',
            label: 'Blocks',
            kicker: 'Block stream',
            headline: 'Set the visual rhythm for blocks and validator attribution without assuming final event fields.',
            summary:
              'This shell tests density and scannability for block cards, validator hints, and finality badges.',
            cards: [
              { label: 'Block cards', value: '20', tone: 'cool', detail: 'Mock stream with finality and validator hints.' },
              { label: 'Badge variants', value: '5', tone: 'neutral', detail: 'Final, pending, delayed, challenged, archived.' },
              { label: 'Producer truth', value: 'Unfrozen', tone: 'alert', detail: 'Still dependent on canonical event producers.' }
            ],
            modules: [
              { name: 'Block card system', status: 'ready for shell', detail: 'Spacing and hierarchy can be tuned now.' },
              { name: 'Validator chips', status: 'shell-only', detail: 'Useful for density testing only.' },
              { name: 'Event joins', status: 'blocked', detail: 'Need stable correlation ids.' }
            ]
          },
          {
            id: 'activity',
            label: 'Activity feed',
            kicker: 'Cross-domain events',
            headline: 'Pressure-test event language before one lifecycle becomes many incompatible UI labels.',
            summary:
              'This shell helps settle vocabulary and visual grouping while the backend chooses the final canonical event families.',
            cards: [
              { label: 'Event families', value: '6', tone: 'neutral', detail: 'Wallet, verifier, settlement, governance, relayer, ops.' },
              { label: 'Grouping patterns', value: '3', tone: 'warm', detail: 'Time, domain, entity.' },
              { label: 'Freeze dependency', value: 'Hard', tone: 'alert', detail: 'Needs event family and field stability.' }
            ],
            modules: [
              { name: 'Family grouping', status: 'shell-only', detail: 'IA useful before any schema exists.' },
              { name: 'Correlation chips', status: 'blocked', detail: 'Need stable event identifiers.' },
              { name: 'Deep links', status: 'blocked', detail: 'Need explorer entity model and routes.' }
            ]
          },
          {
            id: 'entity',
            label: 'Entity detail',
            kicker: 'Address and transaction detail',
            headline: 'Design detail pages without lying about fields we do not own yet.',
            summary:
              'This screen is for layout and navigation architecture only until the event and entity model are frozen.',
            cards: [
              { label: 'Detail modules', value: '5', tone: 'cool', detail: 'Header, status, trace, related items, metadata.' },
              { label: 'Identity keys', value: 'TBD', tone: 'alert', detail: 'Cannot claim final joins yet.' },
              { label: 'Route readiness', value: 'IA only', tone: 'neutral', detail: 'Pure shell until event model freeze.' }
            ],
            modules: [
              { name: 'Header anatomy', status: 'ready for shell', detail: 'Can refine without live schemas.' },
              { name: 'Trace ladder', status: 'shell-only', detail: 'Visual shape only.' },
              { name: 'Canonical IDs', status: 'blocked', detail: 'Backend still owns identity semantics.' }
            ]
          }
        ],
        scenarios: [
          { id: 'dense', label: 'Dense event day', state: 'load test', description: 'Tests visual rhythm under heavy event volume.' },
          { id: 'quiet', label: 'Quiet chain period', state: 'empty state', description: 'Tests whether sparse telemetry still feels intentional.' },
          { id: 'incident', label: 'Incident clustering', state: 'stress', description: 'Tests how exceptional event families should stack.' }
        ],
        journey: [
          { title: 'Read block stream', state: 'blocked', detail: 'Needs final block and event fields.' },
          { title: 'Drill into entity', state: 'blocked', detail: 'Needs stable identity keys and entity model.' },
          { title: 'Correlate cross-domain events', state: 'blocked', detail: 'Needs canonical event families and joins.' }
        ]
      },
      {
        id: 'governance',
        label: 'Governance Desk',
        stage: 'blocked',
        description: 'Proposal board, vote detail, treasury lane, and validator oversight shells.',
        readiness: 'Blocked',
        blockedBy: ['Wallet and custody boundary freeze', 'Indexer event model freeze'],
        metrics: [
          { label: 'Draft modules', value: '4' },
          { label: 'Contract coupling', value: 'Medium' },
          { label: 'Fixture scope', value: 'Governance models' }
        ],
        screens: [
          {
            id: 'proposal-board',
            label: 'Proposal board',
            kicker: 'Decision surface',
            headline: 'Test how proposals, disputes, and treasury changes compete for attention.',
            summary:
              'This shell helps sort information hierarchy for governance without binding to any live vote, signer, or treasury contract yet.',
            cards: [
              { label: 'Board lanes', value: '3', tone: 'neutral', detail: 'Active, review, archived.' },
              { label: 'Priority rules', value: '4', tone: 'warm', detail: 'Dispute, treasury, parameter, emergency.' },
              { label: 'Trust dependency', value: 'Medium', tone: 'alert', detail: 'Needs signer and event boundaries to settle.' }
            ],
            modules: [
              { name: 'Proposal cards', status: 'ready for shell', detail: 'Board design can be tested now.' },
              { name: 'Escalation labels', status: 'shell-only', detail: 'Copy and severity mapping only.' },
              { name: 'Real vote clocks', status: 'blocked', detail: 'Need authoritative governance timing semantics.' }
            ]
          },
          {
            id: 'vote-detail',
            label: 'Vote detail',
            kicker: 'Decision anatomy',
            headline: 'Design a decision page that distinguishes user intent from backend authority.',
            summary:
              'This shell separates what the user can sign from what custody or governance-controlled services own.',
            cards: [
              { label: 'Decision modules', value: '5', tone: 'cool', detail: 'Summary, rationale, quorum, timeline, actor map.' },
              { label: 'Signer boundaries', value: 'Unsettled', tone: 'alert', detail: 'Wallet and custody freeze still needed.' },
              { label: 'Copy variants', value: '7', tone: 'neutral', detail: 'Different language for delegates, operators, and observers.' }
            ],
            modules: [
              { name: 'Vote anatomy', status: 'shell-only', detail: 'Page structure can be refined now.' },
              { name: 'Participation rail', status: 'blocked', detail: 'Needs final signing and role ownership.' },
              { name: 'Treasury implications', status: 'blocked', detail: 'Needs custody-backed treasury semantics.' }
            ]
          },
          {
            id: 'oversight',
            label: 'Validator oversight',
            kicker: 'Operational governance',
            headline: 'Explore how governance and validator oversight share a surface without confusing the operator role.',
            summary:
              'This shell lets product and protocol teams test whether validator, dispute, and treasury oversight belong together or should split.',
            cards: [
              { label: 'Oversight cards', value: '6', tone: 'neutral', detail: 'Health, disputes, quorum, treasury, emergency, audits.' },
              { label: 'Actor roles', value: '4', tone: 'cool', detail: 'User, delegate, operator, governance signer.' },
              { label: 'Backend certainty', value: 'Partial', tone: 'alert', detail: 'Still depends on event and signer freeze.' }
            ],
            modules: [
              { name: 'Oversight matrix', status: 'ready for shell', detail: 'IA and role grouping can be tested now.' },
              { name: 'Operational drilldown', status: 'shell-only', detail: 'Only visual hierarchy today.' },
              { name: 'Action affordances', status: 'blocked', detail: 'Do not ship until signer and event truth settle.' }
            ]
          }
        ],
        scenarios: [
          { id: 'routine', label: 'Routine governance', state: 'baseline', description: 'Healthy proposal flow with low urgency.' },
          { id: 'dispute', label: 'Dispute escalation', state: 'stress', description: 'Tests urgency and decision framing.' },
          { id: 'treasury', label: 'Treasury review', state: 'financial', description: 'Tests visibility for spend and signer boundaries.' }
        ],
        journey: [
          { title: 'Scan board', state: 'ready', detail: 'Layout and severity hierarchy can be refined today.' },
          { title: 'Inspect vote detail', state: 'blocked', detail: 'Needs durable signing ownership.' },
          { title: 'Act on governance item', state: 'blocked', detail: 'Not safe until wallet/custody boundary freezes.' }
        ]
      }
    ]
  }
];

export const shellMilestones = [
  {
    gate: 'Runtime API freeze',
    status: 'near',
    note: 'Inventory work exists, but live-code validation currently shows doc and implementation drift.'
  },
  {
    gate: 'RPC and sidecar contracts',
    status: 'closest',
    note: 'The strongest frontend-facing material exists here, but it still depends on runtime reconciliation.'
  },
  {
    gate: 'Bridge and relayer lifecycle',
    status: 'blocked',
    note: 'Replay, timeout, pause, and settlement semantics still belong to backend cleanup.'
  },
  {
    gate: 'Wallet and custody boundary',
    status: 'partial',
    note: 'Boundary is documented, but signer-path enforcement still needs proof in live code.'
  },
  {
    gate: 'Indexer event model',
    status: 'downstream',
    note: 'Cannot freeze honestly before runtime, bridge, and signer semantics stop moving.'
  }
];

export const shellAlerts = [
  'This shell never fetches from RPC, sidecar, gateway, or indexer endpoints.',
  'Every metric and status in this surface is fixture data for IA and local state transition testing only.',
  'Blocked areas remain visible so navigation, severity language, and route density can be tested without promising backend readiness.'
];

export const shellFlowCards = [
  {
    title: 'Account Entry',
    state: 'safe now',
    description: 'Identity, balances, notices, and trust framing can be refined without chain coupling.'
  },
  {
    title: 'Bridge Journey',
    state: 'wait',
    description: 'Session state, refund logic, and pause behavior stay mocked until the relayer lifecycle freezes.'
  },
  {
    title: 'Explorer Narrative',
    state: 'wait',
    description: 'Event cards and entity drilldowns stay schematic until the event model is versioned.'
  }
];