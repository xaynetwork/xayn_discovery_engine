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

import 'package:equatable/equatable.dart';
import 'package:freezed_annotation/freezed_annotation.dart';

part 'feed_market.g.dart';

typedef FeedMarkets = Set<FeedMarket>;

@JsonSerializable()
class FeedMarket extends Equatable {
  final String countryCode;
  final String langCode;

  const FeedMarket({
    required this.countryCode,
    required this.langCode,
  });

  @override
  List<Object> get props => [
        countryCode,
        langCode,
      ];

  factory FeedMarket.fromJson(Map<String, Object?> json) =>
      _$FeedMarketFromJson(json);

  Map<String, dynamic> toJson() => _$FeedMarketToJson(this);
}
