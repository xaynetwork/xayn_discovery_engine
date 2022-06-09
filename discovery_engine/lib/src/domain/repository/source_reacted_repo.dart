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
  /// Fetch by matching source.
  Future<SourceReacted?> fetchBySource(Source source);

  /// Fetch sources of documents with the given reaction.
  Future<List<SourceReacted>> fetchByReaction(bool like);

  /// Save [SourceReacted] to the repository.
  Future<void> save(SourceReacted source);

  /// Remove [SourceReacted] from the repository.
  Future<void> remove(Source source);
}
