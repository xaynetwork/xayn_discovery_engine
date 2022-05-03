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

import 'package:freezed_annotation/freezed_annotation.dart';

part 'trending_topic.freezed.dart';
part 'trending_topic.g.dart';

/// [TrendingTopic] class is used to represent topics that are currently
/// trending, and can be used as a basis for performing a new search.
@freezed
class TrendingTopic with _$TrendingTopic {
  const factory TrendingTopic({
    /// Description of a topic.
    required String name,

    /// Query that can/should be used to perform a new search.
    required String query,

    /// Image representation of a topic.
    required Uri? image,
  }) = _TrendingTopic;

  factory TrendingTopic.fromJson(Map<String, Object?> json) =>
      _$TrendingTopicFromJson(json);
}
