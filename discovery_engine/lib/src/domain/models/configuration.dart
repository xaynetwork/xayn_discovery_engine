/// Class that holds data needed for the initialisation of the discovery engine.
class Configuration {
  final String apiKey;
  final String apiBaseUrl;
  final String feedMarket;
  final String searchMarket;
  final int maxItemsPerSearchBatch;
  final int maxItemsPerFeedBatch;
  final String applicationDirectoryPath;

  const Configuration._({
    required this.apiKey,
    required this.apiBaseUrl,
    required this.feedMarket,
    required this.searchMarket,
    required this.maxItemsPerSearchBatch,
    required this.maxItemsPerFeedBatch,
    required this.applicationDirectoryPath,
  });
}
