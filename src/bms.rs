#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Slice {
    time_point: crate::audio::TimePoint,
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
