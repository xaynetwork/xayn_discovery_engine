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

import 'dart:typed_data' show Float32List;

import 'package:equatable/equatable.dart' show EquatableMixin;

/// 1-Dimensional Embedding
///
/// Values are stored in native byte order in hive.
class Embedding with EquatableMixin {
  final Float32List values;

  Embedding(this.values);

  Embedding.fromList(List<double> values)
      : values = Float32List.fromList(values);

  @override
  List<Object?> get props => [values];
}
