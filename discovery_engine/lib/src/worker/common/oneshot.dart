import 'package:xayn_discovery_engine/src/worker/native/oneshot_io.dart'
    if (dart.library.html) 'package:xayn_discovery_engine/src/worker/web/oneshot_web.dart'
    show createChannel;

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
    if (_sender == null) {
      throw StateError('Sender was already used');
    }

    final sender = _sender!;
    _sender = null;
    return sender;
  }

  Receiver get receiver {
    if (_receiver == null) {
      throw StateError('Receiver was already used');
    }

    final receiver = _receiver!;
    _receiver = null;
    return receiver;
  }
}

class Sender<T extends SendingPort> {
  T? _port;
  Sender(this._port);

  T? get port => _port;

  void send(Object? message, [List<Object>? transfer]) {
    if (_port == null) {
      throw StateError('Sender send method was already called');
    }

    _port!.send(message, transfer);
    _port!.close();
    _port = null;
  }
}

class Receiver<T extends ReceivingPort> {
  T? _port;
  Receiver(this._port);

  Future<Object?> receive() async {
    if (_port == null) {
      throw StateError('Receiver receive method was already called');
    }

    final result = await _port!.receive();
    _port!.close();
    _port = null;

    return result;
  }
}

abstract class ClosingPort {
  void close();
}

abstract class SendingPort extends ClosingPort {
  void send(Object? message, [List<Object>? transfer]);
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
