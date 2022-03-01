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

@Timeout(Duration(seconds: 60))

import 'dart:io';

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/discovery_engine.dart'
    show
        ClientEventSucceeded,
        DiscoveryEngine,
        DocumentId,
        DocumentsUpdated,
        EngineEvent,
        EngineExceptionRaised,
        EngineExceptionReason,
        NextFeedBatchRequestSucceeded,
        UserReaction;
import '../logging.dart' show setupLogging;
import 'utils/create_config.dart'
    show TestEngineData, createConfig, setupTestEngineData;
import 'utils/db.dart' show loadEngineState;
import 'utils/local_newsapi_server.dart' show LocalNewsApiServer;

void main() {
  setupLogging();

  group('DiscoveryEngine changeUserReaction', () {
    late LocalNewsApiServer server;
    late TestEngineData data;

    setUp(() async {
      data = await setupTestEngineData();
    });

    tearDown(() async {
      await server.close();
      await Directory(data.applicationDirectoryPath).delete(recursive: true);
    });

    test('change the user reaction of a document', () async {
      server = await LocalNewsApiServer.start();
      var engine = await DiscoveryEngine.init(
        configuration: createConfig(data, server.port),
      );
      // fetch some documents
      final nextFeedBatchResponse = await engine.requestNextFeedBatch();
      expect(nextFeedBatchResponse, isA<NextFeedBatchRequestSucceeded>());

      // cache engine state before the request of the change user reaction
      await engine.dispose();
      final stateBeforeRequest =
          await loadEngineState(data.applicationDirectoryPath);
      expect(stateBeforeRequest, isNotNull);

      // change the user reaction of the first document
      engine = await DiscoveryEngine.init(
        configuration: createConfig(data, server.port),
      );
      final doc =
          (nextFeedBatchResponse as NextFeedBatchRequestSucceeded).items.first;
      expect(
        await engine.changeUserReaction(
          documentId: doc.documentId,
          userReaction: UserReaction.positive,
        ),
        isA<ClientEventSucceeded>(),
      );

      // check that the `DocumentsUpdated` event has been emitted
      final docUpdatedReaction =
          doc.copyWith(userReaction: UserReaction.positive);
      await expectLater(
        engine.engineEvents,
        emitsInOrder(<EngineEvent>[
          DocumentsUpdated([docUpdatedReaction]),
        ]),
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
      server = await LocalNewsApiServer.start();

      final engine = await DiscoveryEngine.init(
        configuration: createConfig(data, server.port),
      );

      final response = await engine.changeUserReaction(
        documentId: DocumentId(),
        userReaction: UserReaction.positive,
      );

      expect(response, isA<EngineExceptionRaised>());
      expect(
        (response as EngineExceptionRaised).reason,
        EngineExceptionReason.genericError,
      );
    });
  });
}
