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

import 'dart:ffi' show Pointer;

import 'package:equatable/equatable.dart' show EquatableMixin;
import 'package:meta/meta.dart' show visibleForTesting;
import 'package:xayn_discovery_engine/src/domain/models/configuration.dart'
    show Configuration;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarket;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustInitConfig;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/box.dart' show Boxed;
import 'package:xayn_discovery_engine/src/ffi/types/feed_market_vec.dart'
    show FeedMarketSliceFfi;
import 'package:xayn_discovery_engine/src/ffi/types/migration/data.dart';
import 'package:xayn_discovery_engine/src/ffi/types/primitives.dart'
    show BoolFfi, FfiUsizeFfi;
import 'package:xayn_discovery_engine/src/ffi/types/string.dart'
    show OptionStringFfi, StringFfi;
import 'package:xayn_discovery_engine/src/infrastructure/assets/native/data_provider.dart'
    show NativeSetupData;
import 'package:xayn_discovery_engine/src/infrastructure/migration.dart';

class InitConfigFfi with EquatableMixin {
  final String apiKey;
  final String apiBaseUrl;
  final String newsProviderPath;
  final String headlinesProviderPath;
  final List<FeedMarket> feedMarkets;
  final String bert;
  final int maxDocsPerFeedBatch;
  final int maxDocsPerSearchBatch;
  final String? deConfig;
  final String? logFile;
  final String dataDir;
  final bool useEphemeralDb;
  final DartMigrationData? dartMigrationData;

  @override
  List<Object?> get props => [
        apiKey,
        apiBaseUrl,
        newsProviderPath,
        headlinesProviderPath,
        feedMarkets,
        bert,
        maxDocsPerFeedBatch,
        maxDocsPerSearchBatch,
        deConfig,
        logFile,
        dataDir,
        useEphemeralDb,
        dartMigrationData,
      ];

  factory InitConfigFfi(
    Configuration configuration,
    NativeSetupData setupData, {
    String? deConfig,
    DartMigrationData? dartMigrationData,
  }) =>
      InitConfigFfi.fromParts(
        apiKey: configuration.apiKey,
        apiBaseUrl: configuration.apiBaseUrl,
        newsProviderPath: configuration.newsProviderPath,
        headlinesProviderPath: configuration.headlinesProviderPath,
        feedMarkets: configuration.feedMarkets.toList(),
        bert: setupData.smbertConfig.replaceAll('/config.toml', ''),
        maxDocsPerFeedBatch: configuration.maxItemsPerFeedBatch,
        maxDocsPerSearchBatch: configuration.maxItemsPerSearchBatch,
        deConfig: deConfig,
        logFile: configuration.logFile,
        dataDir: configuration.applicationDirectoryPath,
        useEphemeralDb: configuration.useEphemeralDb,
        dartMigrationData: dartMigrationData,
      );

  InitConfigFfi.fromParts({
    required this.apiKey,
    required this.apiBaseUrl,
    required this.newsProviderPath,
    required this.headlinesProviderPath,
    required this.feedMarkets,
    required this.bert,
    required this.maxDocsPerFeedBatch,
    required this.maxDocsPerSearchBatch,
    required this.dataDir,
    required this.useEphemeralDb,
    this.deConfig,
    this.logFile,
    this.dartMigrationData,
  });

  /// Allocates a `Box<RustInitConfig>` initialized based on this instance.
  Boxed<RustInitConfig> allocNative() {
    final place = ffi.alloc_uninitialized_init_config();
    writeNative(place);
    return Boxed(place, ffi.drop_init_config);
  }

  void writeNative(Pointer<RustInitConfig> place) {
    apiKey.writeNative(ffi.init_config_place_of_api_key(place));
    apiBaseUrl.writeNative(ffi.init_config_place_of_api_base_url(place));
    newsProviderPath
        .writeNative(ffi.init_config_place_of_news_provider_path(place));
    headlinesProviderPath
        .writeNative(ffi.init_config_place_of_headlines_provider_path(place));
    feedMarkets.writeVec(ffi.init_config_place_of_markets(place));
    bert.writeNative(ffi.init_config_place_of_bert(place));
    maxDocsPerFeedBatch
        .writeNative(ffi.init_config_place_of_max_docs_per_feed_batch(place));
    maxDocsPerSearchBatch
        .writeNative(ffi.init_config_place_of_max_docs_per_search_batch(place));
    deConfig.writeNative(ffi.init_config_place_of_de_config(place));
    logFile.writeNative(ffi.init_config_place_of_log_file(place));
    dataDir.writeNative(
      ffi.init_config_place_of_data_dir(place),
    );
    useEphemeralDb
        .writeNative(ffi.init_config_place_of_use_ephemeral_db(place));
    dartMigrationData
        .writeNative(ffi.init_config_place_of_dart_migration_data(place));
  }

  @visibleForTesting
  static InitConfigFfi readNative(Pointer<RustInitConfig> config) {
    return InitConfigFfi.fromParts(
      apiKey: StringFfi.readNative(ffi.init_config_place_of_api_key(config)),
      apiBaseUrl:
          StringFfi.readNative(ffi.init_config_place_of_api_base_url(config)),
      newsProviderPath: StringFfi.readNative(
        ffi.init_config_place_of_news_provider_path(config),
      ),
      headlinesProviderPath: StringFfi.readNative(
        ffi.init_config_place_of_headlines_provider_path(config),
      ),
      feedMarkets:
          FeedMarketSliceFfi.readVec(ffi.init_config_place_of_markets(config)),
      bert: StringFfi.readNative(ffi.init_config_place_of_bert(config)),
      maxDocsPerFeedBatch: FfiUsizeFfi.readNative(
        ffi.init_config_place_of_max_docs_per_feed_batch(config),
      ),
      maxDocsPerSearchBatch: FfiUsizeFfi.readNative(
        ffi.init_config_place_of_max_docs_per_search_batch(config),
      ),
      deConfig: OptionStringFfi.readNative(
        ffi.init_config_place_of_de_config(config),
      ),
      logFile: OptionStringFfi.readNative(
        ffi.init_config_place_of_log_file(config),
      ),
      dataDir: StringFfi.readNative(
        ffi.init_config_place_of_data_dir(config),
      ),
      useEphemeralDb: BoolFfi.readNative(
        ffi.init_config_place_of_use_ephemeral_db(config),
      ),
      // dartMigrationData is omitted, must be null in tests
    );
  }
}
