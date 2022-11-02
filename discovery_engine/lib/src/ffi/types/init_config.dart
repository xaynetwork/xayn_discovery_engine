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
  final String? newsProvider;
  final String? similarNewsProvider;
  final String? headlinesProvider;
  final String? trustedHeadlinesProvider;
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
        newsProvider,
        similarNewsProvider,
        headlinesProvider,
        trustedHeadlinesProvider,
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
        newsProvider: configuration.newsProvider,
        similarNewsProvider: configuration.similarNewsProvider,
        headlinesProvider: configuration.headlinesProvider,
        trustedHeadlinesProvider: configuration.trustedHeadlinesProvider,
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
    required this.feedMarkets,
    required this.bert,
    required this.maxDocsPerFeedBatch,
    required this.maxDocsPerSearchBatch,
    required this.dataDir,
    required this.useEphemeralDb,
    this.newsProvider,
    this.similarNewsProvider,
    this.headlinesProvider,
    this.trustedHeadlinesProvider,
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
    newsProvider.writeNative(ffi.init_config_place_of_news_provider(place));
    similarNewsProvider
        .writeNative(ffi.init_config_place_of_similar_news_provider(place));
    headlinesProvider
        .writeNative(ffi.init_config_place_of_headlines_provider(place));
    trustedHeadlinesProvider.writeNative(
      ffi.init_config_place_of_trusted_headlines_provider(place),
    );
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
      newsProvider: OptionStringFfi.readNative(
        ffi.init_config_place_of_news_provider(config),
      ),
      similarNewsProvider: OptionStringFfi.readNative(
        ffi.init_config_place_of_similar_news_provider(config),
      ),
      headlinesProvider: OptionStringFfi.readNative(
        ffi.init_config_place_of_headlines_provider(config),
      ),
      trustedHeadlinesProvider: OptionStringFfi.readNative(
        ffi.init_config_place_of_trusted_headlines_provider(config),
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
