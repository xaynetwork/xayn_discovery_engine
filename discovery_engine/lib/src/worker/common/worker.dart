import 'dart:async' show StreamController, StreamSubscription;
import 'dart:convert' show Converter;

import 'package:meta/meta.dart' show mustCallSuper;
import 'package:xayn_discovery_engine/src/worker/common/oneshot.dart'
    show OneshotRequest, Sender;
import 'package:xayn_discovery_engine/src/worker/common/platform_actors.dart'
    show PlatformWorker;
import 'package:xayn_discovery_engine/src/worker/native/platform_worker_io.dart'
    if (dart.library.html) 'package:xayn_discovery_engine/src/worker/web/platform_worker_web.dart'
    show createPlatformWorker;

typedef Emmiter<Response> = void Function(Response event, [Sender? sender]);
typedef EventHandler<Request, Response> = Future<void> Function(
  Request event,
  Emmiter<Response> emit,
);

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
///   ExampleWorker(dynamic initialMessage) : super(initialMessage) {
///     on<SomeRequest>(_onSomeRequest);
///   }
///
///   void _onSomeRequest(SomeRequest event, Emmiter<Response> emit) {
///     emit(SomeResponse());
///   }
/// }
///
/// void main(dynamic initialMessage) => ExampleWorker(initialMessage);
/// ```
abstract class Worker<Request, Response> {
  /// Underlying [PlatformWorker] used for communication with a [Manager].
  final PlatformWorker _worker;

  final _requestController =
      StreamController<OneshotRequest<Request>>.broadcast();
  final _subscriptions = <StreamSubscription<dynamic>>[];
  final _handlerTypes = <Type>[];

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
    final subscription = _worker.messages
        .map(requestConverter.convert)
        .listen(_requestController.add);
    _subscriptions.add(subscription);
  }

  /// TODO: documentation needed
  void on<E extends Request>(
    EventHandler<E, Response> handler, {
    // EventTransformer<E>? transformer,
    dynamic transformer,
  }) {
    assert(() {
      final handlerExists = _handlerTypes.any((type) => type == E);
      if (handlerExists) {
        throw StateError(
          'on<$E> was called multiple times. '
          'There should only be a single event handler per event type.',
        );
      }
      _handlerTypes.add(E);
      return true;
    }());

    // TODO: wrap in a transformer and apply the handler
    final subscription = _requestController.stream
        .where((event) => event.payload is E)
        .cast<E>()
        .listen(null);
    _subscriptions.add(subscription);
  }

  /// Serializes the [Response] to a proper message format and sends it via
  /// the attached [Sender] if available, and also through the [PlatformWorker].
  void _emitResponse(Response event, [Sender? sender]) {
    final dynamic message = responseConverter.convert(event);

    // We send the reponse message to the sender that came with request
    sender?.send(message);
    // We send the reponse message through the main platform channel
    _worker.send(message);
  }

  /// Performs a cleanup that includes closing requests StreamController,
  /// canceling any ongoing subscriptions and disposing the underlying
  /// [PlatformWorker].
  @mustCallSuper
  Future<void> dispose() async {
    await _requestController.close();
    await Future.wait<void>(_subscriptions.map((s) => s.cancel()));
    _worker.dispose();
  }
}
