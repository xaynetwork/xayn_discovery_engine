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

import 'package:json_annotation/json_annotation.dart'
    show MissingRequiredKeysException, DisallowedNullValueException;
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/domain/assets/asset.dart';
import 'package:xayn_discovery_engine/src/infrastructure/assets/native/json_manifest_reader.dart'
    show JsonManifestReader;

void main() {
  group('JsonManifestReader', () {
    group('read', () {
      test(
          'when given a properly formated manifest file it should read it '
          'without throwing Exceptions', () async {
        final manifest =
            await JsonManifestReader().read('../asset_manifest.json');

        // list of Assets is not empty
        expect(manifest.assets, isNotEmpty);
        // all Asset members are not empty strings
        expect(
          manifest.assets.every(
            (it) => it.urlSuffix.isNotEmpty && it.checksum.checksum.isNotEmpty,
          ),
          isTrue,
        );
        // all Fragment members are not empty strings
        expect(
          manifest.assets.fold<List<Fragment>>(
            [],
            (aggr, it) => [...aggr, ...it.fragments],
          ).every(
            (it) => it.urlSuffix.isNotEmpty && it.checksum.checksum.isNotEmpty,
          ),
          isTrue,
        );
      });
    });

    group('Manifest.fromJson', () {
      test(
          'if a required key is missing it should throw "MissingRequiredKeysException"',
          () async {
        final json = {
          'assets': [
            // this should have more keys
            {
              'url_suffix': 'smbert_v0000/vocab.txt',
            },
          ],
        };

        expect(
          () => Manifest.fromJson(json),
          throwsMissingRequiredKeysException,
        );
      });

      test(
          'if a required key is null it should throw "DisallowedNullValueException"',
          () async {
        final json = {
          'assets': [
            {
              'id': 'smbertVocab',
              'url_suffix': 'smbert_v0000/vocab.txt',
              // this should be a "String" not null
              'checksum': null,
              'fragments': <Map<String, String>>[],
            },
          ]
        };

        expect(
          () => Manifest.fromJson(json),
          throwsDisallowedNullValueException,
        );
      });

      test('if a key is wrong type it should throw "TypeError"', () async {
        final json = {
          'assets': [
            {
              'id': 'smbertVocab',
              'url_suffix': 'smbert_v0000/vocab.txt',
              // this should be a "String" not an "int"
              'checksum': 123,
              'fragments': <Map<String, String>>[],
            },
          ]
        };

        expect(
          () => Manifest.fromJson(json),
          throwsTypeError,
        );
      });
    });
  });
}

/// A matcher for [TypeError].
final throwsTypeError = throwsA(isA<TypeError>());

/// A matcher for [MissingRequiredKeysException].
final throwsMissingRequiredKeysException =
    throwsA(isA<MissingRequiredKeysException>());

/// A matcher for [DisallowedNullValueException].
final throwsDisallowedNullValueException =
    throwsA(isA<DisallowedNullValueException>());
