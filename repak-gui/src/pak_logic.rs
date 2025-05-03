use crate::install_mod::{InstallableMod, AES_KEY};
use crate::utils::collect_files;
use colored::Colorize;
use log::{debug, error, info, warn};
use path_clean::PathClean;
use path_slash::PathExt;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use repak::utils::AesKey;
use repak::Version;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use tempfile::tempdir;
use walkdir::WalkDir;
use uasset_mesh_patch_rivals::{Logger, PatchFixer};

struct PrintLogger;

impl Logger for PrintLogger {
    fn log<S: Into<String>>(&self, buf: S) {
        let s = Into::<String>::into(buf);
        info!("{}", s);
    }
}

fn mesh_patch(paths: &mut Vec<PathBuf>, mod_dir: &PathBuf) -> Result<(), repak::Error> {
    let uasset_files = paths
        .iter()
        .filter(|p| {
            p.extension().and_then(|ext| ext.to_str()) == Some("uasset")
                && (p.to_str().unwrap().to_lowercase().contains("meshes"))
        })
        .cloned()
        .collect::<Vec<PathBuf>>();

    let patched_cache_file = mod_dir.join("patched_files");
    info!("Patching files...");
    let file = OpenOptions::new()
        .read(true) // Allow reading
        .write(true) // Allow writing
        .create(true)
        .truncate(false) // Create the file if it doesn’t exist
        .open(&patched_cache_file)?;

    let patched_files = BufReader::new(&file)
        .lines()
        .map(|l| l.unwrap().clone())
        .collect::<Vec<_>>();

    let mut cache_writer = BufWriter::new(&file);

    paths.push(patched_cache_file);
    let print_logger = PrintLogger;
    let mut fixer = PatchFixer {
        logger: print_logger,
    };
    'outer: for uassetfile in &uasset_files {
        let mut sizes: Vec<i64> = vec![];
        let mut offsets: Vec<i64> = vec![];

        let dir_path = uassetfile.parent().unwrap();
        let uexp_file = dir_path.join(
            uassetfile
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .replace(".uasset", ".uexp"),
        );

        if !uexp_file.exists() {
            panic!("UEXP file doesnt exist");
            // damn
        }

        let rel_uasset = &uassetfile
            .strip_prefix(mod_dir)
            .expect("file not in input directory")
            .to_slash()
            .expect("failed to convert to slash path");

        let rel_uexp = &uexp_file
            .strip_prefix(mod_dir)
            .expect("file not in input directory")
            .to_slash()
            .expect("failed to convert to slash path");

        for i in &patched_files {
            if i.as_str() == *rel_uexp || i.as_str() == *rel_uasset {
                info!(
                    "Skipping {} (File has already been patched before)",
                    i.yellow()
                );
                continue 'outer;
            }
        }

        fs::copy(
            &uexp_file,
            dir_path.join(format!(
                "{}.bak",
                uexp_file.file_name().unwrap().to_str().unwrap()
            )),
        )?;
        fs::copy(
            uassetfile,
            dir_path.join(format!(
                "{}.bak",
                uassetfile.file_name().unwrap().to_str().unwrap()
            )),
        )?;

        info!("Processing {}", &uassetfile.to_str().unwrap().yellow());
        let mut rdr = BufReader::new(File::open(uassetfile.clone())?);
        let (exp_cnt, exp_offset) = fixer.read_uasset(&mut rdr)?;
        fixer.read_exports(&mut rdr, &mut sizes, &mut offsets, exp_offset, exp_cnt)?;

        let backup_file = format!("{}.bak", uexp_file.to_str().unwrap());
        let backup_file_size = fs::metadata(uassetfile)?.len();
        let tmpfile = format!("{}.temp", uexp_file.to_str().unwrap());

        drop(rdr);

        let mut r = BufReader::new(File::open(&backup_file)?);
        let mut o = BufWriter::new(File::create(&tmpfile)?);

        let exp_rd = fixer.read_uexp(&mut r, backup_file_size, &backup_file, &mut o, &offsets);
        match exp_rd {
            Ok(_) => {}
            Err(e) => match e.kind() {
                ErrorKind::InvalidData => {
                    panic!("{}", e.to_string())
                }
                ErrorKind::Other => {
                    fs::remove_file(&tmpfile)?;
                    continue 'outer;
                }
                _ => {
                    panic!("{}", e.to_string())
                }
            },
        }
        // fs::remove_file(&uexp_file)?;

        fs::copy(&tmpfile, &uexp_file)?;
        unsafe {
            fixer.clean_uasset(uassetfile.clone(), &sizes)?;
        }

        writeln!(&mut cache_writer, "{}", &rel_uasset)?;
        writeln!(&mut cache_writer, "{}", &rel_uexp)?;

        fs::remove_file(&tmpfile)?;
        cache_writer.flush()?;
    }

    info!("Done patching files!!");
    Ok(())
}

pub fn extract_pak_to_dir(pak: &InstallableMod, install_dir: PathBuf) -> Result<(), repak::Error> {
    let pak_reader = pak.clone().reader.clone().unwrap();

    let mount_point = PathBuf::from(pak_reader.mount_point());
    let prefix = Path::new("../../../");

    struct UnpakEntry {
        entry_path: String,
        out_path: PathBuf,
        out_dir: PathBuf,
    }

    let entries = pak_reader
        .files()
        .into_iter()
        .map(|entry| {
            let full_path = mount_point.join(&entry);
            let out_path =
                install_dir
                    .join(full_path.strip_prefix(prefix).map_err(|_| {
                        repak::Error::PrefixMismatch {
                            path: full_path.to_string_lossy().to_string(),
                            prefix: prefix.to_string_lossy().to_string(),
                        }
                    })?)
                    .clean();

            if !out_path.starts_with(&install_dir) {
                return Err(repak::Error::WriteOutsideOutput(
                    out_path.to_string_lossy().to_string(),
                ));
            }

            let out_dir = out_path.parent().expect("will be a file").to_path_buf();

            Ok(Some(UnpakEntry {
                entry_path: entry.to_string(),
                out_path,
                out_dir,
            }))
        })
        .filter_map(|x| x.transpose())
        .collect::<Result<Vec<_>, _>>()?;

    entries.par_iter().for_each(|entry| {
        log::debug!("Unpacking: {}", entry.entry_path);
        fs::create_dir_all(&entry.out_dir).unwrap();
        let mut reader = BufReader::new(File::open(&pak.mod_path).unwrap());
        let buffer = pak_reader
            .get(&entry.entry_path, &mut reader)
            .expect("Failed to read entry");
        File::create(&entry.out_path)
            .unwrap()
            .write_all(&buffer)
            .unwrap();
        log::info!("Unpacked: {:?}", entry.out_path);
    });
    Ok(())
}

use retoc::*;
use crate::archive_mods::{extract_rar, extract_zip};

fn convert_to_iostore_directory(
    pak: &InstallableMod,
    mod_dir: PathBuf,
    to_pak_dir: PathBuf,
    packed_files_count: &AtomicI32,
) -> Result<(), repak::Error> {

    let mut pak_name = pak.mod_name.clone();
    pak_name.push_str(".pak");

    let mut utoc_name = pak.mod_name.clone();
    utoc_name.push_str(".utoc");

    let action = ActionToZen::new(
        to_pak_dir.clone(),
        mod_dir.join(utoc_name),
        EngineVersion::UE5_3,
    );
    let mut config = Config {
        container_header_version_override: None,
        ..Default::default()
    };

    let aes_toc =
        retoc::AesKey::from_str("0C263D8C22DCB085894899C3A3796383E9BF9DE0CBFB08C9BF2DEF2E84F29D74")
            .unwrap();

    config.aes_keys.insert(FGuid::default(), aes_toc.clone());
    let config = Arc::new(config);

    action_to_zen(action, config).expect("Failed to convert to zen");

    // NOW WE CREATE THE FAKE PAK FILE WITH THE CONTENTS BEING A TEXT FILE LISTING ALL CHUNKNAMES

    if pak.audio_mod{
        repak_dir(pak, to_pak_dir.clone(), mod_dir.clone(),packed_files_count)?; // if its an audio mod we need the full pak
        return Ok(());
    }

    let output_file = File::create(mod_dir.join(pak_name))?;

    let mut paths = vec![];
    collect_files(&mut paths, &to_pak_dir)?;

    if pak.fix_mesh {
        mesh_patch(&mut paths, &to_pak_dir.to_path_buf())?;
    }

    let rel_paths = paths
        .par_iter()
        .map(|p| {
            let rel = &p
                .strip_prefix(to_pak_dir.clone())
                .expect("file not in input directory")
                .to_slash()
                .expect("failed to convert to slash path");
            rel.to_string()
        })
        .collect::<Vec<_>>();


    let builder = repak::PakBuilder::new()
        .compression(vec![pak.compression])
        .key(AES_KEY.clone().0);

    let mut pak_writer = builder.writer(
        BufWriter::new(output_file),
        Version::V11,
        pak.mount_point.clone(),
        Some(pak.path_hash_seed.parse().unwrap()),
    );
    let entry_builder = pak_writer.entry_builder();

    let rel_paths_bytes: Vec<u8> = rel_paths.join("\n").into_bytes();
    let entry = entry_builder
        .build_entry(true, rel_paths_bytes, "chunknames")
        .expect("Failed to build entry");


    pak_writer.write_entry("chunknames".to_string(), entry)?;
    pak_writer.write_index()?;

    log::info!("Wrote pak file successfully");
    packed_files_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Ok(())

    // now generate the fake pak file
}

fn create_repak_from_pak(
    pak: &InstallableMod,
    mod_dir: PathBuf,
    packed_files_count: &AtomicI32,
) -> Result<(), repak::Error> {
    // extract the pak first into a temporary dir
    let temp_dir = tempdir().map_err(repak::Error::Io)?;
    let temp_path = temp_dir.path(); // Get the path of the temporary directory

    extract_pak_to_dir(pak, temp_path.to_path_buf())?;
    convert_to_iostore_directory(
        pak,
        mod_dir.clone(),
        temp_path.to_path_buf(),
        packed_files_count,
    )?;
    // repak_dir(pak, PathBuf::from(temp_path), mod_dir,packed_files_count)?;
    Ok(())
}

// leaving this here for legacy reasons
pub fn repak_dir(
    pak: &InstallableMod,
    to_pak_dir: PathBuf,
    mod_dir: PathBuf,
    installed_mods_ptr: &AtomicI32,
) -> Result<(), repak::Error> {
    let mut pak_name = pak.mod_name.clone();
    pak_name.push_str(".pak");
    let output_file = File::create(mod_dir.join(pak_name))?;

    let mut paths = vec![];
    collect_files(&mut paths, &to_pak_dir)?;

    if pak.fix_mesh {
        mesh_patch(&mut paths, &to_pak_dir.to_path_buf())?;
    }


    paths.sort();

    let builder = repak::PakBuilder::new()
        .compression(vec![pak.compression])
        .key(AES_KEY.clone().0);

    let mut pak_writer = builder.writer(
        BufWriter::new(output_file),
        Version::V11,
        pak.mount_point.clone(),
        Some(pak.path_hash_seed.parse().unwrap()),
    );
    let entry_builder = pak_writer.entry_builder();

    let partial_entry = paths
        .par_iter()
        .map(|p| {
            let rel = &p
                .strip_prefix(to_pak_dir.clone())
                .expect("file not in input directory")
                .to_slash()
                .expect("failed to convert to slash path");

            let entry = entry_builder
                .build_entry(true, std::fs::read(p).expect("WTF"), rel)
                .expect("Failed to build entry");
            (rel.to_string(), entry)
        })
        .collect::<Vec<_>>();

    let mut rel_paths = vec![];
    for (path, entry) in partial_entry {
        debug!("Writing: {}", path);
        pak_writer.write_entry(path.clone(), entry)?;
        installed_mods_ptr.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        rel_paths.push(path);
    }

    let rel_paths_bytes: Vec<u8> = rel_paths.join("\n").into_bytes();

    let entry = entry_builder
        .build_entry(true, rel_paths_bytes, "chunknames")
        .expect("Failed to build entry");

    pak_writer.write_entry("chunknames".to_string(), entry)?;
    pak_writer.write_index()?;

    log::info!("Wrote pak file successfully");
    Ok::<(), repak::Error>(())
}

pub fn install_mods_in_viewport(
    mods: &mut [InstallableMod],
    mod_directory: &Path,
    installed_mods_ptr: &AtomicI32,
    stop_thread: &AtomicBool,
) {
    for installable_mod in mods.iter_mut() {
        if stop_thread.load(Ordering::SeqCst) {
            warn!("Stopping thread");
            break;
        }

        if installable_mod.is_archive{
            let tempdir = tempdir().unwrap();
            if installable_mod.mod_path.to_str().unwrap().ends_with(".zip") {
                extract_zip(installable_mod.mod_path.to_str().unwrap(), tempdir.path().to_str().unwrap()).expect("Unable to extract zip file");
            }
            else if installable_mod.mod_path.to_str().unwrap().ends_with(".rar"){
                extract_rar(installable_mod.mod_path.to_str().unwrap(), tempdir.path().to_str().unwrap()).expect("Unable to extract rar file");
            }
            // now walkdir and collect all files inside it, if the name ends with utoc, ucas or pok copy it to game directory
            for entry in WalkDir::new(tempdir.path()) {
                let entry = entry.expect("Failed to read directory entry");
                let path = entry.path();

                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if ext.eq_ignore_ascii_case("utoc")
                            || ext.eq_ignore_ascii_case("ucas")
                            || ext.eq_ignore_ascii_case("pak")
                        {
                            let dest_path = Path::new(&PathBuf::from(&mod_directory))
                                .join(path.file_name().unwrap());

                            fs::copy(path, dest_path)
                                .expect("Failed to copy mod file");
                            installed_mods_ptr.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        }
                    }
                }

            }
            continue;
        }

        if !installable_mod.repak && !installable_mod.is_dir {
            // just move files to the correct location
            info!("Installing mod: {}", installable_mod.mod_name);
            std::fs::copy(
                &installable_mod.mod_path,
                mod_directory.join(format!("{}.pak", &installable_mod.mod_name)),
            )
            .unwrap();
            installed_mods_ptr.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }

        if installable_mod.repak {
            if let Err(e) = create_repak_from_pak(
                installable_mod,
                PathBuf::from(mod_directory),
                installed_mods_ptr,
            ) {
                error!("Failed to create repak from pak: {}", e);
            }
        }

        if installable_mod.is_dir {
            match convert_to_iostore_directory(installable_mod, PathBuf::from(&mod_directory), PathBuf::from(&installable_mod.mod_path) ,installed_mods_ptr)
            {
                Ok(_) => {
                    info!("Installed mod: {}", installable_mod.mod_name);
                }
                Err(e) => {
                    error!("Failed to create repak from pak: {}", e);
                }
            }
        }


    }
    // set i32 to -255 magic value to indicate mod installation is done
    AtomicI32::store(installed_mods_ptr, -255, Ordering::SeqCst);
}
