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

import 'dart:io' show HttpServer, HttpStatus, InternetAddress, Directory, File;

const kMockDataPath = '/test/assets/utils/input';

class LocalAssetServer {
  final HttpServer _server;
  final String _mockDataPath;
  final Map<String, int> _callCount = {};
  int _failCount = 0;

  Map<String, int> get callCount => _callCount;

  LocalAssetServer._(
    this._server, {
    String? mockDataPath,
  }) : _mockDataPath = mockDataPath ?? kMockDataPath {
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
      final filePath = '${Directory.current.path}$_mockDataPath${request.uri}';
      final file = File(filePath);
      final callCount = _callCount[filePath] ?? 0;

      if (callCount < _failCount) {
        request.response.statusCode = HttpStatus.serviceUnavailable;
        _callCount[filePath] = callCount + 1;
      } else if (!file.existsSync()) {
        request.response.statusCode = HttpStatus.notFound;
      } else {
        await file.openRead().pipe(request.response);
      }

      await request.response.close();
    }
  }

  static Future<LocalAssetServer> start({
    int port = 8080,
    String? mockDataPath,
  }) async {
    final server = await HttpServer.bind(InternetAddress.anyIPv4, port);
    return LocalAssetServer._(server, mockDataPath: mockDataPath);
  }

  Future<void> close() async {
    await _server.close();
  }
}
