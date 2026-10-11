#[derive(Debug)]
pub struct Error {
    pub path: String,
    pub kind: ErrorKind,
}

#[derive(Debug)]
pub enum ErrorKind {
    IndexOutOfBounds,
    KeyNotFound,
    ValueOutOfBounds,
    ValueNotInteger,
    ValueNotString,
    ValueNotMap,
    ValueNotSeq,
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl Error {
    pub fn new(path: String, kind: ErrorKind) -> Error {
        Error { path: path, kind }
    }

    pub fn err<T>(path: String, kind: ErrorKind) -> Result<T> {
        Err(Error::new(path, kind))
    }
}

impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path, self.kind)
    }
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ErrorKind::*;
        match self {
            IndexOutOfBounds => write!(f, "Index out of bounds"),
            KeyNotFound => write!(f, "Key not found"),
            ValueOutOfBounds => write!(f, "Value out of bounds"),
            ValueNotInteger => write!(f, "Value is not an integer"),
            ValueNotString => write!(f, "Value is not a string"),
            ValueNotMap => write!(f, "Value is not a map"),
            ValueNotSeq => write!(f, "Value is not a sequence"),
            Other(e) => write!(f, "{}", e),
        }
    }
}

impl Error {
    pub fn from_error(err: Box<dyn std::error::Error + Send + Sync>, path: String) -> Error {
        Error {
            path,
            kind: ErrorKind::Other(err),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait Found<T> {
    fn found(self) -> Result<Option<T>>;
}

impl<T> Found<T> for Result<T> {
    fn found(self) -> Result<Option<T>> {
        match self {
            Ok(v) => Ok(Some(v)),
            Err(Error {
                kind: ErrorKind::KeyNotFound,
                ..
            })
            | Err(Error {
                kind: ErrorKind::IndexOutOfBounds,
                ..
            }) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

pub enum Value<'a> {
    Scalar(Box<dyn Scalar + 'a>),
    Map(Box<dyn Map + 'a>),
    Seq(Box<dyn Seq + 'a>),
}

impl<'a> Value<'a> {
    pub fn as_i64(&self) -> Result<i64> {
        if let Value::Scalar(s) = self {
            s.as_i64()
        } else {
            Error::err(self.path(), ErrorKind::ValueNotInteger)
        }
    }

    pub fn as_string(&self) -> Result<String> {
        if let Value::Scalar(s) = self {
            s.as_string()
        } else {
            Error::err(self.path(), ErrorKind::ValueNotInteger)
        }
    }

    pub fn into_map(self) -> Result<Box<dyn Map + 'a>> {
        if let Value::Map(m) = self {
            Ok(m)
        } else {
            Error::err(self.path(), ErrorKind::ValueNotMap)
        }
    }

    pub fn map_get(&'a self, key: &str) -> Result<Value<'a>> {
        if let Value::Map(m) = self {
            m.get(key)
        } else {
            Error::err(self.path(), ErrorKind::ValueNotMap)
        }
    }

    pub fn map_iterate(&'a self, handle: &mut dyn FnMut(&str, &Value) -> Result<()>) -> Result<()> {
        if let Value::Map(m) = self {
            m.iterate(handle)
        } else {
            Error::err(self.path(), ErrorKind::ValueNotMap)
        }
    }

    pub fn into_seq(self) -> Result<Box<dyn Seq + 'a>> {
        if let Value::Seq(m) = self {
            Ok(m)
        } else {
            Error::err(self.path(), ErrorKind::ValueNotSeq)
        }
    }

    pub fn seq_get(&'a self, index: usize) -> Result<Value<'a>> {
        if let Value::Seq(s) = self {
            s.get(index)
        } else {
            Error::err(self.path(), ErrorKind::ValueNotSeq)
        }
    }

    pub fn seq_iterate(
        &'a self,
        handle: &mut dyn FnMut(usize, &Value) -> Result<()>,
    ) -> Result<()> {
        if let Value::Seq(s) = self {
            s.iterate(handle)
        } else {
            Error::err(self.path(), ErrorKind::ValueNotSeq)
        }
    }

    pub fn clone(&'a self) -> Value<'a> {
        match self {
            Self::Scalar(v) => Self::Scalar((*v).clone()),
            Self::Map(v) => Self::Map((*v).clone()),
            Self::Seq(v) => Self::Seq((*v).clone()),
        }
    }

    pub fn path(&self) -> String {
        match self {
            Self::Scalar(v) => v.path(),
            Self::Map(v) => v.path(),
            Self::Seq(v) => v.path(),
        }
        .clone()
    }
}

pub trait Path {
    fn path(&self) -> String;
}

pub trait Scalar: Path {
    fn as_i64(&self) -> Result<i64>;
    fn as_string(&self) -> Result<String>;
    fn clone<'b>(&'b self) -> Box<dyn Scalar + 'b>;
}

pub trait Map: Path {
    fn get<'b>(&'b self, key: &str) -> Result<Value<'b>>;
    fn iterate<'b>(&'b self, handle: &mut dyn FnMut(&str, &Value<'b>) -> Result<()>) -> Result<()>;
    fn clone<'b>(&'b self) -> Box<dyn Map + 'b>;
}

pub trait Seq: Path {
    fn get<'b>(&'b self, index: usize) -> Result<Value<'b>>;
    fn iterate<'b>(&'b self, handle: &mut dyn FnMut(usize, &Value<'b>) -> Result<()>)
    -> Result<()>;
    fn clone<'b>(&'b self) -> Box<dyn Seq + 'b>;
}

pub trait ConfigureDevice {
    fn configure(&mut self, configuration: &dyn Map, index: usize) -> Result<()>;
}

#[cfg(test)]
mod test {
    use super as conf;
    use super::{Error, ErrorKind, Map as _, Path as _, Result, Scalar as _, Seq as _};
    use std::collections::BTreeMap;

    enum Value {
        Integer(i64),
        Map(BTreeMap<String, Value>),
        Seq(Vec<Value>),
    }
    struct Integer<'a> {
        v: &'a Value,
        path: String,
    }

    impl<'a> Integer<'a> {
        fn new(v: &'a Value) -> Self {
            Self {
                v,
                path: String::new(),
            }
        }
    }

    impl<'a> conf::Scalar for Integer<'a> {
        fn as_i64(&self) -> Result<i64> {
            if let Value::Integer(v) = self.v {
                Ok(*v)
            } else {
                return Err(Error {
                    path: self.path.clone(),
                    kind: ErrorKind::ValueNotInteger,
                });
            }
        }
        fn as_string(&self) -> Result<String> {
            return Err(Error {
                path: self.path.clone(),
                kind: ErrorKind::ValueNotString,
            });
        }

        fn clone<'b>(&'b self) -> Box<dyn conf::Scalar + 'b> {
            Box::new(Integer {
                v: self.v,
                path: self.path.clone(),
            })
        }
    }
    impl<'a> conf::Path for Integer<'a> {
        fn path(&self) -> String {
            self.path.clone()
        }
    }

    macro_rules! check_error {
        ($res: expr, $path: expr, $kind: ident) => {{
            if let Err(err) = $res {
                assert_eq!(err.path, $path);
                if let conf::ErrorKind::$kind = err.kind {
                } else {
                    panic!("{:?} != {:?}", conf::ErrorKind::$kind, err.kind);
                }
            } else {
                panic!("Not an error");
            }
        }};
    }

    #[test]
    fn test_scalar() {
        let v = Value::Integer(82);
        let s = Integer::new(&v);
        assert_eq!(s.as_i64().unwrap(), 82);
    }

    #[test]
    fn test_map() {
        let mut bmap = BTreeMap::new();
        let m = ValueMap::new(&bmap);
        check_error!(m.get("foo"), ".foo", KeyNotFound);
        drop(m);
        bmap.insert("bar".to_string(), Value::Integer(78));
        bmap.insert("foo".to_string(), Value::Integer(4));
        let m = ValueMap::new(&bmap);
        assert_eq!(m.get("foo").unwrap().as_i64().unwrap(), 4);
        let v = conf::Value::Map(Box::new(m));
        {
            let bar = v.map_get("bar").unwrap();
            let i = bar.as_i64().unwrap();
            assert_eq!(i, 78i64);
        }
    }

    #[test]
    fn test_sequence() {
        let vec = vec![Value::Integer(12), Value::Integer(-43), Value::Integer(89)];
        let seq = ValueSequence::new(&vec);
        let i = seq.get(0).unwrap().as_i64().unwrap();
        assert_eq!(i, 12);
        check_error!(seq.get(3), "[3]", IndexOutOfBounds);
        let v = conf::Value::Seq(Box::new(seq));
        let seq = v.into_seq().unwrap();
        {
            let _f = seq.get(1);
        }
    }

    struct ValueMap<'a> {
        map: &'a BTreeMap<String, Value>,
        path: String,
    }

    impl<'a> ValueMap<'a> {
        fn new(map: &'a BTreeMap<String, Value>) -> Self {
            Self {
                map,
                path: String::new(),
            }
        }
    }

    fn build_value<'a>(v: &'a Value, path: String) -> conf::Value<'a> {
        match v {
            Value::Integer(_) => conf::Value::Scalar(Box::new(Integer { v, path })),
            Value::Map(m) => conf::Value::Map(Box::new(ValueMap { map: m, path: path })),
            Value::Seq(s) => conf::Value::Seq(Box::new(ValueSequence { seq: s, path: path })),
        }
    }
    impl<'a> conf::Map for ValueMap<'a> {
        fn get<'b>(&'b self, key: &str) -> Result<conf::Value<'b>> {
            if let Some(v) = self.map.get(&key.to_string()) {
                Ok(build_value(v, ".".to_string() + &self.path + key))
            } else {
                Error::err(self.path() + "." + key, ErrorKind::KeyNotFound)
            }
        }

        fn iterate<'b>(
            &'b self,
            handle: &mut dyn FnMut(&str, &conf::Value<'b>) -> Result<()>,
        ) -> Result<()> {
            for (k, v) in self.map {
                handle(k, &build_value(v, self.path() + "." + k))?;
            }
            Ok(())
        }

        fn clone<'b>(&'b self) -> Box<dyn conf::Map + 'b> {
            Box::new(ValueMap {
                map: self.map,
                path: self.path.clone(),
            })
        }
    }

    impl conf::Path for ValueMap<'_> {
        fn path(&self) -> String {
            self.path.clone()
        }
    }

    struct ValueSequence<'a> {
        seq: &'a Vec<Value>,
        path: String,
    }

    impl<'a> ValueSequence<'a> {
        fn new(seq: &'a Vec<Value>) -> Self {
            Self {
                seq,
                path: String::new(),
            }
        }
    }

    impl<'a> conf::Seq for ValueSequence<'a> {
        fn get<'b>(&'b self, index: usize) -> Result<conf::Value<'b>> {
            if let Some(v) = self.seq.get(index) {
                Ok(build_value(v, format!("{}[{}]", self.path, index)))
            } else {
                Error::err(
                    format!("{}[{}]", self.path, index),
                    ErrorKind::IndexOutOfBounds,
                )
            }
        }

        fn iterate<'b>(
            &'b self,
            handle: &mut dyn FnMut(usize, &conf::Value<'b>) -> Result<()>,
        ) -> Result<()> {
            for (index, v) in self.seq.iter().enumerate() {
                handle(index, &build_value(v, format!("{}[{}]", self.path, index)))?;
            }
            Ok(())
        }

        fn clone<'b>(&'b self) -> Box<dyn conf::Seq + 'b> {
            Box::new(ValueSequence {
                seq: self.seq,
                path: self.path.clone(),
            })
        }
    }

    impl conf::Path for ValueSequence<'_> {
        fn path(&self) -> String {
            self.path.clone()
        }
    }

    /*
        #[test]
        fn test_nested_errors() {
            let s1 = Integer::new(23);
            let mut m1 = ValueMap::new();
            let mut m2 = ValueMap::new();
            let mut seq1 = ValueSequence::new();
            m1.map.insert("s1".to_string(), Value::Scalar(&s1));
            seq1.seq.push(Value::Map(&m1));
            m2.map.insert("seq1".to_string(), Value::Seq(&seq1));
            check_error!(
                m2.get("seq1")
                    .unwrap()
                    .seq_get(0)
                    .unwrap()
                    .map_get("s1")
                    .unwrap()
                    .as_i64(0..14),
                ".seq1[0].s1",
                ValueOutOfBounds
            );
    }
        */
}
