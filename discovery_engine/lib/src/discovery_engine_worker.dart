// Copyright 2021 Xayn AG
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

import 'dart:convert' show Converter;
import 'package:xayn_discovery_engine/src/api/api.dart'
    show ClientEvent, EngineEvent, EngineExceptionReason;
import 'package:xayn_discovery_engine/src/api/codecs/json_codecs.dart'
    show JsonToOneshotRequestConverter, EngineEventToJsonConverter;
import 'package:xayn_discovery_engine/src/worker/worker.dart'
    show ConverterException, OneshotRequest, Sender, Worker;

class DiscoveryEngineWorker extends Worker<ClientEvent, EngineEvent> {
  final _requestConverter = JsonToOneshotRequestConverter();
  final _responseConverter = EngineEventToJsonConverter();

  @override
  Converter<Object, OneshotRequest<ClientEvent>> get requestConverter =>
      _requestConverter;

  @override
  Converter<EngineEvent, Object> get responseConverter => _responseConverter;

  DiscoveryEngineWorker(Object message) : super(message);

  Sender? _getSenderFromMessageOrNull(Object? incomingMessage) {
    if (incomingMessage == null) return null;

    try {
      // the conversion could fail because of a bad event so we still
      // might be able to extract only the sender from the message
      return _requestConverter.getSenderFromJson(incomingMessage);
    } catch (e) {
      // we ignore the error because we are already in the `onError` method,
      // (so any `ConverterException` that was thrown is being handled already)
      // and as a last resort, we are trying to get the `Sender` from the
      // original message, and use it to send back an `EngineExceptionEvent`
      // to a proper `Oneshot` channel
      return null;
    }
  }

  @override
  void onError(Object error, {Object? incomingMessage}) {
    var reason = EngineExceptionReason.genericError;

    if (error is ConverterException) {
      reason = EngineExceptionReason.converterException;
    }
    // Add other types here

    final errorEvent = EngineEvent.engineExceptionRaised(reason);
    final sender = _getSenderFromMessageOrNull(incomingMessage);
    // send an error event using main channel or Sender if available
    send(errorEvent, sender);
  }

  @override
  Future<void> onMessage(request) async {
    final clientEvent = request.payload;
    // This is just initial handler to respond with some events
    //
    // TODO: replace with proper handler
    // Events can be grouped by type
    // if (clientEvent is SystemClientEvent) {
    //   // pass the event to dedicated manager
    // } else if (clientEvent is FeedClientEvent) {
    //   // pass the event to DocumentManager
    // } else if (clientEvent is DocumentClientEvent) {
    //   // pass the event to DocumentManager
    // } else {
    //   // handle wrong event type???
    // }
    final response = await clientEvent.when(
      init: (configuration) async {
        return const EngineEvent.clientEventSucceeded();
      },
      resetEngine: () async {
        return const EngineEvent.clientEventSucceeded();
      },
      configurationChanged: (feedMarket, maxItemsPerFeedBatch) async {
        return const EngineEvent.clientEventSucceeded();
      },
      feedRequested: () async {
        return const EngineEvent.feedRequestSucceeded([]);
      },
      nextFeedBatchRequested: () async {
        return const EngineEvent.nextFeedBatchRequestSucceeded([]);
      },
      feedDocumentsClosed: (documentIds) async {
        return const EngineEvent.clientEventSucceeded();
      },
      documentFeedbackChanged: (documentId, feedback) async {
        return const EngineEvent.clientEventSucceeded();
      },
      documentTimeSpent: (documentId, mode, seconds) async {
        return const EngineEvent.clientEventSucceeded();
      },
    );

    send(response, request.sender);
  }
}

/// This method acts as an entry point:
/// - for Isolate.spawn on native platform
/// - for the compiled web worker file
void main(Object message) => DiscoveryEngineWorker(message);
