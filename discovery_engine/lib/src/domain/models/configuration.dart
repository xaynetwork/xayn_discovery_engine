import 'package:freezed_annotation/freezed_annotation.dart';

part 'configuration.freezed.dart';
part 'configuration.g.dart';

/// Class that holds data needed for the initialisation of the discovery engine.
@freezed
class Configuration with _$Configuration {
  const factory Configuration({
    required String apiKey,
    required String apiBaseUrl,
    required String feedMarket,
    required String searchMarket,
    required int maxItemsPerSearchBatch,
    required int maxItemsPerFeedBatch,
    required String applicationDirectoryPath,
  }) = _Configuration;

  factory Configuration.fromJson(Map<String, dynamic> json) =>
      _$ConfigurationFromJson(json);
}
