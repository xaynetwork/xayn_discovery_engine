import 'dart:isolate' show SendPort;

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/api/api.dart';
import 'package:xayn_discovery_engine/src/api/codecs/json_codecs.dart'
    show OneshotRequestToJsonConverter, kPayloadKey, kSenderKey;
import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show ConverterException, Oneshot, OneshotRequest;

void main() {
  group('OneshotRequestToJsonConverter', () {
    final converter = OneshotRequestToJsonConverter();
    late Oneshot channel;

    setUp(() {
      channel = Oneshot();
    });

    test(
        'when converting "FeedRequested" event, should contain a "SendPort" '
        'and a proper payload', () {
      const event = ClientEvent.feedRequested();
      final request = OneshotRequest(channel.sender, event);
      final message = converter.convert(request);

      expect(message[kSenderKey], isA<SendPort>());
      expect(message[kPayloadKey], equals({'type': 'feedRequested'}));
    });

    test('when converting a "bad" event, should throw "ConverterException"',
        () {
      const event = BadEvent();
      final request = OneshotRequest(channel.sender, event);

      expect(() => converter.convert(request), throwsConverterException);
    });
  });
}

/// A type matcher for [ConverterException].
final isConverterException = isA<ConverterException>();

/// A matcher for [ConverterException].
final throwsConverterException = throwsA(isConverterException);

///
///
///
///
///
class BadEvent implements ClientEvent {
  const BadEvent();
  @override
  TResult map<TResult extends Object?>({
    required TResult Function(Init value) init,
    required TResult Function(ResetEngine value) resetEngine,
    required TResult Function(ConfigurationChanged value) configurationChanged,
    required TResult Function(FeedRequested value) feedRequested,
    required TResult Function(NextFeedBatchRequested value)
        nextFeedBatchRequested,
    required TResult Function(FeedDocumentsClosed value) feedDocumentsClosed,
    required TResult Function(DocumentStatusChanged value)
        documentStatusChanged,
    required TResult Function(DocumentFeedbackChanged value)
        documentFeedbackChanged,
    required TResult Function(DocumentClosed value) documentClosed,
  }) {
    throw UnimplementedError();
  }

  @override
  TResult? mapOrNull<TResult extends Object?>({
    TResult Function(Init value)? init,
    TResult Function(ResetEngine value)? resetEngine,
    TResult Function(ConfigurationChanged value)? configurationChanged,
    TResult Function(FeedRequested value)? feedRequested,
    TResult Function(NextFeedBatchRequested value)? nextFeedBatchRequested,
    TResult Function(FeedDocumentsClosed value)? feedDocumentsClosed,
    TResult Function(DocumentStatusChanged value)? documentStatusChanged,
    TResult Function(DocumentFeedbackChanged value)? documentFeedbackChanged,
    TResult Function(DocumentClosed value)? documentClosed,
  }) {
    throw UnimplementedError();
  }

  @override
  TResult maybeMap<TResult extends Object?>({
    required TResult Function() orElse,
    TResult Function(Init value)? init,
    TResult Function(ResetEngine value)? resetEngine,
    TResult Function(ConfigurationChanged value)? configurationChanged,
    TResult Function(FeedRequested value)? feedRequested,
    TResult Function(NextFeedBatchRequested value)? nextFeedBatchRequested,
    TResult Function(FeedDocumentsClosed value)? feedDocumentsClosed,
    TResult Function(DocumentStatusChanged value)? documentStatusChanged,
    TResult Function(DocumentFeedbackChanged value)? documentFeedbackChanged,
    TResult Function(DocumentClosed value)? documentClosed,
  }) {
    throw UnimplementedError();
  }

  @override
  TResult maybeWhen<TResult extends Object?>({
    required TResult Function() orElse,
    TResult Function(Configuration configuration)? init,
    TResult Function()? resetEngine,
    TResult Function(String? feedMarket, int? maxItemsPerFeedBatch)?
        configurationChanged,
    TResult Function()? feedRequested,
    TResult Function()? nextFeedBatchRequested,
    TResult Function(Set<DocumentId> documentIds)? feedDocumentsClosed,
    TResult Function(DocumentId documentId, DocumentStatus status)?
        documentStatusChanged,
    TResult Function(DocumentId documentId, DocumentFeedback feedback)?
        documentFeedbackChanged,
    TResult Function(DocumentId documentId)? documentClosed,
  }) {
    throw UnimplementedError();
  }

  @override
  Map<String, dynamic> toJson() {
    throw UnimplementedError();
  }

  @override
  TResult when<TResult extends Object?>({
    required TResult Function(Configuration configuration) init,
    required TResult Function() resetEngine,
    required TResult Function(String? feedMarket, int? maxItemsPerFeedBatch)
        configurationChanged,
    required TResult Function() feedRequested,
    required TResult Function() nextFeedBatchRequested,
    required TResult Function(Set<DocumentId> documentIds) feedDocumentsClosed,
    required TResult Function(DocumentId documentId, DocumentStatus status)
        documentStatusChanged,
    required TResult Function(DocumentId documentId, DocumentFeedback feedback)
        documentFeedbackChanged,
    required TResult Function(DocumentId documentId) documentClosed,
  }) {
    throw UnimplementedError();
  }

  @override
  TResult? whenOrNull<TResult extends Object?>({
    TResult Function(Configuration configuration)? init,
    TResult Function()? resetEngine,
    TResult Function(String? feedMarket, int? maxItemsPerFeedBatch)?
        configurationChanged,
    TResult Function()? feedRequested,
    TResult Function()? nextFeedBatchRequested,
    TResult Function(Set<DocumentId> documentIds)? feedDocumentsClosed,
    TResult Function(DocumentId documentId, DocumentStatus status)?
        documentStatusChanged,
    TResult Function(DocumentId documentId, DocumentFeedback feedback)?
        documentFeedbackChanged,
    TResult Function(DocumentId documentId)? documentClosed,
  }) {
    throw UnimplementedError();
  }
}
