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

import 'dart:io'
    show Directory, File, HttpRequest, HttpServer, HttpStatus, InternetAddress;

const kMockDataPath = '/test/discovery_engine/utils/data/';

class LocalNewsApiServer {
  final HttpServer _server;

  LocalNewsApiServer._(this._server) {
    _handleRequests();
  }

  Future<void> _handleRequests() async {
    await for (final request in _server) {
      switch (request.uri.path) {
        case '/_sn':
          await _replyWithMockedData(request, 'climate-change.json');
          break;
        case '/_lh':
          await _replyWithMockedData(request, 'latest-headlines.json');
          break;
        default:
          request.response
            ..statusCode = HttpStatus.notFound
            ..write('Unsupported request: path ${request.uri.path} not found');
      }

      await request.response.close();
    }
  }

  static Future<LocalNewsApiServer> start([int port = 9090]) async {
    final server = await HttpServer.bind(InternetAddress.anyIPv4, port);
    return LocalNewsApiServer._(server);
  }

  Future<void> close() async {
    await _server.close();
  }
}

Future<void> _replyWithMockedData(HttpRequest request, String filename) async {
  final filePath = '${Directory.current.path}$kMockDataPath$filename';
  final file = File(filePath);
  await file.openRead().pipe(request.response);
}
