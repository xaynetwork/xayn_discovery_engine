import 'dart:async' show StreamSubscription;
import 'dart:convert' show Converter;

import 'package:meta/meta.dart' show mustCallSuper;
import 'package:xayn_discovery_engine/src/worker/common/oneshot.dart'
    show OneshotRequest, Sender;
import 'package:xayn_discovery_engine/src/worker/common/platform_actors.dart'
    show PlatformWorker;
import 'package:xayn_discovery_engine/src/worker/native/platform_worker_io.dart'
    if (dart.library.html) 'package:xayn_discovery_engine/src/worker/web/platform_worker_web.dart'
    show createPlatformWorker;

typedef Emitter<Response> = void Function(Response response, [Sender? sender]);

/// TODO: documentation needed
///
/// Example:
///
/// ```
/// class ExampleWorker extends Worker<Request, Response> {
///   final _requestCodec = RequestCodec();
///   final _responseCodec = ResponseCodec();
///
///   @override
///   Converter<dynamic, OneshotRequest<Request>> get requestConverter =>
///     _requestCodec.decoder;
///
///   @override
///   Converter<Response, dynamic> get responseConverter =>
///     _responseCodec.encoder;
///
///   ExampleWorker(dynamic initialMessage) : super(initialMessage);
///
///   @override
///   void onMessage(OneshotRequest<Request> request, Emitter<Response> send) {
///     send(SomeResponse(), request.sender);
///   }
///
///   @override
///   void onError(Object error, Emitter<Response> send) {
///     send(WorkerError(error));
///   }
/// }
///
/// void main(dynamic initialMessage) => ExampleWorker(initialMessage);
/// ```
abstract class Worker<Request, Response> {
  /// Underlying [PlatformWorker] used for communication with a [Manager].
  final PlatformWorker _worker;

  late final StreamSubscription<dynamic> _subscription;

  /// Converter for incoming messages.
  Converter<dynamic, OneshotRequest<Request>> get requestConverter;

  /// Converter for outgoing messages.
  Converter<Response, dynamic> get responseConverter;

  Worker(dynamic initialMessage)
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
  void onMessage(OneshotRequest<Request> request, Emitter<Response> send);

  /// Called with the error object upon any errors from [PlatformWorker]
  /// messages stream.
  void onError(Object error, Emitter<Response> send);

  void _onMessage(dynamic message) {
    try {
      // let's convert incoming messages to a `OneshotRequest<Request>`
      final OneshotRequest<Request> request = requestConverter.convert(message);
      onMessage(request, send);
    } catch (e) {
      onError(e, send);
    }
  }

  /// Serializes the [Response] to a proper message format and sends it via
  /// the [Sender] attached to the [Request] if available or directly through
  /// the [PlatformWorker] channel.
  void send(Response event, [Sender? sender]) {
    final dynamic message = responseConverter.convert(event);

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
