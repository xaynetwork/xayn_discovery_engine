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

import 'dart:convert' show utf8;

import 'package:csv/csv.dart' show CsvToListConverter;
import 'package:equatable/equatable.dart' show Equatable;
import 'package:freezed_annotation/freezed_annotation.dart';
import 'package:fuzzy/fuzzy.dart' show Fuzzy, FuzzyOptions;

part 'source.freezed.dart';
part 'source.g.dart';

/// A source of news/headlines/articles.
class Source extends Equatable {
  final String _repr;

  /// Must only be created anew by the Engine.
  ///
  /// Through other places can (de-)serialize it, iff they do not
  /// modify it using that mechanism.
  @protected
  Source(this._repr) {
    if (_repr.isEmpty) {
      throw ArgumentError('source can\'t be empty');
    }
  }

  @override
  List<Object?> get props => [_repr];

  @override
  String toString() => _repr;

  String get value => _repr;

  /// Must only be created anew by the Engine.
  ///
  /// Through other places can (de-)serialize it, iff they do not
  /// modify it using that mechanism.
  factory Source.fromJson(Object value) => Source(value as String);

  /// Returns a representation which can be used to create JSON.
  ///
  /// Be aware that this is not guaranteed to be a `Map`, it might
  /// be any arbitrary JSON Value.
  Object toJson() => _repr;
}

extension ToStringListExt on Set<Source> {
  List<String> toStringList() => map((s) => s._repr).toList();
}

@freezed
class AvailableSource with _$AvailableSource {
  @Assert('name.isNotEmpty')
  @Assert('domain.isNotEmpty')
  factory AvailableSource({
    required String name,
    required String domain,
  }) = _AvailableSource;

  factory AvailableSource.fromJson(Map<String, Object?> json) =>
      _$AvailableSourceFromJson(json);
}

class AvailableSources extends Fuzzy<AvailableSource> {
  AvailableSources(List<AvailableSource> availableSources)
      : super(
          availableSources,
          options: FuzzyOptions(
            findAllMatches: false,
            isCaseSensitive: false,
            minMatchCharLength: 3,
            minTokenCharLength: 3,
            shouldNormalize: false,
            shouldSort: true,
            threshold: 0.2,
            tokenize: true,
          ),
        );

  static Future<AvailableSources> fromBytes(Stream<List<int>> bytes) async {
    const converter = CsvToListConverter(
      fieldDelimiter: ';',
      textDelimiter: '\b',
      textEndDelimiter: '\b',
      eol: '\n',
      shouldParseNumbers: false,
      allowInvalid: false,
    );
    final sources =
        await bytes.transform(utf8.decoder).transform(converter).toList();

    return AvailableSources(
      sources
          .map(
            (source) => AvailableSource(
              name: source[0] as String,
              domain: source[1] as String,
            ),
          )
          .toList(),
    );
  }
}

final mockedAvailableSources =
    AvailableSources([AvailableSource(name: 'Example', domain: 'example.com')]);
