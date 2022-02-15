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
    show Document, UserReaction;
import 'package:xayn_discovery_engine/src/domain/models/news_resource.dart'
    show NewsResource;
import 'package:xayn_discovery_engine/src/domain/models/time_spent.dart'
    show TimeSpent;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/domain/models/user_reacted.dart'
    show UserReacted;

class MockEngine implements Engine {
  final Map<String, int> callCounter = {};
  late Document doc0;
  late Document doc1;
  late ActiveDocumentData active0;
  late ActiveDocumentData active1;

  MockEngine() {
    final resource = NewsResource.fromJson(const <String, Object>{
      'title': 'Example',
      'sourceUrl': 'domain.com',
      'snippet': 'snippet',
      'url': 'http://domain.com/news',
      'source_url': 'http://domain.com',
      'datePublished': '1980-01-01T00:00:00.000000',
      'provider': <String, String>{
        'name': 'domain',
        'thumbnail': 'http://thumbnail.domain.com',
      },
      'rank': 10,
      'score': 0.1,
      'country': 'EN',
      'language': 'en',
      'topic': 'news',
    });
    final stackId = StackId();

    doc0 = Document(
      stackId: stackId,
      batchIndex: 0,
      resource: resource,
    );
    doc1 = Document(
      stackId: stackId,
      batchIndex: 1,
      resource: resource,
    );
    active0 = ActiveDocumentData(Uint8List(0));
    active1 = ActiveDocumentData(Uint8List(1));
  }

  void _incrementCount(String key) {
    final count = getCallCount(key);
    callCounter[key] = count + 1;
  }

  int getCallCount(String key) {
    return callCounter[key] ?? 0;
  }

  void resetCallCounter() {
    callCounter.clear();
  }

  @override
  Uint8List serialize() {
    _incrementCount('serialize');
    return Uint8List(0);
  }

  @override
  Map<Document, ActiveDocumentData> getFeedDocuments(int maxDocuments) {
    _incrementCount('getFeedDocuments');

    if (maxDocuments < 1) {
      return {};
    } else if (maxDocuments == 1) {
      return {doc0: active0};
    } else {
      return {doc0: active0, doc1: active1};
    }
  }

  @override
  void timeLogged(TimeSpent timeSpen) {
    _incrementCount('timeLogged');
  }

  @override
  void userReacted(UserReacted userReacted) {
    _incrementCount('userReacted');
  }
}
