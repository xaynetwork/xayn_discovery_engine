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

import 'package:equatable/equatable.dart' show EquatableMixin;
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show UserReaction;
import 'package:xayn_discovery_engine/src/domain/models/embedding.dart'
    show Embedding;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId;
import 'package:xayn_discovery_engine/src/domain/models/view_mode.dart'
    show DocumentViewMode;

/// TimeSpent event with metadata as passed to the engine.
class TimeSpent with EquatableMixin {
  final DocumentId id;
  final UserReaction reaction;
  final Duration viewTime;
  final DocumentViewMode viewMode;
  final Embedding smbertEmbedding;

  const TimeSpent({
    required this.id,
    required this.reaction,
    required this.viewTime,
    required this.viewMode,
    required this.smbertEmbedding,
  });

  @override
  List<Object?> get props => [
        id,
        reaction,
        viewTime,
        viewMode,
        smbertEmbedding,
      ];
}
