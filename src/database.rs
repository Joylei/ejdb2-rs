use crate::{
    exec::Query,
    jbl::JBL,
    jql::JQL,
    printer::AsJson,
    utils::check_rc,
    xstr::{StringPtr, XString},
    EjdbError, JsonPrintFlags, Result,
};
use core::ptr;

use ejdb2_sys as sys;

pub struct Database {
    ptr: sys::EJDB,
    pub(crate) ejdb_opts: sys::EJDB_OPTS,
    pub(crate) db_path: XString,
    pub(crate) http_host: Option<XString>,
}

impl Database {
    #[inline]
    pub(crate) fn new(
        db_path: XString,
        http_host: Option<XString>,
        ejdb_opts: sys::EJDB_OPTS,
    ) -> Result<Self> {
        let mut ptr = ptr::null_mut();
        let rc = unsafe { sys::ejdb_open(&ejdb_opts, &mut ptr) };
        if rc != 0 {
            return Err(EjdbError::OpenError { rc, file: db_path });
        }
        Ok(Self {
            ptr,
            ejdb_opts,
            db_path,
            http_host,
        })
    }

    #[inline(always)]
    pub(crate) fn raw_ptr(&self) -> sys::EJDB {
        self.ptr
    }

    /// remove index if existing
    #[inline]
    pub fn remove_index<'a, 'b>(
        &self,
        collection: impl Into<StringPtr<'a>>,
        path: impl Into<StringPtr<'b>>,
        mode: sys::ejdb_idx_mode_t,
    ) -> Result<()> {
        let coll = collection.into();
        let path = path.into();
        let rc =
            unsafe { sys::ejdb_remove_index(self.raw_ptr(), coll.as_ptr(), path.as_ptr(), mode) };
        check_rc(rc)
    }

    /// create index with given parameters if not existing
    #[inline]
    pub fn ensure_index<'a, 'b>(
        &self,
        collection: impl Into<StringPtr<'a>>,
        path: impl Into<StringPtr<'b>>,
        mode: sys::ejdb_idx_mode_t,
    ) -> Result<()> {
        let coll = collection.into();
        let path = path.into();
        let rc =
            unsafe { sys::ejdb_ensure_index(self.raw_ptr(), coll.as_ptr(), path.as_ptr(), mode) };
        check_rc(rc)
    }

    /// create collection with given name if not existing
    #[inline]
    pub fn ensure_collection<'a>(&self, collection: impl Into<StringPtr<'a>>) -> Result<()> {
        let coll = collection.into();
        let rc = unsafe { sys::ejdb_ensure_collection(self.raw_ptr(), coll.as_ptr()) };
        check_rc(rc)
    }

    /// rename collection
    #[inline]
    pub fn rename_collection<'a, 'b>(
        &self,
        old_name: impl Into<StringPtr<'a>>,
        new_name: impl Into<StringPtr<'b>>,
    ) -> Result<()> {
        let old_name = old_name.into();
        let new_name = new_name.into();
        let rc = unsafe {
            sys::ejdb_rename_collection(self.raw_ptr(), old_name.as_ptr(), new_name.as_ptr())
        };
        check_rc(rc)
    }

    /// remove collection
    #[inline]
    pub fn remove_collection<'a>(&self, collection: impl Into<StringPtr<'a>>) -> Result<()> {
        let coll = collection.into();
        let rc = unsafe { sys::ejdb_remove_collection(self.raw_ptr(), coll.as_ptr()) };
        check_rc(rc)
    }

    /// perform online backup without blocking read/write
    /// @returns backup finish time in milliseconds since epoch
    #[inline]
    pub fn online_backup<'a>(&self, target_file: impl Into<StringPtr<'a>>) -> Result<u64> {
        let target_file = target_file.into();
        let mut ts = 0_u64;
        let rc = unsafe {
            let ts = &mut ts as *mut u64;
            sys::ejdb_online_backup(self.raw_ptr(), ts, target_file.as_ptr())
        };
        check_rc(rc).and(Ok(ts))
    }

    /// retrieve document by specified id
    #[inline]
    pub fn get<'a>(&self, collection: impl Into<StringPtr<'a>>, id: i64) -> Result<JBL> {
        let mut jblp = ptr::null_mut();
        let coll = collection.into();
        let rc = unsafe { sys::ejdb_get(self.raw_ptr(), coll.as_ptr(), id, &mut jblp) };
        check_rc(rc).map(|_| JBL::from_ptr(jblp))
    }

    /// save document under specified id,
    /// or insert new document if id not specified
    #[inline]
    pub fn put<'a, 'b>(
        &self,
        collection: impl Into<StringPtr<'a>>,
        json: impl Into<StringPtr<'b>>,
        id: Option<i64>,
    ) -> Result<i64> {
        let jbl = JBL::from_json(json)?;
        let coll = collection.into();
        let mut ret_id = 0_i64;
        let rc = match id {
            Some(id) => {
                ret_id = id;
                unsafe { sys::ejdb_put(self.raw_ptr(), coll.as_ptr(), jbl.raw_ptr(), id) }
            }
            _ => unsafe {
                let id_ptr = &mut ret_id as *mut i64;
                sys::ejdb_put_new(self.raw_ptr(), coll.as_ptr(), jbl.raw_ptr(), id_ptr)
            },
        };
        check_rc(rc).and(Ok(ret_id))
    }

    /// apply JSON patch to document identified by id
    #[inline]
    pub fn patch<'a, 'b>(
        &self,
        collection: impl Into<StringPtr<'a>>,
        json: impl Into<StringPtr<'b>>,
        id: i64,
    ) -> Result<()> {
        let coll = collection.into();
        let json = json.into();
        let rc = unsafe { sys::ejdb_patch(self.raw_ptr(), coll.as_ptr(), json.as_ptr(), id) };
        check_rc(rc)
    }

    /// apply JSON merge patch to document identified by id
    /// or insert new document under specified id
    #[inline]
    pub fn merge_or_put<'a, 'b>(
        &self,
        collection: impl Into<StringPtr<'a>>,
        json: impl Into<StringPtr<'b>>,
        id: i64,
    ) -> Result<()> {
        let coll = collection.into();
        let json = json.into();
        let rc =
            unsafe { sys::ejdb_merge_or_put(self.raw_ptr(), coll.as_ptr(), json.as_ptr(), id) };
        check_rc(rc)
    }

    ///remove document identified by given id
    #[inline]
    pub fn del<'a>(&self, collection: impl Into<StringPtr<'a>>, id: i64) -> Result<()> {
        let coll = collection.into();
        let rc = unsafe { sys::ejdb_del(self.raw_ptr(), coll.as_ptr(), id) };
        check_rc(rc)
    }

    /// return JSON document described database structure
    #[inline]
    pub fn get_meta(&self, flag: Option<JsonPrintFlags>) -> Result<XString> {
        let jbl = {
            let mut jblp = ptr::null_mut();
            let rc = unsafe { sys::ejdb_get_meta(self.raw_ptr(), &mut jblp) };
            check_rc(rc)?;
            JBL::from_ptr(jblp)
        };
        jbl.as_json(flag)
    }

    #[inline]
    pub fn collection<'a, 'b>(&'a self, name: impl Into<StringPtr<'b>>) -> Collection<'a> {
        Collection::new(self, name)
    }
    #[inline]
    pub fn query<'a, 'b>(&'a self, jql: impl Into<StringPtr<'b>>) -> Result<Query<'a>> {
        let jql = JQL::create(jql)?;
        Ok(Query::new(jql, self))
    }
    #[inline]
    pub fn query_with_collection<'a, 'b, 'c>(
        &'a self,
        jql: impl Into<StringPtr<'b>>,
        collection: impl Into<StringPtr<'c>>,
    ) -> Result<Query<'a>> {
        let jql = JQL::create_with_collection(jql, collection)?;
        Ok(Query::new(jql, self))
    }
}

impl Drop for Database {
    #[inline]
    fn drop(&mut self) {
        let rc = unsafe { sys::ejdb_close(&mut self.ptr) };
        debug_assert!(rc == 0);
    }
}

pub struct Collection<'c> {
    db: &'c Database,
    name: XString,
}

impl<'c> Collection<'c> {
    #[inline]
    pub(crate) fn new<'a>(db: &'c Database, name: impl Into<StringPtr<'a>>) -> Self {
        Self {
            db,
            name: name.into().to_owned(),
        }
    }

    /// get collection name
    #[inline]
    pub fn name(&self) -> &XString {
        &self.name
    }
    /// rename collection
    #[inline]
    pub fn rename<'a>(&mut self, name: impl Into<StringPtr<'a>>) -> Result<&mut Self> {
        let name = name.into().to_owned();
        let res = self.db.rename_collection(self.name(), &name);
        if res.is_ok() {
            self.name = name;
        }
        res.and(Ok(self))
    }

    /// create index with given parameters if not existing
    #[inline]
    pub fn ensure_index<'a>(
        &self,
        path: impl Into<StringPtr<'a>>,
        mode: sys::ejdb_idx_mode_t,
    ) -> Result<()> {
        self.db.ensure_index(self.name(), path, mode)
    }
    /// remove index if existing
    #[inline]
    pub fn remove_index<'a>(
        &self,
        path: impl Into<StringPtr<'a>>,
        mode: sys::ejdb_idx_mode_t,
    ) -> Result<()> {
        self.db.remove_index(self.name(), path, mode)
    }
    /// create collection with given name if not existing
    #[inline]
    pub fn ensure_collection(&self) -> Result<()> {
        self.db.ensure_collection(self.name())
    }

    /// remove collection
    #[inline]
    pub fn remove(self) -> core::result::Result<(), CollectionRemoveError<'c>> {
        let res = self.db.remove_collection(self.name());
        res.map_err(|e| CollectionRemoveError {
            collection: self,
            error: e,
        })
    }
    /// retrieve document by specified id
    #[inline]
    pub fn get(&self, id: i64) -> Result<JBL> {
        self.db.get(self.name(), id)
    }
    /// save document under specified id
    /// or insert new document if id not specified
    #[inline]
    pub fn put<'a>(&self, json: impl Into<StringPtr<'a>>, id: Option<i64>) -> Result<i64> {
        self.db.put(self.name(), json, id)
    }

    /// apply JSON patch to document identified by id
    #[inline]
    pub fn patch<'a>(&self, json: impl Into<StringPtr<'a>>, id: i64) -> Result<()> {
        self.db.patch(self.name(), json, id)
    }
    /// apply JSON merge patch to document identified by id
    /// or insert new document under specified id
    #[inline]
    pub fn merge_or_put<'a>(&self, json: impl Into<StringPtr<'a>>, id: i64) -> Result<()> {
        self.db.merge_or_put(self.name(), json, id)
    }

    ///remove document identified by given id
    #[inline]
    pub fn del(&self, id: i64) -> Result<()> {
        self.db.del(self.name(), id)
    }
}

pub struct CollectionRemoveError<'a> {
    pub collection: Collection<'a>,
    pub error: EjdbError,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::*;

    #[test]
    fn test_get() {
        catch(|| {
            let db = TestDb::new_with_seed()?;
            let jbl = db.collection("c1").get(1)?;
            let val = jbl.get_str("b")?;
            assert_eq!(val, "cde1");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_rename() {
        catch(|| {
            let db = TestDb::new_with_seed()?;
            let jbl = db.collection("c1").rename("c2")?.get(1)?;
            Ok(())
        })
        .unwrap();
    }
}
