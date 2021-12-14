import 'package:xayn_discovery_engine/discovery_engine.dart'
    show
        Configuration,
        DiscoveryEngine,
        EngineExceptionRaised,
        EngineInitException,
        NextFeedBatchAvailable,
        NextFeedBatchRequestSucceeded,
        NextFeedBatchRequested,
        ResetEngine;

Future<void> main() async {
  // provide initial configuration for the engine
  const config = Configuration(
    apiKey: '**********',
    apiBaseUrl: 'https://example-api.dev',
    feedMarket: 'de-DE',
    maxItemsPerFeedBatch: 50,
    applicationDirectoryPath: './',
  );

  late DiscoveryEngine engine;

  try {
    // Initialise the engine.
    //
    // This will spawn a Worker inside an Isolate (or WebWorker), instantiate
    // all the modules and binaries and establish communication channels
    print('Starting the Discovery Engine...');
    engine = await DiscoveryEngine.init(configuration: config);
    print('Engine initialized successfuly.');
  } on EngineInitException catch (e) {
    // message what went wrong
    print(e.message);
    // original exception that caused the issue, might contain more info
    print(e.exception);
  } catch (e) {
    // something else went wrong, shouldn't happen
    print(e);
  }

  // set up a listener if you want to consume events from `Stream<EngineEvent>`,
  final subscription = engine.engineEvents.listen((event) {
    print('\n[Event stream listener]: new event received!');
    print(event);
    // next batch of content is available to request so the app can
    // let the user know or already send a request for the next batch
    if (event is NextFeedBatchAvailable) {
      // you can just fire and forget
      engine.send(const NextFeedBatchRequested());
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
  await subscription.cancel();
  print('\ndisposing Discovery Engine...');
  await engine.dispose();
  print('Engine disposed.');

  // after disposing the engine you can't no longer send events
  final resp = await engine.send(const ResetEngine());
  print('\n${(resp as EngineExceptionRaised).reason}');
}
