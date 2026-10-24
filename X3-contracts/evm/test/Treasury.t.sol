// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Test.sol";
import "../contracts/treasury/Treasury.sol";

contract ThresholdOwner {
    struct Proposal {
        address target;
        bytes data;
        uint256 approvals;
        bool executed;
        mapping(address => bool) approved;
    }

    mapping(address => bool) public isSigner;
    uint256 public threshold;
    uint256 public nonce;
    mapping(bytes32 => Proposal) private proposals;

    address public reentryTarget;
    bytes public reentryCall;
    bool public reenterOnce;

    constructor(address[3] memory signers, uint256 threshold_) {
        require(threshold_ > 0 && threshold_ <= signers.length, "INVALID_THRESHOLD");
        for (uint256 i = 0; i < signers.length; i++) {
            require(signers[i] != address(0) && !isSigner[signers[i]], "INVALID_SIGNER");
            isSigner[signers[i]] = true;
        }
        threshold = threshold_;
    }

    function propose(address target, bytes calldata data) external returns (bytes32 id) {
        require(isSigner[msg.sender], "NOT_SIGNER");
        require(target != address(0), "ZERO_TARGET");
        id = keccak256(abi.encode(target, data, nonce++));
        Proposal storage proposal = proposals[id];
        proposal.target = target;
        proposal.data = data;
        proposal.approved[msg.sender] = true;
        proposal.approvals = 1;
    }

    function approve(bytes32 id) external {
        Proposal storage proposal = proposals[id];
        require(isSigner[msg.sender], "NOT_SIGNER");
        require(proposal.target != address(0), "UNKNOWN_PROPOSAL");
        require(!proposal.executed, "ALREADY_EXECUTED");
        require(!proposal.approved[msg.sender], "ALREADY_APPROVED");
        proposal.approved[msg.sender] = true;
        proposal.approvals++;
    }

    function execute(bytes32 id) external returns (bytes memory result) {
        Proposal storage proposal = proposals[id];
        require(proposal.target != address(0), "UNKNOWN_PROPOSAL");
        require(!proposal.executed, "ALREADY_EXECUTED");
        require(proposal.approvals >= threshold, "THRESHOLD_NOT_MET");
        proposal.executed = true;

        bool success;
        (success, result) = proposal.target.call(proposal.data);
        if (!success) {
            assembly {
                revert(add(result, 32), mload(result))
            }
        }
    }

    function armReentry(address target, bytes calldata data) external {
        reentryTarget = target;
        reentryCall = data;
        reenterOnce = true;
    }

    receive() external payable {
        if (reenterOnce) {
            reenterOnce = false;
            (bool success, bytes memory result) = reentryTarget.call(reentryCall);
            if (!success) {
                assembly {
                    revert(add(result, 32), mload(result))
                }
            }
        }
    }
}

contract EtherSender {
    function send(address payable recipient) external payable {
        (bool success,) = recipient.call{value: msg.value}("");
        require(success, "SEND_FAILED");
    }
}

contract TreasuryTest is Test {
    Treasury public treasury;
    ThresholdOwner public safe;
    address public dev = payable(address(0xDDD));
    address public dao = payable(address(0xAAA));
    address public lp = payable(address(0x3));
    address public user = address(0x123);
    address public attacker = address(0xBAD);
    address public signerOne = address(0xA11CE);
    address public signerTwo = address(0xB0B);
    address public signerThree = address(0xCAFE);

    function setUp() public {
        address[3] memory signers = [signerOne, signerTwo, signerThree];
        safe = new ThresholdOwner(signers, 2);
        treasury = new Treasury(dev, dao, lp, address(safe));
        vm.deal(address(treasury), 100 ether);
    }

    function testConstructor() public view {
        assertEq(treasury.devWallet(), dev);
        assertEq(treasury.daoWallet(), dao);
        assertEq(treasury.lpWallet(), lp);
        assertEq(treasury.devSplitBps(), 2000);
        assertEq(treasury.daoSplitBps(), 5000);
        assertEq(treasury.lpSplitBps(), 3000);
        assertEq(treasury.owner(), address(safe));
    }

    function testConstructorZeroDev() public {
        vm.expectRevert("ZERO_DEV");
        new Treasury(address(0), dao, lp, address(safe));
    }

    function testConstructorZeroDao() public {
        vm.expectRevert("ZERO_DAO");
        new Treasury(dev, address(0), lp, address(safe));
    }

    function testConstructorZeroLp() public {
        vm.expectRevert("ZERO_LP");
        new Treasury(dev, dao, address(0), address(safe));
    }

    function testConstructorRejectsEOAOwner() public {
        vm.expectRevert("OWNER_NOT_CONTRACT");
        new Treasury(dev, dao, lp, attacker);
    }

    function testConstructorZeroOwner() public {
        vm.expectRevert("OWNER_NOT_CONTRACT");
        new Treasury(dev, dao, lp, address(0));
    }

    function testSetSplits() public {
        _executeAsOwner(abi.encodeCall(treasury.setSplits, (3000, 4000, 3000)));
        assertEq(treasury.devSplitBps(), 3000);
        assertEq(treasury.daoSplitBps(), 4000);
        assertEq(treasury.lpSplitBps(), 3000);
    }

    function testSetSplitsInvalidSum() public {
        bytes32 id = _proposeAsOwner(abi.encodeCall(treasury.setSplits, (3000, 3000, 3000)));
        vm.prank(signerTwo);
        safe.approve(id);
        vm.expectRevert("Must sum to 10000");
        safe.execute(id);
    }

    function testSetSplitsUnauthorized() public {
        vm.prank(attacker);
        vm.expectRevert();
        treasury.setSplits(3000, 4000, 3000);
    }

    function testRouteFeeOnlyOwner() public {
        vm.prank(attacker);
        vm.expectRevert();
        treasury.routeFee(user, 1 ether, "test");
    }

    function testRouteFee() public {
        uint256 devBefore = dev.balance;
        uint256 daoBefore = dao.balance;
        uint256 lpBefore = lp.balance;
        uint256 treasuryBefore = address(treasury).balance;

        _executeAsOwner(abi.encodeCall(treasury.routeFee, (user, 100 ether, "fees")));

        uint256 expectedDev = (100 ether * 2000) / 10000;
        uint256 expectedDao = (100 ether * 5000) / 10000;
        uint256 expectedLp = 100 ether - expectedDev - expectedDao;

        assertEq(dev.balance - devBefore, expectedDev);
        assertEq(dao.balance - daoBefore, expectedDao);
        assertEq(lp.balance - lpBefore, expectedLp);
        assertEq(expectedDev + expectedDao + expectedLp, 100 ether);
        assertEq(treasuryBefore - address(treasury).balance, 100 ether);
        assertEq(treasury.accounting(user), 100 ether);
    }

    function testRouteFeeZeroFrom() public {
        bytes32 id = _proposeAsOwner(abi.encodeCall(treasury.routeFee, (address(0), 1 ether, "test")));
        vm.prank(signerTwo);
        safe.approve(id);
        vm.expectRevert("ZERO_FROM");
        safe.execute(id);
    }

    function testRouteFeeZeroAmount() public {
        bytes32 id = _proposeAsOwner(abi.encodeCall(treasury.routeFee, (user, 0, "test")));
        vm.prank(signerTwo);
        safe.approve(id);
        vm.expectRevert("ZERO_AMOUNT");
        safe.execute(id);
    }

    function testRouteFeeRequiresThreshold() public {
        bytes32 id = _proposeAsOwner(abi.encodeCall(treasury.routeFee, (user, 1 ether, "test")));
        vm.expectRevert("THRESHOLD_NOT_MET");
        safe.execute(id);

        vm.prank(signerTwo);
        safe.approve(id);
        safe.execute(id);
        assertEq(treasury.accounting(user), 1 ether);
    }

    function testRouteFeeRejectsInsufficientBalance() public {
        vm.deal(address(treasury), 0);
        bytes32 id = _proposeAsOwner(abi.encodeCall(treasury.routeFee, (user, 1 ether, "test")));
        vm.prank(signerTwo);
        safe.approve(id);
        vm.expectRevert("INSUFFICIENT_BALANCE");
        safe.execute(id);
    }

    function testPauseBlocksRoutingAndDeposits() public {
        _executeAsOwner(abi.encodeCall(treasury.pause, ()));
        assertTrue(treasury.paused());

        bytes32 id = _proposeAsOwner(abi.encodeCall(treasury.routeFee, (user, 1 ether, "test")));
        vm.prank(signerTwo);
        safe.approve(id);
        vm.expectRevert("Pausable: paused");
        safe.execute(id);

        EtherSender sender = new EtherSender();
        vm.expectRevert("SEND_FAILED");
        sender.send{value: 1 ether}(payable(address(treasury)));
    }

    function testUnpauseRestoresRouting() public {
        _executeAsOwner(abi.encodeCall(treasury.pause, ()));
        _executeAsOwner(abi.encodeCall(treasury.unpause, ()));
        _executeAsOwner(abi.encodeCall(treasury.routeFee, (user, 1 ether, "test")));
        assertEq(treasury.accounting(user), 1 ether);
    }

    function testReentrantOwnerPayoutIsRejected() public {
        ThresholdOwner owner = new ThresholdOwner([signerOne, signerTwo, signerThree], 2);
        Treasury guarded = new Treasury(address(owner), dao, lp, address(owner));
        vm.deal(address(guarded), 10 ether);
        owner.armReentry(address(guarded), abi.encodeCall(guarded.routeFee, (user, 1 ether, "reentry")));

        vm.prank(signerOne);
        bytes32 id = owner.propose(address(guarded), abi.encodeCall(guarded.routeFee, (user, 1 ether, "test")));
        vm.prank(signerTwo);
        owner.approve(id);
        vm.expectRevert("DEV_TRANSFER_FAILED");
        owner.execute(id);
        assertEq(guarded.accounting(user), 0);
    }

    function testOwnershipTransferRequiresContractAcceptance() public {
        ThresholdOwner nextOwner = new ThresholdOwner([signerOne, signerTwo, signerThree], 2);
        _executeAsOwner(abi.encodeCall(treasury.transferOwnership, (address(nextOwner))));
        assertEq(treasury.pendingOwner(), address(nextOwner));
        assertEq(treasury.owner(), address(safe));

        vm.prank(attacker);
        vm.expectRevert("Ownable2Step: caller is not the new owner");
        treasury.acceptOwnership();

        vm.prank(signerOne);
        bytes32 id = nextOwner.propose(address(treasury), abi.encodeCall(treasury.acceptOwnership, ()));
        vm.prank(signerTwo);
        nextOwner.approve(id);
        nextOwner.execute(id);
        assertEq(treasury.owner(), address(nextOwner));
        assertEq(treasury.pendingOwner(), address(0));
    }

    function testOwnershipTransferRejectsEOA() public {
        bytes32 id = _proposeAsOwner(abi.encodeCall(treasury.transferOwnership, (attacker)));
        vm.prank(signerTwo);
        safe.approve(id);
        vm.expectRevert("OWNER_NOT_CONTRACT");
        safe.execute(id);
        assertEq(treasury.owner(), address(safe));
        assertEq(treasury.pendingOwner(), address(0));
    }

    function testRenounceOwnershipDisabled() public {
        bytes32 id = _proposeAsOwner(abi.encodeCall(treasury.renounceOwnership, ()));
        vm.prank(signerTwo);
        safe.approve(id);
        vm.expectRevert("RENOUNCE_DISABLED");
        safe.execute(id);
        assertEq(treasury.owner(), address(safe));
    }

    function testReceiveEther() public {
        uint256 contractBalanceBefore = address(treasury).balance;
        (bool ok,) = address(treasury).call{value: 1 ether}("");
        assertTrue(ok);
        assertEq(address(treasury).balance, contractBalanceBefore + 1 ether);
    }

    function _proposeAsOwner(bytes memory data) internal returns (bytes32 id) {
        vm.prank(signerOne);
        id = safe.propose(address(treasury), data);
    }

    function _executeAsOwner(bytes memory data) internal returns (bytes memory result) {
        bytes32 id = _proposeAsOwner(data);
        vm.prank(signerTwo);
        safe.approve(id);
        return safe.execute(id);
    }
}
