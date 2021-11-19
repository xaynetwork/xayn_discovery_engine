import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:xayn_discovery_engine/src/api/events/client_groups.dart'
    show ClientEvent;
import 'package:xayn_discovery_engine/src/domain/models/configuration.dart';

part 'system_events.freezed.dart';
part 'system_events.g.dart';

@freezed
class SystemClientEvent with _$SystemClientEvent implements ClientEvent {
  /// Event created upon every app startup, with some data needed
  /// for the engine to work, like personalisation and feed market
  /// (for performing background queries).
  const factory SystemClientEvent.init(
    Configuration configuration, {
    @Default(true) bool isPersonalisationOn,
  }) = Init;

  /// Event created when the app decides to reset the AI (start fresh).
  const factory SystemClientEvent.resetEngine() = ResetEngine;

  /// Event created when the user toggles the AI on/off.
  ///
  /// When the personalisation is OFF:
  ///  - we are still reranking all the incoming results, but we don't use
  /// personal data to do it
  ///  - we are preventing storing queries and documents in the history,
  /// and sending/processing document-related events (likes, dislikes, etc.)
  ///
  /// Every document gets a rank from the reranker only once. When we toggle
  /// we switch between the API rank and Engine rank.
  const factory SystemClientEvent.personalizationChanged(bool isOn) =
      PersonalizationChanged;

  /// Event created when the user changes market for the feed ie.
  /// in global settings or changes some parameters for search,
  /// like market or count (nb of items per page).
  const factory SystemClientEvent.configurationChanged({
    String? feedMarket,
    String? searchMarket,
    int? maxItemsPerSearchBatch,
    int? maxItemsPerFeedBatch,
  }) = ConfigurationChanged;

  /// Converts json Map to [SystemClientEvent].
  factory SystemClientEvent.fromJson(Map<String, dynamic> json) =>
      _$SystemClientEventFromJson(json);
}
