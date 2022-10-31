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

import 'dart:ffi';

import 'package:xayn_discovery_engine/src/domain/models/active_data.dart';
import 'package:xayn_discovery_engine/src/domain/models/document.dart';
import 'package:xayn_discovery_engine/src/domain/models/view_mode.dart';
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart';
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/date_time.dart';
import 'package:xayn_discovery_engine/src/ffi/types/document/news_resource.dart';
import 'package:xayn_discovery_engine/src/ffi/types/document/user_reaction.dart';
import 'package:xayn_discovery_engine/src/ffi/types/duration.dart';
import 'package:xayn_discovery_engine/src/ffi/types/embedding.dart';
import 'package:xayn_discovery_engine/src/ffi/types/primitives.dart';
import 'package:xayn_discovery_engine/src/ffi/types/uuid.dart';

class MigrationDocument {
  final Document document;
  final ActiveDocumentData? activeData;

  MigrationDocument(this.document, this.activeData);

  void writeNative(Pointer<RustMigrationDocument> place) {
    document.documentId.writeNative(ffi.migration_document_place_of_id(place));
    document.stackId
        .writeNative(ffi.migration_document_place_of_stack_id(place));
    // ignore: deprecated_member_use_from_same_package
    (activeData?.smbertEmbedding)
        .writeNative(ffi.migration_document_place_of_smbert_embedding(place));
    document.userReaction
        .writeNative(ffi.migration_document_place_of_reaction(place));
    document.resource
        .writeNative(ffi.migration_document_place_of_resource(place));
    ffi.init_migration_document_is_active_at(place, document.isActive ? 1 : 0);
    ffi.init_migration_document_is_searched_at(
      place,
      document.isSearched ? 1 : 0,
    );

    // ignore: deprecated_member_use_from_same_package
    document.batchIndex
        .writeNative(ffi.migration_document_place_of_batch_index(place));

    // ignore: deprecated_member_use_from_same_package
    document.timestamp
        .writeNative(ffi.migration_document_place_of_timestamp(place));

    (activeData?.viewTime[DocumentViewMode.web])
        .writeNative(ffi.migration_document_place_of_web_view_time(place));
    (activeData?.viewTime[DocumentViewMode.reader])
        .writeNative(ffi.migration_document_place_of_reader_view_time(place));
    (activeData?.viewTime[DocumentViewMode.story])
        .writeNative(ffi.migration_document_place_of_story_view_time(place));
  }
}
