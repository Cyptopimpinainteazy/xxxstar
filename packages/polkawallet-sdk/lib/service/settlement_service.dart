import 'package:polkawallet_sdk/plugin/index.dart';

/// Settlement service — cross-chain atomic settlement
class SettlementService {
  final PolkawalletPlugin plugin;
  SettlementService(this.plugin);

  Future<Map<String, dynamic>> createIntent(Map<String, dynamic> params) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.settlement.createIntent(account, ${_encode(params)})',
      ),
    );
  }

  Future<Map<String, dynamic>> lockEscrow(Map<String, dynamic> params) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.settlement.lockEscrow(account, ${_encode(params)})',
      ),
    );
  }

  Future<Map<String, dynamic>> claimSettlement(String intentId, String secret) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.settlement.claimSettlement(account, "$intentId", "$secret")',
      ),
    );
  }

  Future<Map<String, dynamic>> refundSettlement(String intentId) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.settlement.refundSettlement(account, "$intentId")',
      ),
    );
  }

  Future<Map<String, dynamic>> submitBtcProof(Map<String, dynamic> params) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.settlement.submitBtcProof(account, ${_encode(params)})',
      ),
    );
  }

  Future<Map<String, dynamic>> depositBond(Map<String, dynamic> params) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.settlement.depositBond(account, ${_encode(params)})',
      ),
    );
  }

  Future<Map<String, dynamic>?> getIntent(String intentId) async {
    final res = await plugin.sdk.webView!.evalJavascript(
      'plugin.settlement.getIntent("$intentId")',
    );
    return res != null ? Map<String, dynamic>.from(res) : null;
  }

  String _encode(Map<String, dynamic> params) {
    return Uri.encodeComponent(params.toString());
  }
}
