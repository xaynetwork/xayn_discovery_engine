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

  LocalAssetServer._(
    this._server, {
    String? mockDataPath,
  }) : _mockDataPath = mockDataPath ?? kMockDataPath {
    _handleRequests();
  }

  Future<void> _handleRequests() async {
    await for (final request in _server) {
      final filePath = '${Directory.current.path}$_mockDataPath${request.uri}';
      final file = File(filePath);

      if (!file.existsSync()) {
        request.response.statusCode = HttpStatus.notFound;
      } else {
        await file.openRead().pipe(request.response);
      }

      await request.response.close();
    }
  }

  static Future<LocalAssetServer> start() async {
    final server = await HttpServer.bind(InternetAddress.anyIPv4, 8080);
    return LocalAssetServer._(server);
  }

  Future<void> close() async {
    await _server.close();
  }
}
