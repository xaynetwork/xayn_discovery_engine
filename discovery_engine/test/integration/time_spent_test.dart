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

@Timeout(Duration(seconds: 80))

import 'dart:io' show Directory;

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/discovery_engine.dart'
    show
        ClientEventSucceeded,
        DocumentId,
        DocumentViewMode,
        EngineExceptionRaised,
        EngineExceptionReason,
        NextFeedBatchRequestSucceeded,
        UserReaction,
        cfgFeatureStorage;

import '../logging.dart' show setupLogging;
import 'utils/db.dart' show loadEngineState;
import 'utils/helpers.dart'
    show TestEngineData, initEngine, setupTestEngineData;
import 'utils/local_newsapi_server.dart' show LocalNewsApiServer;

void main() {
  setupLogging();

  group('DiscoveryEngine logDocumentTime', () {
    late LocalNewsApiServer server;
    late TestEngineData data;

    setUp(() async {
      data = await setupTestEngineData();
      server = await LocalNewsApiServer.start();
    });

    tearDown(() async {
      await server.close();
      await Directory(data.applicationDirectoryPath).delete(recursive: true);
    });

    test('log the view time of a document', () async {
      data.useInMemoryDb = false;
      var engine = await initEngine(data, server.port);

      // fetch some documents
      final nextFeedBatchResponse = await engine.requestNextFeedBatch();
      expect(nextFeedBatchResponse, isA<NextFeedBatchRequestSucceeded>());

      // like a document in order to create a coi
      final doc =
          (nextFeedBatchResponse as NextFeedBatchRequestSucceeded).items.first;
      expect(
        await engine.changeUserReaction(
          documentId: doc.documentId,
          userReaction: UserReaction.positive,
        ),
        isA<ClientEventSucceeded>(),
      );

      // cache engine state before the request of the document view time
      await engine.dispose();
      final stateBeforeRequest =
          await loadEngineState(data.applicationDirectoryPath);
      expect(stateBeforeRequest, isNotNull);

      engine = await initEngine(data, server.port);
      // check that the `ClientEventSucceeded` event will be emitted
      expect(
        engine.engineEvents,
        emitsInOrder(<Matcher>[
          isA<ClientEventSucceeded>(),
        ]),
      );

      // log the view time of the first document (adds the time to the coi)
      expect(
        await engine.logDocumentTime(
          documentId: doc.documentId,
          mode: DocumentViewMode.story,
          seconds: 10,
        ),
        isA<ClientEventSucceeded>(),
      );

      // check that the engine state has changed
      await engine.dispose();
      final stateAfterRequest =
          await loadEngineState(data.applicationDirectoryPath);
      expect(stateAfterRequest, isNotNull);
      expect(stateBeforeRequest, isNot(equals(stateAfterRequest)));
    });

    test(
        'if a document id is invalid, the engine should throw an'
        ' EngineExceptionRaised event', () async {
      final engine = await initEngine(data, server.port);

      final response = await engine.logDocumentTime(
        documentId: DocumentId(),
        mode: DocumentViewMode.story,
        seconds: 1,
      );

      expect(response, isA<EngineExceptionRaised>());
      expect(
        (response as EngineExceptionRaised).reason,
        EngineExceptionReason.genericError,
      );
      await engine.dispose();
    });

    test(
        'if the view time is negative, the engine should throw an'
        ' EngineExceptionRaised event', () async {
      final engine = await initEngine(data, server.port);

      final nextFeedBatchResponse = await engine.requestNextFeedBatch();
      final doc =
          (nextFeedBatchResponse as NextFeedBatchRequestSucceeded).items.first;

      final succeededResponse = await engine.logDocumentTime(
        documentId: doc.documentId,
        mode: DocumentViewMode.story,
        seconds: 0,
      );
      expect(succeededResponse, isA<ClientEventSucceeded>());

      final failedResponse = await engine.logDocumentTime(
        documentId: doc.documentId,
        mode: DocumentViewMode.story,
        seconds: -1,
      );

      expect(failedResponse, isA<EngineExceptionRaised>());
      expect(
        (failedResponse as EngineExceptionRaised).reason,
        EngineExceptionReason.genericError,
      );
      await engine.dispose();
    });
  });
}
