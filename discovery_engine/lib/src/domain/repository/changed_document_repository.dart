import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;

/// Repository interface for ids of documents whose status has changed since the
/// previous call of the feedback loop.
abstract class ChangedDocumentRepository {
  Future<List<DocumentId>> fetchAllIds();
  Future<void> add(DocumentId id);
  Future<void> removeAll();
}
