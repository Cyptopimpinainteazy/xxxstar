import 'package:polkawallet_sdk/plugin/index.dart';

/// X3VM service — compile, deploy, call x3 lang contracts
class X3VmService {
  final PolkawalletPlugin plugin;
  X3VmService(this.plugin);

  Future<Map<String, dynamic>> compile(String source, {bool optimize = true}) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.x3vm.compile(${Uri.encodeComponent(source)}, {optimize: $optimize})',
      ),
    );
  }

  Future<Map<String, dynamic>> deploy(String bytecodeHex, {String? gasLimit}) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.x3vm.deploy(account, "$bytecodeHex"${gasLimit != null ? ", {gasLimit: $gasLimit}" : ""})',
      ),
    );
  }

  Future<Map<String, dynamic>> call(Map<String, dynamic> params) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.x3vm.call(account, ${_encode(params)})',
      ),
    );
  }

  Future<Map<String, dynamic>> flashLoan(Map<String, dynamic> params) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.x3vm.flashLoan(account, ${_encode(params)})',
      ),
    );
  }

  Future<Map<String, dynamic>> query(Map<String, dynamic> params) async {
    return Map<String, dynamic>.from(
      await plugin.sdk.webView!.evalJavascript(
        'plugin.x3vm.query(${_encode(params)})',
      ),
    );
  }

  String _encode(Map<String, dynamic> params) {
    return Uri.encodeComponent(params.toString());
  }
}
