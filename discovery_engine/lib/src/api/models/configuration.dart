class Configuration {
  final String apiKey;
  final String apiBaseUrl;
  final String feedMarket;
  final String searchMarket;
  final int maxItemsPerSearchPage;
  final int maxItemsPerFeedPage;
  final String applicationDirectoryPath;

  const Configuration._({
    required this.apiKey,
    required this.apiBaseUrl,
    required this.feedMarket,
    required this.searchMarket,
    required this.maxItemsPerSearchPage,
    required this.maxItemsPerFeedPage,
    required this.applicationDirectoryPath,
  });
}
