import 'package:polkawallet_sdk/plugin/index.dart';

/// Kernel service — Comit submission and account management
class KernelService {
  final PolkawalletPlugin plugin;
  KernelService(this.plugin);

  Future<Map<String, dynamic>> submitComitV2(Map<String, dynamic> params) async {
    final res = await plugin.sdk.webView!.evalJavascript(
      'plugin.kernel.submitComitV2(account, ${_encode(params)})',
    );
    return Map<String, dynamic>.from(res);
  }

  Future<String> getBalance(String account, int assetId) async {
    final res = await plugin.sdk.webView!.evalJavascript(
      'plugin.kernel.getBalance("$account", $assetId)',
    );
    return res.toString();
  }

  Future<List<Map<String, dynamic>>> getAllBalances(String account) async {
    final res = await plugin.sdk.webView!.evalJavascript(
      'plugin.kernel.getAllBalances("$account")',
    );
    return List<Map<String, dynamic>>.from(res);
  }

  Future<Map<String, dynamic>> getAccount(String address) async {
    final res = await plugin.sdk.webView!.evalJavascript(
      'plugin.kernel.getAccount("$address")',
    );
    return Map<String, dynamic>.from(res);
  }

  Future<String> getNonce(String address) async {
    final res = await plugin.sdk.webView!.evalJavascript(
      'plugin.kernel.getNonce("$address")',
    );
    return res.toString();
  }

  String _encode(Map<String, dynamic> params) {
    return Uri.encodeComponent(params.toString());
  }
}
