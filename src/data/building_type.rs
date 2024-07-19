use std::{convert::Infallible, str::FromStr};

use bevy::math::Vec2;

/// This module defines the `BuildingType` and `RoofShape` enums and the `PartialBuilding` and `Building` structs.
///
/// `BuildingType` is an enumeration of the different types of buildings that can be encountered in the dataset.
/// `RoofShape` is an enumeration of the different shapes of roofs that can be encountered in the dataset.
///
///  `Building` represents a building. It includes a `BuildingType` but also other things like the number of levels.
///  A `PartialBuilding` is the same as a `Building`, but only filled with known information from the data. Should be filled with more information at a later stage.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildingType {
    Apartments,
    Barracks,
    Bungalow,
    Cabin,
    Detached,
    Dormitory,
    Farm,
    Hotel,
    House,
    Houseboat,
    Residential,
    SemidetachedHouse,
    StaticCaravan,
    Terrace,
    Commercial,
    Industrial,
    Kiosk,
    Office,
    Retail,
    Supermarket,
    Warehouse,
    Bakehouse,
    Bridge,
    Civic,
    College,
    FireStation,
    Government,
    Hospital,
    Kindergarten,
    Museum,
    Public,
    School,
    Toilets,
    TrainStation,
    Transportation,
    University,
    Other,
}

/// Maps a `BuildingType` to a range of levels that is reasonable for that type of building.
pub fn get_random_range_building(building_type: BuildingType) -> (i32, i32) {
    match building_type {
        // Most important
        BuildingType::House => (2, 2),
        BuildingType::Apartments => (3, 6),
        BuildingType::Commercial => (1, 4),
        BuildingType::Industrial => (2, 4),
        BuildingType::School => (2, 4),
        // Less important
        BuildingType::Barracks => (1, 2),
        BuildingType::Bungalow => (1, 1),
        BuildingType::Cabin => (1, 1),
        BuildingType::Detached => (2, 3),
        BuildingType::Dormitory => (2, 4),
        BuildingType::Farm => (1, 1),
        BuildingType::Hotel => (3, 6),
        BuildingType::Houseboat => (1, 2),
        BuildingType::Residential => (2, 5),
        BuildingType::SemidetachedHouse => (2, 3),
        BuildingType::StaticCaravan => (1, 1),
        BuildingType::Terrace => (2, 3),
        BuildingType::Kiosk => (1, 1),
        BuildingType::Office => (2, 8),
        BuildingType::Retail => (2, 3),
        BuildingType::Supermarket => (1, 1),
        BuildingType::Warehouse => (2, 2),
        BuildingType::Bakehouse => (1, 1),
        BuildingType::Bridge => (1, 1),
        BuildingType::Civic => (2, 4),
        BuildingType::College => (2, 4),
        BuildingType::FireStation => (1, 2),
        BuildingType::Government => (2, 4),
        BuildingType::Hospital => (2, 6),
        BuildingType::Kindergarten => (1, 2),
        BuildingType::Museum => (1, 3),
        BuildingType::Public => (2, 4),
        BuildingType::Toilets => (1, 1),
        BuildingType::TrainStation => (1, 3),
        BuildingType::Transportation => (1, 3),
        BuildingType::University => (2, 6),
        BuildingType::Other => (1, 1),
    }
}

/// Error type for parsing a `BuildingType`.
/// Can realistically only happen when the building was marked as present but type was not specified https://wiki.openstreetmap.org/wiki/Tag:building%3Dyes
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct BuildingParseError;

/// Convert a string to a `BuildingType`.
impl FromStr for BuildingType {
    type Err = BuildingParseError;
    fn from_str(s: &str) -> Result<Self, BuildingParseError> {
        match s {
            "apartments" => Ok(BuildingType::Apartments),
            "barracks" => Ok(BuildingType::Barracks),
            "bungalow" => Ok(BuildingType::Bungalow),
            "cabin" => Ok(BuildingType::Cabin),
            "detached" => Ok(BuildingType::Detached),
            "dormitory" => Ok(BuildingType::Dormitory),
            "farm" => Ok(BuildingType::Farm),
            "hotel" => Ok(BuildingType::Hotel),
            "house" => Ok(BuildingType::House),
            "houseboat" => Ok(BuildingType::Houseboat),
            "residential" => Ok(BuildingType::Residential),
            "semidetached_house" => Ok(BuildingType::SemidetachedHouse),
            "static_caravan" => Ok(BuildingType::StaticCaravan),
            "terrace" => Ok(BuildingType::Terrace),
            "commercial" => Ok(BuildingType::Commercial),
            "industrial" => Ok(BuildingType::Industrial),
            "kiosk" => Ok(BuildingType::Kiosk),
            "office" => Ok(BuildingType::Office),
            "retail" => Ok(BuildingType::Retail),
            "supermarket" => Ok(BuildingType::Supermarket),
            "warehouse" => Ok(BuildingType::Warehouse),
            "bakehouse" => Ok(BuildingType::Bakehouse),
            "bridge" => Ok(BuildingType::Bridge),
            "civic" => Ok(BuildingType::Civic),
            "college" => Ok(BuildingType::College),
            "fire_station" => Ok(BuildingType::FireStation),
            "government" => Ok(BuildingType::Government),
            "hospital" => Ok(BuildingType::Hospital),
            "kindergarten" => Ok(BuildingType::Kindergarten),
            "museum" => Ok(BuildingType::Museum),
            "public" => Ok(BuildingType::Public),
            "school" => Ok(BuildingType::School),
            "toilets" => Ok(BuildingType::Toilets),
            "train_station" => Ok(BuildingType::TrainStation),
            "transportation" => Ok(BuildingType::Transportation),
            "university" => Ok(BuildingType::University),
            "yes" => Err(BuildingParseError), // This is a special case, see BuildingParseError
            _ => Ok(BuildingType::Other),
        }
    }
}

/// # See also
/// [Reference]: https://wiki.openstreetmap.org/wiki/Key:roof:shape
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoofShape {
    Flat,
    Gabled,
    Shed,
    Hipped,
    Gambrel,
    Mansard,
}

/// Convert a string to a `RoofShape`.
impl FromStr for RoofShape {
    type Err = Infallible; // Can't fail

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "flat" => Ok(RoofShape::Flat),
            "gabled" => Ok(RoofShape::Gabled),
            "shed" => Ok(RoofShape::Shed),
            "hipped" => Ok(RoofShape::Hipped),
            "gambrel" => Ok(RoofShape::Gambrel),
            "mansard" => Ok(RoofShape::Mansard),
            _ => Ok(RoofShape::Flat), // Default to flat
        }
    }
}

/// Same as for building, but only filled with known information. Should be filled with more information at a later stage.
pub struct PartialBuilding {
    pub id: u64,
    pub building_type: Option<BuildingType>,
    pub levels: Option<i32>,
    pub base: Vec<Vec2>,
    pub roof_shape: Option<RoofShape>,
    pub roof_levels: Option<i32>,
    pub inside_area: BuildingLandUseType,
}

/// What land use area a building is in, useful for determining building type if that is not known
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildingLandUseType {
    Commercial,
    Education,
    Industrial,
    Residential,
    Unknown,
    NOTNECESSARY, // Special case, for when the building type is already known so we do not need the land use to infer the building type
}

/// Convert a string to a `BuildingLandUseType`.
impl FromStr for BuildingLandUseType {
    type Err = Infallible; // Can't fail

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "commercial" => Ok(BuildingLandUseType::Commercial),
            "retail" => Ok(BuildingLandUseType::Commercial),
            "education" => Ok(BuildingLandUseType::Education),
            "industrial" => Ok(BuildingLandUseType::Industrial),
            "residential" => Ok(BuildingLandUseType::Residential),
            _ => Ok(BuildingLandUseType::Unknown), // Default to unknown
        }
    }
}
