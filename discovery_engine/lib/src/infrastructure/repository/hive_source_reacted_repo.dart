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
import 'package:xayn_discovery_engine/discovery_engine.dart' show Source;
import 'package:xayn_discovery_engine/src/domain/models/source_reacted.dart'
    show SourceReacted;
import 'package:xayn_discovery_engine/src/domain/repository/source_reacted_repo.dart'
    show SourceReactedRepository;
import 'package:xayn_discovery_engine/src/infrastructure/box_name.dart'
    show sourceReactedBox;

/// Hive implementation of [SourceReactedRepository].
class HiveSourceReactedRepository implements SourceReactedRepository {
  Box<SourceReacted> get box => Hive.box<SourceReacted>(sourceReactedBox);

  @override
  Future<List<SourceReacted>> fetchAll() async => box.values.toList();

  @override
  Future<SourceReacted?> fetchBySource(Source source) async =>
      box.get(source.value);

  @override
  Future<List<SourceReacted>> fetchByReaction(bool like) async =>
      box.values.where((source) => source.liked == like).toList();

  @override
  Future<SourceReacted?> fetchLightestLiked() async {
    final likes = box.values.where((source) => source.liked);
    if (likes.isEmpty) return null;

    return likes.reduce(
      (lightest, source) => source.weight < lightest.weight ? source : lightest,
    );
  }

  @override
  Future<SourceReacted?> fetchOldestDisliked() async {
    final dislikes = box.values.where((source) => !source.liked);
    if (dislikes.isEmpty) return null;

    return dislikes.reduce(
      (oldest, source) =>
          source.timestamp.isBefore(oldest.timestamp) ? source : oldest,
    );
  }

  @override
  Future<void> save(SourceReacted source) =>
      box.put(source.source.value, source);

  @override
  Future<void> remove(Source source) => box.delete(source.value);
}
