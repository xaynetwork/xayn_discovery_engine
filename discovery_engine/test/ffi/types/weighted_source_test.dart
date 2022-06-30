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

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/domain/models/source.dart';
import 'package:xayn_discovery_engine/src/domain/models/source_reacted.dart';
import 'package:xayn_discovery_engine/src/ffi/types/weighted_source.dart';
import 'package:xayn_discovery_engine/src/ffi/types/weighted_source_vec.dart';

void main() {
  test('reading written weighted source works', () {
    final source = SourceReacted(Source('sub.example.net'), true);
    final weightedSource = WeightedSourceFfi.fromSourceReacted(source);
    final boxed = weightedSource.allocNative();
    final res = WeightedSourceFfi.readNative(boxed.ref);
    boxed.free();
    expect(res, equals(weightedSource));
  });

  test('reading written weighted source vec works', () {
    final sources = [
      SourceReacted(Source('example.org'), true),
      SourceReacted(Source('example.com'), false),
    ];
    final weightedSources = sources.map(WeightedSourceFfi.fromSourceReacted);

    final boxed = sources.allocVec();
    final res = WeightedSourceListFfi.consumeVec(boxed);
    boxed.free();
    expect(res, equals(weightedSources));
  });
}
