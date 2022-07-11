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
import 'package:hive/hive.dart'
    show HiveType, HiveField, TypeAdapter, BinaryReader, BinaryWriter;
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show feedMarketTypeId;

part 'feed_market.freezed.dart';
part 'feed_market.g.dart';

typedef FeedMarkets = Set<FeedMarket>;

@freezed
class FeedMarket with _$FeedMarket {
  @HiveType(typeId: feedMarketTypeId)
  const factory FeedMarket({
    /// Language code as per ISO ISO 639-1 definition.
    @HiveField(1) required String langCode,

    /// Country code as per ISO 3166-1 alpha-2 definition.
    @HiveField(0) required String countryCode,
  }) = _FeedMarket;

  factory FeedMarket.fromJson(Map<String, Object?> json) =>
      _$FeedMarketFromJson(json);
}
