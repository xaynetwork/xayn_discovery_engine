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

import 'dart:async' show StreamController, StreamSubscription;
import 'dart:convert' show Converter;

import 'package:meta/meta.dart' show mustCallSuper;
import 'package:xayn_discovery_engine/src/worker/common/exceptions.dart'
    show
        ConverterException,
        ManagerDisposedException,
        ResponseTimeoutException,
        WorkerSpawnException;
import 'package:xayn_discovery_engine/src/worker/common/oneshot.dart'
    show Oneshot, OneshotRequest;
import 'package:xayn_discovery_engine/src/worker/common/platform_actors.dart'
    show PlatformManager;

import 'package:xayn_discovery_engine/src/worker/native/platform_manager.dart'
    if (dart.library.html) 'package:xayn_discovery_engine/src/worker/web/platform_manager.dart'
    show createPlatformManager;

const kDefaultRequestTimeout = Duration(seconds: 10);

/// [Manager] is providing a platform agnostic way of spawning a Worker and
/// establishing a communication with it.
///
/// To implement a [Manager] please specify [Request] and [Response] types
/// that might be send and received and provide [Converter]s for (de)serializing
/// those types into a message format capable of going through the
/// manager/worker boundary. Usually this could be either json or something
/// more optimised, like a list of bytes.
///
/// **Important!**
///
/// Please pass a proper "entry point" for the respective PlatformWorker
/// to the `super` constructor.
///
/// For web version please provide a path to the Worker js file, for the native
/// version it should be the static entry point method used to spawn an Isolate.
///
/// Example:
/// ```
/// class ExampleManager extends Manager<Request, Response> {
///   final _requestCodec = RequestToJsonCodec();
///   final _responseCodec = JsonToResponseCodec();
///
///   ExampleManager._(PlatformManager manager) : super(manager);
///
///   @override
///   Converter<OneshotRequest<Request>, Map> get requestConverter =>
///     _requestCodec.encoder;
///
///   @override
///   Converter<Map, Response> get responseConverter =>
///     _responseCodec.decoder;
///
///   static Future<MockManager> create(Object entryPoint) async {
///     final platformManager = await Manager.spawnWorker(entryPoint);
///     return ExampleManager._(platformManager);
///   }
///
///   Future<ExampleResponse> ping() {
///     try {
///       return await send(PingRequest());
///     } catch (error) {
///       // something went wrong
///       return PingFailed('$error');
///     }
///   }
/// }
///
/// void main() async {
///   final manager = await ExampleManager.create(
///       kIsWeb ? 'worker.dart.js' : ExampleWorker.entryPoint);
///
///   // let's send a ping request
///   final response = await manager.ping();
///
///   if (response is PongResponse) {
///     // success, everything went well
///   } else {
///     // we got a different response, probably an exception event
///   }
/// }
/// ```
abstract class Manager<Request extends Object, Response extends Object> {
  /// Underlying platform manager used for spawning
  /// and communication with a Worker.
  final PlatformManager _manager;

  final _responseController = StreamController<Response>.broadcast();
  final _subscriptions = <StreamSubscription<Object>>[];

  /// Converter for outgoing messages.
  Converter<OneshotRequest<Request>, Object> get requestConverter;

  /// Converter for incoming messages.
  Converter<Object, Response> get responseConverter;

  /// Stream of [Response] returned from the Worker.
  Stream<Response> get responses => _responseController.stream;

  Manager(this._manager) {
    _bindPlatformManager();
  }

  /// Returns a new instance of [PlatformManager] which spawns a Worker upon
  /// its creation. If this process fails it will throw a [WorkerSpawnException].
  static Future<PlatformManager> spawnWorker(Object? entryPoint) async {
    try {
      return await createPlatformManager(entryPoint);
    } catch (e) {
      throw WorkerSpawnException('$e');
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
      _responseController.addError,
      onError: _responseController.addError,
    );

    _subscriptions.addAll([
      messageSubscription,
      errorsSubscription,
    ]);
  }

  /// Sends a [Request] through [PlatformManager] to a spawned Worker
  /// and returns a Future with a [Response].
  ///
  /// [Request] is serialized via provided [Converter] to a format suitable
  /// for transfering across the boundary between [Manager] and Worker.
  /// To keep track of sent [Request] a [Oneshot] channel is created
  /// and the request is wrapped together with Sender's port in a [OneshotRequest].
  ///
  /// The response message from the Worker is deserialized to an appropriate
  /// [Request] and retured to the caller.
  Future<Response> send(Request event, {Duration? timeout}) async {
    if (_responseController.isClosed) {
      throw ManagerDisposedException(
        'Send method can not be used after the Manager was disposed',
      );
    }

    final channel = Oneshot();
    final request = OneshotRequest(channel.sender, event);

    // Prepare request message and send it via PlatformManager
    final requestMessage = requestConverter.convert(request);
    _manager.send(requestMessage, [channel.sender.platformPort]);

    // Wait for a message and convert it to proper [Response] object
    final responseMessage = await channel.receiver.receive()
        // Wait for [Response] message only for a specified
        // [Duration], otherwise throw a timeout exception
        .timeout(
      timeout ?? kDefaultRequestTimeout,
      onTimeout: () {
        // close the port of the Receiver
        channel.receiver.dispose();

        throw ResponseTimeoutException(
          'Worker couldn\'t respond in time to requested event: $event',
        );
      },
    );

    try {
      final response = responseConverter.convert(responseMessage);

      // Add a [Response] to the main stream
      _responseController.add(response);

      return response;
    } on ConverterException {
      rethrow;
    } catch (e) {
      // we need to catch also some runtime exceptions like "TypeError", etc.
      throw ConverterException(
        'Response Converter failed when converting a message from the Worker',
        payload: responseMessage,
        source: e,
      );
    }
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
