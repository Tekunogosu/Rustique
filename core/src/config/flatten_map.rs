#![allow(unused)]

use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use clap::ValueEnum;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize, Serializer};
use serde::ser::SerializeMap;
use crate::config::config_structs::{CellAttr, CellColor, ColumnProperties};


#[derive(Debug)]
#[derive(Clone)]
pub struct FlattenMap(IndexMap<String, ColumnProperties>);

impl Serialize for FlattenMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        let mut map = serializer.serialize_map(None)?;
        for (key, value) in &self.0 {
            if let Some(color) = &value.color {
                map.serialize_entry(&format!("{key}.color"), color)?;
            }
            if let Some(attr) = &value.attribute {
                map.serialize_entry(&format!("{key}.attribute"), attr)?;
            }
        }

        map.end()
    }
}

impl Deref for FlattenMap {
    type Target = IndexMap<String, ColumnProperties>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FlattenMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FlattenMap
{
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    pub fn with(&mut self, key: &str, color: Option<CellColor>, attribute: Option<CellAttr>) -> &mut Self {
        self.0.insert(key.to_string(), ColumnProperties { color, attribute });
        self
    }
}

impl<'de> Deserialize<'de> for FlattenMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct FlatMapVisitor;

        impl<'de> serde::de::Visitor<'de> for FlatMapVisitor {
            type Value = FlattenMap;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map with dotted keys")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut result = IndexMap::new();

                while let Some((key, value)) = map.next_entry::<String, serde_json::Value>()? {
                    if let Some(dot_pos) = key.find('.') {
                        let (column, attr) = key.split_at(dot_pos);
                        let attr = &attr[1..]; // Skip the dot

                        let entry = result.entry(column.to_string())
                            .or_insert_with(ColumnProperties::default);

                        if attr == "color" {
                            if let Some(color_str) = value.as_str() {
                                entry.color = Option::from(<CellColor as FromStr>::from_str(color_str).unwrap_or(CellColor::Reset));
                            }
                        } else if attr == "attribute" {
                            if let Some(attr_str) = value.as_str() {
                                entry.attribute = Option::from(<CellAttr as FromStr>::from_str(attr_str).unwrap_or(CellAttr::NoHidden));
                            }
                        }
                    }
                }

                Ok(FlattenMap(result))
            }
        }

        deserializer.deserialize_map(FlatMapVisitor)
    }
}