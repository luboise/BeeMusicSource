#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Slice {
    pub measure: u64,
    pub submeasure: f32,
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
