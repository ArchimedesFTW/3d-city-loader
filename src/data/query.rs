//! Defines queries for loading external data.

use crate::common::{DataFormat, AppError};

use std::path::PathBuf;

/// A query in internal format that can be executed to load geographic data.
/// 
/// Note that it cannot be assumed that this query is syntactically correct or
/// that the resources queried actually exist!
#[derive(Clone, Debug)]
pub enum DataQuery {
    /// A query in [OverpassQL]. Note that the output is assumed to be [OsmJSON]!
    /// 
    /// [OverpassQL]: https://wiki.openstreetmap.org/wiki/Overpass_API/Overpass_QL
    /// [OsmJSON]: https://wiki.openstreetmap.org/wiki/API_v0.6#JSON_Format
    OverpassQL {
        value: String,
    },
    /// A file on the local file system.
    File {
        format: DataFormat,
        file_path: PathBuf,
    },
}

/// The type of a user's query in the UI.
/// 
/// Note that there are additional options here compared to `DataQuery`. This is
/// because some of the query types are just a convenience thing and are mapped
/// to other data query types.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum InputQueryType {
    City,
    File,
    Overpass,
}

/// Converts a query string given by the user to a query in internal format.
pub fn parse_data_query(
    query_type: InputQueryType,
    string: &str,
) -> Result<DataQuery, AppError> {
    match query_type {
        InputQueryType::City => {
            if string.contains('"') {
                return Err(AppError::InputSyntax {
                    message: "city query may not contain quotes".to_owned(),
                });
            }

            // city queries are mapped to overpass QL queries
            // ->. stores the result of the area[name=...] query in searchArea
            // it then finds all "way", and then appends the nodes inside
            // `out body` means outputting all tags
            Ok(DataQuery::OverpassQL {
                value: format!(r#"[out:json];
                    area[name="{}"]->.searchArea;
                    (
                        way["highway"](area.searchArea);
                        way["building"](area.searchArea);
                        way["landuse"](area.searchArea);
                        way["natural"="water"](area.searchArea);
                        way["waterway"~"river|stream|canal|ditch"](area.searchArea);
                    )->.result;
                    (.result; .result >;);
                    out body;"#,
                    string,
                ),
            })
        },
        InputQueryType::Overpass => {
            Ok(DataQuery::OverpassQL { value: string.to_owned() })
        },
        InputQueryType::File => {
            let file_path = PathBuf::from(string);
            let extension = file_path.extension();
            let format = match extension {
                Some(ext) if ext == "json" => DataFormat::OsmJson,
                Some(ext) if ext == "geojson" => DataFormat::GeoJson,
                Some(ext) => return Err(AppError::InputSyntax {
                    message: format!("unsupported file extension {:?}", ext),
                }),
                None => return Err(AppError::InputSyntax {
                    message: format!("file without file extension"),
                }),
            };

            Ok(DataQuery::File { format, file_path })
        },
    }
}
