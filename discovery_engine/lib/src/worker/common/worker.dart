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

typedef Emitter<Response> = void Function(Response response);

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
///   Converter<dynamic, Request> get requestConverter =>
///     _requestCodec.decoder;
///
///   @override
///   Converter<Response, dynamic> get responseConverter =>
///     _responseCodec.encoder;
///
///   ExampleWorker(dynamic initialMessage) : super(initialMessage);
///
///   @override
///   void onMessage(Request request, Emitter<Response> emit) {
///     emit(SomeResponse());
///   }
///
///   @override
///   void onError(Object error, Emitter<Response> emit) {
///     emit(WorkerError(error));
///   }
/// }
///
/// void main(dynamic initialMessage) => ExampleWorker(initialMessage);
/// ```

// TODO: maybe rename this to InMsg, OutMsg
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
    _subscription = _worker.messages
        // let's convert incoming messages to a `OneshotRequest<Request>`
        .map(requestConverter.convert)
        .listen(
          (oneshotReq) => onMessage(
            oneshotReq.payload,
            _emitBuilder(oneshotReq.sender),
          ),
          onError: (Object error) => onError(
            error,
            _emitBuilder(),
          ),
        );
  }

  /// Handles events from [PlatformWorker] messages stream.
  void onMessage(Request request, Emitter<Response> emit);

  /// Called with the error object upon any errors from [PlatformWorker]
  /// messages stream.
  void onError(Object error, Emitter<Response> emit);

  /// Creates an `emit` function which serializes the [Response] to a proper
  /// message format and sends it via the attached [Sender] if available,
  /// and also through the [PlatformWorker].
  Emitter<Response> _emitBuilder([Sender? sender]) {
    return (Response event) {
      final dynamic message = responseConverter.convert(event);

      // We send the reponse message to the sender that came with request
      sender?.send(message);
      // We send the reponse message through the main platform channel
      _worker.send(message);
    };
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
