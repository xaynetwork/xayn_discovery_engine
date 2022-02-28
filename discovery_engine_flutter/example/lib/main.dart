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

enum EngineState {
  notReady,
  initializing,
  ready,
  initFailed,
}

class MyApp extends StatefulWidget {
  const MyApp({Key? key}) : super(key: key);

  @override
  State<MyApp> createState() => _MyAppState();
}

class _MyAppState extends State<MyApp> {
  DiscoveryEngine? _engine;
  double progress = .0;
  EngineState engineState = EngineState.notReady;

  // Platform messages are asynchronous, so we initialize in an async method.
  Future<void> initEngine() async {
    if (engineState != EngineState.notReady) return;
    setState(() => engineState = EngineState.initializing);

    // provide initial configuration for the engine
    final appDir = await getApplicationDocumentsDirectory();
    final manifest = await FlutterManifestReader().read();
    final copier = FlutterBundleAssetCopier(
      appDir: appDir.path,
      bundleAssetsPath: 'assets',
    );
    await copier.copyAssets(manifest);

    final config = Configuration(
      apiKey: '**********',
      apiBaseUrl: 'https://example-api.dev',
      assetsUrl: 'https://ai-assets.xaynet.dev',
      maxItemsPerFeedBatch: 50,
      applicationDirectoryPath: appDir.path,
      feedMarkets: {const FeedMarket(countryCode: 'DE', langCode: 'de')},
      manifest: manifest,
    );

    try {
      // Initialise the engine.
      //
      // This will spawn a Worker inside an Isolate (or WebWorker), instantiate
      // all the modules and binaries and establish communication channels
      print('Starting the Discovery Engine...');
      _engine = await DiscoveryEngine.init(
        configuration: config,
        onAssetsProgress: (event) => event.whenOrNull(
          fetchingAssetsProgressed: (percentage) =>
              setState(() => progress = percentage),
        ),
      );
      setState(() => engineState = EngineState.ready);
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

    if (_engine == null) {
      setState(() => engineState = EngineState.initFailed);
    }
  }

  Future<void> requestNews() async {
    if (_engine == null) return;
    final nextBatchResponse = await _engine!.requestNextFeedBatch();

    print('-- nextBatchResponse --');
    print(nextBatchResponse);
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
              if (engineState == EngineState.initializing)
                Text('Fetching assets: ${progress.toStringAsFixed(0)}%'),
              if (engineState == EngineState.ready) ...[
                const Text('Engine initialized'),
                ElevatedButton(
                  onPressed: requestNews,
                  child: const Text('Request News'),
                ),
              ],
              if (engineState == EngineState.notReady)
                ElevatedButton(
                  onPressed: initEngine,
                  child: const Text('Initialize engine'),
                ),
              if (engineState == EngineState.initFailed)
                const Text('Failure to initialize the engine'),
            ],
          ),
        ),
      ),
    );
  }
}
