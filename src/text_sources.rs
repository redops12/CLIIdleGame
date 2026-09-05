use std::collections::HashMap;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

const WASTELAND: &str = include_str!("../assets/wasteland.txt");
const INTRO: &str = include_str!("../assets/intro.txt");
const BARTLEBY: &str = include_str!("../assets/bartleby.txt");

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextSource {
    Wasteland,
    Intro,
    IntroCapital,
    Bartleby,
}

static TEXT_SOURCES: LazyLock<HashMap<TextSource, Vec<String>>> =
LazyLock::new(|| {
    HashMap::from([
        (TextSource::Wasteland, WASTELAND.lines().map(String::from).collect()),
        (TextSource::Intro, INTRO.lines().map(str::to_lowercase).collect()),
        (TextSource::IntroCapital, INTRO.lines().map(String::from).collect()),
        (TextSource::Bartleby, BARTLEBY.lines().map(String::from).collect()),
    ])
});

pub fn get_lines_from_source(source: TextSource, offset: usize) -> &'static str {
    let lines = TEXT_SOURCES.get(&source).expect("Text source not found");
    lines.get(offset).map(|s| s.as_str()).expect("Offset out of bounds for text source")
}
