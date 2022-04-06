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

import 'package:hive/hive.dart';
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show Source;
import 'package:xayn_discovery_engine/src/domain/repository/type_id.dart'
    show sourceFilterTypeId, sourceFilterModeTypeId;

part 'source_preference.g.dart';

@HiveType(typeId: sourceFilterModeTypeId)
enum PreferenceMode {
  @HiveField(0)
  trusted,

  @HiveField(1)
  excluded,
}

@HiveType(typeId: sourceFilterTypeId)
class SourcePreference {
  @HiveField(0)
  final Source source;

  @HiveField(1)
  final PreferenceMode mode;

  SourcePreference(this.source, this.mode);
}
