import 'package:equatable/equatable.dart';
import 'package:freezed_annotation/freezed_annotation.dart';

part 'feed_market.g.dart';

typedef FeedMarkets = Set<FeedMarket>;

@JsonSerializable()
class FeedMarket extends Equatable {
  final String countyCode;
  final String langCode;

  const FeedMarket({
    required this.countyCode,
    required this.langCode,
  });

  @override
  List<Object> get props => [
        countyCode,
        langCode,
      ];

  factory FeedMarket.fromJson(Map<String, Object?> json) =>
      _$FeedMarketFromJson(json);

  Map<String, dynamic> toJson() => _$FeedMarketToJson(this);
}
