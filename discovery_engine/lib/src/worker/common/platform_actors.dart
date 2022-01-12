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

abstract class PlatformActor {
  /// Stream of incoming messages
  Stream<Object> get messages;

  /// Method for sending messages to the other [PlatformActor]
  void send(Object message, [List<Object>? transfer]);

  /// Method for performing platform specific cleanup. It's called
  /// by the wrapper class that makes use of [PlatformActor].
  void dispose();
}

/// Base class for PlatformManager actor
abstract class PlatformManager extends PlatformActor {
  /// Stream of error messages from a [PlatformWorker]
  Stream<Object> get errors;
}

/// Base class for PlatformWorker actor
abstract class PlatformWorker extends PlatformActor {}
