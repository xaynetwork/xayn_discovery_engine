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

import 'package:freezed_annotation/freezed_annotation.dart';

part 'configuration.freezed.dart';
part 'configuration.g.dart';

/// Class that holds data needed for the initialisation of the discovery engine.
@freezed
class Configuration with _$Configuration {
  const factory Configuration({
    required String apiKey,
    required String apiBaseUrl,
    required String feedMarket,
    required int maxItemsPerFeedBatch,
    required String applicationDirectoryPath,
  }) = _Configuration;

  factory Configuration.fromJson(Map<String, Object?> json) =>
      _$ConfigurationFromJson(json);
}
