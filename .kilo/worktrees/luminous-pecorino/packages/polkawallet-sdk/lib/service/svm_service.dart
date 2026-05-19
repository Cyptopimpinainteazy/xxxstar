import 'package:polkawallet_sdk/plugin/index.dart';

/// SVM service — Solana VM operations
class SvmService {
  final PolkawalletPlugin plugin;
  SvmService(this.plugin);

  Future<Map<String, dynamic>> createAccount(Map<String, dynamic> params) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.svm.createAccount(account, ${_encode(params)})',
      ),
    );
  }

  Future<Map<String, dynamic>> deployProgram(Map<String, dynamic> params) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.svm.deployProgram(account, ${_encode(params)})',
      ),
    );
  }

  Future<Map<String, dynamic>> transfer(Map<String, dynamic> params) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.svm.transfer(account, ${_encode(params)})',
      ),
    );
  }

  Future<Map<String, dynamic>?> getAccount(String pubkey) async {
    final res = await plugin.sdk.webView!.evalJavascript(
      'plugin.svm.getAccount("$pubkey")',
    );
    return res != null ? Map<String, dynamic>.from(res) : null;
  }

  String _encode(Map<String, dynamic> params) {
    return Uri.encodeComponent(params.toString());
  }
}
