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

import 'dart:typed_data' show Float32List;

import 'package:equatable/equatable.dart' show EquatableMixin;
import 'package:hive/hive.dart';

import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show Document;
import 'package:xayn_discovery_engine/src/domain/models/embedding.dart'
    show Embedding;
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
  @Deprecated('only used in migration')
  @HiveField(0)
  final Embedding smbertEmbedding;
  @HiveField(1)
  final Map<DocumentViewMode, Duration> viewTime;

  ActiveDocumentData()
      :
        // ignore: deprecated_member_use_from_same_package
        smbertEmbedding = Embedding(Float32List(0)),
        viewTime = {};

  /// Returns sum of [Duration] from all the registered [DocumentViewMode] times.
  Duration get sumDuration =>
      viewTime.values.reduce((aggregate, duration) => aggregate + duration);

  /// Add a time interval to the running total for the given view mode.
  void addViewTime(DocumentViewMode mode, Duration time) {
    viewTime.update(mode, (total) => total + time, ifAbsent: () => time);
  }

  /// Get the time spent in the given view mode.
  Duration getViewTime(DocumentViewMode mode) =>
      viewTime[mode] ?? Duration.zero;

  @override
  List<Object?> get props => [
        // ignore: deprecated_member_use_from_same_package
        smbertEmbedding,
        viewTime,
      ];
}
