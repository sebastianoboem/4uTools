use std::collections::HashMap;

pub const CATEGORY_DISPLAY_ORDER: &[&str] = &[
    "Immagini",
    "Video",
    "Documenti",
    "Audio",
    "Archivi",
];

pub fn default_categories() -> HashMap<&'static str, Vec<&'static str>> {
    HashMap::from([
        (
            "Immagini",
            vec![
                ".jpg", ".jpeg", ".png", ".gif", ".bmp", ".tiff", ".raw", ".ico", ".heic",
                ".heif", ".webp",
            ],
        ),
        (
            "Video",
            vec![".mp4", ".avi", ".mkv", ".mov", ".wmv", ".flv", ".webm"],
        ),
        (
            "Documenti",
            vec![
                ".doc", ".docx", ".pdf", ".txt", ".xlsx", ".xls", ".pptx", ".ppt", ".odt",
            ],
        ),
        (
            "Audio",
            vec![".mp3", ".wav", ".aac", ".flac", ".ogg", ".wma", ".m4a"],
        ),
        (
            "Archivi",
            vec![".zip", ".rar", ".7z", ".tar", ".gz"],
        ),
    ])
}

pub fn categories_in_display_order() -> Vec<(&'static str, Vec<&'static str>)> {
    let cats = default_categories();
    CATEGORY_DISPLAY_ORDER
        .iter()
        .filter_map(|name| cats.get(*name).map(|exts| (*name, exts.clone())))
        .collect()
}

pub fn default_deselected_extensions() -> Vec<&'static str> {
    vec![
        ".txt", ".gif", ".bmp", ".tiff", ".raw", ".ico", ".mkv", ".flv", ".flac", ".ogg",
        ".aac", ".tar", ".gz",
    ]
}
