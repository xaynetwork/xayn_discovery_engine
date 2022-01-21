// Copyright 2021 Xayn AG
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

import 'dart:typed_data' show Uint8List;

import 'package:xayn_discovery_engine/src/domain/engine/engine.dart'
    show Engine;
import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';
import 'package:xayn_discovery_engine/src/domain/models/document.dart';
import 'package:xayn_discovery_engine/src/domain/models/web_resource.dart'
    show WebResource;

final resource = WebResource.fromJson(const <String, Object>{
  'title': 'Example',
  'displayUrl': 'domain.com',
  'snippet': 'snippet',
  'url': 'http://domain.com',
  'datePublished': '1980-01-01T00:00:00.000000',
  'provider': <String, String>{
    'name': 'domain',
    'thumbnail': 'http://thumbnail.domain.com',
  },
});

final doc1 = Document(
  personalizedRank: 0,
  webResource: resource,
);

final doc2 = Document(
  personalizedRank: 1,
  webResource: resource,
);

final active1 = ActiveDocumentData(Uint8List(0));
final active2 = ActiveDocumentData(Uint8List(1));

class MockEngine implements Engine {
  @override
  Map<Document, ActiveDocumentData> getFeedDocuments(int maxDocuments) {
    if (maxDocuments < 1) {
      return {};
    } else if (maxDocuments == 1) {
      return {doc1: active1};
    } else {
      return {doc1: active1, doc2: active2};
    }
  }

  @override
  void timeLogged(
    DocumentId docId, {
    required Uint8List smbertEmbedding,
    required Duration seconds,
  }) {
    // TODO: implement timeLogged
  }

  @override
  void userReacted(
    DocumentId docId, {
    required Object stackId,
    required Uint8List smbertEmbedding,
    required DocumentFeedback reaction,
  }) {
    // TODO: implement userReacted
  }
}
