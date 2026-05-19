// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/token/ERC20/extensions/ERC20Burnable.sol";
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

/**
 * @title BOT Token
 * @author Botchain Team
 * @notice ERC20 token for the Botchain ecosystem
 * @dev Standard ERC20 with faucet function for testing and owner mint capability
 *
 * The BOT token is used for:
 * - Paying marriage fees to create child agents
 * - Staking for network participation
 * - Governance voting
 * - Trading on the DEX
 */
contract BOT is ERC20, ERC20Burnable, Ownable, ReentrancyGuard {
    /// @notice Maximum supply cap (1 billion tokens)
    uint256 public constant MAX_SUPPLY = 1_000_000_000 * 10 ** 18;

    /// @notice Faucet cooldown period (1 hour)
    uint256 public constant FAUCET_COOLDOWN = 1 hours;

    /// @notice Faucet amount per request
    uint256 public constant FAUCET_AMOUNT = 1000 * 10 ** 18;

    /// @notice Last faucet request timestamp per address
    mapping(address => uint256) public lastFaucetRequest;

    /// @notice Faucet enabled flag
    bool public faucetEnabled = true;

    /// @notice Emitted when tokens are minted via faucet
    event FaucetMint(address indexed to, uint256 amount);

    /// @notice Emitted when faucet is toggled
    event FaucetToggled(bool enabled);

    /**
     * @notice Initializes the BOT token
     * @dev Mints initial supply to deployer
     */
    constructor() ERC20("Botchain Token", "BOT") Ownable(msg.sender) {
        // Mint initial supply to deployer (1 million tokens)
        _mint(msg.sender, 1_000_000 * 10 ** 18);
    }

    /**
     * @notice Mint tokens to an address (owner only)
     * @param to Recipient address
     * @param amount Amount to mint
     */
    function mint(address to, uint256 amount) external onlyOwner {
        require(
            totalSupply() + amount <= MAX_SUPPLY,
            "BOT: max supply exceeded"
        );
        _mint(to, amount);
    }

    /**
     * @notice Request tokens from the faucet (for testing)
     * @param to Recipient address
     * @param amount Amount to mint (ignored in production, uses FAUCET_AMOUNT)
     * @dev In production, amount parameter is ignored and FAUCET_AMOUNT is used
     */
    function faucet(address to, uint256 amount) external nonReentrant {
        require(faucetEnabled, "BOT: faucet disabled");
        require(to != address(0), "BOT: zero address");

        // In test mode, allow custom amounts without cooldown
        if (block.chainid == 31337) {
            // Hardhat local network - no restrictions
            _mint(to, amount);
            emit FaucetMint(to, amount);
            return;
        }

        // Production mode - enforce cooldown and fixed amount
        require(
            block.timestamp >= lastFaucetRequest[msg.sender] + FAUCET_COOLDOWN,
            "BOT: faucet cooldown active"
        );
        require(
            totalSupply() + FAUCET_AMOUNT <= MAX_SUPPLY,
            "BOT: max supply exceeded"
        );

        lastFaucetRequest[msg.sender] = block.timestamp;
        _mint(to, FAUCET_AMOUNT);

        emit FaucetMint(to, FAUCET_AMOUNT);
    }

    /**
     * @notice Toggle faucet on/off (owner only)
     * @param enabled New faucet state
     */
    function setFaucetEnabled(bool enabled) external onlyOwner {
        faucetEnabled = enabled;
        emit FaucetToggled(enabled);
    }

    /**
     * @notice Get remaining time until faucet available for address
     * @param account Address to check
     * @return Seconds until faucet available (0 if available now)
     */
    function faucetCooldownRemaining(
        address account
    ) external view returns (uint256) {
        if (block.timestamp >= lastFaucetRequest[account] + FAUCET_COOLDOWN) {
            return 0;
        }
        return (lastFaucetRequest[account] + FAUCET_COOLDOWN) - block.timestamp;
    }
}
