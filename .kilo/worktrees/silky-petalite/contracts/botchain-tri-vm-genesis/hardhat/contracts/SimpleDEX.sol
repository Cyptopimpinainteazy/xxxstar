// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/utils/math/Math.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

/**
 * @title SimpleDEX
 * @author Botchain Team
 * @notice Minimal Uniswap V2-style AMM DEX for BOT token trading
 * @dev Implements constant product (x*y=k) AMM with LP tokens
 *
 * Features:
 * - Add/remove liquidity
 * - Token swaps with 0.3% fee
 * - LP token minting/burning
 * - Price oracle (TWAP placeholder)
 */
contract SimpleDEX is ReentrancyGuard, Ownable {
    using SafeERC20 for IERC20;

    // ============ State Variables ============

    /// @notice Token A (e.g., BOT)
    IERC20 public immutable tokenA;

    /// @notice Token B (e.g., WETH or stablecoin)
    IERC20 public immutable tokenB;

    /// @notice Reserve of token A
    uint256 public reserveA;

    /// @notice Reserve of token B
    uint256 public reserveB;

    /// @notice Total LP tokens supply
    uint256 public totalSupply;

    /// @notice LP token balances
    mapping(address => uint256) public balanceOf;

    /// @notice Trading fee (0.3% = 30 basis points)
    uint256 public constant FEE_BPS = 30;

    /// @notice Basis points denominator
    uint256 public constant BPS_DENOMINATOR = 10000;

    /// @notice Minimum liquidity locked forever (prevents division by zero)
    uint256 public constant MINIMUM_LIQUIDITY = 1000;

    // ============ Events ============

    /// @notice Emitted when liquidity is added
    event LiquidityAdded(
        address indexed provider,
        uint256 amountA,
        uint256 amountB,
        uint256 lpTokens
    );

    /// @notice Emitted when liquidity is removed
    event LiquidityRemoved(
        address indexed provider,
        uint256 amountA,
        uint256 amountB,
        uint256 lpTokens
    );

    /// @notice Emitted on token swap
    event Swap(
        address indexed user,
        address tokenIn,
        uint256 amountIn,
        address tokenOut,
        uint256 amountOut
    );

    /// @notice Emitted on sync (reserves updated)
    event Sync(uint256 reserveA, uint256 reserveB);

    // ============ Constructor ============

    /**
     * @notice Initialize DEX with token pair
     * @param _tokenA First token address
     * @param _tokenB Second token address
     */
    constructor(address _tokenA, address _tokenB) Ownable(msg.sender) {
        require(
            _tokenA != address(0) && _tokenB != address(0),
            "Invalid tokens"
        );
        require(_tokenA != _tokenB, "Identical tokens");
        tokenA = IERC20(_tokenA);
        tokenB = IERC20(_tokenB);
    }

    // ============ Core Functions ============

    /**
     * @notice Add liquidity to the pool
     * @param amountA Amount of token A to add
     * @param amountB Amount of token B to add
     * @param minLpTokens Minimum LP tokens to receive
     * @return lpTokens Amount of LP tokens minted
     */
    function addLiquidity(
        uint256 amountA,
        uint256 amountB,
        uint256 minLpTokens
    ) external nonReentrant returns (uint256 lpTokens) {
        require(amountA > 0 && amountB > 0, "Amounts must be positive");

        // Transfer tokens in
        tokenA.safeTransferFrom(msg.sender, address(this), amountA);
        tokenB.safeTransferFrom(msg.sender, address(this), amountB);

        if (totalSupply == 0) {
            // First liquidity provision
            lpTokens = (Math.sqrt(amountA) * Math.sqrt(amountB)) -
                MINIMUM_LIQUIDITY;
            require(lpTokens > 0, "Insufficient initial liquidity");

            // Lock minimum liquidity forever (to address(0))
            totalSupply = MINIMUM_LIQUIDITY;
            balanceOf[address(0)] = MINIMUM_LIQUIDITY;
        } else {
            // Subsequent liquidity provision
            // LP tokens based on proportion added
            uint256 lpFromA = (amountA * totalSupply) / reserveA;
            uint256 lpFromB = (amountB * totalSupply) / reserveB;
            lpTokens = lpFromA < lpFromB ? lpFromA : lpFromB;
        }

        require(lpTokens >= minLpTokens, "Insufficient LP tokens");

        // Mint LP tokens
        balanceOf[msg.sender] += lpTokens;
        totalSupply += lpTokens;

        // Update reserves
        reserveA += amountA;
        reserveB += amountB;

        emit LiquidityAdded(msg.sender, amountA, amountB, lpTokens);
        emit Sync(reserveA, reserveB);
    }

    /**
     * @notice Remove liquidity from the pool
     * @param lpTokens Amount of LP tokens to burn
     * @param minAmountA Minimum token A to receive
     * @param minAmountB Minimum token B to receive
     * @return amountA Token A received
     * @return amountB Token B received
     */
    function removeLiquidity(
        uint256 lpTokens,
        uint256 minAmountA,
        uint256 minAmountB
    ) external nonReentrant returns (uint256 amountA, uint256 amountB) {
        require(lpTokens > 0, "LP tokens must be positive");
        require(balanceOf[msg.sender] >= lpTokens, "Insufficient LP tokens");

        // Calculate proportional amounts
        amountA = (lpTokens * reserveA) / totalSupply;
        amountB = (lpTokens * reserveB) / totalSupply;

        require(amountA >= minAmountA, "Insufficient amount A");
        require(amountB >= minAmountB, "Insufficient amount B");

        // Burn LP tokens
        balanceOf[msg.sender] -= lpTokens;
        totalSupply -= lpTokens;

        // Update reserves
        reserveA -= amountA;
        reserveB -= amountB;

        // Transfer tokens out
        tokenA.safeTransfer(msg.sender, amountA);
        tokenB.safeTransfer(msg.sender, amountB);

        emit LiquidityRemoved(msg.sender, amountA, amountB, lpTokens);
        emit Sync(reserveA, reserveB);
    }

    /**
     * @notice Swap token A for token B
     * @param amountIn Amount of token A to swap
     * @param minAmountOut Minimum token B to receive
     * @return amountOut Token B received
     */
    function swapAForB(
        uint256 amountIn,
        uint256 minAmountOut
    ) external nonReentrant returns (uint256 amountOut) {
        require(amountIn > 0, "Amount must be positive");
        require(reserveA > 0 && reserveB > 0, "No liquidity");

        // Calculate output with fee
        amountOut = _getAmountOut(amountIn, reserveA, reserveB);
        require(amountOut >= minAmountOut, "Insufficient output");
        require(amountOut < reserveB, "Insufficient liquidity");

        // Transfer in
        tokenA.safeTransferFrom(msg.sender, address(this), amountIn);

        // Update reserves
        reserveA += amountIn;
        reserveB -= amountOut;

        // Transfer out
        tokenB.safeTransfer(msg.sender, amountOut);

        emit Swap(
            msg.sender,
            address(tokenA),
            amountIn,
            address(tokenB),
            amountOut
        );
        emit Sync(reserveA, reserveB);
    }

    /**
     * @notice Swap token B for token A
     * @param amountIn Amount of token B to swap
     * @param minAmountOut Minimum token A to receive
     * @return amountOut Token A received
     */
    function swapBForA(
        uint256 amountIn,
        uint256 minAmountOut
    ) external nonReentrant returns (uint256 amountOut) {
        require(amountIn > 0, "Amount must be positive");
        require(reserveA > 0 && reserveB > 0, "No liquidity");

        // Calculate output with fee
        amountOut = _getAmountOut(amountIn, reserveB, reserveA);
        require(amountOut >= minAmountOut, "Insufficient output");
        require(amountOut < reserveA, "Insufficient liquidity");

        // Transfer in
        tokenB.safeTransferFrom(msg.sender, address(this), amountIn);

        // Update reserves
        reserveB += amountIn;
        reserveA -= amountOut;

        // Transfer out
        tokenA.safeTransfer(msg.sender, amountOut);

        emit Swap(
            msg.sender,
            address(tokenB),
            amountIn,
            address(tokenA),
            amountOut
        );
        emit Sync(reserveA, reserveB);
    }

    // ============ View Functions ============

    /**
     * @notice Get current price of token A in terms of token B
     * @return price Price with 18 decimals precision
     */
    function getPriceAInB() external view returns (uint256 price) {
        require(reserveA > 0, "No liquidity");
        return (reserveB * 1e18) / reserveA;
    }

    /**
     * @notice Get current price of token B in terms of token A
     * @return price Price with 18 decimals precision
     */
    function getPriceBInA() external view returns (uint256 price) {
        require(reserveB > 0, "No liquidity");
        return (reserveA * 1e18) / reserveB;
    }

    /**
     * @notice Get expected output for swapping A to B
     * @param amountIn Amount of token A
     * @return amountOut Expected token B output
     */
    function getAmountOutAToB(
        uint256 amountIn
    ) external view returns (uint256 amountOut) {
        return _getAmountOut(amountIn, reserveA, reserveB);
    }

    /**
     * @notice Get expected output for swapping B to A
     * @param amountIn Amount of token B
     * @return amountOut Expected token A output
     */
    function getAmountOutBToA(
        uint256 amountIn
    ) external view returns (uint256 amountOut) {
        return _getAmountOut(amountIn, reserveB, reserveA);
    }

    /**
     * @notice Get reserves
     * @return _reserveA Reserve of token A
     * @return _reserveB Reserve of token B
     */
    function getReserves()
        external
        view
        returns (uint256 _reserveA, uint256 _reserveB)
    {
        return (reserveA, reserveB);
    }

    // ============ Internal Functions ============

    /**
     * @notice Calculate output amount for a swap
     * @dev Uses constant product formula with fee
     */
    function _getAmountOut(
        uint256 amountIn,
        uint256 reserveIn,
        uint256 reserveOut
    ) internal pure returns (uint256 amountOut) {
        require(amountIn > 0, "Insufficient input");
        require(reserveIn > 0 && reserveOut > 0, "Insufficient liquidity");

        // Apply fee (0.3%)
        uint256 amountInWithFee = amountIn * (BPS_DENOMINATOR - FEE_BPS);
        uint256 numerator = amountInWithFee * reserveOut;
        uint256 denominator = (reserveIn * BPS_DENOMINATOR) + amountInWithFee;

        amountOut = numerator / denominator;
    }

    /**
     * @notice Calculate square root (Babylonian method)
     */
    function _sqrt(uint256 y) internal pure returns (uint256 z) {
        if (y > 3) {
            z = y;
            uint256 x = y / 2 + 1;
            while (x < z) {
                z = x;
                x = (y / x + x) / 2;
            }
        } else if (y != 0) {
            z = 1;
        }
    }
}
