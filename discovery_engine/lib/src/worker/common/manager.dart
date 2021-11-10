import 'dart:async' show Completer, StreamController, StreamSubscription;
import 'dart:convert' show Converter;

import 'package:meta/meta.dart' show mustCallSuper;
import 'package:xayn_discovery_engine/src/worker/common/exceptions.dart'
    show ResponseTimeoutException, WorkerSpawnException;
import 'package:xayn_discovery_engine/src/worker/common/oneshot.dart'
    show Oneshot, OneshotRequest;
import 'package:xayn_discovery_engine/src/worker/common/platform_actors.dart'
    show PlatformManager;

import 'package:xayn_discovery_engine/src/worker/native/platform_manager_io.dart'
    if (dart.library.html) 'package:xayn_discovery_engine/src/worker/web/platform_manager_web.dart'
    show createPlatformManager;

const kDefaultRequestTimeout = Duration(seconds: 10);

/// TODO: documentation needed
///
/// **Important!**
///
/// Please pass a proper "entry point" for the respective [PlatformWorker]
/// to the `super` constructor.
///
/// For web version please provide relative path to the [Worker] js file,
/// for the native version it should be the static entry point method used
/// to spawn an [Isolate].
///
/// Example:
/// ```
/// class ExampleManager extends Manager<Request, Response> {
///   final _requestCodec = RequestCodec();
///   final _responseCodec = ResponseCodec();
///
///   ExampleManager() : super(kIsWeb ? 'worker.dart.js' : main);
///
///   @override
///   Converter<Request, dynamic> get requestConverter =>
///     _requestCodec.encoder;
///
///   @override
///   Converter<dynamic, Response> get responseConverter =>
///     _responseCodec.decoder;
///
///   Future<Response> makeRequest(Request req) => send(req);
/// }
/// ```
abstract class Manager<Request, Response> {
  /// Underlying platform manager used for spawning
  /// and communication with a [Worker].
  late final PlatformManager _manager;

  final _isWorkerReady = Completer<bool>();
  final _responseController = StreamController<Response>.broadcast();
  final _subscriptions = <StreamSubscription<dynamic>>[];

  /// Converter for outgoing messages.
  Converter<OneshotRequest<Request>, dynamic> get requestConverter;

  /// Converter for incoming messages.
  Converter<dynamic, Response> get responseConverter;

  /// Stream of [Response] returned from the [Worker].
  Stream<Response> get responses => _responseController.stream;

  /// Returns a status of [Worker] initialization. Can be used to wait before
  /// sending a [Request];
  Future<bool> get isWorkerReady => _isWorkerReady.future;

  Manager(dynamic entryPoint) {
    _initManager(entryPoint);
  }

  void _initManager(dynamic entryPoint) async {
    try {
      _manager = await createPlatformManager(entryPoint);
      _bindPlatformManager();
      _isWorkerReady.complete(true);
    } catch (e) {
      _isWorkerReady.complete(false);
      // TODO: add an error to the main responses stream
      // OR add it to a dedicated errors stream
      _responseController.addError(WorkerSpawnException('$e'));
    }
  }

  /// Subscribes to messages of the underlying [PlatformManager], deserializes
  /// them to an appropriate [Response]s and adds them to a responses stream.
  void _bindPlatformManager() {
    final messageSubscription =
        _manager.messages.map(responseConverter.convert).listen(
              _responseController.add,
              onError: _responseController.addError,
            );
    final errorsSubscription = _manager.errors.listen(
      (dynamic error) => _responseController.addError(error as Object),
      onError: _responseController.addError,
    );

    _subscriptions.addAll([
      messageSubscription,
      errorsSubscription,
    ]);
  }

  /// Sends a [Request] through [PlatformManager] to a spawned [Worker]
  /// and returns a Future with a [Response].
  ///
  /// [Request] is serialized via provided [Converter] to a format suitable
  /// for transfering across the boundry between [Manager] and [Worker].
  /// To keep track of sent [Request] a [Oneshot] channel is created
  /// and the request is wrapped together with [Sender]s port in a [OneshotRequest].
  ///
  /// The response message from the [Worker] is deserialized to an appropriate
  /// [Request] and retured to the caller.
  Future<Response> send(Request event) async {
    // wait for the worker to be spawned
    if (!(await isWorkerReady)) {
      throw WorkerSpawnException(
          'There was an issue with Worker initialization.');
    }

    final channel = Oneshot();
    final sender = channel.sender;
    final request = OneshotRequest(sender, event);

    // Prepare request message and send it via PlatformManager
    final dynamic requestMessage = requestConverter.convert(request);
    _manager.send(requestMessage, [sender.platformPort]);

    // Wait for a message and convert it to proper [Response] object
    final responseMessage = await channel.receiver
        .receive()
        // Wait for [Response] message ony for a specified
        // [Duration], otherwise throw a timeout exception
        .timeout(
          kDefaultRequestTimeout,
          onTimeout: () => throw ResponseTimeoutException(
              'Worker couldn\'t respond in time to $event'),
        );
    final response = responseConverter.convert(responseMessage);

    // Add a [Response] to the main stream
    _responseController.add(response);

    return response;
  }

  /// Performs a cleanup that includes closing responses StreamController,
  /// canceling any ongoing subscriptions and disposing the underlying
  /// [PlatformManager].
  @mustCallSuper
  Future<void> dispose() async {
    await _responseController.close();
    await Future.wait<void>(_subscriptions.map((s) => s.cancel()));
    _manager.dispose();
  }
}
