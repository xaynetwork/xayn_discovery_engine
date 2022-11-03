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

import 'dart:convert' show json, utf8;

import 'dart:io'
    show
        Directory,
        File,
        HttpClient,
        HttpClientRequest,
        HttpClientResponse,
        HttpHeaders,
        HttpRequest,
        HttpServer,
        HttpStatus,
        InternetAddress;

import 'package:test/expect.dart';

const kMockDataPath = '/test/integration/utils/data/';
const kMaxCheckAttempts = 5;

enum ReplyWith {
  error,
  empty,
  data,
  data2,
}

class LocalNewsApiServer {
  final HttpServer _server;
  ReplyWith _replyWith = ReplyWith.data;
  int _snCounter = 0;
  List<String> _snFiles = [
    'search-news.json',
    'search-news2.json',
    'search-news-empty.json'
  ];
  String _lhFile = 'latest-headlines.json';
  CapturedRequest? lastCapturedRequest;
  int requestCount = 0;

  LocalNewsApiServer._(this._server) {
    _handleRequests();
  }

  Future<void> _handleRequests() async {
    await for (final request in _server) {
      requestCount += 1;
      lastCapturedRequest = await CapturedRequest.capture(request);
      switch (_replyWith) {
        case ReplyWith.error:
          _replyWithError(request);
          break;
        case ReplyWith.empty:
          _replyWithEmpty(request);
          break;
        case ReplyWith.data:
          switch (request.uri.path) {
            case '/newscatcher/v2/search':
            case '/_mlt':
              await _replyWithData(
                request,
                _snFiles.asMap()[_snCounter++] ?? 'search-news-empty.json',
              );
              break;
            case '/newscatcher/v2/latest_headlines':
            case '/newscatcher/v2/trusted-sources':
              await _replyWithData(request, _lhFile);
              break;
            case '/_health':
              _replyWithOk(request);
              break;
            default:
              _replyWithError(request);
              break;
          }
          break;
        case ReplyWith.data2:
          switch (request.uri.path) {
            case '/newscatcher/v2/search':
              await _replyWithData(request, 'msft-vs-aapl.json');
              break;
            default:
              throw Exception(
                'News API mock: Endpoint not supported: "${request.uri.path}"',
              );
          }
      }

      await request.response.close();
    }
  }

  set replyWith(ReplyWith flag) => _replyWith = flag;

  set snFile(String filename) => _snFiles = [filename];

  set lhFile(String filename) => _lhFile = filename;

  int get port => _server.port;

  static Future<LocalNewsApiServer> start({bool verbose = false}) async {
    final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    final wrapper = LocalNewsApiServer._(server);
    await _waitUntilServerIsUpOrThrow(server.port, verbose);
    return wrapper;
  }

  Future<void> close() async {
    await _server.close();
  }
}

class CapturedRequest {
  final Uri uri;
  final String? contentType;
  final List<int> content;

  CapturedRequest(this.uri, this.contentType, this.content);

  static Future<CapturedRequest> capture(HttpRequest request) async {
    final uri = request.uri;
    final contentType = request.headers.value(HttpHeaders.contentTypeHeader);
    final content = await request.fold<List<int>>([], (prev, cur) {
      prev.addAll(cur);
      return prev;
    });
    return CapturedRequest(uri, contentType, content);
  }

  String expectUtf8Body(String contentType) {
    expect(contentType, equals(this.contentType));
    return utf8.decode(content);
  }

  Object? expectJsonBody() {
    final string = expectUtf8Body('application/json');
    return json.decode(string);
  }

  void expectJsonQueryParams(Map<String, String> expectedQuery) {
    final json = expectJsonBody();
    expect(json, isA<Map<String, Object?>>());
    final jsonMap = json as Map<String, Object?>;
    for (final entry in expectedQuery.entries) {
      expect(jsonMap[entry.key], equals(entry.value));
    }
  }
}

Future<void> _waitUntilServerIsUpOrThrow(int port, bool verbose) async {
  var attempts = 1;
  HttpClient client = HttpClient();
  while (attempts <= kMaxCheckAttempts) {
    attempts++;
    try {
      client = HttpClient();

      if (verbose) {
        print('Checking if connection works');
      }
      final HttpClientRequest request =
          await client.get('127.0.0.1', port, '/_health');
      if (verbose) {
        print('Connected to 127.0.0.1:$port');
      }
      final HttpClientResponse response = await request.close();
      if (verbose) {
        print('Got a response');
      }
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

void _replyWithEmpty(HttpRequest request) {
  request.response
    ..statusCode = HttpStatus.ok
    ..write('{"status": "ok", "total_pages": 0}');
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
