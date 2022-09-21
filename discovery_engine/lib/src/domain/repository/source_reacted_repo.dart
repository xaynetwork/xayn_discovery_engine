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

import 'package:xayn_discovery_engine/discovery_engine.dart' show Source;
import 'package:xayn_discovery_engine/src/domain/models/source_reacted.dart'
    show SourceReacted;

/// [SourceReacted] repository interface.
abstract class SourceReactedRepository {
  /// Fetch all sources.
  Future<List<SourceReacted>> fetchAll();

  /// Fetch by matching source.
  /// Returns null if no matching source found.
  Future<SourceReacted?> fetchBySource(Source source);

  /// Fetch sources of documents with the given reaction.
  Future<List<SourceReacted>> fetchByReaction(bool like);

  /// Fetch the liked source with minimum weight.
  /// Returns null if there are no liked sources.
  Future<SourceReacted?> fetchLightestLiked();

  /// Fetch the disliked source with oldest timestamp.
  /// Returns null if there are no disliked sources.
  Future<SourceReacted?> fetchOldestDisliked();

  /// Save [SourceReacted] to the repository.
  Future<void> save(SourceReacted source);

  /// Remove [SourceReacted] from the repository.
  Future<void> remove(Source source);

  /// Clears the repository.
  Future<void> clear();

  /// Checks if the repository is semantically empty.
  Future<bool> isEmpty();
}
