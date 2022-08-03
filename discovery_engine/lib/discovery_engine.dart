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

/// Support for doing something awesome.
///
/// More dartdocs go here.
library discovery_engine;

import 'package:logger/logger.dart' show Logger;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/logger.dart' show initLogger, logger;

export 'package:xayn_discovery_engine/src/api/api.dart';
export 'package:xayn_discovery_engine/src/discovery_engine_base.dart';
export 'package:xayn_discovery_engine/src/domain/assets/assets.dart'
    show Manifest, kAssetsPath, tmpFileExt;
export 'package:xayn_discovery_engine/src/infrastructure/assets/assets.dart'
    show createManifestReader;
export 'package:xayn_discovery_engine/src/worker/common/exceptions.dart';

void discoveryEngineInitLogger(Logger logger) => initLogger(logger);
Logger get discoveryEngineLogger => logger;

final cfgFeatureStorage = ffi.rust_cfg_storage() == 1;
