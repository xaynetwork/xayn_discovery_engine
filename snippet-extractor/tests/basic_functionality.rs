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

// from BAnz AT 13.07.2023 B1 page 3
const SAMPLE_TEXT: &str = "6.2  Die  Vergabe  von  Unteraufträgen  hat  nach  Möglichkeit  im  Wettbewerb  zu  erfolgen.  Bei  der  Einholung  von  An-
geboten  für  Unteraufträge  sind  kleine  und  mittlere,  nicht  konzerngebundene  Unternehmen  soweit  möglich  zu  betei-
ligen.  Die  in  Betracht  kommenden  Unternehmen  sind  dem  Auftraggeber  vom  Auftragnehmer  auf  Verlangen  vor  der
Erteilung des Unterauftrags zu benennen.
6.3  Der Auftragnehmer zeigt dem Auftraggeber jeden Unterauftrag sowie jeden Wechsel eines Unterauftragnehmers
nach Erteilung des jeweiligen Unterauftrags bis zum Ende der jeweiligen Vertragslaufzeit unverzüglich und unaufge-
fordert in Textform an. Maßgeblich ist das Datum des Vertragsschlusses. Dabei teilt der Auftragnehmer  mindestens
den Namen und die Anschrift des Unterauftragnehmers mit sowie den Gegenstand des Unterauftrags. Die Anzeige-
pflicht  entfällt,  wenn  dem  Auftraggeber  die  Informationen  bereits  aus  dem  Angebot  des  Auftragnehmers  bzw.  den
Vergabeunterlagen bekannt sind.
6.4  Hat der Auftraggeber in der Bekanntmachung oder in den Vergabeunterlagen Anforderungen über die Eignung
oder Auftragserfüllung für Unterauftragnehmer aufgestellt, sind diese von allen Unterauftragnehmern zu erfüllen. Dies
gilt auch im Fall des Austauschs von Unterauftragnehmern während der Vertragslaufzeit. Der Auftragnehmer legt dem
Auftraggeber  erforderliche  Nachweise  seiner  Unterauftragnehmer  unverzüglich  und  unaufgefordert  mit  der  Anzeige
gemäß Nummer 6.3 vor.
6.5  Vergibt der Auftragnehmer Unteraufträge, so hat er durch entsprechende Vereinbarungen mit den Unterauftrag-
nehmern  dem  Auftraggeber  die  gleichen  Rechte  und  Ansprüche  zu  verschaffen,  die  der  Auftraggeber  gegen  den
Auftragnehmer hat. Hierzu gehören auch die Nutzungsrechte des Auftraggebers an allen vom Auftragnehmer geschul-
deten Vertragsergebnissen.
6.6  Gelingt dies dem Auftragnehmer im Einzelfall nicht, so hat er den Auftraggeber darüber unverzüglich in Textform
zu  unterrichten  und  ihm  auf  Verlangen  Gelegenheit  zu  geben,  an  den  weiteren  Verhandlungen  mit  dem  jeweiligen
Unterauftragnehmer teilzunehmen und die Entscheidung des Auftraggebers abzuwarten.
6.7  Akzeptiert der Unterauftragnehmer die Vereinbarung entsprechender Regelungen nach Abschluss der weiteren
Verhandlungen  nicht,  hat  der  Auftragnehmer  dies  dem  Auftraggeber  in  Textform  anzuzeigen,  das  Verhandlungs-
ergebnis vorzulegen und die Entscheidung des Auftraggebers darüber, ob er seine Einwilligung zum Vertragsschluss
erklärt, einzuholen. Entscheidet sich der Auftraggeber nicht binnen eines Monats nach Zugang der Anzeige, so ist der
Auftragnehmer  berechtigt,  den  Unterauftrag  entsprechend  dem  vorgelegten  Verhandlungsergebnis  abzuschließen.
Erteilt der Auftraggeber seine ausdrückliche Einwilligung zum Vertragsschluss oder erfolgt der Vertragsschluss nach
Ablauf der Monatsfrist, bleibt die Haftung des Auftragnehmers für die vertragsgemäße Ausführung seiner Leistungen
gegenüber dem Auftraggeber unberührt.";

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
        .extract_snippet("default".into(), SAMPLE_TEXT.into())
        .await
        .unwrap();
    let len = snippets.len();
    assert!(len > 1, "need more then 1 snippet (got {len})");
    assert_eq!(snippets, [
        "6.2  Die  Vergabe  von  Unteraufträgen  hat  nach  Möglichkeit  im  Wettbewerb  zu  erfolgen.\n\nBei  der  Einholung  von  An-\ngeboten  für  Unteraufträge  sind  kleine  und  mittlere,  nicht  konzerngebundene  Unternehmen  soweit  möglich  zu  betei-\nligen.\n\nDie  in  Betracht  kommenden  Unternehmen  sind  dem  Auftraggeber  vom  Auftragnehmer  auf  Verlangen  vor  der\nErteilung des Unterauftrags zu benennen.\n\n6.3  Der Auftragnehmer zeigt dem Auftraggeber jeden Unterauftrag sowie jeden Wechsel eines Unterauftragnehmers\nnach Erteilung des jeweiligen Unterauftrags bis zum Ende der jeweiligen Vertragslaufzeit unverzüglich und unaufge-\nfordert in Textform an.\n\nMaßgeblich ist das Datum des Vertragsschlusses.\n\nDabei teilt der Auftragnehmer  mindestens\nden Namen und die Anschrift des Unterauftragnehmers mit sowie den Gegenstand des Unterauftrags.\n\nDie Anzeige-\npflicht  entfällt,  wenn  dem  Auftraggeber  die  Informationen  bereits  aus  dem  Angebot  des  Auftragnehmers  bzw.\n\nden\nVergabeunterlagen bekannt sind.\n\n6.4  Hat der Auftraggeber in der Bekanntmachung oder in den Vergabeunterlagen Anforderungen über die Eignung\noder Auftragserfüllung für Unterauftragnehmer aufgestellt, sind diese von allen Unterauftragnehmern zu erfüllen.\n\nDies\ngilt auch im Fall des Austauschs von Unterauftragnehmern während der Vertragslaufzeit.",
        "Der Auftragnehmer legt dem\nAuftraggeber  erforderliche  Nachweise  seiner  Unterauftragnehmer  unverzüglich  und  unaufgefordert  mit  der  Anzeige\ngemäß Nummer 6.3 vor.\n\n6.5  Vergibt der Auftragnehmer Unteraufträge, so hat er durch entsprechende Vereinbarungen mit den Unterauftrag-\nnehmern  dem  Auftraggeber  die  gleichen  Rechte  und  Ansprüche  zu  verschaffen,  die  der  Auftraggeber  gegen  den\nAuftragnehmer hat.\n\nHierzu gehören auch die Nutzungsrechte des Auftraggebers an allen vom Auftragnehmer geschul-\ndeten Vertragsergebnissen.\n\n6.6  Gelingt dies dem Auftragnehmer im Einzelfall nicht, so hat er den Auftraggeber darüber unverzüglich in Textform\nzu  unterrichten  und  ihm  auf  Verlangen  Gelegenheit  zu  geben,  an  den  weiteren  Verhandlungen  mit  dem  jeweiligen\nUnterauftragnehmer teilzunehmen und die Entscheidung des Auftraggebers abzuwarten.\n\n6.7  Akzeptiert der Unterauftragnehmer die Vereinbarung entsprechender Regelungen nach Abschluss der weiteren\nVerhandlungen  nicht,  hat  der  Auftragnehmer  dies  dem  Auftraggeber  in  Textform  anzuzeigen,  das  Verhandlungs-\nergebnis vorzulegen und die Entscheidung des Auftraggebers darüber, ob er seine Einwilligung zum Vertragsschluss\nerklärt, einzuholen.",
        "Entscheidet sich der Auftraggeber nicht binnen eines Monats nach Zugang der Anzeige, so ist der\nAuftragnehmer  berechtigt,  den  Unterauftrag  entsprechend  dem  vorgelegten  Verhandlungsergebnis  abzuschließen.\n\nErteilt der Auftraggeber seine ausdrückliche Einwilligung zum Vertragsschluss oder erfolgt der Vertragsschluss nach\nAblauf der Monatsfrist, bleibt die Haftung des Auftragnehmers für die vertragsgemäße Ausführung seiner Leistungen\ngegenüber dem Auftraggeber unberührt."
    ]);
}
