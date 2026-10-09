//! Curated summaries of upstream capabilities. Versions always come from AppState.
// Reviewed 2026-10-09: getartcraft.com/apps/{app}; github.com/storytold/{app}.
pub(super) struct Profile {
    pub summary: &'static str,
    pub website: &'static str,
    pub platforms: &'static str,
}
pub(super) fn for_app(id: &str) -> Profile {
    match id {
        "photocraft" => Profile {
            summary: "Edit layered images with local selection tools and flexible color controls.",
            website: "https://getartcraft.com/apps/photocraft",
            platforms: "macOS · Windows · Linux · Web",
        },
        "vectorcraft" => Profile {
            summary: "Build illustrations with precise paths, editable effects and flexible export options.",
            website: "https://getartcraft.com/apps/vectorcraft",
            platforms: "macOS · Windows · Linux · Web",
        },
        "filmcraft" => Profile {
            summary: "Cut footage, refine color and mix sound on a multitrack timeline.",
            website: "https://getartcraft.com/apps/filmcraft",
            platforms: "macOS · Windows · Linux",
        },
        "lightcraft" => Profile {
            summary: "Organize a photo collection and develop raw images while preserving the originals.",
            website: "https://getartcraft.com/apps/lightcraft",
            platforms: "macOS · Windows · Linux · Web",
        },
        "printcraft" => Profile {
            summary: "Read PDFs, rearrange pages and assemble documents with recovery and encryption support.",
            website: "https://getartcraft.com/apps/pdfcraft",
            platforms: "macOS · Windows · Linux · Web",
        },
        "effectcraft" => Profile {
            summary: "Create layered animations and visual effects with cameras, expressions and export tools.",
            website: "https://getartcraft.com/apps/effectcraft",
            platforms: "macOS · Windows · Linux · Web",
        },
        "designcraft" => Profile {
            summary: "Create publications with linked text, reusable page layouts and print-focused output.",
            website: "https://getartcraft.com/apps/designcraft",
            platforms: "macOS · Windows · Linux · Web",
        },
        "soundcraft" => Profile {
            summary: "Record and arrange audio, compose MIDI parts and balance a complete mix.",
            website: "https://github.com/storytold/soundcraft",
            platforms: "macOS · Windows · Linux · Web",
        },
        "cadcraft" => Profile {
            summary: "Draft technical drawings with precise geometry, dimensions and reusable layouts.",
            website: "https://github.com/storytold/cadcraft",
            platforms: "macOS · Windows · Linux · Web",
        },
        "gridcraft" => Profile {
            summary: "Build spreadsheets with formulas, structured tables, charts and familiar workbook formats.",
            website: "https://github.com/storytold/gridcraft",
            platforms: "macOS · Windows · Linux · Web",
        },
        "wordcraft" => Profile {
            summary: "Write and review documents with page layout, references and common file formats.",
            website: "https://github.com/storytold/wordcraft",
            platforms: "macOS · Windows · Linux · Web",
        },
        "deckcraft" => Profile {
            summary: "Design slides, animate content and present decks with notes and presenter tools.",
            website: "https://github.com/storytold/deckcraft",
            platforms: "macOS · Windows · Linux · Web",
        },
        _ => Profile {
            summary: "",
            website: "https://getartcraft.com/apps",
            platforms: "",
        },
    }
}
