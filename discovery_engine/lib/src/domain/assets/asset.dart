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

import 'package:json_annotation/json_annotation.dart';

part 'asset.g.dart';

/// An asset consists of an URL suffix, a [Checksum] and optionally
/// a list of [Fragment]s.
///
/// The base URL (defined by the caller) concatenated with the URL suffix
/// creates the URL to fetch an asset.
///
/// The checksum is the hash of an asset and can be used to verify its
/// integrity after it has been fetched.
///
/// In order to keep larger assets in the http cache of a browser,
/// an asset might be split into multiple fragments.
///
/// Implementation details for fetching assets:
///
/// If the list of fragments is empty, the caller must use the URL suffix of the
/// asset to fetch it.
///
/// If the list of fragments is not empty, the caller must fetch each
/// [Fragment] in the fragments list and concatenate them in the same order
/// as they are defined in the fragments list in order to reassemble the asset.
/// Using the URL suffix of the [Asset] is not allowed. The checksum of the
/// [Asset] can be used to to verify its integrity after it has been
/// reassembled.
@JsonSerializable()
class Asset {
  @JsonKey(disallowNullValue: true, required: true)
  final AssetType id;
  @JsonKey(name: 'url_suffix', disallowNullValue: true, required: true)
  final String urlSuffix;
  @JsonKey(
    fromJson: Checksum._checksumFromString,
    toJson: Checksum._checksumToString,
    disallowNullValue: true,
    required: true,
  )
  final Checksum checksum;

  @JsonKey(disallowNullValue: true, required: true)
  final List<Fragment> fragments;

  Asset(this.id, this.urlSuffix, this.checksum, this.fragments);

  factory Asset.fromJson(Map<String, Object?> json) => _$AssetFromJson(json);
  Map<String, Object?> toJson() => _$AssetToJson(this);
}

// Type of an asset.
enum AssetType {
  smbertVocab,
  smbertModel,
  kpeVocab,
  kpeModel,
  kpeCnn,
  kpeClassifier,
}

/// A fragment of an asset.
@JsonSerializable()
class Fragment {
  @JsonKey(name: 'url_suffix', disallowNullValue: true, required: true)
  final String urlSuffix;
  @JsonKey(
    fromJson: Checksum._checksumFromString,
    toJson: Checksum._checksumToString,
    disallowNullValue: true,
    required: true,
  )
  final Checksum checksum;

  Fragment(this.urlSuffix, this.checksum);

  factory Fragment.fromJson(Map<String, Object?> json) =>
      _$FragmentFromJson(json);
  Map<String, Object?> toJson() => _$FragmentToJson(this);
}

/// The checksum an asset/fragment.
@JsonSerializable(createToJson: false, createFactory: false)
class Checksum {
  @JsonKey(disallowNullValue: true, required: true)
  final String checksum;

  Checksum(this.checksum);

  static Checksum _checksumFromString(String checksum) => Checksum(checksum);
  static String _checksumToString(Checksum? checksum) =>
      checksum?.checksum ?? '';

  /// Returns the sha256 hash (hex-encoded) of the asset/fragment.
  String get checksumAsHex => checksum;
}

@JsonSerializable()
class Manifest {
  @JsonKey(disallowNullValue: true, required: true)
  final List<Asset> assets;

  Manifest(this.assets);

  factory Manifest.fromJson(Map<String, Object?> json) =>
      _$ManifestFromJson(json);

  Map<String, Object?> toJson() => _$ManifestToJson(this);
}
