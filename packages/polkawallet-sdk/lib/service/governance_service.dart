import 'package:polkawallet_sdk/plugin/index.dart';

/// Governance service — proposals, voting, AI governance
class GovernanceService {
  final PolkawalletPlugin plugin;
  GovernanceService(this.plugin);

  Future<Map<String, dynamic>> submitProposal(Map<String, dynamic> params) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.governance.submitProposal(account, ${_encode(params)})',
      ),
    );
  }

  Future<Map<String, dynamic>> vote(Map<String, dynamic> params) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.governance.vote(account, ${_encode(params)})',
      ),
    );
  }

  Future<Map<String, dynamic>> delegate(Map<String, dynamic> params) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.governance.delegate(account, ${_encode(params)})',
      ),
    );
  }

  Future<List<int>> getActiveProposals() async {
    final res = await plugin.sdk.webView!.evalJavascript(
      'plugin.governance.getActiveProposals()',
    );
    return List<int>.from(res ?? []);
  }

  Future<String> getKillSwitchLevel() async {
    final res = await plugin.sdk.webView!.evalJavascript(
      'plugin.governance.getKillSwitchLevel()',
    );
    return res.toString();
  }

  String _encode(Map<String, dynamic> params) {
    return Uri.encodeComponent(params.toString());
  }
}
