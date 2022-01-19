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
  final int _retryCount;
  final Map<String, int> _callCount = {};

  Map<String, int> get callCount => _callCount;

  LocalAssetServer._(
    this._server,
    this._retryCount, {
    String? mockDataPath,
  }) : _mockDataPath = mockDataPath ?? kMockDataPath {
    _handleRequests();
  }

  Future<void> _handleRequests() async {
    await for (final request in _server) {
      final filePath = '${Directory.current.path}$_mockDataPath${request.uri}';
      final file = File(filePath);
      final callCount = _callCount[filePath] ?? 0;

      if (callCount < _retryCount) {
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

  static Future<LocalAssetServer> start({int retryCount = 0}) async {
    final server = await HttpServer.bind(InternetAddress.anyIPv4, 8080);
    return LocalAssetServer._(server, retryCount);
  }

  Future<void> close() async {
    await _server.close();
  }
}
