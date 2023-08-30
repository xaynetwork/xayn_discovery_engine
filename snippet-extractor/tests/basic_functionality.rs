// Copyright 2023 Xayn AG
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

use xayn_snippet_extractor::{Config, Error, SnippetExtractor};
use xayn_test_utils::workspace::find_workspace_dir;

const TEST_TEXT: &str = r"
The GNU Affero General Public License is a free, copyleft license for software and other kinds of works, specifically designed to ensure cooperation with the community in the case of network server software.

The licenses for most software and other practical works are designed to take away your freedom to share and change the works. By contrast, our General Public Licenses are intended to guarantee your freedom to share and change all versions of a program--to make sure it remains free software for all its users.

When we speak of free software, we are referring to freedom, not price. Our General Public Licenses are designed to make sure that you have the freedom to distribute copies of free software (and charge for them if you wish), that you receive source code or can get it if you want it, that you can change the software or use pieces of it in new free programs, and that you know you can do these things.

Developers that use our General Public Licenses protect your rights with two steps: (1) assert copyright on the software, and (2) offer you this License which gives you legal permission to copy, distribute and/or modify the software.

A secondary benefit of defending all users' freedom is that improvements made in alternate versions of the program, if they receive widespread use, become available for other developers to incorporate. Many developers of free software are heartened and encouraged by the resulting cooperation. However, in the case of software used on network servers, this result may fail to come about. The GNU General Public License permits making a modified version and letting the public access it on a server without ever releasing its source code to the public.

The GNU Affero General Public License is designed specifically to ensure that, in such cases, the modified source code becomes available to the community. It requires the operator of a network server to provide the source code of the modified version running there to the users of that server. Therefore, public use of a modified version, on a publicly accessible server, gives the public access to the source code of the modified version.

An older license, called the Affero General Public License and published by Affero, was designed to accomplish similar goals. This is a different license, not a version of the Affero GPL, but Affero has released a new version of the Affero GPL which permits relicensing under this license.

The precise terms and conditions for copying, distribution and modification follow.
";

#[test]
fn test_snippet_extraction_works() -> Result<(), Error> {
    let workspace = find_workspace_dir();
    let mut extractor = SnippetExtractor::new(Config {
        language: "english".into(),
        chunk_size: 50,
        hard_chunk_size_limit: 55,
        use_pipenv: true,
        tokenizers: [(
            "default".into(),
            workspace.join("assets/xaynia_v0201/tokenizer.json"),
        )]
        .into(),
        python_workspace: workspace.join("snippet-extractor"),
    })?;

    assert_eq!(extractor.extract_snippet("default", TEST_TEXT)?, [
        "The GNU Affero General Public License is a free, copyleft license for software and other kinds of works, specifically designed to ensure cooperation with the community in the case of network server software.",
        "The licenses for most software and other practical works are designed to take away your freedom to share and change the works.",
        "By contrast, our General Public Licenses are intended to guarantee your freedom to share and change all versions of a program--to make sure it remains free software for all its users.",
        "When we speak of free software, we are referring to freedom, not price.\nOur General Public Licenses are designed to make sure that\nyou have the freedom to distribute copies of free software",
        "(and charge for them if you wish), that you\nreceive source code or can get it if you want\nit, that you can change the software or use pieces",
        "of it in new free programs, and that you know\nyou can do these things.",
        "Developers that use our General Public Licenses protect your rights with two steps: (1) assert copyright on the software, and (2) offer you this License which gives you legal permission to copy, distribute and/or modify the software.",
        "A secondary benefit of defending all users' freedom is that improvements made in alternate versions of the program, if they receive widespread use, become available for other developers to incorporate.",
        "Many developers of free software are heartened and encouraged by the resulting cooperation.\n\nHowever, in the case of software used on network servers, this result may fail to come about.",
        "The GNU General Public License permits making a modified version and letting the public access it on a server without ever releasing its source code to the public.",
        "The GNU Affero General Public License is designed specifically to ensure that, in such cases, the modified source code becomes available to the community.",
        "It requires the operator of a network server to provide the source code of the modified version running there to the users of that server.",
        "Therefore, public use of a modified version, on a publicly accessible server, gives the public access to the source code of the modified version.",
        "An older license, called the Affero General Public License and published by Affero, was designed to accomplish similar goals.",
        "This is a different license, not a version of the Affero GPL, but Affero has released a new version of the Affero GPL which permits relicensing under this license.",
        "The precise terms and conditions for copying, distribution and modification follow.",
    ]);

    Ok(())
}
