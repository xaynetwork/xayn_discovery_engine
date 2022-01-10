// ignore_for_file: avoid_print

import 'package:flutter/material.dart';
import 'dart:async';

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
