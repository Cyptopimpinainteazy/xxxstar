/// Polkawallet Plugin for X3 Chain x3chain
///
/// This Dart package provides the Polkawallet mobile app integration
/// for X3 Chain, including:
/// - x3vm smart contract interaction
/// - Atomic cross-VM trades (EVM/SVM/X3)
/// - Cross-chain settlement (HTLC-based)
/// - .x3 domain registration
/// - Governance (conviction voting, AI proposals)
/// - Treasury management
/// - SVM (Solana VM) operations
library polkawallet_plugin_atlas;

import 'dart:async';
import 'package:polkawallet_sdk/plugin/index.dart';
import 'package:polkawallet_sdk/api/types/networkParams.dart';
import 'package:polkawallet_sdk/storage/keyring.dart';

import 'service/kernel_service.dart';
import 'service/settlement_service.dart';
import 'service/atomic_trade_service.dart';
import 'service/domain_service.dart';
import 'service/verifier_service.dart';
import 'service/governance_service.dart';
import 'service/treasury_service.dart';
import 'service/svm_service.dart';
import 'service/x3vm_service.dart';

class PluginAtlas extends PolkawalletPlugin {
  PluginAtlas()
      : basic = PluginBasicData(
          name: 'x3-chain',
          ss58: 42,
          primaryColor: 0xFF6366F1,
          gradientColor: 0xFFA78BFA,
          backgroundImage: null,
          icon: 'packages/polkawallet_plugin_atlas/assets/x3_logo.svg',
          iconDisabled:
              'packages/polkawallet_plugin_atlas/assets/x3_logo_gray.svg',
          jsCodeVersion: 10000,
          isTestNet: false,
          isXCMSupport: true,
        );

  @override
  final PluginBasicData basic;

  // Service instances
  late final KernelService kernel;
  late final SettlementService settlement;
  late final AtomicTradeService trades;
  late final DomainService domains;
  late final VerifierService verifier;
  late final GovernanceService governance;
  late final TreasuryService treasury;
  late final SvmService svm;
  late final X3VmService x3vm;

  @override
  List<NetworkParams> get nodeList => [
        NetworkParams()
          ..name = 'X3 Chain (Local)'
          ..endpoint = 'ws://127.0.0.1:9944'
          ..ss58 = 42,
        NetworkParams()
          ..name = 'X3 Chain Testnet'
          ..endpoint = 'wss://testnet.x3-chain.io'
          ..ss58 = 42,
        NetworkParams()
          ..name = 'X3 Chain Mainnet'
          ..endpoint = 'wss://rpc.x3-chain.io'
          ..ss58 = 42,
      ];

  @override
  Map<String, Widget Function(BuildContext)> get tokenIcons => {
        'X3': (_) => const Icon(Icons.circle, color: Color(0xFF6366F1)),
      };

  @override
  List<String> get defaultTokens =>
      ['X3', 'xATLAS', 'USDC', 'USDT', 'ETH', 'SOL', 'BTC'];

  @override
  Future<void> onWillStart(Keyring keyring) async {
    // Initialize all services with the JS API bridge
    kernel = KernelService(this);
    settlement = SettlementService(this);
    trades = AtomicTradeService(this);
    domains = DomainService(this);
    verifier = VerifierService(this);
    governance = GovernanceService(this);
    treasury = TreasuryService(this);
    svm = SvmService(this);
    x3vm = X3VmService(this);

    await super.onWillStart(keyring);
  }

  // ---------------------------------------------------------------------------
  // JS Service bridge (loads the compiled JS from polkawallet-plugin)
  // ---------------------------------------------------------------------------

  @override
  Map<String, Function> get jsCallMap => {
        // Kernel
        'kernel.submitComitV2': kernel.submitComitV2,
        'kernel.getBalance': kernel.getBalance,
        'kernel.getAllBalances': kernel.getAllBalances,
        'kernel.getAccount': kernel.getAccount,
        'kernel.getNonce': kernel.getNonce,

        // Settlement
        'settlement.createIntent': settlement.createIntent,
        'settlement.lockEscrow': settlement.lockEscrow,
        'settlement.claimSettlement': settlement.claimSettlement,
        'settlement.refundSettlement': settlement.refundSettlement,
        'settlement.submitBtcProof': settlement.submitBtcProof,
        'settlement.depositBond': settlement.depositBond,
        'settlement.getIntent': settlement.getIntent,

        // Atomic Trades
        'trades.createTradeBatch': trades.createTradeBatch,
        'trades.executeTradeBatch': trades.executeTradeBatch,
        'trades.swap': trades.swap,
        'trades.cancelTradeBatch': trades.cancelTradeBatch,
        'trades.getBatch': trades.getBatch,
        'trades.getTwap': trades.getTwap,

        // Domains
        'domains.registerDomain': domains.registerDomain,
        'domains.setRecords': domains.setRecords,
        'domains.getDomain': domains.getDomain,
        'domains.resolve': domains.resolve,
        'domains.isDomainAvailable': domains.isDomainAvailable,

        // Verifier / x3vm
        'verifier.submitJob': verifier.submitJob,
        'verifier.getJob': verifier.getJob,
        'x3vm.compile': x3vm.compile,
        'x3vm.deploy': x3vm.deploy,
        'x3vm.call': x3vm.call,
        'x3vm.flashLoan': x3vm.flashLoan,

        // Governance
        'governance.submitProposal': governance.submitProposal,
        'governance.vote': governance.vote,
        'governance.delegate': governance.delegate,
        'governance.getActiveProposals': governance.getActiveProposals,
        'governance.getKillSwitchLevel': governance.getKillSwitchLevel,

        // Treasury
        'treasury.submitProposal': treasury.submitProposal,
        'treasury.deposit': treasury.deposit,
        'treasury.getStats': treasury.getStats,
      };
}
