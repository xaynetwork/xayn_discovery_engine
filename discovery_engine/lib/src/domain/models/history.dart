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

import 'package:equatable/equatable.dart';
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart';

/// A compact representation of a `Document` in the document history.
class HistoricDocument with EquatableMixin {
  final DocumentId id;
  final Uri url;
  final String snippet;
  final String title;

  HistoricDocument({
    required this.id,
    required this.url,
    required this.snippet,
    required this.title,
  });

  @override
  List<Object?> get props => [id, url, snippet, title];
}
