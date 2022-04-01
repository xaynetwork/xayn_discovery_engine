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

import 'dart:convert' show utf8;

import 'dart:io'
    show
        Directory,
        File,
        HttpRequest,
        HttpServer,
        HttpStatus,
        InternetAddress,
        HttpClient,
        HttpClientRequest,
        HttpClientResponse;

const kMockDataPath = '/test/integration/utils/data/';
const kMaxCheckAttempts = 5;

class LocalNewsApiServer {
  final HttpServer _server;
  bool _returnError = false;
  String _snFile = 'climate-change.json';
  String _lhFile = 'latest-headlines.json';
  Uri? lastUri;

  LocalNewsApiServer._(this._server) {
    _handleRequests();
  }

  Future<void> _handleRequests() async {
    await for (final request in _server) {
      lastUri = request.uri;
      if (_returnError) {
        _replyWithError(request);
      } else {
        switch (request.uri.path) {
          case '/_sn':
            await _replyWithData(request, _snFile);
            break;
          case '/_lh':
            await _replyWithData(request, _lhFile);
            break;
          case '/_health':
            _replyWithOk(request);
            break;
          default:
            _replyWithError(request);
        }
      }

      await request.response.close();
    }
  }

  set replyWithError(bool flag) => _returnError = flag;

  set snFile(String filename) => _snFile = filename;

  set lhFile(String filename) => _lhFile = filename;

  int get port => _server.port;

  static Future<LocalNewsApiServer> start() async {
    final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    final wrapper = LocalNewsApiServer._(server);
    await _waitUntilServerIsUpOrThrow(server.port);
    return wrapper;
  }

  Future<void> close() async {
    await _server.close();
  }
}

Future<void> _waitUntilServerIsUpOrThrow(int port) async {
  var attempts = 1;
  HttpClient client = HttpClient();
  while (attempts <= kMaxCheckAttempts) {
    attempts++;
    try {
      client = HttpClient();

      print('Checking if connection works');
      final HttpClientRequest request =
          await client.get('127.0.0.1', port, '/_health');
      print('Connected to 127.0.0.1:$port');
      final HttpClientResponse response = await request.close();
      print('Got a response');
      final stringData = await response.transform(utf8.decoder).join();
      if (stringData != 'OK') {
        throw Exception('received wrong response $stringData');
      }

      return;
    } catch (e) {
      print(e);
      await Future<void>.delayed(Duration(seconds: attempts));
    } finally {
      client.close();
    }
  }

  throw Exception(
    'Was not able to connect to test server after $attempts attempts',
  );
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

void _replyWithOk(HttpRequest request) {
  request.response
    ..statusCode = HttpStatus.ok
    ..write('OK');
}
