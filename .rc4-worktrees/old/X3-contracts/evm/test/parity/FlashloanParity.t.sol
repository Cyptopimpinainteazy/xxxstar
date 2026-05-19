// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Test} from "forge-std/Test.sol";
import {stdJson} from "forge-std/StdJson.sol";
import {X3Flashloan} from "../../contracts/flashloan/X3Flashloan.sol";
import {IX3FlashloanReceiver} from "../../contracts/interfaces/IX3FlashloanReceiver.sol";

/// @dev Mintable mock used by the parity harness.
contract MockERC20 {
    string public constant name = "Mock";
    string public constant symbol = "MCK";
    uint8 public constant decimals = 18;
    mapping(address => uint256) public balanceOf;

    function mint(address to, uint256 amount) external {
        balanceOf[to] += amount;
    }

    function transfer(address to, uint256 amount) external returns (bool) {
        require(balanceOf[msg.sender] >= amount, "balance");
        balanceOf[msg.sender] -= amount;
        balanceOf[to] += amount;
        return true;
    }

    function transferFrom(address from, address to, uint256 amount) external returns (bool) {
        require(balanceOf[from] >= amount, "balance");
        balanceOf[from] -= amount;
        balanceOf[to] += amount;
        return true;
    }
}

contract HonestBorrower is IX3FlashloanReceiver {
    bytes32 private constant OK = keccak256("X3Flashloan.onFlashloan.OK");
    function onFlashloan(address asset, uint256 amount, uint256 fee, bytes calldata)
        external
        returns (bytes32)
    {
        MockERC20(asset).transfer(msg.sender, amount + fee);
        return OK;
    }
}

contract DeadbeatBorrower is IX3FlashloanReceiver {
    function onFlashloan(address asset, uint256 amount, uint256 fee, bytes calldata)
        external
        returns (bytes32)
    {
        MockERC20(asset).transfer(msg.sender, amount + fee);
        return bytes32("WRONG_ACK");
    }
}

contract UnderpayBorrower is IX3FlashloanReceiver {
    bytes32 private constant OK = keccak256("X3Flashloan.onFlashloan.OK");
    function onFlashloan(address asset, uint256 amount, uint256 fee, bytes calldata)
        external
        returns (bytes32)
    {
        MockERC20(asset).transfer(msg.sender, amount + fee - 1);
        return OK;
    }
}

/// @notice Drives every JSON vector in
///         `X3-contracts/shared/test-vectors/flashloan_repay_or_revert.json`
///         through the EVM `X3Flashloan` and asserts the on-chain behavior
///         matches the spec doc. Companion to the Rust harness in
///         `X3-contracts/shared/parity-core/tests/parity_vectors.rs`.
contract FlashloanParityTest is Test {
    using stdJson for string;

    X3Flashloan internal pool;
    MockERC20 internal asset;
    uint16 internal feeBps;

    function setUp() public {
        // Seed the pool with enough liquidity to satisfy every vector amount
        // (largest in the published file is 100e18).
        asset = new MockERC20();
        feeBps = 9;
        pool = new X3Flashloan(feeBps);
        asset.mint(address(pool), 1_000_000 ether);
    }

    function _vectorsJson() internal view returns (string memory) {
        // Foundry runs from X3-contracts/evm; vectors live two levels up.
        string memory root = vm.projectRoot();
        string memory path = string.concat(root, "/../shared/test-vectors/flashloan_repay_or_revert.json");
        return vm.readFile(path);
    }

    function testParityVectors() public {
        string memory json = _vectorsJson();

        uint16 jsonFeeBps = uint16(json.readUint(".fee_bps"));
        assertEq(jsonFeeBps, feeBps, "fee_bps drift between JSON and EVM setup");

        uint256 count = json.readUintArray(".vectors[*].amount").length;
        assertGt(count, 0, "no vectors found");

        for (uint256 i = 0; i < count; i++) {
            string memory base = string.concat(".vectors[", vm.toString(i), "]");
            string memory id = json.readString(string.concat(base, ".id"));
            uint256 amount = json.readUint(string.concat(base, ".amount"));
            string memory kind = json.readString(string.concat(base, ".borrower_kind"));
            string memory result = json.readString(string.concat(base, ".expected.result"));

            uint256 prePoolBal = asset.balanceOf(address(pool));

            if (_eq(kind, "honest")) {
                HonestBorrower b = new HonestBorrower();
                asset.mint(address(b), pool.quoteFee(amount));
                if (_eq(result, "ok")) {
                    pool.flashloan(address(asset), amount, address(b), bytes(""));
                    uint256 want = json.readUint(string.concat(base, ".expected.pool_delta"));
                    uint256 got = asset.balanceOf(address(pool)) - prePoolBal;
                    assertEq(got, want, string.concat(id, ": pool_delta mismatch"));
                } else {
                    revert(string.concat(id, ": honest vector marked revert is unsupported by harness"));
                }
            } else if (_eq(kind, "deadbeat")) {
                DeadbeatBorrower b = new DeadbeatBorrower();
                asset.mint(address(b), pool.quoteFee(amount));
                vm.expectRevert(X3Flashloan.CallbackFailed.selector);
                pool.flashloan(address(asset), amount, address(b), bytes(""));
                assertEq(asset.balanceOf(address(pool)), prePoolBal, string.concat(id, ": revert leaked balance"));
            } else if (_eq(kind, "underpay")) {
                UnderpayBorrower b = new UnderpayBorrower();
                asset.mint(address(b), pool.quoteFee(amount));
                vm.expectRevert(X3Flashloan.NotRepaid.selector);
                pool.flashloan(address(asset), amount, address(b), bytes(""));
                assertEq(asset.balanceOf(address(pool)), prePoolBal, string.concat(id, ": revert leaked balance"));
            } else {
                revert(string.concat(id, ": unknown borrower_kind"));
            }
        }
    }

    function _eq(string memory a, string memory b) private pure returns (bool) {
        return keccak256(bytes(a)) == keccak256(bytes(b));
    }
}
