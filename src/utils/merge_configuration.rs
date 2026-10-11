use crate::utils::device_configuration as conf;

pub struct MergeMap<'a> {
    maps: Vec<&'a dyn conf::Map>,
}

impl<'a> MergeMap<'a> {
    pub fn new() -> Self {
        Self { maps: Vec::new() }
    }

    pub fn add(&mut self, map: &'a dyn conf::Map) {
        self.maps.push(map);
    }
}

impl<'a> conf::Map for MergeMap<'a> {
    fn get<'b>(&'b self, key: &str) -> conf::Result<conf::Value<'b>> {
        for map in &self.maps {
            if let Ok(v) = map.get(key) {
                return Ok(v);
            }
        }
        if let Some(first) = self.maps.get(0) {
            Err(conf::Error {
                path: first.path(),
                kind: conf::ErrorKind::KeyNotFound,
            })
        } else {
            Err(conf::Error {
                path: "".to_string(),
                kind: conf::ErrorKind::ValueNotMap,
            })
        }
    }

    fn iterate<'b>(
        &'b self,
        handle: &mut dyn FnMut(&str, &conf::Value<'b>) -> conf::Result<()>,
    ) -> conf::Result<()> {
        for map in &self.maps {
            map.iterate(handle)?;
        }
        Ok(())
    }

    fn clone<'b>(&'b self) -> Box<dyn conf::Map + 'b> {
        Box::new(MergeMap {
            maps: self.maps.clone(),
        })
    }
}
impl<'a> conf::Path for MergeMap<'a> {
    fn path(&self) -> String {
        if let Some(first) = self.maps.get(0) {
            first.path()
        } else {
            "".to_string()
        }
    }
}
