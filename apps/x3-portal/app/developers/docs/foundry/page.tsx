'use client';

import React from 'react';
import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';

export default function FoundryGuidePage() {
  return (
    <DocLayout
      title="Foundry Development Guide"
      description="Build and test smart contracts with Foundry on X3 Atlas Sphere"
    >
      <p className="lead text-xl text-gray-400 mb-8">
        Foundry is a blazing fast, portable toolkit for Ethereum development written in Rust.
        Perfect for developers who prefer command-line workflows and Solidity-based testing.
      </p>

      <h2>Installation</h2>
      <CodeBlock language="bash">
{`# Install Foundry
curl -L https://foundry.paradigm.xyz | bash
foundryup

# Verify installation
forge --version
cast --version
anvil --version`}
      </CodeBlock>

      <h2>Project Setup</h2>
      <CodeBlock language="bash">
{`# Create new project
forge init my-x3-project
cd my-x3-project

# Install OpenZeppelin contracts
forge install OpenZeppelin/openzeppelin-contracts

# Update remappings
echo '@openzeppelin/=lib/openzeppelin-contracts/' >> remappings.txt`}
      </CodeBlock>

      <h2>Foundry Configuration</h2>
      <CodeBlock language="toml" title="foundry.toml">
{`[profile.default]
src = "src"
out = "out"
libs = ["lib"]
solc = "0.8.20"
optimizer = true
optimizer_runs = 200

[rpc_endpoints]
x3_local = "http://127.0.0.1:9933"
x3_testnet = "https://rpc.testnet.atlas-sphere.io"

[etherscan]
x3_testnet = { key = "\${EXPLORER_API_KEY}", url = "https://api.explorer.testnet.atlas-sphere.io/api" }

[fmt]
line_length = 100
tab_width = 4
bracket_spacing = true`}
      </CodeBlock>

      <h2>Writing Contracts</h2>
      <CodeBlock language="solidity" title="src/AtlasToken.sol">
{`// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

contract AtlasToken is ERC20, Ownable {
    constructor(uint256 initialSupply) ERC20("Atlas Token", "ATL") Ownable(msg.sender) {
        _mint(msg.sender, initialSupply);
    }

    function mint(address to, uint256 amount) external onlyOwner {
        _mint(to, amount);
    }
}`}
      </CodeBlock>

      <h2>Writing Tests</h2>
      <CodeBlock language="solidity" title="test/AtlasToken.t.sol">
{`// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Test.sol";
import "../src/AtlasToken.sol";

contract AtlasTokenTest is Test {
    AtlasToken public token;
    address public owner;
    address public alice;
    address public bob;

    uint256 constant INITIAL_SUPPLY = 1_000_000 ether;

    function setUp() public {
        owner = address(this);
        alice = makeAddr("alice");
        bob = makeAddr("bob");

        token = new AtlasToken(INITIAL_SUPPLY);
    }

    function test_InitialSupply() public view {
        assertEq(token.totalSupply(), INITIAL_SUPPLY);
        assertEq(token.balanceOf(owner), INITIAL_SUPPLY);
    }

    function test_Transfer() public {
        uint256 amount = 100 ether;
        token.transfer(alice, amount);
        
        assertEq(token.balanceOf(alice), amount);
        assertEq(token.balanceOf(owner), INITIAL_SUPPLY - amount);
    }

    function test_TransferEvent() public {
        uint256 amount = 100 ether;
        
        vm.expectEmit(true, true, false, true);
        emit IERC20.Transfer(owner, alice, amount);
        
        token.transfer(alice, amount);
    }

    function testFuzz_Transfer(uint256 amount) public {
        amount = bound(amount, 0, INITIAL_SUPPLY);
        token.transfer(alice, amount);
        assertEq(token.balanceOf(alice), amount);
    }

    function testFail_TransferInsufficientBalance() public {
        vm.prank(alice);
        token.transfer(bob, 1 ether);
    }

    function test_Mint() public {
        uint256 amount = 1000 ether;
        token.mint(alice, amount);
        assertEq(token.balanceOf(alice), amount);
    }

    function test_RevertWhen_NonOwnerMints() public {
        vm.prank(alice);
        vm.expectRevert();
        token.mint(bob, 1000 ether);
    }
}`}
      </CodeBlock>

      <h2>Running Tests</h2>
      <CodeBlock language="bash">
{`# Run all tests
forge test

# Run with verbosity
forge test -vvv

# Run specific test
forge test --match-test test_Transfer

# Run with gas reporting
forge test --gas-report

# Run fuzz tests with more runs
forge test --fuzz-runs 10000

# Generate coverage report
forge coverage`}
      </CodeBlock>

      <h2>Deployment</h2>
      <CodeBlock language="solidity" title="script/Deploy.s.sol">
{`// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "forge-std/Script.sol";
import "../src/AtlasToken.sol";

contract DeployScript is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        
        vm.startBroadcast(deployerPrivateKey);
        
        AtlasToken token = new AtlasToken(1_000_000 ether);
        console.log("AtlasToken deployed at:", address(token));
        
        vm.stopBroadcast();
    }
}`}
      </CodeBlock>

      <CodeBlock language="bash">
{`# Deploy to local node
forge script script/Deploy.s.sol --rpc-url x3_local --broadcast

# Deploy to testnet
forge script script/Deploy.s.sol --rpc-url x3_testnet --broadcast --verify

# Verify existing contract
forge verify-contract CONTRACT_ADDRESS AtlasToken --chain 5330`}
      </CodeBlock>

      <h2>Useful Cast Commands</h2>
      <CodeBlock language="bash">
{`# Call view function
cast call TOKEN_ADDRESS "balanceOf(address)(uint256)" USER_ADDRESS --rpc-url x3_testnet

# Send transaction
cast send TOKEN_ADDRESS "transfer(address,uint256)" RECIPIENT 1000000000000000000 \\
  --rpc-url x3_testnet --private-key $PRIVATE_KEY

# Get block number
cast block-number --rpc-url x3_testnet

# Decode calldata
cast 4byte-decode 0xa9059cbb...

# Convert units
cast to-wei 1 ether
cast from-wei 1000000000000000000`}
      </CodeBlock>

      <Callout type="info" title="Pro Tip">
        Use <code>forge snapshot</code> to track gas usage across test runs and 
        <code>forge snapshot --diff</code> to compare changes.
      </Callout>
    </DocLayout>
  );
}
