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

import 'package:xayn_discovery_engine/src/domain/assets/asset.dart'
    show Manifest;
import 'package:xayn_discovery_engine/src/domain/assets/manifest_reader.dart'
    show ManifestReader;

class MockManifestReader extends ManifestReader {
  final Map<String, Object> json;

  MockManifestReader(this.json);

  @override
  Future<String> loadManifestAsString() {
    throw UnimplementedError();
  }

  @override
  Future<Manifest> read() async {
    return Manifest.fromJson(json);
  }
}

const goodJson = {
  'assets': [
    {
      'id': 'smbertVocab',
      'url_suffix': 'dummy-asset',
      'checksum':
          'd9b2aefb1febe2dd6e403f634e18917a8c0dd1a440c976e9fe126b465ae9fc8d',
      'fragments': <Map<String, String>>[],
    },
    {
      'id': 'smbertModel',
      'url_suffix': 'dummy-asset',
      'checksum':
          'd9b2aefb1febe2dd6e403f634e18917a8c0dd1a440c976e9fe126b465ae9fc8d',
      'fragments': [
        {
          'url_suffix': 'dummy-asset',
          'checksum':
              'd9b2aefb1febe2dd6e403f634e18917a8c0dd1a440c976e9fe126b465ae9fc8d'
        },
        {
          'url_suffix': 'dummy-asset',
          'checksum':
              'd9b2aefb1febe2dd6e403f634e18917a8c0dd1a440c976e9fe126b465ae9fc8d'
        },
        {
          'url_suffix': 'dummy-asset',
          'checksum':
              'd9b2aefb1febe2dd6e403f634e18917a8c0dd1a440c976e9fe126b465ae9fc8d'
        },
      ]
    },
    {
      'id': 'kpeVocab',
      'url_suffix': 'dummy-asset',
      'checksum':
          'd9b2aefb1febe2dd6e403f634e18917a8c0dd1a440c976e9fe126b465ae9fc8d',
      'fragments': <Map<String, String>>[],
    },
    {
      'id': 'kpeModel',
      'url_suffix': 'dummy-asset',
      'checksum':
          'd9b2aefb1febe2dd6e403f634e18917a8c0dd1a440c976e9fe126b465ae9fc8d',
      'fragments': <Map<String, String>>[],
    },
    {
      'id': 'kpeCnn',
      'url_suffix': 'dummy-asset',
      'checksum':
          'd9b2aefb1febe2dd6e403f634e18917a8c0dd1a440c976e9fe126b465ae9fc8d',
      'fragments': <Map<String, String>>[],
    },
    {
      'id': 'kpeClassifier',
      'url_suffix': 'dummy-asset',
      'checksum':
          'd9b2aefb1febe2dd6e403f634e18917a8c0dd1a440c976e9fe126b465ae9fc8d',
      'fragments': <Map<String, String>>[],
    }
  ]
};

final wrongChecksumJson = {
  'assets': [
    ...(goodJson['assets'] as List<Map<String, Object>>).map(
      (it) => {
        ...it,
        'checksum': '123',
        'fragments': [
          ...(it['fragments'] as List<Map<String, Object>>).map(
            (fr) => {
              ...fr,
              'checksum': '123',
            },
          )
        ]
      },
    )
  ]
};
