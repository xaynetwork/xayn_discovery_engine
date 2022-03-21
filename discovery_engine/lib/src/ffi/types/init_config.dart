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
import 'package:xayn_discovery_engine/src/ffi/types/string.dart'
    show OptionStringFfi, StringFfi, StringListFfi;
import 'package:xayn_discovery_engine/src/infrastructure/assets/native/data_provider.dart'
    show NativeSetupData;

class InitConfigFfi with EquatableMixin {
  final String apiKey;
  final String apiBaseUrl;
  final List<FeedMarket> feedMarkets;
  final List<String> excludedSources;
  final String smbertVocab;
  final String smbertModel;
  final String kpeVocab;
  final String kpeModel;
  final String kpeCnn;
  final String kpeClassifier;
  final String? aiConfig;

  @override
  List<Object?> get props => [
        apiKey,
        apiBaseUrl,
        feedMarkets,
        excludedSources,
        smbertVocab,
        smbertModel,
        kpeVocab,
        kpeModel,
        kpeCnn,
        kpeClassifier,
        aiConfig,
      ];

  factory InitConfigFfi(
    Configuration configuration,
    NativeSetupData setupData, {
    String? aiConfig,
  }) =>
      InitConfigFfi.fromParts(
        apiKey: configuration.apiKey,
        apiBaseUrl: configuration.apiBaseUrl,
        feedMarkets: configuration.feedMarkets.toList(),
        excludedSources: configuration.excludedSources.toList(),
        smbertVocab: setupData.smbertVocab,
        smbertModel: setupData.smbertModel,
        kpeVocab: setupData.kpeVocab,
        kpeModel: setupData.kpeModel,
        kpeCnn: setupData.kpeCnn,
        kpeClassifier: setupData.kpeClassifier,
        aiConfig: aiConfig,
      );

  InitConfigFfi.fromParts({
    required this.apiKey,
    required this.apiBaseUrl,
    required this.feedMarkets,
    required this.excludedSources,
    required this.smbertVocab,
    required this.smbertModel,
    required this.kpeVocab,
    required this.kpeModel,
    required this.kpeCnn,
    required this.kpeClassifier,
    this.aiConfig,
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
    feedMarkets.writeVec(ffi.init_config_place_of_markets(place));
    excludedSources
        .writeNative(ffi.init_config_place_of_excluded_sources(place));
    smbertVocab.writeNative(ffi.init_config_place_of_smbert_vocab(place));
    smbertModel.writeNative(ffi.init_config_place_of_smbert_model(place));
    kpeVocab.writeNative(ffi.init_config_place_of_kpe_vocab(place));
    kpeModel.writeNative(ffi.init_config_place_of_kpe_model(place));
    kpeCnn.writeNative(ffi.init_config_place_of_kpe_cnn(place));
    kpeClassifier.writeNative(ffi.init_config_place_of_kpe_classifier(place));
    aiConfig.writeNative(ffi.init_config_place_of_ai_config(place));
  }

  @visibleForTesting
  static InitConfigFfi readNative(Pointer<RustInitConfig> config) {
    return InitConfigFfi.fromParts(
      apiKey: StringFfi.readNative(ffi.init_config_place_of_api_key(config)),
      apiBaseUrl:
          StringFfi.readNative(ffi.init_config_place_of_api_base_url(config)),
      feedMarkets:
          FeedMarketSliceFfi.readVec(ffi.init_config_place_of_markets(config)),
      excludedSources: StringListFfi.readNative(
        ffi.init_config_place_of_excluded_sources(config),
      ),
      smbertVocab:
          StringFfi.readNative(ffi.init_config_place_of_smbert_vocab(config)),
      smbertModel:
          StringFfi.readNative(ffi.init_config_place_of_smbert_model(config)),
      kpeVocab:
          StringFfi.readNative(ffi.init_config_place_of_kpe_vocab(config)),
      kpeModel:
          StringFfi.readNative(ffi.init_config_place_of_kpe_model(config)),
      kpeCnn: StringFfi.readNative(ffi.init_config_place_of_kpe_cnn(config)),
      kpeClassifier:
          StringFfi.readNative(ffi.init_config_place_of_kpe_classifier(config)),
      aiConfig: OptionStringFfi.readNative(
        ffi.init_config_place_of_ai_config(config),
      ),
    );
  }
}
