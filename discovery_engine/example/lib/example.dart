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

import 'package:xayn_discovery_engine/discovery_engine.dart';

Future<void> runExample() async {
  // provide initial configuration for the engine
  final config = Configuration(
    apiKey: '**********',
    apiBaseUrl: 'https://example-api.dev',
    // assetsUrl: '<replace with a working URL to assets server>',
    assetsUrl: 'https://ai-assets.xaynet.dev',
    maxItemsPerFeedBatch: 50,
    applicationDirectoryPath: './',
    feedMarkets: {const FeedMarket(countryCode: 'DE', langCode: 'de')},
  );

  late DiscoveryEngine? engine;

  try {
    // Initialise the engine.
    //
    // This will spawn a Worker inside an Isolate (or WebWorker), instantiate
    // all the modules and binaries and establish communication channels
    print('Starting the Discovery Engine...');
    engine = await DiscoveryEngine.init(
      configuration: config,
      onAssetsProgress: (event) {
        if (event is FetchingAssetsStarted) {
          print('Fetching Assets Started');
        } else if (event is FetchingAssetsProgressed) {
          print('Fetching Assets Progress: ${event.percentage}');
        } else if (event is FetchingAssetsFinished) {
          print('Fetching Assets Finished');
        }
      },
    );
    print('Engine initialized successfully.');
  } on EngineInitException catch (e) {
    // message what went wrong
    print(e.message);
    // original exception that caused the issue, might contain more info
    print(e.exception);
  } catch (e) {
    // something else went wrong, shouldn't happen
    print(e);
  }

  if (engine == null) return;

  // set up a listener if you want to consume events from `Stream<EngineEvent>`,
  final subscription = engine.engineEvents.listen((event) {
    print('\n[Event stream listener]: new event received!');
    print(event);
    // next batch of content is available to request so the app can
    // let the user know or already send a request for the next batch
    if (event is NextFeedBatchAvailable) {
      // you can just fire and forget
      engine!.send(const NextFeedBatchRequested());
    }
  });

  // you can also use `async await` style for request/response
  final requestFeedResponse = await engine.requestFeed();

  requestFeedResponse.whenOrNull(
    feedRequestSucceeded: (items) {
      print(
        '\n[FeedRequestSucceeded]:\nupdate app state with Documents: $items',
      );
    },
    feedRequestFailed: (reason) {
      print('\nrequest failed because of: $reason');
    },
    engineExceptionRaised: (reason) {
      print('\nthere was an engine failure caused by $reason');
    },
  );

  // you can use the `send` method directly
  const event = NextFeedBatchRequested();
  final nextFeedBatchRequestedResponse = await engine.send(event);

  if (nextFeedBatchRequestedResponse is NextFeedBatchRequestSucceeded) {
    print(
      '\n[NextFeedBatchRequestSucceeded]:\nitems: ${nextFeedBatchRequestedResponse.items}',
    );
  }

  // just to wait so the event listener can be called before we dispose
  await Future<void>.delayed(const Duration(milliseconds: 10));

  // clean up
  print('\ndisposing Discovery Engine...');
  await subscription.cancel();
  await engine.dispose();
  print('Engine disposed.');

  print('\nAfter disposing the engine you can not send any events.');
  print('Trying to do that will cause:');
  final resp = await engine.send(const ResetEngine());
  print('${(resp as EngineExceptionRaised).reason}');
}
