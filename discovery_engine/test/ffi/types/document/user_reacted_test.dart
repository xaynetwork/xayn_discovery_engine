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

import 'package:test/test.dart';
import 'package:xayn_discovery_engine/src/domain/models/document.dart'
    show UserReaction;
import 'package:xayn_discovery_engine/src/domain/models/embedding.dart'
    show Embedding;
import 'package:xayn_discovery_engine/src/domain/models/feed_market.dart'
    show FeedMarket;
import 'package:xayn_discovery_engine/src/domain/models/unique_id.dart'
    show DocumentId, StackId;
import 'package:xayn_discovery_engine/src/domain/models/user_reacted.dart'
    show UserReacted;
import 'package:xayn_discovery_engine/src/ffi/types/document/user_reacted.dart'
    show UserReactedFfi;

void main() {
  test('reading written user reacted instance yields same result', () {
    final document = UserReacted(
      id: DocumentId(),
      stackId: StackId(),
      title: 'title',
      snippet: 'Cloning brought back the dodo.',
      smbertEmbedding: Embedding.fromList([.9, .1]),
      reaction: UserReaction.negative,
      market: const FeedMarket(langCode: 'de', countryCode: 'DE'),
    );
    final boxed = document.allocNative();
    final res = UserReactedFfi.readNative(boxed.ref);
    boxed.free();
    expect(res, equals(document));
  });
}
