#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, PartialOrd)]
pub struct Slice {
    pub time_point: crate::audio::TimePoint,
    // Room here later to add de-duplication of keysounds and custom keysound IDs
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Stem {
    pub audio_path: std::path::PathBuf,
    pub slices: Vec<Slice>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default)]
pub struct Project {
    pub stems: Vec<Stem>,
    pub bpm_changes: Vec<crate::audio::BPMChange>,
}

pub fn normalise_project_path(path: impl AsRef<std::path::Path>) -> std::path::PathBuf {
    let path = path.as_ref();

    if path.is_file() {
        path.to_path_buf()
    } else {
        path.join("project.jonnah")
    }
}

pub fn load_project(
    path: impl AsRef<std::path::Path>,
) -> Result<Project, Box<dyn std::error::Error>> {
    let load_path = normalise_project_path(path.as_ref());

    let project =
        serde_json::from_reader(std::io::BufReader::new(std::fs::File::open(load_path)?))?;

    Ok(project)
}

pub fn save_project(
    project: &Project,
    path: impl AsRef<std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let save_path = normalise_project_path(path.as_ref());

    eprintln!("Saving project to {}", save_path.display());

    serde_json::to_writer_pretty(
        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .map_err(|e| e.to_string())?,
        &project,
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}
