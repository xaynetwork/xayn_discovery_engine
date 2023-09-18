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

use xayn_snippet_extractor::{
    pool::{self, SnippetExtractorPool},
    Config,
    Error,
    SnippetExtractor,
};
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
        tokenizers: [(
            "default".into(),
            workspace.join("assets/xaynia_v0201/tokenizer.json"),
        )]
        .into(),
        python_workspace: workspace.join("snippet-extractor"),
        ..Default::default()
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

#[tokio::test]
async fn test_extractor_can_be_reused() {
    let limit_to_one_thread = (num_cpus::get() as f32).recip() / 2.;
    let workspace = find_workspace_dir();
    let pool = SnippetExtractorPool::new(&Config {
        language: "english".into(),
        chunk_size: 50,
        hard_chunk_size_limit: 55,
        tokenizers: [(
            "default".into(),
            workspace.join("assets/xaynia_v0201/tokenizer.json"),
        )]
        .into(),
        python_workspace: workspace.join("snippet-extractor"),
        pool: pool::Config {
            threads_per_cpu: limit_to_one_thread,
            ..Default::default()
        },
        automatically_restart_child: false,
        force_initialization: true,
    })
    .unwrap();

    let extractor = pool.get().await.unwrap();
    extractor
        .extract_snippet("default".into(), TEST_TEXT.into())
        .await
        .unwrap();
    let extractor = pool.get().await.unwrap();
    extractor
        .extract_snippet("default".into(), TEST_TEXT.into())
        .await
        .unwrap();
}

// from DRACULA by Bram Stoker
const PUBLIC_DOMAIN_TEXT: &str = "
3 May. Bistritz.—Left Munich at 8:35 P. M., on 1st May, arriving at Vienna early next morning; should have arrived at 6:46, but train was an hour late. Buda-Pesth seems a wonderful place, from the glimpse which I got of it from the train and the little I could walk through the streets. I feared to go very far from the station, as we had arrived late and would start as near the correct time as possible. The impression I had was that we were leaving the West and entering the East; the most western of splendid bridges over the Danube, which is here of noble width and depth, took us among the traditions of Turkish rule.

We left in pretty good time, and came after nightfall to Klausenburgh. Here I stopped for the night at the Hotel Royale. I had for dinner, or rather supper, a chicken done up some way with red pepper, which was very good but thirsty. (Mem., get recipe for Mina.) I asked the waiter, and he said it was called “paprika hendl,” and that, as it was a national dish, I should be able to get it anywhere along the Carpathians. I found my smattering of German very useful here; indeed, I don’t know how I should be able to get on without it.

Having had some time at my disposal when in London, I had visited the British Museum, and made search among the books and maps in the library regarding Transylvania; it had struck me that some foreknowledge of the country could hardly fail to have some importance in dealing with a nobleman of that country. I find that the district he named is in the extreme east of the country, just on the borders of three states, Transylvania, Moldavia and Bukovina, in the midst of the Carpathian mountains; one of the wildest and least known portions of Europe. I was not able to light on any map or work giving the exact locality of the Castle Dracula, as there are no maps of this country as yet to compare with our own Ordnance Survey maps; but I found that Bistritz, the post town named by Count Dracula, is a fairly well-known place. I shall enter here some of my notes, as they may refresh my memory when I talk over my travels with Mina.

In the population of Transylvania there are four distinct nationalities: Saxons in the South, and mixed with them the Wallachs, who are the descendants of the Dacians; Magyars in the West, and Szekelys in the East and North. I am going among the latter, who claim to be descended from Attila and the Huns. This may be so, for when the Magyars conquered the country in the eleventh century they found the Huns settled in it. I read that every known superstition in the world is gathered into the horseshoe of the Carpathians, as if it were the centre of some sort of imaginative whirlpool; if so my stay may be very interesting. (Mem., I must ask the Count all about them.)

I did not sleep well, though my bed was comfortable enough, for I had all sorts of queer dreams. There was a dog howling all night under my window, which may have had something to do with it; or it may have been the paprika, for I had to drink up all the water in my carafe, and was still thirsty. Towards morning I slept and was wakened by the continuous knocking at my door, so I guess I must have been sleeping soundly then. I had for breakfast more paprika, and a sort of porridge of maize flour which they said was “mamaliga,” and egg-plant stuffed with forcemeat, a very excellent dish, which they call “impletata.” (Mem., get recipe for this also.) I had to hurry breakfast, for the train started a little before eight, or rather it ought to have done so, for after rushing to the station at 7:30 I had to sit in the carriage for more than an hour before we began to move. It seems to me that the further east you go the more unpunctual are the trains. What ought they to be in China?

All day long we seemed to dawdle through a country which was full of beauty of every kind. Sometimes we saw little towns or castles on the top of steep hills such as we see in old missals; sometimes we ran by rivers and streams which seemed from the wide stony margin on each side of them to be subject to great floods. It takes a lot of water, and running strong, to sweep the outside edge of a river clear. At every station there were groups of people, sometimes crowds, and in all sorts of attire. Some of them were just like the peasants at home or those I saw coming through France and Germany, with short jackets and round hats and home-made trousers; but others were very picturesque. The women looked pretty, except when you got near them, but they were very clumsy about the waist. They had all full white sleeves of some kind or other, and most of them had big belts with a lot of strips of something fluttering from them like the dresses in a ballet, but of course there were petticoats under them. The strangest figures we saw were the Slovaks, who were more barbarian than the rest, with their big cow-boy hats, great baggy dirty-white trousers, white linen shirts, and enormous heavy leather belts, nearly a foot wide, all studded over with brass nails. They wore high boots, with their trousers tucked into them, and had long black hair and heavy black moustaches. They are very picturesque, but do not look prepossessing. On the stage they would be set down at once as some old Oriental band of brigands. They are, however, I am told, very harmless and rather wanting in natural self-assertion.
";

#[tokio::test]
async fn test_getting_started_example() {
    // WARNING: If the results of this change (getting_started.md)[../../docs/source/getting_started/getting_started.md] has to change, too
    let workspace = find_workspace_dir();
    let pool = SnippetExtractorPool::new(&Config {
        tokenizers: [(
            "default".into(),
            workspace.join("assets/xaynia_v0201/tokenizer.json"),
        )]
        .into(),
        python_workspace: workspace.join("snippet-extractor"),
        automatically_restart_child: false,
        force_initialization: false,
        ..Default::default()
    })
    .unwrap();

    let extractor = pool.get().await.unwrap();
    let snippets = extractor
        .extract_snippet("default".into(), PUBLIC_DOMAIN_TEXT.into())
        .await
        .unwrap();
    let len = snippets.len();
    assert!(len > 1, "need more then 1 snippet (got {len})");
    dbg!(PUBLIC_DOMAIN_TEXT);
    assert_eq!(snippets, [
        "3 May.\n\nBistritz.—Left Munich at 8:35 P. M., on 1st May, arriving at Vienna early next morning; should have arrived at 6:46, but train was an hour late.\n\nBuda-Pesth seems a wonderful place, from the glimpse which I got of it from the train and the little I could walk through the streets.\n\nI feared to go very far from the station, as we had arrived late and would start as near the correct time as possible.\n\nThe impression I had was that we were leaving the West and entering the East; the most western of splendid bridges over the Danube, which is here of noble width and depth, took us among the traditions of Turkish rule.\n\nWe left in pretty good time, and came after nightfall to Klausenburgh.\n\nHere I stopped for the night at the Hotel Royale.\n\nI had for dinner, or rather supper, a chicken done up some way with red pepper, which was very good but thirsty.\n\n(Mem., get recipe for Mina.)\n\nI asked the waiter, and he said it was called “paprika hendl,” and that, as it was a national dish, I should be able to get it anywhere along the Carpathians.\n\nI found my smattering of German very useful here; indeed, I don’t know how I should be able to get on without it.\n\nHaving had some time at my disposal when in London, I had visited the British Museum, and made search among the books and maps in the library regarding Transylvania; it had struck me that some foreknowledge of the country could hardly fail to have some importance in dealing with a nobleman of that country.\n\nI find that the district he named is in the extreme east of the country, just on the borders of three states, Transylvania, Moldavia and Bukovina, in the midst of the Carpathian mountains; one of the wildest and least known portions of Europe.",
        "I was not able to light on any map or work giving the exact locality of the Castle Dracula, as there are no maps of this country as yet to compare with our own Ordnance Survey maps; but I found that Bistritz, the post town named by Count Dracula, is a fairly well-known place.\n\nI shall enter here some of my notes, as they may refresh my memory when I talk over my travels with Mina.\n\nIn the population of Transylvania there are four distinct nationalities: Saxons in the South, and mixed with them the Wallachs, who are the descendants of the Dacians; Magyars in the West, and Szekelys in the East and North.\n\nI am going among the latter, who claim to be descended from Attila and the Huns.\n\nThis may be so, for when the Magyars conquered the country in the eleventh century they found the Huns settled in it.\n\nI read that every known superstition in the world is gathered into the horseshoe of the Carpathians, as if it were the centre of some sort of imaginative whirlpool; if so my stay may be very interesting.\n\n(Mem., I must ask the Count all about them.)\n\nI did not sleep well, though my bed was comfortable enough, for I had all sorts of queer dreams.\n\nThere was a dog howling all night under my window, which may have had something to do with it; or it may have been the paprika, for I had to drink up all the water in my carafe, and was still thirsty.\n\nTowards morning I slept and was wakened by the continuous knocking at my door, so I guess I must have been sleeping soundly then.\n\nI had for breakfast more paprika, and a sort of porridge of maize flour which they said was “mamaliga,” and egg-plant stuffed with forcemeat, a very excellent dish, which they call “impletata.” (Mem., get recipe for this also.)",
        "I had to hurry breakfast, for the train started a little before eight, or rather it ought to have done so, for after rushing to the station at 7:30 I had to sit in the carriage for more than an hour before we began to move.\n\nIt seems to me that the further east you go the more unpunctual are the trains.\n\nWhat ought they to be in China?\n\nAll day long we seemed to dawdle through a country which was full of beauty of every kind.\n\nSometimes we saw little towns or castles on the top of steep hills such as we see in old missals; sometimes we ran by rivers and streams which seemed from the wide stony margin on each side of them to be subject to great floods.\n\nIt takes a lot of water, and running strong, to sweep the outside edge of a river clear.\n\nAt every station there were groups of people, sometimes crowds, and in all sorts of attire.\n\nSome of them were just like the peasants at home or those I saw coming through France and Germany, with short jackets and round hats and home-made trousers; but others were very picturesque.\n\nThe women looked pretty, except when you got near them, but they were very clumsy about the waist.\n\nThey had all full white sleeves of some kind or other, and most of them had big belts with a lot of strips of something fluttering from them like the dresses in a ballet, but of course there were petticoats under them.\n\nThe strangest figures we saw were the Slovaks, who were more barbarian than the rest, with their big cow-boy hats, great baggy dirty-white trousers, white linen shirts, and enormous heavy leather belts, nearly a foot wide, all studded over with brass nails.\n\nThey wore high boots, with their trousers tucked into them, and had long black hair and heavy black moustaches.\n\nThey are very picturesque, but do not look prepossessing.\n\nOn the stage they would be set down at once as some old Oriental band of brigands.\n\nThey are, however, I am told, very harmless and rather wanting in natural self-assertion.",
    ]);
}
