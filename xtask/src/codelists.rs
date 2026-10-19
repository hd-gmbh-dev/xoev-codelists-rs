//! Codelist datasets: which codelists belong to which publisher and whether
//! they carry a version history (XÖV code type 3) or a version fixed by the
//! standard (code type 1/2).
//!
//! The list was extracted from the `xwasser-baukasten.xsd` of XWasser 1.0.1: a
//! `Code.*Type` is type 3 exactly when its appinfo has a `<codeliste>` block but
//! no `<versionCodeliste>`, i.e. the sender picks the version.

pub struct Codelist {
    pub urn: &'static str,
    /// Type-3 codelists accumulate versions; type-1/2 keep the pinned one.
    pub versioned: bool,
}

pub struct Publisher {
    pub crate_name: &'static str,
    pub codelists: &'static [Codelist],
}

const fn versioned(urn: &'static str) -> Codelist {
    Codelist {
        urn,
        versioned: true,
    }
}

pub const PUBLISHERS: &[Publisher] = &[
    Publisher {
        crate_name: "xwasser",
        codelists: &[
            versioned("urn:xoev-de:xwasser:codeliste:untersuchung-anlass"),
            versioned("urn:xoev-de:xwasser:codeliste:art-entnahmearmatur"),
            versioned("urn:xoev-de:xwasser:codeliste:art-objekt"),
            versioned("urn:xoev-de:xwasser:codeliste:art-probennahmestelle"),
            versioned("urn:xoev-de:xwasser:codeliste:aufbereitungsstoffe-desinfektionsverfahren"),
            versioned("urn:xoev-de:xwasser:codeliste:desinfektionsart"),
            versioned("urn:xoev-de:xwasser:codeliste:massnahmen"),
            versioned("urn:xoev-de:xwasser:codeliste:medium"),
            versioned("urn:xoev-de:xwasser:codeliste:parameterauspraegung"),
            versioned("urn:xoev-de:xwasser:codeliste:probenentnahmegeraet"),
            versioned("urn:xoev-de:xwasser:codeliste:probengefaess"),
            versioned("urn:xoev-de:xwasser:codeliste:probennahmeverfahren"),
            versioned("urn:xoev-de:xwasser:codeliste:shapth-parameter-einheit"),
            versioned("urn:xoev-de:xwasser:codeliste:shapth-parameter"),
            versioned("urn:xoev-de:xwasser:codeliste:unterkategorie-probennahmestelle"),
            versioned("urn:xoev-de:xwasser:codeliste:untersuchungsverfahren"),
        ],
    },
    Publisher {
        crate_name: "destatis",
        codelists: &[
            versioned("urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:ags"),
            versioned("urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:bezirk"),
            versioned("urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:kreis"),
            versioned("urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:rs"),
            versioned("urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:staat"),
            versioned(
                "urn:de:bund:destatis:bevoelkerungsstatistik:schluessel:staatsangehoerigkeit",
            ),
        ],
    },
    Publisher {
        crate_name: "xunternehmen",
        codelists: &[
            versioned("urn:xoev-de:xunternehmen:codeliste:artgesetzlichervertreter"),
            versioned("urn:xoev-de:xunternehmen:codeliste:rechtsformen"),
        ],
    },
    Publisher {
        crate_name: "dsmeld",
        codelists: &[versioned(
            "urn:de:dsmeld:schluesseltabelle:pass.und.ausweisdokumente",
        )],
    },
    Publisher {
        crate_name: "xauslaender",
        codelists: &[versioned(
            "urn:xoev-de:xauslaender:codeliste:sprachenkatalog",
        )],
    },
    Publisher {
        crate_name: "bmi-bit",
        codelists: &[versioned("urn:xoev-de:bund:bmi:bit:codeliste:dvdv.praefix")],
    },
];

/// File name of a codelist dataset: last URN segment, `_`/`.` replaced by `-`.
pub fn slug(urn: &str) -> String {
    urn.rsplit(':')
        .next()
        .unwrap_or(urn)
        .replace(['_', '.'], "-")
}
/// Look up a publisher by crate name, e.g. `xwasser`.
pub fn publisher(crate_name: &str) -> anyhow::Result<&'static Publisher> {
    PUBLISHERS
        .iter()
        .find(|publisher| publisher.crate_name == crate_name)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "unknown publisher '{crate_name}' (known: {})",
                PUBLISHERS
                    .iter()
                    .map(|publisher| publisher.crate_name)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}
