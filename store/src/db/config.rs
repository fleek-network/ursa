use num_cpus;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Config {
    pub create_if_missing: bool,
    pub parallelism: i32,
    pub write_buffer_size: usize,
    pub max_open_files: i32,
    pub set_blob_file_size: todo!(),
    pub max_background_jobs: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            create_if_missing: true,
            parallelism: num_cpus::get() as i32,
            write_buffer_size: 256 * 1024 * 1024,
            max_open_files: 1024,
            set_blob_file_size: (),
            max_background_jobs: 0
        }
    }
}
