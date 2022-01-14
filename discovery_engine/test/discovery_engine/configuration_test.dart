import 'package:test/test.dart';
import 'package:xayn_discovery_engine/discovery_engine.dart';

void main() {
  test(
    'GIVEN empty set of FeedMarket WHEN create a Configuration THEN throw AssertError',
    () {
      const values = <FeedMarket>{};
      expect(
        () => Configuration(
          feedMarkets: values,
          apiKey: '',
          apiBaseUrl: '',
          maxItemsPerFeedBatch: -1,
          applicationDirectoryPath: '',
        ),
        throwsA(isA<AssertionError>()),
      );
    },
  );
}
