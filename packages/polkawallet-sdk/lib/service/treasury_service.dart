import 'package:polkawallet_sdk/plugin/index.dart';

/// Treasury service — spending proposals, yield strategies
class TreasuryService {
  final PolkawalletPlugin plugin;
  TreasuryService(this.plugin);

  Future<Map<String, dynamic>> submitProposal(Map<String, dynamic> params) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.treasury.submitProposal(account, ${_encode(params)})',
      ),
    );
  }

  Future<Map<String, dynamic>> deposit(String amount) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.treasury.deposit(account, "$amount")',
      ),
    );
  }

  Future<Map<String, dynamic>?> getStats() async {
    final res = await plugin.sdk.webView!.evalJavascript(
      'plugin.treasury.getStats()',
    );
    return res != null ? Map<String, dynamic>.from(res) : null;
  }

  String _encode(Map<String, dynamic> params) {
    return Uri.encodeComponent(params.toString());
  }
}
