use crate::*;
use core::ptr;
use ejdb2_sys as sys;
use rand::RngCore;

/// builder to build database object
pub struct EJDB2Builder {
    ejdb_opts: sys::EJDB_OPTS,
    db_path: XString,
    http_host: Option<XString>,
}

impl EJDB2Builder {
    pub fn new<P: Into<XString>>(path: P) -> Self {
        let mut rng = rand::thread_rng();
        let mut ejdb_opts = sys::EJDB_OPTS::default();
        let path = path.into();
        ejdb_opts.kv.path = path.as_ptr();
        ejdb_opts.kv.random_seed = rng.next_u32();
        Self {
            ejdb_opts,
            db_path: path,
            http_host: None,
        }
    }

    /// build database object
    pub fn build(self) -> Result<Database> {
        let rc = unsafe { sys::ejdb_init() };
        if rc != 0 {
            return Err(EjdbError::InitError(rc));
        }

        //println!("Running EJDB with options: {:#?}", &ejdb_opts);
        Database::new(self.db_path, self.http_host, self.ejdb_opts)
    }
    /// bitmask of database file open modes
    #[inline]
    pub fn oflags(mut self, oflags: DatabaseOpenMode) -> Self {
        self.ejdb_opts.kv.oflags = oflags.bits();
        self
    }
    /// do not wait and raise error if database is locked by another process
    #[inline]
    pub fn file_lock_fail_fast(mut self, file_lock_fail_fast: bool) -> Self {
        self.ejdb_opts.kv.file_lock_fail_fast = file_lock_fail_fast;
        self
    }
    /// use write-ahead-log or not, default: false
    #[inline]
    pub fn wal(mut self, wal: bool) -> Self {
        self.ejdb_opts.no_wal = !wal;
        self
    }
    /// max sorting buffer size, default 16Mb, min 1Mb
    #[inline]
    pub fn sort_buffer_sz(mut self, sort_buffer_sz: u32) -> Self {
        self.ejdb_opts.sort_buffer_sz = sort_buffer_sz;
        self
    }
    /// buffer size during query execution, default 64Kb, min 16Kb
    #[inline]
    pub fn document_buffer_sz(mut self, document_buffer_sz: u32) -> Self {
        self.ejdb_opts.document_buffer_sz = document_buffer_sz;
        self
    }

    #[cfg(not(windows))]
    #[inline]
    pub fn enable_http<T: Into<XString>>(
        mut self,
        port: u16,
        host: Option<T>,
        read_anon: bool,
    ) -> Self {
        self.ejdb_opts.http.enabled = true;
        self.ejdb_opts.http.port = port as i32;
        self.http_host = host.map(|x| {
            let host = x.into();
            self.ejdb_opts.http.bind = host.as_ptr();
            host
        });
        self.ejdb_opts.http.read_anon = read_anon;
        self.ejdb_opts.http.blocking = false;
        self
    }
}
