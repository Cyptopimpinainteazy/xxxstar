import 'package:polkawallet_sdk/plugin/index.dart';

/// Domain service — .x3 domain registration and DNS
class DomainService {
  final PolkawalletPlugin plugin;
  DomainService(this.plugin);

  Future<Map<String, dynamic>> registerDomain(String domain) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.domains.registerDomain(account, "$domain")',
      ),
    );
  }

  Future<Map<String, dynamic>> setRecords(Map<String, dynamic> params) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.domains.setRecords(account, ${_encode(params)})',
      ),
    );
  }

  Future<Map<String, dynamic>?> getDomain(String domain) async {
    final res = await plugin.sdk.webView!.evalJavascript(
      'plugin.domains.getDomain("$domain")',
    );
    return res != null ? Map<String, dynamic>.from(res) : null;
  }

  Future<String?> resolve(String domain) async {
    final res = await plugin.sdk.webView!.evalJavascript(
      'plugin.domains.resolve("$domain")',
    );
    return res?.toString();
  }

  Future<bool> isDomainAvailable(String domain) async {
    final res = await plugin.sdk.webView!.evalJavascript(
      'plugin.domains.isDomainAvailable("$domain")',
    );
    return res == true;
  }

  String _encode(Map<String, dynamic> params) {
    return Uri.encodeComponent(params.toString());
  }
}
