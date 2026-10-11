use crate::utils::device_configuration as conf;
use crate::utils::device_configuration::ConfigureDevice;
use crate::utils::device_configuration::{Found as _, Map as _, Path as _};
use crate::utils::error::DynResult;
use crate::utils::merge_configuration::MergeMap;
use std::str::FromStr;

fn path_err<E, P, R>(error: E, path: P) -> conf::Result<R>
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
    P: Into<String>,
{
    Err(conf::Error::from_error(error.into(), path.into()))
}

fn path_error<E, P>(error: E, path: P) -> conf::Error
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
    P: Into<String>,
{
    conf::Error::from_error(error.into(), path.into())
}

// Create a new label base on old one. If the old label contains a numeric part then it's updated
fn label_offset(label: &str, offset: usize) -> String {
    if offset == 0 {
        return label.to_string();
    }
    if let Some((mut digit_end, _)) = label.rmatch_indices(|c: char| c.is_digit(10)).next() {
        let digit_start = label[0..digit_end]
            .rmatch_indices(|c: char| !c.is_digit(10))
            .next()
            .map(|(p, _)| p + 1) // Move position to first digit
            .unwrap_or(0);
        digit_end += 1;
        // Shouldn't fail since we already checked that it's all digits
        let n = usize::from_str(&label[digit_start..digit_end]).unwrap();
        format!(
            "{0}{1:02$}{3}",
            &label[..digit_start],
            n + offset,
            digit_end - digit_start,
            &label[digit_end..]
        )
    } else {
        format!("{}_{}", label, offset)
    }
}
pub trait CreateDevice {
    fn new_device(
        &mut self,
        name: &str,
        gear_type: &str,
    ) -> DynResult<&mut (dyn ConfigureDevice + Send + Sync)>;
}

fn parse_gears<'a, C>(
    map: &'a dyn conf::Map,
    templates: Option<&'a Box<dyn conf::Map + 'a>>,
    create: &mut C,
) -> conf::Result<()>
where
    C: CreateDevice,
{
    map.iterate(&mut |gear_name_base, gear_conf| {
        let gear_conf = gear_conf.clone().into_map()?;
        let mut merged = MergeMap::new();
        #[allow(unused_assignments)] // The inner value is referenced if there's a template
        let mut template = None;
        merged.add(gear_conf.as_ref());
        if let Some(template_name_value) = gear_conf.get("template").found()? {
            let template_name = template_name_value.as_string()?;
            let Some(templates) = templates else {
                return path_err("No templates", gear_conf.path());
            };
            template = Some(templates.get(&template_name)?.into_map()?);
            merged.add(template.as_ref().unwrap().as_ref())
        }

        let Ok(device_type) = merged.get("type").and_then(|v| v.as_string()) else {
            log::warn!("Gear configuration has no 'type' tag");
            return Ok(());
        };
        log::debug!("Type: {}", device_type);
        let count = merged.get("count").and_then(|v| v.as_i64()).unwrap_or(1) as usize;
        for index in 0..count {
            let name_step = if let Some(v) = merged.get("nameStep").found()? {
                v.as_i64()?
            } else {
                1i64
            };

            let dev_name = label_offset(gear_name_base, index * name_step as usize);
            let device = create
                .new_device(&dev_name, &device_type)
                .map_err(|e| path_error(e, gear_conf.path()))?;
            device.configure(&merged, index)?;
        }

        Ok(())
    })
}

fn parse_dali<C>(map: &conf::Value, create: &mut C) -> conf::Result<()>
where
    C: CreateDevice,
{
    let templates = match map.map_get("templates") {
        Ok(v) => Some(v.into_map()?),
        Err(e) => match e {
            conf::Error {
                kind: conf::ErrorKind::KeyNotFound,
                ..
            } => None,
            _ => return Err(e),
        },
    };
    let gears = map.map_get("gears")?.into_map()?;
    parse_gears(&*gears, templates.as_ref(), create)?;
    Ok(())
}

pub fn parse_config<R, C>(conf_file: R, create: &mut C) -> DynResult<()>
where
    R: std::io::Read,
    C: CreateDevice,
{
    let root = build_tree(conf_file)?;
    let root_map = root.as_map()?;
    let dali = root_map.get("dali")?;
    parse_dali(&dali, create)?;
    Ok(())
}

struct YamlMap<'a> {
    map: &'a yaml_serde::Mapping,
    path: String,
}

impl<'a> conf::Map for YamlMap<'a> {
    fn get(&self, key: &str) -> conf::Result<conf::Value<'a>> {
        if let Some(v) = self.map.get(key) {
            let conf_value = serde_to_conf_value(v, self.path() + "." + key);
            Ok(conf_value)
        } else {
            conf::Error::err(self.path() + "." + key, conf::ErrorKind::KeyNotFound)
        }
    }

    fn iterate<'b>(
        &'b self,
        handle: &mut dyn FnMut(&str, &conf::Value<'b>) -> conf::Result<()>,
    ) -> conf::Result<()> {
        for (kv, v) in self.map {
            let k = match kv {
                yaml_serde::Value::String(s) => s,
                yaml_serde::Value::Number(n) => &format!("{n}"),
                _ => continue,
            };
            let conf_value = serde_to_conf_value(v, self.path() + "." + k);
            handle(k, &conf_value)?;
        }
        Ok(())
    }

    fn clone<'b>(&'b self) -> Box<dyn conf::Map + 'b> {
        Box::new(YamlMap {
            map: self.map,
            path: self.path.clone(),
        })
    }
}

impl conf::Path for YamlMap<'_> {
    fn path(&self) -> String {
        self.path.clone()
    }
}

struct YamlSeq<'a> {
    seq: &'a yaml_serde::Sequence,
    path: String,
}

impl<'a> conf::Seq for YamlSeq<'a> {
    fn get(&self, index: usize) -> conf::Result<conf::Value<'a>> {
        if let Some(v) = self.seq.as_slice().get(index) {
            let conf_value = serde_to_conf_value(v, format!("[{}]{}", index, self.path()));
            Ok(conf_value)
        } else {
            conf::Error::err(
                format!("[{}]{}", index, self.path()),
                conf::ErrorKind::KeyNotFound,
            )
        }
    }

    fn iterate<'b>(
        &'b self,
        handle: &mut dyn FnMut(usize, &conf::Value<'b>) -> conf::Result<()>,
    ) -> conf::Result<()> {
        for (index, v) in self.seq.iter().enumerate() {
            handle(
                index,
                &serde_to_conf_value(v, format!("[{}]{}", index, self.path())),
            )?;
        }
        Ok(())
    }

    fn clone<'b>(&'b self) -> Box<dyn conf::Seq + 'b> {
        Box::new(Self {
            seq: self.seq,
            path: self.path.clone(),
        })
    }
}

impl conf::Path for YamlSeq<'_> {
    fn path(&self) -> String {
        self.path.clone()
    }
}

struct YamlScalar<'a> {
    scalar: &'a yaml_serde::Value,
    path: String,
}

impl<'a> conf::Scalar for YamlScalar<'a> {
    fn as_i64(&self) -> conf::Result<i64> {
        if let Some(v) = self.scalar.as_i64() {
            Ok(v)
        } else {
            conf::Error::err(self.path(), conf::ErrorKind::ValueNotInteger)
        }
    }

    fn as_string(&self) -> conf::Result<String> {
        if let Some(v) = self.scalar.as_str() {
            Ok(v.to_string())
        } else {
            conf::Error::err(self.path(), conf::ErrorKind::ValueNotString)
        }
    }
    fn clone<'b>(&'b self) -> Box<dyn conf::Scalar + 'b> {
        Box::new(Self {
            scalar: self.scalar,
            path: self.path.clone(),
        })
    }
}

impl conf::Path for YamlScalar<'_> {
    fn path(&self) -> String {
        self.path.clone()
    }
}
fn serde_to_conf_value<'a>(serde_value: &'a yaml_serde::Value, path: String) -> conf::Value<'a> {
    match serde_value {
        yaml_serde::Value::Mapping(m) => conf::Value::Map(Box::new(YamlMap { map: m, path })),
        yaml_serde::Value::Sequence(s) => conf::Value::Seq(Box::new(YamlSeq { seq: s, path })),
        v => conf::Value::Scalar(Box::new(YamlScalar { scalar: v, path })),
    }
}

struct YamlRoot {
    map: yaml_serde::Mapping,
}

impl YamlRoot {
    fn new(map: yaml_serde::Mapping) -> Self {
        Self { map }
    }

    fn as_map<'b>(&'b self) -> conf::Result<Box<dyn conf::Map + 'b>> {
        Ok(Box::new(YamlMap {
            map: &self.map,
            path: "".to_string(),
        }))
    }
}

fn build_tree<R>(conf_file: R) -> DynResult<YamlRoot>
where
    R: std::io::Read,
{
    let root: yaml_serde::Mapping = yaml_serde::from_reader(conf_file)?;
    Ok(YamlRoot::new(root))
}

#[test]
fn test_label_offset() {
    assert_eq!(label_offset("23", 1), "24".to_string());
    assert_eq!(label_offset("BAR9R", 1), "BAR10R".to_string());
    assert_eq!(label_offset("BAR09R", 1), "BAR10R".to_string());
    assert_eq!(label_offset("BAR009R", 1), "BAR010R".to_string());
    assert_eq!(label_offset("BAR009R", 99), "BAR108R".to_string());
    assert_eq!(label_offset("BAR", 99), "BAR_99".to_string());
}

#[cfg(test)]
mod test {
    use std::fs::File;
    #[test]
    fn test_build_tree() -> super::conf::Result<()> {
        let conf_file = File::open("test/sim.yaml").unwrap();
        let root = super::build_tree(&conf_file).unwrap();
        let v = root
            .as_map()?
            .get("dali")?
            .map_get("gears")?
            .map_get("BAR01")?
            .map_get("shortAddress")?
            .as_i64()?;
        assert_eq!(v, 1);
        Ok(())
    }
}
