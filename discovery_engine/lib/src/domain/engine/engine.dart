import 'package:xayn_discovery_engine/src/domain/models/active_data.dart'
    show ActiveDocumentData;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;

/// Interface to Discovery Engine core.
abstract class Engine {
  /// Retrieves at most [maxDocuments] feed documents.
  Map<Document, ActiveDocumentData> getFeedDocuments(int maxDocuments);
}
