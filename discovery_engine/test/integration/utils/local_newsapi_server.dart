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

const kMockDataPath = '/test/integration/utils/data/';

class LocalNewsApiServer {
  final HttpServer _server;
  bool _returnError = false;
  String _snFile = 'climate-change.json';
  String _lhFile = 'latest-headlines.json';

  LocalNewsApiServer._(this._server) {
    _handleRequests();
  }

  Future<void> _handleRequests() async {
    await for (final request in _server) {
      switch (request.uri.path) {
        case '/_sn':
        case '/_lh':
          await handleNewsAPIRequest(request);
          break;
        default:
          _replyWithError(request);
      }

      await request.response.close();
    }
  }

  Future<void> handleNewsAPIRequest(HttpRequest request) async {
    if (_returnError) {
      _replyWithError(request);
      return;
    } else {
      switch (request.uri.path) {
        case '/_sn':
          await _replyWithData(request, _snFile);
          break;
        case '/_lh':
          await _replyWithData(request, _lhFile);
          break;
        default:
          _replyWithError(request);
      }
    }
  }

  set replyWithError(bool flag) => _returnError = flag;

  set snFile(String filename) => _snFile = filename;

  set lhFile(String filename) => _lhFile = filename;

  int get port => _server.port;

  static Future<LocalNewsApiServer> start() async {
    final server = await HttpServer.bind(InternetAddress.anyIPv4, 0);
    return LocalNewsApiServer._(server);
  }

  Future<void> close() async {
    await _server.close();
  }
}

Future<void> _replyWithData(HttpRequest request, String filename) async {
  final filePath = '${Directory.current.path}$kMockDataPath$filename';
  final file = File(filePath);
  await file.openRead().pipe(request.response);
}

void _replyWithError(HttpRequest request) {
  request.response
    ..statusCode = HttpStatus.notFound
    ..write('Unsupported request: path ${request.uri.path} not found');
}
