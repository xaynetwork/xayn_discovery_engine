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
      'url_suffix': 'smbert_v0000/vocab.txt',
      'checksum':
          '9e5e90102c699455e9039ff903284e0689394dd345bb11456706f087984d2eb7',
      'fragments': <Map<String, String>>[],
    },
    {
      'id': 'smbertModel',
      'url_suffix': 'smbert_v0000/smbert.onnx',
      'checksum':
          'f1d29bfc97bf7ee86900e37531343c5a1f16ba091e9d1632a1e81b71da1b75ff',
      'fragments': [
        {
          'url_suffix': 'smbert_v0000/smbert.onnx_11MB_00',
          'checksum':
              'd9b2aefb1febe2dd6e403f634e18917a8c0dd1a440c976e9fe126b465ae9fc8d'
        },
        {
          'url_suffix': 'smbert_v0000/smbert.onnx_11MB_01',
          'checksum':
              '43fd56f56bb9bb18bc9c33966325732b2d7e58bfe2504a2c5c164b071c1b8653'
        },
        {
          'url_suffix': 'smbert_v0000/smbert.onnx_11MB_02',
          'checksum':
              'd2e084143fcb04ffc0e548123e1f62fc19c5d73906b36e7771a085079cbf4d3c'
        },
      ]
    },
    {
      'id': 'kpeVocab',
      'url_suffix': 'kpe_v0000/vocab.txt',
      'checksum':
          '9e5e90102c699455e9039ff903284e0689394dd345bb11456706f087984d2eb7',
      'fragments': <Map<String, String>>[],
    },
    {
      'id': 'kpeModel',
      'url_suffix': 'kpe_v0000/bert-quantized.onnx',
      'checksum':
          'd9b2aefb1febe2dd6e403f634e18917a8c0dd1a440c976e9fe126b465ae9fc8d',
      'fragments': <Map<String, String>>[],
    },
    {
      'id': 'kpeCnn',
      'url_suffix': 'kpe_v0000/cnn.binparams',
      'checksum':
          '43fd56f56bb9bb18bc9c33966325732b2d7e58bfe2504a2c5c164b071c1b8653',
      'fragments': <Map<String, String>>[],
    },
    {
      'id': 'kpeClassifier',
      'url_suffix': 'kpe_v0000/classifier.binparams',
      'checksum':
          'd2e084143fcb04ffc0e548123e1f62fc19c5d73906b36e7771a085079cbf4d3c',
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
