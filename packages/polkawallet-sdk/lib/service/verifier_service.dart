import 'package:polkawallet_sdk/plugin/index.dart';

/// Verifier service — x3vm job submission and execution verification
class VerifierService {
  final PolkawalletPlugin plugin;
  VerifierService(this.plugin);

  Future<Map<String, dynamic>> submitJob(Map<String, dynamic> params) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.verifier.submitJob(account, ${_encode(params)})',
      ),
    );
  }

  Future<Map<String, dynamic>?> getJob(String jobId) async {
    final res = await plugin.sdk.webView!.evalJavascript(
      'plugin.verifier.getJob("$jobId")',
    );
    return res != null ? Map<String, dynamic>.from(res) : null;
  }

  String _encode(Map<String, dynamic> params) {
    return Uri.encodeComponent(params.toString());
  }
}
