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

import 'package:build/build.dart';
import 'package:source_gen/source_gen.dart';

import 'package:xayn_discovery_engine/src/generators/event_map_generator.dart';

Builder mapEventBuilder(BuilderOptions options) =>
    SharedPartBuilder([EngineEventMapGenerator()], 'generateEventMap');
