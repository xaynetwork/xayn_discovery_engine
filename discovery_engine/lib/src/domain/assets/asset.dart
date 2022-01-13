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
@JsonSerializable(createToJson: false)
class Asset {
  @JsonKey(disallowNullValue: true, required: true)
  final AssetType id;
  @JsonKey(name: 'url_suffix', disallowNullValue: true, required: true)
  final String urlSuffix;
  @JsonKey(
    fromJson: Checksum._checksumFromString,
    disallowNullValue: true,
    required: true,
  )
  @JsonKey(disallowNullValue: true, required: true)
  final Checksum checksum;

  @JsonKey(disallowNullValue: true, required: true)
  final List<Fragment> fragments;

  Asset(this.id, this.urlSuffix, this.checksum, this.fragments);

  factory Asset.fromJson(Map json) => _$AssetFromJson(json);
}

// Type of an asset.
enum AssetType {
  smbertVocab,
  smbertModel,
}

/// A fragment of an asset.
@JsonSerializable(createToJson: false)
class Fragment {
  @JsonKey(name: 'url_suffix', disallowNullValue: true, required: true)
  final String urlSuffix;
  @JsonKey(
    fromJson: Checksum._checksumFromString,
    disallowNullValue: true,
    required: true,
  )
  final Checksum checksum;

  Fragment(this.urlSuffix, this.checksum);

  factory Fragment.fromJson(Map json) => _$FragmentFromJson(json);
}

/// The checksum an asset/fragment.
@JsonSerializable(createToJson: false)
class Checksum {
  @JsonKey(disallowNullValue: true, required: true)
  final String checksum;

  Checksum(this.checksum);

  static Checksum _checksumFromString(String checksum) => Checksum(checksum);

  /// Returns the sha256 hash (hex-encoded) of the asset/fragment.
  String get checksumAsHex => checksum;
}

@JsonSerializable(createToJson: false)
class Manifest {
  @JsonKey(disallowNullValue: true, required: true)
  final List<Asset> assets;

  Manifest(this.assets);

  factory Manifest.fromJson(Map json) => _$ManifestFromJson(json);
}
