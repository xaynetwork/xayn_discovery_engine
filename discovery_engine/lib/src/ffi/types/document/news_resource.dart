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

import 'dart:ffi' show Pointer, Uint64Pointer;

import 'package:xayn_discovery_engine/src/domain/models/news_resource.dart'
    show NewsResource;
import 'package:xayn_discovery_engine/src/domain/models/source.dart'
    show Source;
import 'package:xayn_discovery_engine/src/ffi/genesis.ffigen.dart'
    show RustNewsResource;
import 'package:xayn_discovery_engine/src/ffi/load_lib.dart' show ffi;
import 'package:xayn_discovery_engine/src/ffi/types/date_time.dart'
    show DateTimeUtcFfi;
import 'package:xayn_discovery_engine/src/ffi/types/primitives.dart';
import 'package:xayn_discovery_engine/src/ffi/types/string.dart' show StringFfi;
import 'package:xayn_discovery_engine/src/ffi/types/uri.dart' show UriFfi;

extension NewsResourceFfi on NewsResource {
  void writeNative(Pointer<RustNewsResource> place) {
    title.writeNative(ffi.news_resource_place_of_title(place));
    snippet.writeNative(ffi.news_resource_place_of_snippet(place));
    url.writeNative(ffi.news_resource_place_of_url(place));
    sourceDomain
        .toString()
        .writeNative(ffi.news_resource_place_of_source_domain(place));
    UriFfi.writeNativeOption(
      image,
      ffi.news_resource_place_of_image(place),
    );
    datePublished.writeNative(ffi.news_resource_place_of_date_published(place));
    ffi.news_resource_place_of_rank(place).value = rank;
    PrimitivesFfi.writeNativeOptionF32(
      score,
      ffi.news_resource_place_of_score(place),
    );
    country.writeNative(ffi.news_resource_place_of_country(place));
    language.writeNative(ffi.news_resource_place_of_language(place));
    topic.writeNative(ffi.news_resource_place_of_topic(place));
  }

  static NewsResource readNative(Pointer<RustNewsResource> resource) {
    return NewsResource(
      title: StringFfi.readNative(ffi.news_resource_place_of_title(resource)),
      snippet:
          StringFfi.readNative(ffi.news_resource_place_of_snippet(resource)),
      url: UriFfi.readNative(ffi.news_resource_place_of_url(resource)),
      sourceDomain: Source(
        StringFfi.readNative(
          ffi.news_resource_place_of_source_domain(resource),
        ),
      ),
      image: UriFfi.readNativeOption(
        ffi.news_resource_place_of_image(resource),
      ),
      datePublished: DateTimeUtcFfi.readNative(
        ffi.news_resource_place_of_date_published(resource),
      ),
      rank: ffi.news_resource_place_of_rank(resource).value,
      score: PrimitivesFfi.readNativeOptionF32(
        ffi.news_resource_place_of_score(resource),
      ),
      country:
          StringFfi.readNative(ffi.news_resource_place_of_country(resource)),
      language:
          StringFfi.readNative(ffi.news_resource_place_of_language(resource)),
      topic: StringFfi.readNative(ffi.news_resource_place_of_topic(resource)),
    );
  }
}
