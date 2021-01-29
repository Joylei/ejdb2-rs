use std::{
    env, fs,
    ops::{Deref, DerefMut},
    thread,
    time::Duration,
};

use rand::Rng;

use crate::{Database, DatabaseOpenMode, EJDB2Builder, Result};
pub(crate) struct TestDb {
    file: String,
    db: Database,
}

impl TestDb {
    pub fn new() -> Self {
        let num = next_u64(100000);
        let file = format!("{}-{}", get_tmp_path(), num);
        eprintln!("db file: {}", &file);
        let file_ref: &str = file.as_ref();
        let opts = EJDB2Builder::new(file_ref).oflags(DatabaseOpenMode::IWKV_TRUNC);
        let db = opts.build().unwrap();
        Self { file, db }
    }

    pub fn new_with_seed() -> Result<Self> {
        let db = Self::new();

        let col = db.collection("c1");
        col.ensure_collection()?;
        col.put("{\"a\":\"abc1\",\"b\":\"cde1\",\"c\":0}", Some(1))?;
        col.put("{\"a\":\"abc2\",\"b\":\"cde2\",\"c\":null}", Some(2))?;
        col.put("{\"a\":\"abc3\",\"b\":\"cde3\",\"c\":5}", Some(3))?;
        col.put("{\"a\":\"abc4\",\"b\":\"cde4\",\"c\":4}", Some(4))?;
        col.put("{\"a\":\"abc5\",\"b\":\"cde9\",\"c\":3}", Some(5))?;
        col.put("{\"a\":\"abc6\",\"b\":\"cde8\",\"c\":2}", Some(6))?;
        col.put("{\"a\":\"abc7\",\"b\":\"cde7\",\"c\":1}", Some(7))?;
        col.put("{\"a\":\"abc8\",\"b\":\"cde6\",\"c\":9}", Some(8))?;

        Ok(db)
    }
}

impl Deref for TestDb {
    type Target = Database;
    fn deref(&self) -> &Self::Target {
        &self.db
    }
}

impl DerefMut for TestDb {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.db
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        let res = retry(|| fs::remove_file(&self.file).map_err(|e| e.into()), 10);
        if let Err(e) = res {
            eprintln!("{}", e)
        }

        let wal_file = format!("{}-wal", &self.file);
        let res = retry(
            || fs::remove_file(wal_file.as_str()).map_err(|e| e.into()),
            5,
        );
        if let Err(e) = res {
            eprintln!("{}", e)
        }
    }
}

fn next_u64(max_val: u64) -> u64 {
    let mut rng = rand::thread_rng();
    let y: f64 = rng.gen(); //0-1
    (y * max_val as f64) as u64
}

fn get_tmp_path() -> String {
    let file_name = format!("ejdb_test_{:?}", thread::current().id())
        .replace("(", "")
        .replace(")", "");
    let result = env::temp_dir().join(file_name).to_str().unwrap().to_owned();
    //println!("{}", result);
    result
}

pub(crate) fn retry<F: FnMut() -> Result<R>, R: Default>(mut f: F, times: usize) -> Result<R> {
    let mut res: Result<R> = Ok(Default::default());
    for _ in 0..times {
        res = (f)();
        if res.is_ok() {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    res
}

/// make it easier to write code with ?
pub(crate) fn catch<F: FnOnce() -> Result<R>, R>(f: F) -> Result<R> {
    (f)()
}
