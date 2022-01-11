import 'dart:convert' show jsonEncode;
import 'dart:io' show Directory, File;

import 'package:json_annotation/json_annotation.dart'
    show MissingRequiredKeysException, DisallowedNullValueException;
import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/domain/assets/asset.dart';
import 'package:xayn_discovery_engine/src/infrastructure/assets/native/json_manifest_reader.dart'
    show JsonManifestReader;

void main() {
  group('JsonManifestReader', () {
    group('read', () {
      File? file;

      tearDown(() async {
        if (file != null) {
          await file!.delete();
        }
      });

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
            (it) =>
                it.id.isNotEmpty &&
                it.urlSuffix.isNotEmpty &&
                it.checksum.checksum.isNotEmpty,
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

      test(
          'if a required key is missing it should throw "MissingRequiredKeysException"',
          () async {
        file = await createTmpManifest({
          'assets': [
            // this should have more keys
            {
              'url_suffix': 'smbert_v0000/vocab.txt',
            },
          ],
        });

        expect(
          () => JsonManifestReader().read(file!.path),
          throwsMissingRequiredKeysException,
        );
      });

      test(
          'if a required key is null it should throw "DisallowedNullValueException"',
          () async {
        file = await createTmpManifest({
          'assets': [
            {
              'id': 'smbertVocab',
              'url_suffix': 'smbert_v0000/vocab.txt',
              // this should be a "String" not null
              'checksum': null,
              'fragments': <Map<String, String>>[],
            },
          ]
        });

        expect(
          () => JsonManifestReader().read(file!.path),
          throwsDisallowedNullValueException,
        );
      });

      test('if a key is wrong type it should throw "TypeError"', () async {
        file = await createTmpManifest({
          'assets': [
            {
              'id': 'smbertVocab',
              'url_suffix': 'smbert_v0000/vocab.txt',
              // this should be a "String" not an "int"
              'checksum': 123,
              'fragments': <Map<String, String>>[],
            },
          ]
        });

        expect(
          () => JsonManifestReader().read(file!.path),
          throwsTypeError,
        );
      });
    });
  });
}

Future<File> createTmpManifest(Map<String, Object?> json) async {
  final path = Directory.systemTemp.path;
  final file = await File('$path/temp.json').create();
  return file.writeAsString(jsonEncode(json));
}

/// A matcher for [TypeError].
final throwsTypeError = throwsA(isA<TypeError>());

/// A matcher for [MissingRequiredKeysException].
final throwsMissingRequiredKeysException =
    throwsA(isA<MissingRequiredKeysException>());

/// A matcher for [DisallowedNullValueException].
final throwsDisallowedNullValueException =
    throwsA(isA<DisallowedNullValueException>());
