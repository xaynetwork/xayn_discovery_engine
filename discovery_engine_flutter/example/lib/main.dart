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

// ignore_for_file: avoid_print

import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';
import 'package:xayn_discovery_engine_flutter/discovery_engine.dart';

void main() {
  runApp(const MyApp());
}

class MyApp extends StatefulWidget {
  const MyApp({Key? key}) : super(key: key);

  @override
  State<MyApp> createState() => _MyAppState();
}

class _MyAppState extends State<MyApp> {
  // Platform messages are asynchronous, so we initialize in an async method.
  Future<void> initEngine() async {
    // provide initial configuration for the engine
    final appDir = await getApplicationDocumentsDirectory();
    final manifest = await FlutterManifestReader().read();
    final config = Configuration(
      apiKey: '**********',
      apiBaseUrl: 'https://example-api.dev',
      assetsUrl: 'https://ai-assets.xaynet.dev',
      maxItemsPerFeedBatch: 50,
      applicationDirectoryPath: appDir.path,
      feedMarkets: {const FeedMarket(countryCode: 'DE', langCode: 'de')},
      manifest: manifest,
    );

    late DiscoveryEngine? engine;

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

    if (engine == null) return;

    // set up a listener if you want to consume events from `Stream<EngineEvent>`,
    engine.engineEvents.listen((event) {
      print('\n[Event stream listener]: new event received!');
      print(event);
    });

    // you can also use `async await` style for request/response
    final requestFeedResponse = await engine.requestFeed();

    print('-- requestFeedResponse --');
    print(requestFeedResponse);
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: Scaffold(
        appBar: AppBar(
          title: const Text('Plugin example app'),
        ),
        body: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              ElevatedButton(
                onPressed: initEngine,
                child: const Text('Init engine'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
