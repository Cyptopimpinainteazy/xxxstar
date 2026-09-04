import 'package:polkawallet_sdk/plugin/index.dart';

/// Atomic trade service — cross-VM DEX trades
class AtomicTradeService {
  final PolkawalletPlugin plugin;
  AtomicTradeService(this.plugin);

  Future<Map<String, dynamic>> createTradeBatch(Map<String, dynamic> params) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.trades.createTradeBatch(account, ${_encode(params)})',
      ),
    );
  }

  Future<Map<String, dynamic>> executeTradeBatch(String batchId) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.trades.executeTradeBatch(account, "$batchId")',
      ),
    );
  }

  Future<Map<String, dynamic>> swap(Map<String, dynamic> params) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.trades.swap(account, ${_encode(params)})',
      ),
    );
  }

  Future<Map<String, dynamic>> cancelTradeBatch(String batchId) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.trades.cancelTradeBatch(account, "$batchId")',
      ),
    );
  }

  Future<Map<String, dynamic>?> getBatch(String batchId) async {
    final res = await plugin.sdk.webView!.evalJavascript(
      'plugin.trades.getBatch("$batchId")',
    );
    return res != null ? Map<String, dynamic>.from(res) : null;
  }

  Future<Map<String, dynamic>?> getTwap(String tokenA, String tokenB) async {
    final res = await plugin.sdk.webView!.evalJavascript(
      'plugin.trades.getTwap("$tokenA", "$tokenB")',
    );
    return res != null ? Map<String, dynamic>.from(res) : null;
  }

  String _encode(Map<String, dynamic> params) {
    return Uri.encodeComponent(params.toString());
  }
}
