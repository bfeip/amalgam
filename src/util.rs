pub fn float_eq(a: f32, b:f32, variation: f32) -> bool {
    f32::abs(a - b) < variation
}

#[cfg(test)]
pub mod test_util {
    use std::path::PathBuf;
    use std::{env, fs};

    pub fn get_repo_root() -> PathBuf {
        let mut cur_dir = env::current_dir().expect("Couldn't get working dir?");
        loop {
            let contents = match fs::read_dir(&cur_dir) {
                Ok(contents) => contents,
                Err(_err) => {
                    panic!("Failed to read contents of {}", cur_dir.display());
                }
            };
            for dir_item in contents {
                if dir_item.is_err() {
                    continue;
                }
                let item_name = dir_item.unwrap().file_name();
                if item_name.to_str().unwrap() == "Cargo.toml" {
                    return cur_dir;
                }
            }
            if cur_dir.pop() == false {
                panic!("Failed to find repo root");
            }
        }
    }


    pub fn get_test_midi_file_path() -> PathBuf {
        let test_midi_file_path_from_root: PathBuf = ["data", "basic_test.mid"].iter().collect();
        let repo_root = get_repo_root();
        repo_root.join(test_midi_file_path_from_root)
    }
}