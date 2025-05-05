pub mod patch_meshes;
pub mod pak_files;
pub mod iotoc;
pub mod archives;

use std::fs;
use crate::install_mod::InstallableMod;
use iotoc::convert_to_iostore_directory;
use log::{error, info, warn};
use pak_files::create_repak_from_pak;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use tempfile::tempdir;
use walkdir::WalkDir;
use crate::install_mod::install_mod_logic::archives::*;


pub fn install_from_archive(installable_mod: &InstallableMod, mod_directory: &Path, installed_mods_ptr: &AtomicI32){

    let tempdir = tempdir().unwrap();
    if installable_mod.mod_path.to_str().unwrap().ends_with(".zip") {
        extract_zip(
            installable_mod.mod_path.to_str().unwrap(),
            tempdir.path().to_str().unwrap(),
        )
        .expect("Unable to extract zip file");
    } else if installable_mod.mod_path.to_str().unwrap().ends_with(".rar") {
        extract_rar(
            installable_mod.mod_path.to_str().unwrap(),
            tempdir.path().to_str().unwrap(),
        )
        .expect("Unable to extract rar file");
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

                    fs::copy(path, dest_path).expect("Failed to copy mod file");
                    installed_mods_ptr.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
                // IF ONLY PAK IS FOUND WE NEED TO EXTRACT AND INSTALL THE PAK
                else if ext.eq_ignore_ascii_case("pak") {
                    create_repak_from_pak(
                        installable_mod, 
                        mod_directory.to_path_buf(),
                         installed_mods_ptr).expect("Failed to install .pak mod from archive");
                }
            }
        }
    }
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

        if installable_mod.is_archive {
            install_from_archive(installable_mod, mod_directory, installed_mods_ptr);
            continue;
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

        // This shit shouldnt even be possible why do I still have this in the codebase???
        if !installable_mod.repak && !installable_mod.is_dir {
            // just move files to the correct location
            info!("Copying mod instead of repacking: {}", installable_mod.mod_name);
            std::fs::copy(
                &installable_mod.mod_path,
                mod_directory.join(format!("{}.pak", &installable_mod.mod_name)),
            )
            .unwrap();
            installed_mods_ptr.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            continue;
        }

        if installable_mod.is_dir {
            let res = convert_to_iostore_directory(
                installable_mod, PathBuf::from(&mod_directory), PathBuf::from(&installable_mod.mod_path),installed_mods_ptr,
            );
            if let Err(e) = res
            {
                error!("Failed to create repak from pak: {}", e);
            } 
            else {
                info!("Installed mod: {}", installable_mod.mod_name);
            }
        }
    }
    // set i32 to -255 magic value to indicate mod installation is done
    AtomicI32::store(installed_mods_ptr, -255, Ordering::SeqCst);
}
