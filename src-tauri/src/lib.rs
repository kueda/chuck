use std::path::PathBuf;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn create_internal_dir_for_archive(app: &tauri::AppHandle, path: &str) -> PathBuf {
    use tauri::Manager;
    let inpath = std::path::Path::new(&path);
    let mut file = std::fs::File::open(inpath).unwrap();
    let fname = inpath.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();
    let local_data_dir = app.path().app_local_data_dir().unwrap();
    use sha2::{Sha256, Digest};
    // use std::io::Read;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).unwrap();
    let file_hash = hasher.finalize();
    let file_hash_string = format!("{fname}-{:x}", file_hash);
    let target_dir = local_data_dir.join(file_hash_string);
    std::fs::create_dir_all(&target_dir).unwrap();
    target_dir
}

fn unzip_archive(path: &str, target_dir: &PathBuf) {
    let inpath = std::path::Path::new(&path);
    let file = std::fs::File::open(inpath).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = match file.enclosed_name() {
            Some(path) => target_dir.join(path),
            None => continue,
        };

        {
            let comment = file.comment();
            if !comment.is_empty() {
                println!("File {i} comment: {comment}");
            }
        }

        if file.is_dir() {
            println!("File {} extracted to \"{}\"", i, outpath.display());
            std::fs::create_dir_all(&outpath).unwrap();
        } else {
            println!(
                "File {} extracted to \"{}\" ({} bytes)",
                i,
                outpath.display(),
                file.size()
            );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p).unwrap();
                }
            }
            let mut outfile = std::fs::File::create(&outpath).unwrap();
            std::io::copy(&mut file, &mut outfile).unwrap();
        }

        // Get and Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode)).unwrap();
            }
        }
    }
}

fn read_core_file_locations(target_dir: &PathBuf) -> Vec<PathBuf> {
    let meta_path = target_dir.join("meta.xml");
    let contents = std::fs::read_to_string(&meta_path).unwrap();
    let doc = roxmltree::Document::parse(&contents).unwrap();

    doc.descendants()
        .filter(|n| n.has_tag_name("core"))
        .flat_map(|core| core.descendants())
        .filter(|n| n.has_tag_name("location"))
        .filter_map(|n| n.text())
        .map(|text| target_dir.join(text))
        .collect()
}

fn create_duckdb(core_files: Vec<PathBuf>, target_dir: &PathBuf) -> PathBuf {
    let dirname = target_dir.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();
    let dbname = dirname
        .split(".zip-")
        .next()
        .unwrap_or("unknown");
    let dbpath = target_dir.join(format!("{dbname}.db"));
    let conn = duckdb::Connection::open(&dbpath)
        .expect("Failed to open duckdb db");
    let csv_path = if let Some(p) = core_files[0].to_str() {
        p
    } else {
        panic!("aaaghgh");
    };
    conn.execute(
        &format!(
            "
                CREATE TABLE observations AS
                SELECT * FROM read_csv_auto('{}')
            ",
            csv_path
        ),
        []
    ).expect("Failed to insert into duckdb");
    dbpath
}

#[tauri::command]
fn open_archive(app: tauri::AppHandle, path: &str) -> usize {
    println!("called open_archive, path: {path}");

    // get internal path
    let target_dir = create_internal_dir_for_archive(&app, &path);
    println!("Created target dir: {:?}", target_dir);

    // unzip
    unzip_archive(&path, &target_dir);
    println!("Copied zip contents to {:?}", target_dir);

    // read metadata from {target_dir}/meta.xml
    let core_files = read_core_file_locations(&target_dir);
    println!("Core files: {:?}", core_files);

    // TODO: create duckdb from core file
    let duckdb_path = create_duckdb(core_files, &target_dir);
    println!("Created duckdb: {:?}", duckdb_path);

    let conn = duckdb::Connection::open(&duckdb_path)
        .expect("Failed to open duckdb db");
    let count: usize = conn.query_row(
        "SELECT COUNT(*) FROM observations",
        [],
        |row| row.get(0)
    ).expect("Failed to get obs count");
    println!("Obs count: {}", count);

    count
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, open_archive])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
