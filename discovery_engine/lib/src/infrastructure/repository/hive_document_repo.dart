import 'package:hive/hive.dart' show Hive, Box;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/repository/document_repository.dart'
    show DocumentRepository;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show documentBox;

class HiveDocumentRepository implements DocumentRepository {
  bool _isLoaded = false;
  final _idDocMap = <DocumentId, Document>{};
  final _idKeyMap = <DocumentId, dynamic>{};

  Box<Document> get box => Hive.box<Document>(documentBox);

  void _loadMaps() {
    box.toMap().forEach((dynamic key, doc) {
      _idDocMap[doc.documentId] = doc;
      _idKeyMap[doc.documentId] = key;
    });

    _isLoaded = true;
  }

  @override
  Future<Document?> fetchById(DocumentId id) async {
    if (!_isLoaded) _loadMaps();
    return _idDocMap[id];
  }

  @override
  Future<List<Document>> fetchAll() async => box.values.toList();

  @override
  Future<void> update(Document doc) async {
    if (!_isLoaded) _loadMaps();

    dynamic key = _idKeyMap[doc.documentId];
    if (key == null) {
      // add new doc to box, generating a new db key
      key = await box.add(doc);
      _idKeyMap[doc.documentId] = key;
    } else {
      // update existing doc in box
      await box.put(key, doc);
    }

    _idDocMap[doc.documentId] = doc;
  }
}
