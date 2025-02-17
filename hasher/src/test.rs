/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use std::fs;
use std::fs::{remove_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use regex::RegexSet;

use hasher::{calc_dir_hash, get_diff};

#[ignore]
#[test]
fn test_hash_eq() {
    let dirs = vec!["A/B", "A/C", "AA/D/E", "AAA/D/F/G", "Q"];
    let files = vec!["test1.out", "test2.txt", "test3.txt"];
    let data = vec!["", "dsfsdfsdfsdfpsd", "11213213\nddfdsf    \n     "];

    let cmp_dirs = create_data_for_test(dirs, files, data);

    let ext4_dirs = RegexSet::new([r"^/?lost\+found($|/)"]).unwrap();
    let hash_options = Default::default();
    let (hash_fst, fst_content) =
        calc_dir_hash(cmp_dirs[0].as_path(), &ext4_dirs, &hash_options).unwrap();
    let (hash_snd, snd_content) =
        calc_dir_hash(cmp_dirs[1].as_path(), &ext4_dirs, &hash_options).unwrap();
    let diff = get_diff(
        &fst_content,
        &snd_content,
        &ext4_dirs,
        &ext4_dirs,
        &hash_options,
    );
    assert_eq!(hash_fst, hash_snd);
    assert_eq!(diff.len(), 0);
}

#[ignore]
#[test]
fn test_hash_not_eq() {
    let dirs = vec!["A/B", "A/C", "AA/D/E", "AAA/D/F/G", "Q"];
    let files = vec!["test1.out", "test2.txt", "test3.txt"];
    let data = vec!["", "dsfsdfsdfsdfpsd", "11213213\nddfdsf    \n     "];

    let cmp_dirs = create_data_for_test(dirs, files, data);

    let err_dir = cmp_dirs[0].as_path().join("ERR");
    fs::create_dir(err_dir.clone())
        .with_context(|| format!("failed create folder '{}'", err_dir.display()))
        .unwrap();

    let ext4_dirs = RegexSet::new([r"^/?lost\+found($|/)"]).unwrap();
    let hash_options = Default::default();
    let (hash_fst, fst_content) =
        calc_dir_hash(cmp_dirs[0].as_path(), &ext4_dirs, &hash_options).unwrap();
    let (hash_snd, snd_content) =
        calc_dir_hash(cmp_dirs[1].as_path(), &ext4_dirs, &hash_options).unwrap();
    assert_ne!(hash_fst, hash_snd);

    let diff = get_diff(
        &fst_content,
        &snd_content,
        &ext4_dirs,
        &ext4_dirs,
        &hash_options,
    );
    assert_ne!(diff.len(), 0);
}

#[ignore]
#[test]
fn test_hash_eq_skip() {
    let dirs = vec!["A/B", "A/C", "AA/D/E", "AAA/D/F/G", "Q"];
    let files = vec!["test1.out", "test2.txt", "test3.txt"];
    let data = vec!["", "dsfsdfsdfsdfpsd", "11213213\nddfdsf    \n     "];

    let cmp_dirs = create_data_for_test(dirs, files, data);

    let err_dir = cmp_dirs[0].as_path().join("lost+found");
    fs::create_dir(err_dir.clone())
        .with_context(|| format!("failed create folder '{}'", err_dir.display()))
        .unwrap();

    let ext4_dirs = RegexSet::new([r"^/?lost\+found($|/)"]).unwrap();
    let hash_options = Default::default();
    let (hash_fst, fst_content) =
        calc_dir_hash(cmp_dirs[0].as_path(), &ext4_dirs, &hash_options).unwrap();
    let (hash_snd, snd_content) =
        calc_dir_hash(cmp_dirs[1].as_path(), &ext4_dirs, &hash_options).unwrap();
    let diff = get_diff(
        &fst_content,
        &snd_content,
        &ext4_dirs,
        &ext4_dirs,
        &hash_options,
    );
    println!("{:?}", diff);
    assert_eq!(hash_fst, hash_snd);
    assert_eq!(diff.len(), 0);
}

fn create_data_for_test(dirs: Vec<&str>, files: Vec<&str>, data: Vec<&str>) -> Vec<PathBuf> {
    let temp_dir = Path::new("/tmp")
        .join(format!("{}-{}", "diffuzzer", "hash-test"))
        .into_boxed_path();
    let temp_dir = temp_dir.as_ref();
    if temp_dir.exists() {
        remove_dir_all(temp_dir).unwrap();
    }

    let cmp_dirs = vec![temp_dir.join("fst"), temp_dir.join("snd")];

    for cmp_dir in cmp_dirs.iter() {
        for dir in dirs.iter() {
            let target_dir = cmp_dir.join(dir);
            fs::create_dir_all(target_dir.clone())
                .with_context(|| {
                    format!(
                        "failed to create temporary directory at '{}'",
                        target_dir.display()
                    )
                })
                .unwrap();
        }
    }

    for cmp_dir in cmp_dirs.iter() {
        for i in 0..files.len() {
            let file = files[i];
            let dir = dirs[i % dirs.len()];
            let inp = data[i % data.len()];
            let target_path = cmp_dir.join(dir).join(file);
            let mut file = File::create(target_path.clone())
                .with_context(|| format!("failed to create file at {}", target_path.display()))
                .unwrap();
            file.write_all(inp.as_ref())
                .with_context(|| format!("failed to write to file at {}", target_path.display()))
                .unwrap()
        }
    }

    cmp_dirs
}
