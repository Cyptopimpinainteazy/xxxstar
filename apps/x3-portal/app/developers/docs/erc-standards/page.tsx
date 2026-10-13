'use client';

import React from 'react';
import DocLayout, { CodeBlock, Callout } from '@/components/docs/DocLayout';

export default function ErcStandardsPage() {
  return (
    <DocLayout
      title="ERC Token Standards"
      description="Implementing ERC-20, ERC-721, and ERC-1155 tokens on X3 Atlas Sphere"
    >
      <p className="lead text-xl text-gray-400 mb-8">
        X3 Atlas Sphere's EVM fully supports all Ethereum token standards. Deploy ERC-20,
        ERC-721, and ERC-1155 tokens that can interact with SVM programs via Comits.
      </p>

      <h2>ERC-20: Fungible Tokens</h2>
      <p>
        The most common token standard for fungible assets like currencies, governance tokens, and stablecoins.
      </p>
      <CodeBlock language="solidity" title="contracts/MyToken.sol">
{`// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Burnable.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Permit.sol";

contract MyToken is ERC20, ERC20Burnable, ERC20Permit {
    constructor() ERC20("MyToken", "MTK") ERC20Permit("MyToken") {
        _mint(msg.sender, 1000000 * 10 ** decimals());
    }
}

// With minting capability
contract MintableToken is ERC20 {
    address public minter;

    constructor() ERC20("Mintable", "MINT") {
        minter = msg.sender;
    }

    function mint(address to, uint256 amount) external {
        require(msg.sender == minter, "Not minter");
        _mint(to, amount);
    }
}`}
      </CodeBlock>

      <h2>ERC-721: Non-Fungible Tokens</h2>
      <p>
        For unique assets like NFTs, game items, and digital collectibles.
      </p>
      <CodeBlock language="solidity" title="contracts/MyNFT.sol">
{`// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC721/ERC721.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/ERC721URIStorage.sol";
import "@openzeppelin/contracts/token/ERC721/extensions/ERC721Enumerable.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

contract MyNFT is ERC721, ERC721URIStorage, ERC721Enumerable, Ownable {
    uint256 private _nextTokenId;

    constructor() ERC721("MyNFT", "MNFT") Ownable(msg.sender) {}

    function safeMint(address to, string memory uri) public onlyOwner returns (uint256) {
        uint256 tokenId = _nextTokenId++;
        _safeMint(to, tokenId);
        _setTokenURI(tokenId, uri);
        return tokenId;
    }

    // Required overrides
    function _update(address to, uint256 tokenId, address auth)
        internal override(ERC721, ERC721Enumerable) returns (address)
    {
        return super._update(to, tokenId, auth);
    }

    function _increaseBalance(address account, uint128 value)
        internal override(ERC721, ERC721Enumerable)
    {
        super._increaseBalance(account, value);
    }

    function tokenURI(uint256 tokenId)
        public view override(ERC721, ERC721URIStorage) returns (string memory)
    {
        return super.tokenURI(tokenId);
    }

    function supportsInterface(bytes4 interfaceId)
        public view override(ERC721, ERC721URIStorage, ERC721Enumerable) returns (bool)
    {
        return super.supportsInterface(interfaceId);
    }
}`}
      </CodeBlock>

      <h2>ERC-1155: Multi-Token Standard</h2>
      <p>
        Efficient for managing multiple token types (fungible and non-fungible) in a single contract.
      </p>
      <CodeBlock language="solidity" title="contracts/GameItems.sol">
{`// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC1155/ERC1155.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

contract GameItems is ERC1155, Ownable {
    uint256 public constant GOLD = 0;
    uint256 public constant SILVER = 1;
    uint256 public constant SWORD = 2;
    uint256 public constant SHIELD = 3;

    constructor() ERC1155("https://game.atlas-sphere.io/api/items/{id}.json") Ownable(msg.sender) {
        // Mint fungible currencies
        _mint(msg.sender, GOLD, 10**18, "");
        _mint(msg.sender, SILVER, 10**18, "");
        
        // Mint unique items
        _mint(msg.sender, SWORD, 1, "");
        _mint(msg.sender, SHIELD, 1, "");
    }

    function mint(address to, uint256 id, uint256 amount, bytes memory data) public onlyOwner {
        _mint(to, id, amount, data);
    }

    function mintBatch(
        address to,
        uint256[] memory ids,
        uint256[] memory amounts,
        bytes memory data
    ) public onlyOwner {
        _mintBatch(to, ids, amounts, data);
    }
}`}
      </CodeBlock>

      <h2>Cross-VM Token Bridging</h2>
      <Callout type="info" title="Canonical Ledger Integration">
        ERC tokens on X3 can be registered in the Canonical Ledger, enabling them to be 
        accessed from both EVM and SVM environments through Comit transactions.
      </Callout>

      <CodeBlock language="solidity" title="contracts/BridgeableToken.sol">
{`// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

interface IAtlasBridge {
    function lockForSVM(address token, uint256 amount, bytes32 svmRecipient) external;
    function unlockFromSVM(address token, address recipient, uint256 amount) external;
}

contract BridgeableToken is ERC20 {
    IAtlasBridge public immutable bridge;

    constructor(address _bridge) ERC20("Bridgeable", "BRDG") {
        bridge = IAtlasBridge(_bridge);
        _mint(msg.sender, 1000000 * 10 ** decimals());
    }

    // Lock tokens to be used on SVM side
    function bridgeToSVM(uint256 amount, bytes32 svmRecipient) external {
        transfer(address(bridge), amount);
        bridge.lockForSVM(address(this), amount, svmRecipient);
    }
}`}
      </CodeBlock>

      <h2>Best Practices</h2>
      <ul>
        <li><strong>Use OpenZeppelin</strong> - Battle-tested implementations with security audits</li>
        <li><strong>Add Permit</strong> - ERC20Permit enables gasless approvals</li>
        <li><strong>Implement Enumerable</strong> - For NFT collections needing iteration</li>
        <li><strong>Metadata standards</strong> - Follow ERC-721 metadata JSON schema</li>
        <li><strong>Access control</strong> - Use roles for privileged functions</li>
      </ul>

      <Callout type="warning" title="Security">
        Always audit your token contracts before mainnet deployment. Common issues include
        reentrancy, integer overflow (use Solidity 0.8+), and missing access controls.
      </Callout>
    </DocLayout>
  );
}
