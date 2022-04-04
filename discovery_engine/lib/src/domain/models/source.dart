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

import 'package:equatable/equatable.dart' show Equatable;
import 'package:meta/meta.dart' show protected;

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
