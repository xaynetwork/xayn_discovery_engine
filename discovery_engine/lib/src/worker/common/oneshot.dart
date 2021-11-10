import 'package:xayn_discovery_engine/src/worker/native/oneshot_io.dart'
    if (dart.library.html) 'package:xayn_discovery_engine/src/worker/web/oneshot_web.dart'
    show createChannel, createPlatformSendingPort;

class Oneshot {
  Sender? _sender;
  Receiver? _receiver;

  Oneshot._(this._sender, this._receiver);

  factory Oneshot() {
    final channel = createChannel();
    final sender = Sender(channel.sendingPort);
    final receiver = Receiver(channel.receivingPort);
    return Oneshot._(sender, receiver);
  }

  Sender get sender {
    final sender = _sender;
    if (sender == null) {
      throw StateError('Sender was already used');
    }

    _sender = null;
    return sender;
  }

  Receiver get receiver {
    final receiver = _receiver;
    if (receiver == null) {
      throw StateError('Receiver was already used');
    }

    _receiver = null;
    return receiver;
  }
}

class Sender<T extends SendingPort> {
  T? _port;
  Sender(this._port);

  /// Creates a Sender from `port` passed with the message. Ment to be used
  /// during message deserialization process.
  Sender.fromPlatformPort(Object port)
      : _port = createPlatformSendingPort(port) as T?;

  Object get platformPort => _port?.port as Object;

  void send(dynamic message) {
    final port = _port;

    if (port == null) {
      throw StateError('Sender send method was already called');
    }

    port.send(message);
    port.close();
    _port = null;
  }
}

class Receiver<T extends ReceivingPort> {
  T? _port;
  Receiver(this._port);

  Future<Object?> receive() async {
    final port = _port;

    if (port == null) {
      throw StateError('Receiver receive method was already called');
    }

    final result = await port.receive();
    port.close();
    _port = null;

    return result;
  }
}

abstract class ClosingPort {
  void close();
}

abstract class SendingPort extends ClosingPort {
  Object get port;

  void send(dynamic message);
}

abstract class ReceivingPort extends ClosingPort {
  Future<Object?> receive();
}

class OneshotChannel {
  final SendingPort sendingPort;
  final ReceivingPort receivingPort;

  OneshotChannel(this.sendingPort, this.receivingPort);
}

class OneshotRequest<T> {
  final Sender sender;
  final T payload;

  OneshotRequest(this.sender, this.payload);
}
