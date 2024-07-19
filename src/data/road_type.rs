/// This module defines the `RoadType` enum and the `Road` struct.
/// 
/// `RoadType` is an enumeration of the different types of roads that can be encountered in the dataset.
/// It includes types such as Primary, Secondary, Residential, Footway, Path, and Unclassified.
///
/// `Road` is a struct that represents a road. It includes a `RoadType`, a `width`, and a `color`.
/// The `width` and `color` attributes are used for rendering the road.
///
/// This module also provides functionality to convert from a string to a `RoadType`, 
/// and to map a `RoadType` to a `width` and a `color`.

use bevy::render::color::Color;

use std::str::FromStr;

use strum_macros::EnumIter;

/// All road types from highway tag in OSM.
/// 
/// # See also
/// https://wiki.openstreetmap.org/wiki/Key:highway
#[repr(u32)]
#[derive(Clone, Copy, Debug, EnumIter, Eq, PartialEq)]
pub enum RoadType {
    // A restricted access major divided highway, normally with 2 or more running lanes plus emergency hard shoulder. Equivalent to the Freeway, Autobahn, etc..
    Motorway, // https://wiki.openstreetmap.org/wiki/Tag:highway%3Dmotorway
    // TODO motorway is divied, we should implement a way to handle that

    // The most important roads in a country's system that aren't motorways. (Need not necessarily be a divided highway.)
    Trunk, // https://wiki.openstreetmap.org/wiki/Tag:highway%3Dtrunk

    // The next most important roads in a country's system. (Often link larger towns.) 
    Primary,  // https://wiki.openstreetmap.org/wiki/Tag:highway%3Dprimary   
    Secondary, // https://wiki.openstreetmap.org/wiki/Tag:highway%3Dprimary
    Tertiary, // https://wiki.openstreetmap.org/wiki/Tag:highway%3Dtertiary

    // The least important through roads in a country's system – i.e. minor roads of a lower classification than tertiary, but which serve a purpose other than access to properties. (Often link villages and hamlets.)
    Unclassified,  // https://wiki.openstreetmap.org/wiki/Tag:highway%3Dunclassified

    // Roads which serve as an access to housing, without function of connecting settlements. Often lined with housing.
    Residential, // https://wiki.openstreetmap.org/wiki/Tag:highway%3Dresidential

    /// **LINK ROADS**
    /// Roads which serve as a connection between other roads, usually in a rural area.
    /// TODO: link roads are usually curves, we can give more detail to them

    MotorwayLink, // https://wiki.openstreetmap.org/wiki/Tag:highway%3Dmotorway_link
    TrunkLink, // https://wiki.openstreetmap.org/wiki/Tag:highway%3Dtrunk_link
    PrimaryLink, // https://wiki.openstreetmap.org/wiki/Tag:highway%3Dprimary_link
    SecondaryLink, // https://wiki.openstreetmap.org/wiki/Tag:highway%3Dsecondary_link
    TertiaryLink, // https://wiki.openstreetmap.org/wiki/Tag:highway%3Dtertiary_link

    /// **SPECIAL ROAD TYPES**
    /// Ignored for now
    /// Living_street, Pedestrian are the most important ones

    /// **PATHS**
    /// A path mainly or exclusively for pedestrians.

    
    // For designated footpaths; mainly/exclusively for pedestrian (and for bikes ofc ;) )
    Footway, // https://wiki.openstreetmap.org/wiki/Tag:highway%3Dfootway

    // For flights of steps (stairs) on footways. Use with step_count=* to indicate the number of steps
    Steps, // https://wiki.openstreetmap.org/wiki/Tag:highway%3Dsteps

    // A non-specific path. Use highway=footway for paths mainly for walkers, highway=cycleway for one also usable by cyclists, highway=bridleway for ones available to horse riders as well as walkers and highway=track for ones which is passable by agriculture or similar vehicles
    Path,       // https://wiki.openstreetmap.org/wiki/Tag:highway%3Dpath


    /// **ROADS THAT ARE NOT COVERED BY THE ENUM**
    /// These are the quite some roads that are not covered by the enum
    /// We can add them to the enum, but it will be a lot of work
    /// So instead we have this type so we can see that the road is not covered by the enum
    NotCovered // Added a new variant to handle roads have a type, but are not covered by the enum
}

/// Convert a string to a `RoadType`.
impl FromStr for RoadType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() { // Using to_lowercase() for case-insensitivity
            "motorway" => Ok(RoadType::Motorway),
            "trunk" => Ok(RoadType::Trunk),
            "primary" => Ok(RoadType::Primary),
            "secondary" => Ok(RoadType::Secondary),
            "tertiary" => Ok(RoadType::Tertiary),
            "residential" => Ok(RoadType::Residential),
            "unclassified" => Ok(RoadType::Unclassified),
            "motorway_link" => Ok(RoadType::MotorwayLink),
            "trunk_link" => Ok(RoadType::TrunkLink),
            "primary_link" => Ok(RoadType::PrimaryLink),
            "secondary_link" => Ok(RoadType::SecondaryLink),
            "tertiary_link" => Ok(RoadType::TertiaryLink),
            "footway" => Ok(RoadType::Footway),
            "steps" => Ok(RoadType::Steps),
            "path" => Ok(RoadType::Path),
            _ =>  Ok(RoadType::NotCovered),
        }
    }
}

/// Represents the width of 1 lane of the road.
/// TODO bigger should be better
pub fn road_type_to_width(road_type: &RoadType) -> f32 {
    match road_type {
        RoadType::Motorway => 4.0,
        RoadType::Trunk => 4.0,
        RoadType::Primary => 3.75,
        RoadType::Secondary => 3.75,
        RoadType::Tertiary => 3.75,
        RoadType::Residential => 3.5,
        RoadType::MotorwayLink => 4.0,  // In between motorway and trunk TODO, possible keep it the same 
        RoadType::TrunkLink => 4.0,    // In between trunk and primary
        RoadType::PrimaryLink => 3.75,  // In between primary and secondary
        RoadType::SecondaryLink => 3.75, // In between secondary and tertiary
        RoadType::TertiaryLink => 3.75, // Same as tertiary
        RoadType::Footway => 0.5, // Pedestrian
        RoadType::Steps => 0.5, // Pedestrian
        RoadType::Path => 0.5,  // Pedestrian
        RoadType::Unclassified => 3.0,
        RoadType::NotCovered => 1.0,
        // Handle any missing cases appropriately, should not happen
        // _ => 1.0, // Default width for unspecified cases
    }
}

/// Map a road type to a height range.
/// This is used to randomly pick height in between to prevent z-fighting (actually y-fighting)
/// Roads are always between 0.01 and 0.02
/// Under 0.01 is rivers and lakes, above 0.02 is buildings
fn road_type_to_height_range(road_type: &RoadType) -> (f32, f32) {
    match road_type {
        RoadType::Motorway => (0.013, 0.017),
        RoadType::Trunk => (0.013, 0.017),
        RoadType::Primary => (0.013, 0.017),
        RoadType::Secondary => (0.013, 0.017),
        RoadType::Tertiary => (0.013, 0.017),
        RoadType::Residential => (0.010, 0.013),
        RoadType::MotorwayLink => (0.010, 0.013),
        RoadType::TrunkLink => (0.010, 0.013),
        RoadType::PrimaryLink => (0.010, 0.013),
        RoadType::SecondaryLink => (0.010, 0.013),
        RoadType::TertiaryLink => (0.010, 0.013),
        RoadType::Footway => (0.017, 0.02),
        RoadType::Steps => (0.017, 0.02),
        RoadType::Path => (0.017, 0.02),
        RoadType::Unclassified => (0.010, 0.013),
        RoadType::NotCovered => (0.010, 0.013),
        // _ => (1.6, 1.7), // Default height range for unspecified cases
    }
}

/// Maps a road type to a random height in the range specified by `road_type_to_height_range`.
pub fn road_type_to_random_height(road_type: &RoadType) -> f32 {
    let (min, max) = road_type_to_height_range(road_type);
    let height = rand::random::<f32>() * (max - min) + min;
    height
}

pub fn road_type_to_default_lanes(road_type: &RoadType) -> u32 {
    match road_type {
        RoadType::Motorway => 4, // Motorways typically have multiple lanes
        RoadType::Trunk => 3, // Trunks are major roads, but less than motorways
        RoadType::Primary => 2, // Primary roads are major roads in the road network
        RoadType::Secondary => 2, // Secondary roads are important, but less so than primary roads
        RoadType::Tertiary => 2, // Tertiary roads connect local areas
        RoadType::Residential => 1, // Residential roads typically have a single lane
        RoadType::MotorwayLink => 2, // Links to motorways, often multiple lanes but fewer than the main motorway
        RoadType::TrunkLink => 2, // Links to trunks, slightly less capacity
        RoadType::PrimaryLink => 1, // Links to primary roads, typically one lane
        RoadType::SecondaryLink => 1, // Links to secondary roads, typically one lane
        RoadType::TertiaryLink => 1, // Links to tertiary roads, typically one lane
        RoadType::Footway => 1, // Pedestrian paths have no lanes
        RoadType::Steps => 1, // Steps are for pedestrians and have no lanes
        RoadType::Path => 1, // Paths are for pedestrians and have no lanes
        RoadType::Unclassified => 1, // Unclassified roads can vary but default to one lane
        RoadType::NotCovered => 1, // For roads not covered by other categories, default to one lane
        // _ => 1, // Default lane count for unspecified cases
    }
}

/// Maps a `RoadType` to a color.
pub fn road_type_to_color(road_type: &RoadType) -> Color {
    match road_type {
        RoadType::Motorway => Color::rgba(0.5, 0.0, 0.0, 1.0), // Dark red
        RoadType::Trunk => Color::rgba(1.0, 0.5, 0.0, 1.0), // Orange
        RoadType::Primary => Color::rgba(1.0, 1.0, 0.0, 1.0), // Yellow
        RoadType::Secondary => Color::rgba(0.0, 0.0, 1.0, 1.0), // Blue
        RoadType::Tertiary => Color::rgba(0.0, 1.0, 0.0, 1.0), // Green
        RoadType::Residential => Color::rgba(1.0, 1.0, 1.0, 1.0), // White
        RoadType::MotorwayLink => Color::rgba(0.3, 0.0, 0.0, 1.0), // Darker red
        RoadType::TrunkLink => Color::rgba(0.8, 0.4, 0.0, 1.0), // Dark orange
        RoadType::PrimaryLink => Color::rgba(0.8, 0.8, 0.0, 1.0), // Dark yellow
        RoadType::SecondaryLink => Color::rgba(0.0, 0.0, 0.8, 1.0), // Dark blue
        RoadType::TertiaryLink => Color::rgba(0.0, 0.8, 0.0, 1.0), // Dark green
        RoadType::Footway => Color::rgba(0.6, 0.6, 0.6, 1.0), // Light grey
        RoadType::Steps => Color::rgba(0.55, 0.55, 0.55, 1.0), // Grey
        RoadType::Path => Color::rgba(0.75, 0.75, 0.75, 1.0), // Silver
        RoadType::Unclassified => Color::rgba(1.0, 1.0, 0.4, 1.0), // Yellow to indicate unclassified
        RoadType::NotCovered => Color::rgba(1.0, 0.0, 1.0, 1.0), // Pink to indicate an unspecified road type
        // _ => Color::rgba(1.0, 0.0, 1.0, 1.0), // Pink to indicate an unspecified road type
    }
}

