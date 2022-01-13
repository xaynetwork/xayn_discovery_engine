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

import 'package:xayn_discovery_engine/src/worker/native/oneshot.dart'
    if (dart.library.html) 'package:xayn_discovery_engine/src/worker/web/oneshot.dart'
    show createChannel, createPlatformSendingPort;

class Oneshot {
  final Sender _sender;
  final Receiver _receiver;

  Oneshot._(this._sender, this._receiver);

  factory Oneshot() {
    final channel = createChannel();
    final sender = Sender(channel.sendingPort);
    final receiver = Receiver(channel.receivingPort);
    return Oneshot._(sender, receiver);
  }

  Sender get sender => _sender;

  Receiver get receiver => _receiver;
}

class Sender<T extends SendingPort> {
  T? _port;
  Sender(this._port);

  /// Creates a Sender from `port` passed with the message. Ment to be used
  /// during message deserialization process.
  Sender.fromPlatformPort(Object port)
      : _port = createPlatformSendingPort(port) as T?;

  Object get platformPort {
    final port = _port;

    if (port == null) {
      throw StateError('Sender port in no longer accessible');
    }

    return port.port;
  }

  void send(Object message) {
    final port = _port;
    _port = null;

    if (port == null) {
      throw StateError('Sender send method was already called');
    }

    port.send(message);
    port.close();
  }
}

class Receiver<T extends ReceivingPort> {
  T? _port;
  Receiver(this._port);

  Future<Object> receive() async {
    final port = _port;
    _port = null;

    if (port == null) {
      throw StateError('Receiver receive method was already called');
    }

    final result = await port.receive();
    port.close();

    return result;
  }

  void dispose() {
    _port?.close();
    _port = null;
  }
}

abstract class ClosingPort {
  void close();
}

abstract class SendingPort extends ClosingPort {
  Object get port;

  void send(Object message);
}

abstract class ReceivingPort extends ClosingPort {
  Future<Object> receive();
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
