// SPDX-License-Identifier: Apache-2.0
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {X3Flashloan} from "../../contracts/flashloan/X3Flashloan.sol";
import {IX3FlashloanReceiver} from "../../contracts/interfaces/IX3FlashloanReceiver.sol";

/// @notice Minimal mintable ERC20 used by flashloan tests. Hand-rolled to keep
///         the Foundry workspace dependency-free at launch.
contract MockERC20 {
    string public name = "MOCK";
    string public symbol = "MOCK";
    uint8 public constant decimals = 18;
    mapping(address => uint256) public balanceOf;

    function mint(address to, uint256 amount) external {
        balanceOf[to] += amount;
    }

    function transfer(address to, uint256 amount) external returns (bool) {
        balanceOf[msg.sender] -= amount;
        balanceOf[to] += amount;
        return true;
    }

    function transferFrom(address from, address to, uint256 amount) external returns (bool) {
        balanceOf[from] -= amount;
        balanceOf[to] += amount;
        return true;
    }
}

/// @notice Honest borrower: repays principal + fee, returns the OK ack.
contract HonestBorrower is IX3FlashloanReceiver {
    bytes32 internal constant OK = keccak256("X3Flashloan.onFlashloan.OK");

    function onFlashloan(
        address asset,
        uint256 amount,
        uint256 fee,
        bytes calldata
    ) external returns (bytes32) {
        MockERC20(asset).transfer(msg.sender, amount + fee);
        return OK;
    }
}

/// @notice Dishonest borrower: keeps the principal, returns a wrong ack.
contract DeadbeatBorrower is IX3FlashloanReceiver {
    function onFlashloan(
        address,
        uint256,
        uint256,
        bytes calldata
    ) external pure returns (bytes32) {
        return bytes32(uint256(0xbad));
    }
}

/// @notice Dishonest borrower: returns the OK ack but underpays by 1 wei.
contract UnderpayBorrower is IX3FlashloanReceiver {
    bytes32 internal constant OK = keccak256("X3Flashloan.onFlashloan.OK");

    function onFlashloan(
        address asset,
        uint256 amount,
        uint256 fee,
        bytes calldata
    ) external returns (bytes32) {
        uint256 owed = amount + fee;
        if (owed > 0) {
            MockERC20(asset).transfer(msg.sender, owed - 1);
        }
        return OK;
    }
}

contract X3FlashloanTest is Test {
    MockERC20 internal token;
    X3Flashloan internal pool;

    function setUp() public {
        token = new MockERC20();
        pool = new X3Flashloan(9); // 9 bps == 0.09%
        token.mint(address(pool), 1_000_000 ether);
    }

    function test_HonestBorrowerRepays() public {
        HonestBorrower borrower = new HonestBorrower();
        // Pre-fund the borrower with the fee so it can repay > principal.
        uint256 amount = 100 ether;
        uint256 fee = pool.quoteFee(amount);
        token.mint(address(borrower), fee);

        uint256 prePool = token.balanceOf(address(pool));
        pool.flashloan(address(token), amount, address(borrower), "");
        uint256 postPool = token.balanceOf(address(pool));

        assertEq(postPool, prePool + fee, "pool must accrue fee");
    }

    function test_DeadbeatBorrowerReverts() public {
        DeadbeatBorrower borrower = new DeadbeatBorrower();
        vm.expectRevert(X3Flashloan.CallbackFailed.selector);
        pool.flashloan(address(token), 100 ether, address(borrower), "");
    }

    function test_UnderpayBorrowerReverts() public {
        UnderpayBorrower borrower = new UnderpayBorrower();
        uint256 amount = 100 ether;
        uint256 fee = pool.quoteFee(amount);
        token.mint(address(borrower), fee);

        vm.expectRevert();
        pool.flashloan(address(token), amount, address(borrower), "");
    }

    function testFuzz_FeeIsAlwaysAdditive(uint128 amount) public {
        // Fee must round up so a sequence of 1-wei loans cannot drain.
        vm.assume(amount > 0);
        uint256 fee = pool.quoteFee(amount);
        assertGe(fee, 1, "non-zero amount must accrue at least 1 wei fee");
    }
}
