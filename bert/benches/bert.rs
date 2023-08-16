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

use std::{hint::black_box, path::Path};

use criterion::{criterion_group, criterion_main, Criterion};
use xayn_ai_bert::{AveragePooler, Config, Runtime};
use xayn_test_utils::asset::{ort, xaynia};

const TOKEN_SIZE: usize = 250;
const SEQUENCE: &str = "Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam nonumy
eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua. At vero eos et
accusam et justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus est
Lorem ipsum dolor sit amet. Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam
nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua. At vero
eos et accusam et justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus
est Lorem ipsum dolor sit amet. Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed diam
nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam voluptua. At vero
eos et accusam et justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus
est Lorem ipsum dolor sit amet. Duis autem vel eum iriure dolor in hendrerit in vulputate velit
esse molestie consequat, vel illum dolore eu feugiat nulla facilisis at vero eros et accumsan et
iusto odio dignissim qui blandit praesent luptatum zzril delenit augue duis dolore te feugait nulla
facilisi. Lorem ipsum dolor sit amet, consectetuer adipiscing elit, sed diam nonummy nibh euismod
tincidunt ut laoreet dolore magna aliquam erat volutpat. Ut wisi enim ad minim veniam, quis nostrud
exerci tation ullamcorper suscipit lobortis nisl ut aliquip ex ea commodo consequat. Duis autem vel
eum iriure dolor in hendrerit in vulputate velit esse";

fn bench_bert(manager: &mut Criterion, name: &str, dir: &Path, rt: Runtime) {
    let pipeline = Config::new(dir)
        .unwrap()
        .with_token_size(TOKEN_SIZE)
        .unwrap()
        .with_runtime(rt)
        .with_pooler::<AveragePooler>()
        .build()
        .unwrap();
    manager.bench_function(name, |bencher| {
        bencher.iter(|| black_box(pipeline.run(black_box(SEQUENCE)).unwrap()))
    });
}

fn bench_bert_tract(manager: &mut Criterion) {
    bench_bert(manager, "Bert Tract", &xaynia().unwrap(), Runtime::Tract);
}

fn bench_bert_ort(manager: &mut Criterion) {
    bench_bert(
        manager,
        "Bert Ort",
        &xaynia().unwrap(),
        Runtime::Ort(ort().unwrap()),
    );
}

criterion_group! {
    name = bench;
    config = Criterion::default();
    targets =
        bench_bert_tract,
        bench_bert_ort,
}

criterion_main! {
    bench,
}
