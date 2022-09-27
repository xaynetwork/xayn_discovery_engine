// Copyright 2022 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

import 'dart:io' show HttpServer, HttpStatus, InternetAddress;

const _bytes = [48, 48, 10];
const bytesMap = {
  'smbertVocab': _bytes,
  'smbertModel': _bytes,
  'availableSources': _bytes,
};

class LocalAssetServer {
  final HttpServer _server;
  final Map<Uri, int> _callCount = {};
  int _failCount = 0;

  int get callCountSum => _callCount.values.reduce((sum, val) => sum + val);

  LocalAssetServer._(this._server) {
    _handleRequests();
  }

  /// Set's the number of times each request will respond with
  /// "503 - Service Unavailable" status before it will be successful.
  void setRequestFailCount(int count) {
    assert(count >= 0, 'Request failure count can\'t be negative');
    _failCount = count;
  }

  /// Resets fail and call counters.
  void resetRequestFailCount() {
    setRequestFailCount(0);
    _callCount.clear();
  }

  Future<void> _handleRequests() async {
    await for (final request in _server) {
      final callCount = _callCount[request.uri] ?? 0;
      final assetNameArgs = '${request.uri}'.substring(1).split('_');

      if (callCount < _failCount) {
        request.response.statusCode = HttpStatus.serviceUnavailable;
        _callCount[request.uri] = callCount + 1;
      } else if (bytesMap[assetNameArgs.first] == null) {
        request.response.statusCode = HttpStatus.notFound;
      } else {
        final bytes = bytesMap[assetNameArgs.last] ??
            [
              bytesMap[assetNameArgs.first]!.elementAt(
                int.parse(assetNameArgs.last),
              )
            ];

        await Stream.fromIterable([bytes]).pipe(request.response);
      }

      await request.response.close();
    }
  }

  static Future<LocalAssetServer> start([int port = 8080]) async {
    final server = await HttpServer.bind(InternetAddress.anyIPv4, port);
    return LocalAssetServer._(server);
  }

  Future<void> close() async {
    await _server.close();
  }
}
