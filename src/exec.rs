use core::ops::{Deref, DerefMut};
use core::{cell::UnsafeCell, ffi::c_void, mem};

use crate::{
    channel::Channel,
    jql::{self, JQL},
    printer,
    printer::{AsJson, JsonPrinter},
    utils::{self, check_rc},
    xstr::XString,
    Database, EjdbError, JsonPrintFlags, Result,
};

#[cfg(feature = "std")]
use std::{collections::HashMap, hash::Hash};

use ejdb2_sys as sys;

pub type Explain = fn(&XString);

pub struct Query<'a> {
    db: &'a Database,
    jql: JQL,
    skip: Option<usize>,
    limit: Option<usize>,
    log: Option<UnsafeCell<Explain>>,
}

impl<'a> Query<'a> {
    #[inline]
    pub fn new(jql: JQL, db: &'a Database) -> Self {
        Self {
            db,
            jql,
            skip: None,
            limit: None,
            log: None,
        }
    }
}

impl<'a> Query<'a> {
    /// configure jql
    #[inline(always)]
    pub fn jql(&mut self) -> &mut JQL {
        &mut self.jql
    }
    #[inline(always)]
    pub fn skip(mut self, val: usize) -> Self {
        self.skip = Some(val);
        self
    }
    #[inline(always)]
    pub fn take(mut self, val: usize) -> Self {
        self.limit = Some(val);
        self
    }

    /// log query plan
    #[inline(always)]
    pub fn log(mut self, f: Explain) -> Self {
        self.log = Some(UnsafeCell::new(f));
        self
    }
    /// exec query and return matched count
    #[inline]
    pub fn count(&self) -> Result<usize> {
        self.fold(0_usize, |acc, _| Ok(acc + 1))
    }

    /// exec query and return matched count
    /// Note: no query plan log for this query
    #[inline]
    pub fn count_fast(&self) -> Result<usize> {
        let mut count: i64 = 0;
        let limit = self.limit.unwrap_or(0) as i64;
        let rc = unsafe {
            let count_ptr = &mut count as *mut _;
            sys::ejdb_count(self.db.raw_ptr(), self.jql.raw_ptr(), count_ptr, limit)
        };
        check_rc(rc).map(|_| if count < 0 { 0 } else { count as usize })
    }

    /// exec query and return if any matched doc
    #[inline]
    pub fn any(&self) -> Result<bool> {
        self.count().map(|c| c > 0)
    }

    /// exec query and return first matched doc
    #[inline]
    pub fn first<F, T>(&self, f: F) -> Result<Option<T>>
    where
        F: FnMut(&JsonDoc) -> Result<T>,
    {
        let mut visitor = visitor_impl::FirstVisitor {
            q: self,
            f,
            v: Ok(None),
        };
        self.exec_with(&mut visitor)?;
        visitor.v
    }
    #[inline]
    pub fn first_or_default<F, T>(&self, f: F) -> Result<T>
    where
        F: FnMut(&JsonDoc) -> Result<T>,
        T: Default,
    {
        self.first(f).map(|x| x.unwrap_or_default())
    }
    /// exec query and return all matched docs
    #[cfg(any(feature = "std", feature = "alloc"))]
    #[inline]
    pub fn to_vec<F, T>(&self, mut f: F) -> Result<Vec<T>>
    where
        F: FnMut(&JsonDoc) -> Result<T>,
    {
        self.fold(Vec::new(), |acc, doc| {
            let mut acc = acc;
            let v = (f)(doc)?;
            acc.push(v);
            Ok(acc)
        })
    }

    /// exec query and return all matched docs
    #[cfg(any(feature = "std"))]
    #[inline]
    pub fn to_map<F, K, V>(&self, mut f: F) -> Result<HashMap<K, V>>
    where
        F: FnMut(&JsonDoc) -> Result<(K, V)>,
        K: Eq + Hash,
    {
        self.fold(HashMap::new(), |acc, doc| {
            let mut acc = acc;
            let (k, v) = (f)(doc)?;
            if let Some(x) = acc.get_mut(&k) {
                *x = v;
            } else {
                acc.insert(k, v);
            }
            Ok(acc)
        })
    }

    /// exec query and aggregate value based on all matched docs
    #[inline]
    pub fn fold<F, T>(&self, initial: T, mut f: F) -> Result<T>
    where
        F: FnMut(T, &JsonDoc) -> Result<T>,
    {
        let mut acc = Some(initial);
        self.for_each(|doc| {
            let v = mem::take(&mut acc).unwrap();
            (f)(v, doc).map(|v| acc = Some(v))
        })
        .map(|_| acc.unwrap())
    }

    #[inline]
    pub fn for_each<F>(&self, f: F) -> Result<()>
    where
        F: FnMut(&JsonDoc) -> Result<()>,
    {
        let mut visitor = visitor_impl::ForEachVisitor {
            q: self,
            f,
            v: Ok(()),
        };
        self.exec_with(&mut visitor)?;
        visitor.get()
    }

    #[inline]
    pub fn scan<F, T>(&self, initial: T, f: F) -> Result<T>
    where
        F: FnMut(&mut T, &JsonDoc) -> Result<Option<T>>,
    {
        let mut visitor = visitor_impl::ScanVisitor {
            q: self,
            f,
            acc: initial,
            v: Ok(()),
        };
        self.exec_with(&mut visitor)?;
        visitor.get()
    }

    pub fn exec(&self) -> Result<()> {
        self.exec_with(&mut visitor_impl::Empty {})
    }

    pub fn exec_with<V: Visitor>(&self, visitor: &mut V) -> Result<()> {
        let mut chan = Channel(visitor, Ok(VisitStep::Stop));
        let mut ux = sys::_EJDB_EXEC::default();
        ux.db = self.db.raw_ptr();
        ux.q = self.jql.raw_ptr();
        ux.visitor = Some(visit_doc::<V>);
        if let Some(skip) = self.skip {
            ux.skip = skip as i64;
        }
        if let Some(limit) = self.limit {
            ux.limit = limit as i64;
        }
        ux.opaque = &mut chan as *mut _ as *mut c_void;

        let rc = match self.log {
            Some(ref c) => {
                let xstr = XString::new();
                ux.log = xstr.as_mut_ptr();
                let rc = unsafe { sys::ejdb_exec(&mut ux as *mut _) };
                let f = unsafe { &mut *c.get() };
                (f)(&xstr);
                rc
            }
            _ => unsafe { sys::ejdb_exec(&mut ux as *mut _) },
        };
        chan.get()?;
        check_rc(rc)
    }
}

pub mod visitor_impl {
    use super::*;

    pub(crate) struct FirstVisitor<'a, T, F> {
        pub q: &'a Query<'a>,
        pub f: F,
        pub v: Result<Option<T>>,
    }

    impl<'a, T, F> Visitor for FirstVisitor<'a, T, F>
    where
        F: FnMut(&JsonDoc) -> Result<T>,
    {
        #[inline(always)]
        fn on_next(&mut self, doc: &JsonDoc) -> Result<VisitStep> {
            let v = (&mut self.f)(doc)?;
            self.v = Ok(Some(v));
            Ok(VisitStep::Stop)
        }
    }

    pub(crate) struct ForEachVisitor<'a, F> {
        pub q: &'a Query<'a>,
        pub f: F,
        pub v: Result<()>,
    }

    impl<F> ForEachVisitor<'_, F> {
        #[inline(always)]
        pub fn get(self) -> Result<()> {
            self.v
        }
    }

    impl<'a, F> Visitor for ForEachVisitor<'a, F>
    where
        F: FnMut(&JsonDoc) -> Result<()>,
    {
        #[inline(always)]
        fn on_next(&mut self, doc: &JsonDoc) -> Result<VisitStep> {
            (&mut self.f)(doc)?;
            Ok(VisitStep::Next)
        }
    }

    pub(crate) struct ScanVisitor<'a, T, F> {
        pub q: &'a Query<'a>,
        pub f: F,
        pub acc: T,
        pub v: Result<()>,
    }

    impl<T, F> ScanVisitor<'_, T, F> {
        #[inline(always)]
        pub fn get(self) -> Result<T> {
            let acc = self.acc;
            self.v.map(|_| acc)
        }
    }

    impl<'a, T, F> Visitor for ScanVisitor<'a, T, F>
    where
        F: FnMut(&mut T, &JsonDoc) -> Result<Option<T>>,
    {
        #[inline(always)]
        fn on_next(&mut self, doc: &JsonDoc) -> Result<VisitStep> {
            let res = (&mut self.f)(&mut self.acc, doc)?;
            match res {
                Some(acc) => {
                    self.acc = acc;
                    Ok(VisitStep::Next)
                }
                _ => Ok(VisitStep::Stop),
            }
        }
    }

    /// dummy placeholder
    pub struct Empty {}

    impl Visitor for Empty {
        #[inline]
        fn on_next(&mut self, _doc: &JsonDoc) -> Result<VisitStep> {
            Ok(VisitStep::Stop)
        }
    }
}

unsafe extern "C" fn visit_doc<V: Visitor>(
    ctx: *mut sys::_EJDB_EXEC,
    doc: sys::EJDB_DOC,
    step: *mut i64,
) -> u64 {
    let ctx = &mut *ctx;
    //nothing to do
    if ctx.opaque.is_null() {
        // *step=1 //default behavior of EJDB2
        return 0;
    }
    utils::catch_unwind(|| {
        let doc = JsonDoc { doc };
        let chan = &mut *(ctx.opaque as *mut Channel<&mut V, VisitStep>);
        *step = chan.unwrap(VisitStep::Stop, |c| c.on_next(&doc)).into();
    })
    .unwrap_or_else(|e| {
        *step = 0; //stop visitor
        let chan = &mut *(ctx.opaque as *mut Channel<&mut V, VisitStep>);
        #[cfg(feature = "std")]
        {
            chan.set(Err(EjdbError::Panic(e)));
        }
        #[cfg(not(feature = "std"))]
        {
            chan.set(Err(e));
        }
    });
    0
}
/// doc visitor
pub trait Visitor {
    fn on_next(&mut self, doc: &JsonDoc) -> Result<VisitStep>;
}

pub enum VisitStep {
    Stop,
    Prev,
    Next,
    Custom(i64),
}

impl From<VisitStep> for i64 {
    fn from(step: VisitStep) -> i64 {
        match step {
            VisitStep::Stop => 0,
            VisitStep::Prev => -1,
            VisitStep::Next => 1,
            VisitStep::Custom(v) => {
                if v < -2 {
                    0
                } else {
                    v
                }
            }
        }
    }
}

pub struct JsonDoc {
    doc: *mut sys::_EJDB_DOC,
}

impl JsonDoc {
    #[inline]
    pub fn id(&self) -> i64 {
        self.doc().id
    }

    fn doc(&self) -> &mut sys::_EJDB_DOC {
        unsafe { &mut *self.doc }
    }

    #[inline]
    pub fn print<T: JsonPrinter>(
        &self,
        target: &mut T,
        flag: Option<JsonPrintFlags>,
    ) -> Result<()> {
        let flag = flag.unwrap_or(JsonPrintFlags::PRINT_CODEPOINTS);
        printer::doc_print_json(self.doc, target, flag)
    }
}

impl AsJson<XString> for JsonDoc {
    /// more efficient than use print() for XString
    fn as_json(&self, flag: Option<JsonPrintFlags>) -> Result<XString> {
        let size = unsafe { sys::jbl_size(self.doc().raw) as usize };
        let xstr = XString::new_with_size(size * 2);
        let xstr_ptr = xstr.as_mut_ptr() as *mut c_void;
        let flag = flag.unwrap_or(JsonPrintFlags::PRINT_CODEPOINTS).bits;
        let rc = unsafe {
            if !self.doc().node.is_null() {
                sys::jbn_as_json(
                    self.doc().node,
                    Some(sys::jbl_xstr_json_printer),
                    xstr_ptr,
                    flag,
                )
            } else {
                sys::jbl_as_json(
                    self.doc().raw,
                    Some(sys::jbl_xstr_json_printer),
                    xstr_ptr,
                    flag,
                )
            }
        };
        check_rc(rc)?;
        Ok(xstr)
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl AsJson<Vec<u8>> for JsonDoc {
    #[inline]
    fn as_json(&self, flag: Option<JsonPrintFlags>) -> Result<Vec<u8>> {
        let mut buf: Vec<u8> = Vec::new();
        self.print(&mut buf, flag)?;
        Ok(buf)
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl AsJson<String> for JsonDoc {
    #[inline]
    fn as_json(&self, flag: Option<JsonPrintFlags>) -> Result<String> {
        self.as_json(flag)
            .map(|x| unsafe { String::from_utf8_unchecked(x) })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::*;

    #[test]
    fn test_count() {
        catch(|| {
            let db = TestDb::new_with_seed()?;
            let count = db.query("@c1/*")?.count()?;
            assert_eq!(count, 8);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_log() {
        catch(|| {
            let db = TestDb::new_with_seed()?;
            db.query("@c1/*")?
                .log(|log| {
                    println!("jql log:{}", log);
                    assert!(log.size() > 0);
                })
                .exec()?;
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_any_is_true() {
        catch(|| {
            let db = TestDb::new_with_seed()?;
            let is_any = db.query("@c1/*")?.any()?;
            assert!(is_any);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_any_is_false() {
        catch(|| {
            let db = TestDb::new();
            let is_any = db.query("@c1/*")?.any()?;
            assert!(!is_any);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_skip_limit() {
        catch(|| {
            let db = TestDb::new_with_seed()?;
            let count = db.query("@c1/*")?.skip(2).take(3).count()?;
            assert_eq!(count, 3);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_fold() {
        catch(|| {
            let db = TestDb::new_with_seed()?;
            let res = db
                .query("@c1/*")?
                .take(2)
                .fold(0, |acc, _doc| Ok(acc + 1))
                .unwrap();
            assert_eq!(res, 2);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_for_each() {
        catch(|| {
            let db = TestDb::new_with_seed()?;
            let mut acc = 0;
            db.query("@c1/*")?
                .take(2)
                .for_each(|_doc| {
                    acc = acc + 1;
                    Ok(())
                })
                .unwrap();
            assert_eq!(acc, 2);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_first() {
        catch(|| {
            let db = TestDb::new_with_seed()?;
            let json: String = db
                .query("@c1/*")?
                .skip(2)
                .first(|doc| doc.as_json(None))
                .map(|x| x.unwrap_or_default())
                .unwrap();
            assert!(json.len() > 0);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_filter_with_name() {
        catch(|| {
            let db = TestDb::new_with_seed()?;
            let mut query = db.query("@c1/[c > :age]")?;
            query.jql().set_i64("age", 8)?;
            let json: String = query
                .first(|doc| doc.as_json(None))
                .map(|x| x.unwrap_or_default())
                .unwrap();
            assert_eq!(json, "{\"a\":\"abc8\",\"b\":\"cde6\",\"c\":9}");
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn test_filter_with_index() {
        catch(|| {
            let db = TestDb::new_with_seed()?;
            let mut query = db.query("@c1/[c > :age]")?;
            query.jql().set_i64("age", 8)?;
            let json: String = query
                .first(|doc| doc.as_json(None))
                .map(|x| x.unwrap_or_default())
                .unwrap();
            assert_eq!(json, "{\"a\":\"abc8\",\"b\":\"cde6\",\"c\":9}");
            Ok(())
        })
        .unwrap();
    }
}
