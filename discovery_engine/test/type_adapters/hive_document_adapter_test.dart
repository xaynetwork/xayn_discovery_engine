import 'dart:io' show Directory;

import 'package:hive/hive.dart' show Hive, Box;
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document, DocumentAdapter, DocumentFeedback, DocumentFeedbackAdapter;
import 'package:xayn_discovery_engine/src/domain/models/web_resource.dart'
    show WebResource, WebResourceAdapter;
import 'package:xayn_discovery_engine/src/domain/models/web_resource_provider.dart'
    show WebResourceProviderAdapter;
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_document_id_adapter.dart'
    show DocumentIdAdapter;
import 'package:xayn_discovery_engine/src/infrastructure/type_adapters/hive_uri_adapter.dart'
    show UriAdapter;

void main() {
  group('DocumentAdapter', () {
    late Box<Document> box;

    setUpAll(() async {
      Hive.init(Directory.current.path);
      Hive.registerAdapter(DocumentAdapter());
      Hive.registerAdapter(DocumentFeedbackAdapter());
      Hive.registerAdapter(WebResourceAdapter());
      Hive.registerAdapter(WebResourceProviderAdapter());
      Hive.registerAdapter(DocumentIdAdapter());
      Hive.registerAdapter(UriAdapter());

      box = await Hive.openBox<Document>('DocumentAdapter');
    });

    tearDown(() async {
      await box.clear();
    });

    tearDownAll(() async {
      await box.deleteFromDisk();
    });

    test('can write and read `ActiveDocumentData`', () async {
      final dummy = WebResource.fromJson(const <String, Object>{
        'title': 'Example',
        'displayUrl': 'domain.com',
        'snippet': 'snippet',
        'url': 'http://domain.com',
        'datePublished': '1980-01-01T00:00:00.000000',
        'provider': <String, String>{
          'name': 'domain',
          'thumbnail': 'http://thumbnail.domain.com',
        },
      });
      final value = Document(
        personalizedRank: 0,
        feedback: DocumentFeedback.positive,
        isActive: true,
        webResource: dummy,
      );
      final key = await box.add(value);
      final document = box.get(key)!;

      expect(box, hasLength(1));
      expect(document, equals(value));
    });
  });
}
