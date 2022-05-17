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
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarket;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;

/// UserReacted event with meta data as passed to the engine.
class UserReacted with EquatableMixin {
  final DocumentId id;
  final StackId stackId;
  final String title;
  final String snippet;
  final Embedding smbertEmbedding;
  final UserReaction reaction;
  final FeedMarket market;

  UserReacted({
    required this.id,
    required this.stackId,
    required this.title,
    required this.snippet,
    required this.smbertEmbedding,
    required this.reaction,
    required this.market,
  });

  @override
  List<Object?> get props => [
        id,
        stackId,
        snippet,
        smbertEmbedding,
        reaction,
      ];
}
