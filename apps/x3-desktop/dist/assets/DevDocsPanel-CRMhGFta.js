import{c as h,r as p,j as e,a as b,Z as k}from"./index-Bjwn4JM-.js";import{S as w}from"./search-BOg0-BaB.js";import{C as u}from"./chevron-right-BksUNH92.js";import{C}from"./code-CXY5yhpI.js";import{C as M}from"./cpu-e8-YUZ5X.js";import{L as V}from"./layers-CCEDm2od.js";/**
 * @license lucide-react v0.563.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const S=[["path",{d:"M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H19a1 1 0 0 1 1 1v18a1 1 0 0 1-1 1H6.5a1 1 0 0 1 0-5H20",key:"k3hazp"}]],g=h("book",S);/**
 * @license lucide-react v0.563.0 - ISC
 *
 * This source code is licensed under the ISC license.
 * See the LICENSE file in the root directory of this source tree.
 */const E=[["path",{d:"m15 18-6-6 6-6",key:"1wnfg3"}]],j=h("chevron-left",E),d=[{category:"Getting Started",icon:e.jsx(g,{size:14}),items:[{id:"intro",title:"Introduction"},{id:"installation",title:"Installation"},{id:"quickstart",title:"Quickstart"},{id:"run-node",title:"Run a Node"}]},{category:"EVM Development",icon:e.jsx(C,{size:14}),items:[{id:"hardhat",title:"Hardhat Setup"},{id:"foundry",title:"Foundry Setup"},{id:"deploy-evm",title:"Deploy Contracts"},{id:"interact-evm",title:"Interact with Contracts"},{id:"erc-standards",title:"ERC Standards"}]},{category:"SVM Development",icon:e.jsx(M,{size:14}),items:[{id:"accounts",title:"Accounts Model"},{id:"programs",title:"Programs"},{id:"deploy-svm",title:"Deploy Programs"},{id:"spl-tokens",title:"SPL Tokens"}]},{category:"Architecture",icon:e.jsx(V,{size:14}),items:[{id:"dual-vm",title:"Dual-VM Design"},{id:"comits",title:"Comits"},{id:"canonical-ledger",title:"Canonical Ledger"},{id:"cross-vm",title:"Cross-VM Assets"},{id:"atomic-exec",title:"Atomic Execution"},{id:"chain-spec",title:"Chain Spec"}]},{category:"Operations",icon:e.jsx(b,{size:14}),items:[{id:"configuration",title:"Configuration"},{id:"monitoring",title:"Monitoring"},{id:"keys",title:"Key Management"},{id:"authorization",title:"Authorization"},{id:"validator-ops",title:"Validator Setup"},{id:"error-handling",title:"Error Handling"},{id:"best-practices",title:"Best Practices"}]},{category:"Advanced",icon:e.jsx(k,{size:14}),items:[{id:"anchor",title:"Anchor Framework"},{id:"cookbook",title:"Cookbook Recipes"}]}],i=d.flatMap(n=>n.items.map(r=>r.id)),c={intro:{id:"intro",title:"Introduction",category:"Getting Started",description:["X3 Chain is a next-generation blockchain platform that unifies the Ethereum Virtual Machine (EVM) and Solana Virtual Machine (SVM) under a single canonical ledger. This dual-VM architecture enables developers to deploy smart contracts in Solidity or Rust and have them interoperate natively.","The platform introduces Comits — composable micro-transactions that allow atomic cross-VM execution. Combined with a GPU-accelerated swarm network for off-chain computation, X3 Chain provides a complete infrastructure for building high-performance decentralized applications.","This developer portal covers everything you need to start building on X3 Chain, from setting up your environment to deploying production-grade dApps across both virtual machines."]},installation:{id:"installation",title:"Installation",category:"Getting Started",description:["X3 Chain provides a CLI toolchain for local development. The x3-cli package bundles a local devnet node, contract compilation tools, and deployment utilities.","You can install via npm, cargo, or download pre-built binaries for Linux, macOS, and Windows. The recommended approach is using the npm package for frontend-heavy projects and cargo for Rust-native development."],code:`# Install via npm
npm install -g @x3-chain/cli

# Or via cargo
cargo install x3-cli

# Verify installation
x3 --version
x3 node --dev`},quickstart:{id:"quickstart",title:"Quickstart",category:"Getting Started",description:["Get a local X3 node running in under 5 minutes. The devnet node comes pre-configured with test accounts, funded wallets, and both EVM and SVM runtimes enabled.","Once your node is running, you can deploy contracts using Hardhat (EVM) or Anchor (SVM), and interact with them through the unified RPC interface at localhost:9944."],code:`# Start a local dev node
x3 node --dev --tmp

# In another terminal, scaffold a project
x3 init my-dapp --template dual-vm
cd my-dapp

# Deploy to local devnet
x3 deploy --network devnet`},"run-node":{id:"run-node",title:"Run a Node",category:"Getting Started",description:["Running an X3 Chain node connects you to the network as a full peer. Nodes validate transactions, store chain state, and can participate as validators if staked.","There are three node types: full nodes (store complete state), archive nodes (store all historical state), and validator nodes (produce blocks). Each has different hardware requirements."],code:`# Full node
x3 node --chain mainnet --name "my-node"

# Archive node
x3 node --chain mainnet --pruning archive

# Validator node (requires staking)
x3 node --chain mainnet --validator \\
  --name "my-validator" \\
  --telemetry-url "wss://telemetry.x3.network"`},hardhat:{id:"hardhat",title:"Hardhat Setup",category:"EVM Development",description:["Hardhat is the recommended framework for EVM development on X3 Chain. The x3-hardhat plugin adds network configurations, custom tasks for dual-VM interaction, and Comit-aware deployment scripts.","Install the plugin and configure your hardhat.config.ts to point at your local devnet or the X3 testnet/mainnet RPC endpoints."],code:`npm install --save-dev @x3-chain/hardhat-plugin

// hardhat.config.ts
import "@x3-chain/hardhat-plugin";

export default {
  solidity: "0.8.24",
  networks: {
    x3: {
      url: "http://127.0.0.1:9944",
      accounts: [process.env.PRIVATE_KEY],
    },
  },
};`},foundry:{id:"foundry",title:"Foundry Setup",category:"EVM Development",description:["Foundry provides fast Solidity compilation and testing via forge and cast. X3 Chain is natively compatible with Foundry — point your RPC URL at an X3 node and deploy as you would on any EVM chain.","For cross-VM features, use the x3-foundry library which adds SVM precompile interfaces to your Solidity contracts."],code:`forge init my-x3-project
cd my-x3-project

# Deploy with forge
forge create src/Counter.sol:Counter \\
  --rpc-url http://127.0.0.1:9944 \\
  --private-key $PRIVATE_KEY`},"deploy-evm":{id:"deploy-evm",title:"Deploy Contracts",category:"EVM Development",description:["Deploying EVM contracts to X3 Chain follows standard Ethereum patterns. Contracts compile with solc, deploy via standard JSON-RPC methods, and execute on the EVM runtime within the X3 Kernel.","The key difference is that your contracts can interact with SVM programs through the CrossVM precompile at address 0x0000...0800, enabling true dual-VM composability."],code:`// Deploy via ethers.js
const factory = new ethers.ContractFactory(abi, bytecode, signer);
const contract = await factory.deploy();
await contract.waitForDeployment();
console.log("Deployed at:", await contract.getAddress());`},"interact-evm":{id:"interact-evm",title:"Interact with Contracts",category:"EVM Development",description:["Interacting with deployed EVM contracts uses familiar web3 libraries — ethers.js, viem, or web3.js. Connect to the X3 RPC endpoint and call contract methods as you normally would.","For read-only calls, use staticCall. For state-changing transactions, send them via a wallet provider or programmatic signer. X3 supports EIP-1559 fee estimation."]},"erc-standards":{id:"erc-standards",title:"ERC Standards",category:"EVM Development",description:["X3 Chain supports all major ERC standards: ERC-20 (fungible tokens), ERC-721 (NFTs), ERC-1155 (multi-token), ERC-4626 (tokenized vaults), and more. These work identically to mainnet Ethereum.","Additionally, X3 introduces ATRC-1 and ATRC-2 extension standards for cross-VM token bridging and Comit-aware token transfer hooks."]},accounts:{id:"accounts",title:"Accounts Model",category:"SVM Development",description:["The SVM on X3 Chain uses the Solana account model: all state is stored in accounts, programs are stateless executables, and instructions specify which accounts to read/write. This enables parallel transaction execution.","X3 extends the standard account model with dual-VM address mapping — every SVM account has a deterministic EVM address counterpart, enabling seamless cross-VM asset transfers."]},programs:{id:"programs",title:"Programs",category:"SVM Development",description:["SVM programs are compiled to BPF bytecode and deployed as on-chain executables. They process instructions, read/write account data, and emit events. Programs are stateless — all state lives in accounts they operate on.","X3 supports both native Rust programs and Anchor framework programs with full Solana-compatible instruction processing."]},"deploy-svm":{id:"deploy-svm",title:"Deploy Programs",category:"SVM Development",description:["Deploying SVM programs to X3 requires compiling to BPF, packaging the bytecode, and submitting a deploy transaction. The x3-cli provides streamlined commands for each step."],code:`# Build the program
cargo build-bpf --manifest-path program/Cargo.toml

# Deploy to X3 devnet
x3 program deploy \\
  target/deploy/my_program.so \\
  --keypair ~/.config/x3/id.json \\
  --url http://127.0.0.1:9944`},"spl-tokens":{id:"spl-tokens",title:"SPL Tokens",category:"SVM Development",description:["SPL tokens on X3 work identically to Solana SPL tokens. Create token mints, associated token accounts, and manage balances through the SPL Token program.","X3 extends SPL with automatic ERC-20 mirror contracts — every SPL token is automatically accessible as an ERC-20 on the EVM side, with the Canonical Ledger maintaining balance consistency."]},"dual-vm":{id:"dual-vm",title:"Dual-VM Design",category:"Architecture",description:["The X3 Kernel runs both EVM and SVM runtimes within a single Substrate-based blockchain. Each VM has its own execution context but shares the same block production, consensus, and finality layer.","Cross-VM communication is achieved through the Comit Engine, which batches cross-VM calls into atomic execution units. This guarantees that cross-VM operations either fully succeed or fully revert."]},comits:{id:"comits",title:"Comits",category:"Architecture",description:["Comits (Composable Micro-Transactions) are the fundamental unit of cross-VM execution in X3 Chain. A Comit bundles one or more instructions targeting different VMs into a single atomic operation.","The Comit Engine validates, orders, and executes these bundles within a single block, ensuring consistency across both execution environments without requiring separate bridging infrastructure."]},"canonical-ledger":{id:"canonical-ledger",title:"Canonical Ledger",category:"Architecture",description:["The Canonical Ledger is X3 Chain's unified state root that encompasses both EVM and SVM state trees. Rather than maintaining separate ledgers, all state is committed to a single Merkle-Patricia trie.","This design enables efficient state proofs that span both VMs, compact light-client verification, and deterministic state replay for archive nodes."]},"cross-vm":{id:"cross-vm",title:"Cross-VM Assets",category:"Architecture",description:["Cross-VM assets are tokens and NFTs that exist simultaneously on both VMs. The Canonical Ledger ensures a single source of truth for balances, preventing double-spend across execution environments.","Developers can transfer assets between VMs with a single transaction — no bridges, no wrapping, no waiting for confirmations on a separate chain."]},"atomic-exec":{id:"atomic-exec",title:"Atomic Execution",category:"Architecture",description:["X3 guarantees atomic execution of Comit bundles: all instructions within a bundle succeed or all revert. This is enforced at the consensus layer, making it a protocol-level guarantee rather than a smart contract convention.","This enables complex cross-VM DeFi operations like flash loans that span both EVM and SVM in a single block."]},"chain-spec":{id:"chain-spec",title:"Chain Spec",category:"Architecture",description:["The X3 chain specification defines network parameters, genesis state, and runtime configuration. Different chain specs exist for mainnet, testnet, and devnet environments."],code:`{
  "name": "X3 Chain Mainnet",
  "id": "x3-mainnet",
  "chainType": "Live",
  "bootNodes": [
    "/dns/boot1.x3.network/tcp/30333/p2p/12D3Koo..."
  ],
  "properties": {
    "tokenSymbol": "X3",
    "tokenDecimals": 18,
    "ss58Format": 42
  }
}`},configuration:{id:"configuration",title:"Configuration",category:"Operations",description:["X3 nodes are configured via a TOML configuration file, CLI arguments, or environment variables. The configuration covers networking, RPC endpoints, database paths, validator keys, and telemetry.","Default configurations are provided for each network (mainnet, testnet, devnet). Override specific values as needed for your deployment."],code:`# x3-config.toml
[network]
listen_addr = "0.0.0.0:30333"
public_addr = "/dns4/my-node.example.com/tcp/30333"

[rpc]
cors = ["*"]
port = 9944
max_connections = 100

[database]
path = "/data/x3"
cache_size = 1024`},monitoring:{id:"monitoring",title:"Monitoring",category:"Operations",description:["X3 nodes expose Prometheus metrics at /metrics and support Grafana dashboards out of the box. Key metrics include block production time, peer count, transaction pool size, and VM execution time.","Use the Telemetry endpoint to submit node stats to the X3 Telemetry dashboard for public visibility and network health monitoring."]},keys:{id:"keys",title:"Key Management",category:"Operations",description:["X3 supports SR25519, ED25519, and ECDSA key types. Validator keys are managed via the x3 key subcommand. Store keys in the secure keystore — never embed private keys in configuration files."],code:`# Generate a new keypair
x3 key generate --scheme sr25519

# Insert a key into the node keystore
x3 key insert --base-path /data/x3 \\
  --chain mainnet \\
  --scheme sr25519 \\
  --suri "//Alice"`},authorization:{id:"authorization",title:"Authorization",category:"Operations",description:["X3 provides role-based access control for RPC methods and node management. Configure authorized callers, rate limits, and method whitelists to secure your node's RPC interface."]},"validator-ops":{id:"validator-ops",title:"Validator Setup",category:"Operations",description:["Running a validator requires staking X3 tokens, registering session keys on-chain, and maintaining high uptime. Validators earn block rewards proportional to their stake and performance.","Hardware recommendations: 8-core CPU, 32 GB RAM, 1 TB NVMe SSD, and a minimum 100 Mbps network connection."]},"error-handling":{id:"error-handling",title:"Error Handling",category:"Operations",description:["X3 runtime errors follow Substrate's DispatchError pattern. Each pallet defines typed errors that are returned on failed extrinsics. Use the error metadata to decode human-readable error messages."]},"best-practices":{id:"best-practices",title:"Best Practices",category:"Operations",description:["Follow these operational best practices: use separate accounts for staking and session keys, enable Prometheus monitoring, configure alerting for missed blocks, keep your node software up to date, and test upgrades on testnet before mainnet.","For contract development: use audited libraries (OpenZeppelin for EVM, Anchor for SVM), write comprehensive tests, and submit contracts for verification on the X3 Explorer."]},anchor:{id:"anchor",title:"Anchor Framework",category:"Advanced",description:["Anchor is a framework for building SVM programs with less boilerplate. It provides declarative account validation, automatic (de)serialization, and a testing framework similar to Hardhat.","X3 fully supports Anchor programs. Use the standard anchor build and anchor deploy workflow, pointing at an X3 RPC endpoint."],code:`// lib.rs
use anchor_lang::prelude::*;

declare_id!("AtLs...");

#[program]
pub mod my_program {
    use super::*;
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.data.value = 0;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = user, space = 8 + 8)]
    pub data: Account<'info, MyData>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct MyData { pub value: u64 }`},cookbook:{id:"cookbook",title:"Cookbook Recipes",category:"Advanced",description:["Common recipes for X3 Chain development: cross-VM token swaps, Comit-based atomic arbitrage, GPU swarm job submission, multi-sig governance proposals, and on-chain oracle integration.","Each recipe includes complete code examples, deployment scripts, and test cases you can run on devnet."],code:`// Cross-VM Token Swap Recipe
const comit = await x3.createComit([
  // Step 1: Lock ERC-20 on EVM
  { vm: 'evm', to: lockContract, data: encodeLock(amount) },
  // Step 2: Mint SPL on SVM
  { vm: 'svm', program: mintProgram, accounts: [...], data: encodeMint(amount) },
]);
const receipt = await comit.execute();`}},N=()=>{const[n,r]=p.useState("intro"),[l,y]=p.useState(""),[m,f]=p.useState(Object.fromEntries(d.map(t=>[t.category,!0]))),s=c[n]||c.intro,o=i.indexOf(n),x=t=>{f(a=>({...a,[t]:!a[t]}))},v=l?d.map(t=>({...t,items:t.items.filter(a=>a.title.toLowerCase().includes(l.toLowerCase()))})).filter(t=>t.items.length>0):d;return e.jsxs("div",{className:"flex flex-col h-full bg-[#0a0a0f] text-gray-300",children:[e.jsxs("div",{className:"flex items-center justify-between px-5 py-4 border-b border-[#1a1a1a]",children:[e.jsxs("div",{className:"flex items-center gap-2",children:[e.jsx(g,{size:18,className:"text-[#ff6b35]"}),e.jsx("h1",{className:"text-lg font-semibold text-white",children:"Developer Portal"})]}),e.jsxs("div",{className:"relative w-64",children:[e.jsx(w,{size:14,className:"absolute left-3 top-1/2 -translate-y-1/2 text-gray-500"}),e.jsx("input",{type:"text",placeholder:"Search docs...",value:l,onChange:t=>y(t.target.value),className:"w-full pl-9 pr-3 py-1.5 bg-[#111118] border border-[#1a1a1a] rounded text-sm text-gray-300 focus:outline-none focus:border-[#ff6b35]/50"})]})]}),e.jsxs("div",{className:"flex flex-1 overflow-hidden",children:[e.jsx("div",{className:"w-56 border-r border-[#1a1a1a] overflow-y-auto py-3 flex-shrink-0",children:v.map(t=>e.jsxs("div",{className:"mb-1",children:[e.jsxs("button",{onClick:()=>x(t.category),className:"flex items-center gap-2 w-full px-4 py-1.5 text-xs font-semibold text-gray-500 uppercase tracking-wider hover:text-gray-300",children:[t.icon,e.jsx("span",{className:"flex-1 text-left",children:t.category}),e.jsx(u,{size:12,className:`transition-transform ${m[t.category]?"rotate-90":""}`})]}),m[t.category]&&t.items.map(a=>e.jsx("button",{onClick:()=>r(a.id),className:`w-full text-left px-8 py-1.5 text-sm transition-colors ${n===a.id?"text-[#ff6b35] bg-[#ff6b35]/5 border-r-2 border-[#ff6b35]":"text-gray-400 hover:text-gray-200 hover:bg-white/5"}`,children:a.title},a.id))]},t.category))}),e.jsx("div",{className:"flex-1 overflow-y-auto p-6",children:e.jsxs("div",{className:"max-w-3xl",children:[e.jsx("p",{className:"text-xs text-gray-500 mb-1",children:s.category}),e.jsx("h2",{className:"text-2xl font-bold text-white mb-4",children:s.title}),s.description.map((t,a)=>e.jsx("p",{className:"text-gray-400 leading-relaxed mb-4",children:t},a)),s.code&&e.jsx("pre",{className:"bg-[#050508] border border-[#1a1a1a] rounded-lg p-4 text-sm text-green-400/80 font-mono overflow-x-auto mb-6 whitespace-pre-wrap",children:s.code}),e.jsxs("div",{className:"flex items-center justify-between pt-6 mt-6 border-t border-[#1a1a1a]",children:[o>0?e.jsxs("button",{onClick:()=>r(i[o-1]),className:"flex items-center gap-1 text-sm text-gray-400 hover:text-[#ff6b35]",children:[e.jsx(j,{size:14})," Previous: ",c[i[o-1]]?.title]}):e.jsx("span",{}),o<i.length-1?e.jsxs("button",{onClick:()=>r(i[o+1]),className:"flex items-center gap-1 text-sm text-gray-400 hover:text-[#ff6b35]",children:["Next: ",c[i[o+1]]?.title," ",e.jsx(u,{size:14})]}):e.jsx("span",{})]})]})})]})]})};export{N as default};
