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

import 'package:hive/hive.dart';
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show Source;
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show sourceReactedTypeId;

part 'source_reacted.g.dart';

/// Source domain of a document that the user has reacted to.
@HiveType(typeId: sourceReactedTypeId)
class SourceReacted {
  @HiveField(0)
  final Source source;
  @HiveField(1)
  int weight;
  @HiveField(2)
  DateTime timestamp;

  /// True iff the reaction was positive, false iff the reaction was negative.
  @HiveField(3)
  final bool liked;

  SourceReacted(this.source, this.liked)
      : weight = liked ? 1 : -1,
        timestamp = DateTime.now().toUtc();

  void update() {
    timestamp = DateTime.now();
    if (liked) {
      weight++;
    }
  }
}
