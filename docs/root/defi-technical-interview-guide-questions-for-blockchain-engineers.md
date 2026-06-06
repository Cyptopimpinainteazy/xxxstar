# DeFi Technical Interview Guide — questions for blockchain engineers

Short, practical interview script for assessing DeFi candidates. Use these sections to probe fundamentals, architecture, security, scaling (TPS), decentralized storage (Swarm), and GPU-based/off-chain compute. For each prompt, follow up with the suggested deeper probes.

## Core fundamentals
- Explain how a blockchain achieves consensus. Compare PoW, PoS, and hybrid models.
  - Follow-ups: trade-offs in finality, decentralization, and energy use.
- Define state, transactions, and blocks. How does state transition differ on Ethereum vs UTXO chains?
- What is an EVM-compatible chain? Why does EVM compatibility matter?
  - Look for mention of tooling and portability.

## Smart contracts & tooling
- Describe the lifecycle of a smart contract from write → audit → deploy → upgrade.
  - Follow-ups: proxies, immutable contracts, and upgrade patterns.
- Which languages and toolchains do you use for smart contracts? Why choose one over another?
- How do you design gas-efficient contracts? Give concrete optimizations.
- What static analysis and formal verification tools do you rely on? Explain one example workflow.

## DeFi primitives and composability
- Explain AMMs, order-books, lending/borrowing, and liquidations — how do they differ architecturally?
- Walk through how a flash loan works and a real vulnerability pattern enabled by composability.
- How would you architect a new yield strategy that minimizes user exposure to MEV?

## Oracles, external data, and Chainlink
- How do decentralized oracles work and what are the main attack vectors?
- When would you choose a push vs pull oracle model?
- What guarantees does Chainlink provide and what remaining risks should engineers mitigate?

## Liquidity, incentives, and tokenomics
- How do you design incentives to bootstrap and sustain liquidity for a new token/AMM?
- Explain impermanent loss with a numerical example and mitigation strategies.
- How do governance tokens affect protocol security and incentives?

## Security, audits, and incident response
- Describe a recent class of DeFi exploits and how to defend against it.
- What are good practices for test coverage, fuzzing, and staging?
- Describe an incident response playbook for a live exploit (containment → communication → mitigation).

## Scaling & TPS
- What is TPS and why is it not the sole metric for scalability?
  - Follow-ups: throughput vs latency vs finality vs UX.
- Compare optimistic rollups and zk-rollups. Where does each shine for DeFi?
- How would you architect a high-throughput DEX to maximize effective TPS while preserving safety?

## Storage, IPFS, and Swarm
- Compare centralized storage, IPFS, and Swarm for storing off-chain data (UI assets, metadata, large datasets).
- When is Swarm preferable to IPFS for a DeFi-native app?
- How do you ensure availability and integrity for on-chain references to off-chain data?

## GPU & off-chain compute
- When would you use GPU-accelerated compute in a blockchain/DeFi pipeline (e.g., simulations, ML on-chain-oracle preparation, zero-knowledge proof generation)?
- What are challenges of trusting GPU-processed results and how do you design verifiable compute?
- How do cost, latency, and reproducibility trade-offs change if proofs or heavy simulations use GPU farms?

## Advanced architecture & integrations
- Describe a secure cross-chain bridge design. What are the main threat models?
- How do you integrate oracles, L2 middleware, and relayers into a composable DeFi stack?
- Design an on-chain governance upgrade flow minimizing centralization risk.

## Behavioral and system-design probes
- Tell me about a time you found a subtle protocol bug; how did you detect and remediate it?
- Given a protocol with rising TVL but increasing liquidation events, how do you triage and prioritize fixes?

## Interviewer scoring hints (what to listen for)
- Correct mental models: clear state machine reasoning, gas/fee awareness, concrete examples.
- Risk awareness: attacker models, MEV, oracle manipulation, and operator collusion.
- Practical skills: familiarity with Solidity/EVM tooling, test frameworks, fuzzers, and auditing practices.
- Trade-off reasoning: thoughtful discussion of TPS, finality, UX, cost, and decentralization.

## Quick starter question set (10-minute screening)
1. What is slippage and why does it matter for AMMs?
2. How does an oracle create a single point of failure? Give mitigation options.
3. Explain what TPS measures and one thing it misses.
4. When would you use Swarm vs IPFS for metadata?
5. Have you used GPU tooling for cryptographic/ML tasks? What did you optimize?

## Resources
- Ethereum — https://ethereum.org/en/
- EVM docs — https://ethereum.org/en/developers/docs/evm/
- Solidity — https://docs.soliditylang.org/
- Bitcoin — https://bitcoin.org/
- Uniswap — https://uniswap.org/
- Aave docs — https://docs.aave.com/
- Compound — https://compound.finance/
- Chainlink — https://chain.link/
- OpenZeppelin — https://openzeppelin.com/
- MythX — https://mythx.io/
- Slither (static analysis) — https://github.com/crytic/slither
- Etherscan — https://etherscan.io/
- Optimism — https://optimism.io/
- zkSync — https://zksync.io/
- IPFS — https://ipfs.io/
- Swarm — https://swarm.ethereum.org/
- NVIDIA CUDA Toolkit (GPU compute) — https://developer.nvidia.com/cuda-toolkit

Open Items
- None.