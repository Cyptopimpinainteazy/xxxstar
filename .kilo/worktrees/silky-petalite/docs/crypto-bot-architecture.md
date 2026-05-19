# High-Level Architecture for Ultimate Free Claims Crypto Bot

## Overview
The Ultimate Free Claims Crypto Bot is designed to automate the discovery, claiming, and management of free crypto opportunities such as airdrops, faucets, and grants. It integrates with existing project components like the Polkawallet, CRM, and infrastructure DB. The bot follows a modular architecture to ensure scalability, security, and ease of maintenance, incorporating AI for automation and adhering to the BMAD method for iterative development.

Key goals:
- Search and scrape for opportunities across social networks, Google (using dorks), and blockchain events.
- Automate signups, task completion, and claims using AI/LLM.
- Manage multiple wallets, proxies, and timers for efficient claiming.
- Integrate auto-trading for claimed coins and grant application pre-filling with staging.
- Provide API endpoints and CRM dashboard for user interaction.
- Incorporate 20 enhancement ideas (e.g., predictive analytics, scam detection).

## Core Components

### 1. Searcher Module
- **Purpose**: Discovers airdrops, faucets, free coins, and grants via scraping and APIs.
- **Features**:
  - Google dorks for fine-tuned searches.
  - Social network scraping (Twitter, Reddit, Discord) using APIs.
  - Integration with existing rpc-crawler (infra-structure/services/rpc-crawler) for on-chain events.
  - Social sentiment scanner and community-sourced opportunities.
  - Self-updating agent to learn new search methods.
- **Tech**: Python scripts for scraping (e.g., BeautifulSoup, Selenium), integrated with OpenRouter LLM for query optimization.
- **Inputs**: Configurable search terms, dorks, APIs keys.
- **Outputs**: Discovered opportunities stored in DB (airdrops, faucets, grants tables).

### 2. Claimer Module
- **Purpose**: Automates claiming processes for discovered opportunities.
- **Features**:
  - Auto-signup and task completion using LLM for form filling/captchas.
  - Faucet claiming with scheduler for cooldowns/timers.
  - Grant searching, pre-filling forms, and staging for user approval in CRM.
  - Handles timed/delayed limits by queuing returns.
  - Yield optimization: Auto-stake/lend claimed assets.
- **Tech**: Node.js or Python for automation, Puppeteer for browser interactions.
- **Integrations**: Links to Wallet Manager for multi-wallet claims (up to 100 per coin/chain).

### 3. DB Integrator
- **Purpose**: Manages data persistence and querying.
- **Features**:
  - Extends existing schema (infra-structure/db/schema.sql) with tables for proxies, LLM configs, grants, auto-trading logs.
  - Tracks claims, wallets, opportunities, and metrics.
  - Predictive analytics using historical data.
- **Tech**: SQLite/PostgreSQL with migrations (alembic).
- **Integrations**: Seeds data via scripts like seed_chains.py.

### 4. API Layer
- **Purpose**: Exposes bot functionalities for external access and integrations.
- **Features**:
  - Endpoints for searching, claiming status, analytics (performance, yield).
  - Performance Analytics API for metrics.
  - Secure authentication for user-specific data.
- **Tech**: Node.js/Express server (similar to infra-structure/services/chain-db/server.js).
- **Integrations**: Connects to CRM for dashboard and notifications.

### 5. AI Agent
- **Purpose**: Powers intelligent automation and decision-making.
- **Features**:
  - Uses OpenRouter LLM for form filling, task solving, scam detection, and self-updating logic.
  - Automated task solver for captchas/challenges.
  - Regulatory compliance tools (e.g., tax reports).
  - LLM endpoint rotation: Discovers public LLM endpoints (e.g., Ollama, vLLM) via recon scripts using Shodan, FOFA, Censys; validates and seeds to database; periodically re-validates in crawler_daemon.py; client.py rotates through healthy endpoints for queries.
- **Tech**: API calls to OpenRouter and rotated endpoints, with prompt engineering for crypto-specific tasks. Recon and validation handled by scripts/llm_recon.py and crawler_daemon.py.
- **Integrations**: Embedded in Searcher and Claimer modules; database integration for endpoint management.

### 6. Wallet Manager
- **Purpose**: Handles wallet creation, connection, and management.
- **Features**:
  - Multi-chain wallet generation (up to 100 per chain).
  - Auto-connection to Polkawallet.
  - Backup/recovery system with encrypted storage.
  - Balance tracking and auto-trading integration (add coins to exchange for trading).
  - Storage for user's Google and social accounts to enable automatic logins for claims and tasks.
  - Tiered access: Users must stake or hold X3 coin at specific levels to unlock bot features in the wallet, with engagement tiers determining limits (e.g., number of wallets, claim frequency).
  - Inactive wallet reclamation: After 3 months of no sign-ins, automatically transfer free token claims to a treasury wallet, with notifications.
- **Tech**: Web3.js or ethers.js for EVM, Solana SDK for SVM, etc.
- **Integrations**: DB for storage, Polkawallet API.

### 7. Security Module
- **Purpose**: Ensures safe operations.
- **Features**:
  - Proxy/VPN pool management and rotation.
  - Scam detection AI.
  - Encrypted private keys in DB.
  - Load balancing for high-volume claims.
- **Tech**: Proxy libraries (e.g., node-proxy), VPN APIs.

### 8. CRM Module and Integrations
- **Purpose**: User-facing interface and notifications.
- **Features**:
  - Dashboard for stats, leaderboards (gamified claiming), staged grants.
  - Notification hub for alerts.
  - Cross-device sync.
  - Referral engine.
- **Tech**: React components (extend apps/x3-desktop/src/components/panels/infrastructure/AirdropsPanel.tsx).
- **Integrations**: Direct connection to Polkawallet for wallet ops, existing CRM for user management.

## Data Flow
1. Searcher discovers opportunities → Stores in DB.
2. AI Agent analyzes/eligible checks (scam detection, predictions).
3. Wallet Manager assigns wallets.
4. Claimer automates claims/grants → Updates DB with results.
5. API/CRM provides access and notifications.
6. Security wraps all operations with proxies/encryption.

## Additional Ideas Integration
- Phased implementation: Core (1-6), Advanced (7-12), Optimization (13-20).
- Examples: Gamified claiming in CRM, yield optimization post-claim.
- Incorporated user feedback: Staking/holding X3 for feature access, automatic social logins via wallet-stored credentials, and inactive wallet reclamation to treasury.

## Deployment and BMAD
- Deploy as a service in infra-structure/services.
- Iterative BMAD: Build module, measure (tests/metrics), analyze (logs), deploy (integrate).

This architecture ensures modularity and extensibility. Next steps: Extend DB schema based on this outline.