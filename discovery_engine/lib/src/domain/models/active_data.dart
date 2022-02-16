// Copyright 2021 Xayn AG
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

import 'dart:typed_data' show Uint8List;

import 'package:equatable/equatable.dart' show EquatableMixin;
import 'package:hive/hive.dart';
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/view_mode.dart'
    show DocumentViewMode;
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show activeDocumentDataTypeId;

part 'active_data.g.dart';

class DocumentWithActiveData with EquatableMixin {
  final Document document;
  final ActiveDocumentData data;

  DocumentWithActiveData(this.document, this.data);

  @override
  List<Object?> get props => [document, data];
}

/// Additional data pertaining to active documents.
@HiveType(typeId: activeDocumentDataTypeId)
class ActiveDocumentData with EquatableMixin {
  /// S-mBert Embedding
  ///
  /// Is a Float32List cast to a Uint8List.
  //FIXME: Create Embedding class with custom type adapter
  @HiveField(0)
  final Uint8List smbertEmbedding;
  @HiveField(1)
  final Map<DocumentViewMode, Duration> viewTime;

  ActiveDocumentData(this.smbertEmbedding) : viewTime = {};

  /// Add a time interval to the running total for the given view mode.
  void addViewTime(DocumentViewMode mode, Duration time) {
    viewTime.update(mode, (total) => total + time, ifAbsent: () => time);
  }

  /// Get the time spent in the given view mode.
  Duration getViewTime(DocumentViewMode mode) =>
      viewTime[mode] ?? Duration.zero;

  @override
  List<Object?> get props => [smbertEmbedding, viewTime];
}
