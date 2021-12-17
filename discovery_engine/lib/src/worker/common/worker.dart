import 'dart:async' show StreamSubscription;
import 'dart:convert' show Converter;

import 'package:meta/meta.dart' show mustCallSuper;
import 'package:xayn_discovery_engine/src/worker/common/oneshot.dart'
    show OneshotRequest, Sender;
import 'package:xayn_discovery_engine/src/worker/common/platform_actors.dart'
    show PlatformWorker;
import 'package:xayn_discovery_engine/src/worker/native/platform_worker.dart'
    if (dart.library.html) 'package:xayn_discovery_engine/src/worker/web/platform_worker.dart'
    show createPlatformWorker;

/// [Worker] is providing a platform agnostic way of communication
/// with a Manager that spawned it.
///
/// To implement a [Worker] please specify [Request] and [Response] types
/// that might be send and received and provide [Converter]s for (de)serializing
/// those types into a message format capable of going through the
/// manager/worker boundary. Usually this could be either json or something
/// more optimised, like a list of bytes.
///
/// Example:
///
/// ```
/// class ExampleWorker extends Worker<Request, Response> {
///   final _requestCodec = JsonToRequestCodec();
///   final _responseCodec = ResponseToJsonCodec();
///
///   @override
///   Converter<Object, OneshotRequest<Request>> get requestConverter =>
///     _requestCodec.decoder;
///
///   @override
///   Converter<Response, Object> get responseConverter =>
///     _responseCodec.encoder;
///
///   ExampleWorker(Object initialMessage) : super(initialMessage);
///
///   @override
///   void onMessage(OneshotRequest<Request> request) {
///     send(SomeResponse(), request.sender);
///   }
///
///   @override
///   void onError(Object error) {
///     send(WorkerError(error));
///   }
/// }
///
/// // The main function can serve as an entry point for the `Isolate.spawn`
/// // but also for the WebWorker when the file is compiled to JavaScript
/// void main(Object initialMessage) => ExampleWorker(initialMessage);
/// ```
abstract class Worker<Request extends Object, Response extends Object> {
  /// Underlying [PlatformWorker] used for communication with a Manager.
  final PlatformWorker _worker;

  late final StreamSubscription<Object> _subscription;

  /// Converter for incoming messages.
  Converter<Object, OneshotRequest<Request>> get requestConverter;

  /// Converter for outgoing messages.
  Converter<Response, Object> get responseConverter;

  Worker(Object initialMessage)
      : _worker = createPlatformWorker(initialMessage) {
    _bindPlatformWorker();
  }

  /// Subscribes to messages of the underlying [PlatformWorker], deserializes
  /// them to a [OneshotRequest] containing appropriate [Request] and adds
  /// them to a request stream.
  void _bindPlatformWorker() {
    _subscription = _worker.messages.listen(
      _onMessage,
      cancelOnError: false,
    );
  }

  /// Handles events from [PlatformWorker] messages stream.
  Future<void> onMessage(OneshotRequest<Request> request);

  /// Called with the error object upon any errors from [PlatformWorker]
  /// messages stream.
  void onError(
    Object error, {
    Object? incomingMessage,
  });

  Future<void> _onMessage(Object message) async {
    try {
      // let's convert incoming messages to a `OneshotRequest<Request>`
      final OneshotRequest<Request> request = requestConverter.convert(message);
      // we need to await 'onMessage' in case it's async,
      // otherwise it might "swallow" exceptions and send
      // some generic errors to the manager errors stream
      await onMessage(request);
    } catch (e) {
      onError(e, incomingMessage: message);
    }
  }

  /// Serializes the [Response] to a proper message format and sends it via
  /// the [Sender] attached to the [Request] if available or directly through
  /// the [PlatformWorker] channel.
  void send(Response event, [Sender? sender]) {
    final message = responseConverter.convert(event);

    // If [Sender] is available send the reponse message using it, otherwise
    // use the main platform channel
    (sender?.send ?? _worker.send).call(message);
  }

  /// Performs a cleanup that includes closing requests StreamController,
  /// canceling any ongoing subscriptions and disposing the underlying
  /// [PlatformWorker].
  @mustCallSuper
  Future<void> dispose() async {
    await _subscription.cancel();
    _worker.dispose();
  }
}
