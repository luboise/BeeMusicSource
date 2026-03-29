#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, PartialOrd)]
pub struct Slice {
    pub time_point: crate::audio::TimePoint,
    // Room here later to add de-duplication of keysounds and custom keysound IDs
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Stem {
    pub audio_path: std::path::PathBuf,
    pub slices: Vec<Slice>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Project {
    pub stems: Vec<Stem>,
}
