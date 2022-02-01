// Copyright 2022 Xayn AG
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

import 'package:logger/logger.dart' show MemoryOutput, Logger, PrettyPrinter;
import 'package:test/test.dart' show setUp, tearDown, printOnFailure;

import 'package:xayn_discovery_engine/src/logger.dart' show initLogger;

/// Setup the logger to print logs only when a test fail.
void setupLogging() {
  late MemoryOutput memoryOutput;

  setUp(() {
    memoryOutput = MemoryOutput();
    initLogger(
      Logger(
        printer: PrettyPrinter(),
        output: memoryOutput,
      ),
    );
  });

  tearDown(() async {
    for (final event in memoryOutput.buffer) {
      final output = event.lines.join('\n');
      printOnFailure(output);
    }
  });
}
