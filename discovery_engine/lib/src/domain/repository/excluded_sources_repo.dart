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

/// Repository interface for [Uri] sources excluded from the feed.
abstract class ExcludedSourcesRepository {
  /// Get a set of all excluded sources.
  Future<Set<Uri>> getAll();

  /// Persist set of exclueded sources.
  Future<void> save(Set<Uri> sources);
}
