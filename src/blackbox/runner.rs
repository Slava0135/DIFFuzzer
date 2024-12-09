use std::path::Path;

use rand::prelude::StdRng;
use rand::SeedableRng;

use crate::abstract_fs::generator::generate_new;
use crate::mount::mount::FileSystemMount;

pub fn runner_diff_with_end<FS: FileSystemMount>(mut count: usize,
                                                 fs_reference: FS,
                                                 fs_target: FS,
                                                 trace_len: usize,
                                                 seed: u64) {
    let ref_mnt = Path::new("/mnt").join("reference");
    let target_mnt = Path::new("/mnt").join("target");
    let mut rng = StdRng::seed_from_u64(seed);

    while count > 0 {
        let name: &Path = Path::new(&format!("test{}", count));

        let _seq = generate_new(&mut rng, trace_len);
        count -= 1;

        let ref_path: &Path = ref_mnt.join(name).as_path();
        let target_path: &Path = target_mnt.join(name).as_path();

        fs_reference.setup(ref_path).unwrap();
        fs_target.setup(target_path).unwrap();

        //make and exec

        fs_reference.teardown(ref_path).unwrap();
        fs_target.teardown(target_path).unwrap();
    }
}