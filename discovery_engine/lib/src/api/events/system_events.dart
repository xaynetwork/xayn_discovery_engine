import 'package:xayn_discovery_engine/src/api/events/base_events.dart'
    show ClientEvent, EngineEvent;
import 'package:xayn_discovery_engine/src/api/events/document_events.dart'
    show DocumentFeedbackChanged;
import 'package:xayn_discovery_engine/src/domain/models/configuration.dart'
    show Configuration;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;

/// Event created upon every app startup, with some data needed
/// for the engine to work, like personalisation and feed market
/// (for performing background queries).
class Init extends ClientEvent {
  final bool isPersonalisationOn;
  final Configuration config;

  const Init(this.isPersonalisationOn, this.config);
}

/// Event created when the app decides to reset the AI (start fresh).
class ResetEngine extends ClientEvent {
  const ResetEngine();
}

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
class PersonalizationChanged extends ClientEvent {
  final bool isOn;

  const PersonalizationChanged(this.isOn);
}

/// Event created when the user changes market for the feed ie.
/// in global settings or changes some parameters for search,
/// like market or count (nb of items per page).
class ConfigurationChanged extends ClientEvent {
  final String? feedMarket;
  final String? searchMarket;
  final int? maxItemsPerSearchBatch;
  final int? maxItemsPerFeedBatch;

  const ConfigurationChanged({
    this.feedMarket,
    this.searchMarket,
    this.maxItemsPerSearchBatch,
    this.maxItemsPerFeedBatch,
  });
}

/// Event created when the user dismisses categories/topics when doing
/// a "negative" swipe, ie. on item in the news feed.
class ContentCategoriesDismissed extends ClientEvent {
  final DocumentId documentId;
  final Set<String> categories;

  const ContentCategoriesDismissed(this.documentId, this.categories);
}

/// Event created when the user removes "ban" from previously dismisses feed
/// categories/topics.
class ContentCategoriesAccepted extends ClientEvent {
  final Set<String> categories;

  const ContentCategoriesAccepted(this.categories);
}

/// Event created to inform the client that a particular "fire and forget"
/// event, like ie. [DocumentFeedbackChanged], was successfuly processed
/// by the engine.
class ClientEventSucceeded extends EngineEvent {
  const ClientEventSucceeded();
}

enum EngineExceptionReason {
  noInitReceived,
  // other possible errors will be added below
}

/// Event created by the engine for multitude of generic reasons, also
/// as a "failure" event in response to "fire and forget" events, like
/// ie. [DocumentFeedbackChanged].
class EngineExceptionRaised extends EngineEvent {
  final EngineExceptionReason reason;

  const EngineExceptionRaised(this.reason);
}
