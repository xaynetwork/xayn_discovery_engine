import 'dart:async' show Completer, StreamController, StreamSubscription;
import 'dart:convert' show Converter;

import 'package:meta/meta.dart' show mustCallSuper;
import 'package:xayn_discovery_engine/src/worker/common/oneshot.dart'
    show Oneshot, OneshotRequest;
import 'package:xayn_discovery_engine/src/worker/common/platform_actors.dart'
    show PlatformManager;

import 'package:xayn_discovery_engine/src/worker/native/platform_manager_io.dart'
    if (dart.library.html) 'package:xayn_discovery_engine/src/worker/web/platform_manager_web.dart'
    show createPlatformManager;

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

  final _managerCompleter = Completer<PlatformManager>();
  final _responseController = StreamController<Response>.broadcast();
  final _subscriptions = <StreamSubscription<dynamic>>[];

  /// Converter for outgoing messages.
  Converter<OneshotRequest<Request>, dynamic> get requestConverter;

  /// Converter for incoming messages.
  Converter<dynamic, Response> get responseConverter;

  /// Stream of [Response] returned from the [Worker].
  Stream<Response> get responses => _responseController.stream;

  Manager(dynamic entryPoint) {
    createPlatformManager(entryPoint)
        .then((value) => _manager = value)
        .then(_managerCompleter.complete)
        .then((_) => _bindPlatformManager())
        // TODO: proper error handling needed here
        .catchError(_managerCompleter.completeError);
  }

  /// Subscribes to messages of the underlying [PlatformManager], deserializes
  /// them to an appropriate [Response]s and adds them to a responses stream.
  void _bindPlatformManager() {
    final subscription = _manager.messages
        // convert messages to a proper [Response]
        .map(responseConverter.convert)
        .listen(
          _responseController.add,
          // TODO: maybe error handling needed ??
          onError: (Object error) {},
        );
    _subscriptions.add(subscription);
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
    if (!_managerCompleter.isCompleted) {
      await _managerCompleter.future;
    }

    final channel = Oneshot();
    final sender = channel.sender;
    final request = OneshotRequest(sender, event);

    // Prepare request message and send it
    final dynamic requestMessage = requestConverter.convert(request);
    // TODO: check if on the web we actually need to send the port via transfer
    // to be able to use it
    _manager.send(requestMessage, [sender.port!]);

    final responseMessage = await channel.receiver.receive();
    // TODO: should we throw from the codec, or catch exceptions inside and return a proper ErrorEvent?
    final response = responseConverter.convert(responseMessage);

    // Add a response to the stream
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
