use crate::config::flatten_map::FlattenMap;
use clap::ValueEnum;
use comfy_table::{Attribute, Color};
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use std::fmt::Display;
use std::str::FromStr;

#[derive(Deserialize, Debug, Clone)]
pub struct Tables {
    pub list: TableSection,
    pub search: TableSection,
}

impl Serialize for Tables {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("list", &self.list)?;
        map.serialize_entry("search", &self.search)?;
        map.end()
    }
}

impl Default for Tables {
    fn default() -> Self {
        Self {
            list: Self::list_defaults(),
            search: Self::search_defaults(),
        }
    }
}

impl Tables {
    pub fn with_defaults() -> Self {
        Self { list: Self::list_defaults(), search: Self::search_defaults() }
    }
    
    pub fn list_defaults() -> TableSection {
    let mut list = TableSection::new();

        // List headers
        list.headers
            .with(
                ListColumn::Name.as_str(),
                Some(CellColor::Green),
                Some(CellAttr::Bold),
            )
            .with(
                ListColumn::ModId.as_str(),
                Some(CellColor::Green),
                Some(CellAttr::Bold),
            )
            .with(
                ListColumn::Version.as_str(),
                Some(CellColor::Green),
                Some(CellAttr::Bold),
            )
            .with(
                ListColumn::LatestVersion.as_str(),
                Some(CellColor::Green),
                Some(CellAttr::Bold),
            )
            .with(
                ListColumn::Deps.as_str(),
                Some(CellColor::Green),
                Some(CellAttr::Bold),
            )
            .with(
                ListColumn::MissingDeps.as_str(),
                Some(CellColor::Green),
                Some(CellAttr::Bold),
            )
            .with(
                ListColumn::Description.as_str(),
                Some(CellColor::Green),
                Some(CellAttr::Bold),
            );

        // List cells
        list.cells
            .with(ListColumn::Name.as_str(), Some(CellColor::Yellow), None)
            .with(ListColumn::ModId.as_str(), Some(CellColor::Reset), None)
            .with(
                ListColumn::Version.as_str(),
                Some(CellColor::Reset),
                Some(CellAttr::Dim),
            )
            .with(
                ListColumn::LatestVersion.as_str(),
                Some(CellColor::Green),
                Some(CellAttr::Bold),
            )
            .with(ListColumn::Deps.as_str(), Some(CellColor::Reset), None)
            .with(
                ListColumn::MissingDeps.as_str(),
                Some(CellColor::Red),
                Some(CellAttr::Bold),
            )
            .with(
                ListColumn::Description.as_str(),
                Some(CellColor::Reset),
                None,
            ); 
        
        list
    }
    
    pub fn search_defaults() -> TableSection {
        let mut search = TableSection::new();

        // Search headers
        search
            .headers
            .with("mod_id", Some(CellColor::Green), Some(CellAttr::Bold))
            .with("name", Some(CellColor::Green), Some(CellAttr::Bold))
            .with("summary", Some(CellColor::Green), Some(CellAttr::Bold));

        // Search cells
        search
            .cells
            .with("mod_id", Some(CellColor::Magenta), Some(CellAttr::Bold))
            .with("name", Some(CellColor::Reset), None)
            .with("summary", Some(CellColor::Reset), None);
        
        search
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct TableSection {
    pub headers: FlattenMap,
    pub cells: FlattenMap,
}

impl Serialize for TableSection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("headers", &self.headers)?;
        map.serialize_entry("cells", &self.cells)?;
        map.end()
    }
}

impl Default for TableSection {
    fn default() -> Self {
        Self {
            headers: FlattenMap::new(),
            cells: FlattenMap::new(),
        }
    }
}

impl TableSection {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[derive(Clone)]
pub struct ColumnProperties {
    pub color: Option<CellColor>,
    pub attribute: Option<CellAttr>,
}

impl Default for ColumnProperties {
    fn default() -> Self {
        Self {
            color: Option::from(CellColor::Reset),
            attribute: Option::from(CellAttr::NoHidden),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ModPack {}

#[derive(Deserialize, Serialize, Debug)]
pub struct AliasConfig {
    pub name: String,
    pub mod_dir: String,
    pub pinned_game_version: String,
}

#[derive(ValueEnum, Deserialize, Debug, Clone)]
pub enum CellColor {
    Black,
    Blue,
    Cyan,
    DarkCyan,
    DarkBlue,
    Green,
    DarkGreen,
    Grey,
    DarkGrey,
    Magenta,
    DarkMagenta,
    Red,
    DarkRed,
    White,
    Yellow,
    DarkYellow,
    Reset,
}

impl Serialize for CellColor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Black => serializer.serialize_str("black"),
            Self::Blue => serializer.serialize_str("blue"),
            Self::DarkBlue => serializer.serialize_str("dark_blue"),
            Self::Cyan => serializer.serialize_str("cyan"),
            Self::DarkCyan => serializer.serialize_str("dark_cyan"),
            Self::Green => serializer.serialize_str("green"),
            Self::DarkGreen => serializer.serialize_str("dark_green"),
            Self::Grey => serializer.serialize_str("grey"),
            Self::DarkGrey => serializer.serialize_str("dark_grey"),
            Self::Magenta => serializer.serialize_str("magenta"),
            Self::DarkMagenta => serializer.serialize_str("dark_magenta"),
            Self::Red => serializer.serialize_str("red"),
            Self::DarkRed => serializer.serialize_str("dark_red"),
            Self::White => serializer.serialize_str("white"),
            Self::Yellow => serializer.serialize_str("yellow"),
            Self::DarkYellow => serializer.serialize_str("dark_yellow"),
            Self::Reset => serializer.serialize_str("reset"),
        }
    }
}

impl FromStr for CellColor {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "black"         => Ok(Self::Black),
            "blue"          => Ok(Self::Blue),
            "dark_blue"     => Ok(Self::DarkBlue),
            "cyan"          => Ok(Self::Cyan),
            "dark_cyan"     => Ok(Self::DarkCyan),
            "green"         => Ok(Self::Green),
            "dark_green"    => Ok(Self::DarkGreen),
            "magenta"       => Ok(Self::Magenta),
            "dark_magenta"  => Ok(Self::DarkMagenta),
            "red"           => Ok(Self::Red),
            "dark_red"      => Ok(Self::DarkRed),
            "white"         => Ok(Self::White),
            "yellow"        => Ok(Self::Yellow),
            "dark_yellow"   => Ok(Self::DarkYellow),
            "reset"         => Ok(Self::Reset),
            _ => Err(()),
        }
    }
}

// This makes it easier to directly map our enum to Color so it can be used with ValueEnum for CLI
impl From<CellColor> for Color {
    fn from(value: CellColor) -> Self {
        match value {
            CellColor::Black        => Color::Black,
            CellColor::Blue         => Color::Blue,
            CellColor::DarkBlue     => Color::DarkBlue,
            CellColor::Green        => Color::Green,
            CellColor::DarkGreen    => Color::DarkGreen,
            CellColor::Grey         => Color::Grey,
            CellColor::DarkGrey     => Color::DarkGrey,
            CellColor::Magenta      => Color::Magenta,
            CellColor::DarkMagenta  => Color::DarkMagenta,
            CellColor::Red          => Color::Red,
            CellColor::DarkRed      => Color::DarkRed,
            CellColor::White        => Color::White,
            CellColor::Yellow       => Color::Yellow,
            CellColor::DarkYellow   => Color::DarkYellow,
            // This covers Color::Reset by default as well
            _ => Color::Reset,
        }
    }
}

impl Display for CellColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CellColor::Black        => write!(f, "black"),
            CellColor::Blue         => write!(f, "blue"),
            CellColor::DarkBlue     => write!(f, "dark_blue"),
            CellColor::Cyan         => write!(f, "cyan"),
            CellColor::DarkCyan     => write!(f, "dark_cyan"),
            CellColor::Green        => write!(f, "green"),
            CellColor::DarkGreen    => write!(f, "dark_green"),
            CellColor::Grey         => write!(f, "grey"),
            CellColor::DarkGrey     => write!(f, "dark_grey"),
            CellColor::Magenta      => write!(f, "magenta"),
            CellColor::DarkMagenta  => write!(f, "dark_magenta"),
            CellColor::Red          => write!(f, "red"),
            CellColor::DarkRed      => write!(f, "dark_red"),
            CellColor::Reset        => write!(f, "reset"),
            CellColor::White        => write!(f, "white"),
            CellColor::Yellow       => write!(f, "yellow"),
            CellColor::DarkYellow   => write!(f, "dark_yellow"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum)]
pub enum ListColumn {
    Name,
    ModId,
    Version,
    LatestVersion,
    Deps,
    MissingDeps,
    Changelog,
    Description,
    Website,
    GameVersion,
    LastUpdateLocal,
    LastUpdateRemote,
    PinnedVersion,
    HasBackup,
    Filename,
    ModURL,
}

impl ListColumn {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Name          => "name",
            Self::ModId         => "mod_id",
            Self::Version       => "version",
            Self::LatestVersion => "latest_version",
            Self::GameVersion   => "game_version",
            Self::PinnedVersion => "pinned_version",
            Self::Deps          => "deps",
            Self::MissingDeps   => "missing_deps",
            Self::Changelog     => "changelog",
            Self::Description   => "description",
            Self::Website       => "website",
            Self::LastUpdateLocal   => "last_update",
            Self::LastUpdateRemote  => "last_update_remote",
            Self::HasBackup         => "has_backup",
            Self::Filename          => "filename",
            Self::ModURL            => "mod_url",      
        }
    }
}

impl FromStr for ListColumn {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "name"              => Ok(Self::Name),
            "mod_id"            => Ok(Self::ModId),
            "version"           => Ok(Self::Version),
            "latest_version"    => Ok(Self::LatestVersion),
            "deps"              => Ok(Self::Deps),
            "missing_deps"      => Ok(Self::MissingDeps),
            "changelog"         => Ok(Self::Changelog),
            "description"       => Ok(Self::Description),
            "website"           => Ok(Self::Website),
            "game_version"      => Ok(Self::GameVersion),
            "last_update_local" => Ok(Self::LastUpdateLocal),
            "last_update_remote" => Ok(Self::LastUpdateRemote),
            "pinned_version"     => Ok(Self::PinnedVersion),
            "has_backup"         => Ok(Self::HasBackup),
            "filename"           => Ok(Self::Filename),
            "mod_url"           => Ok(Self::ModURL),
            _ => Err(()),
        }
    }
}

impl Display for ListColumn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListColumn::Name            => write!(f, "name"),
            ListColumn::ModId           => write!(f, "mod_id"),
            ListColumn::Version         => write!(f, "version"),
            ListColumn::LatestVersion   => write!(f, "latest_version"),
            ListColumn::GameVersion     => write!(f, "game_version"),
            ListColumn::PinnedVersion   => write!(f, "pinned_version"),
            ListColumn::Deps            => write!(f, "deps"),
            ListColumn::MissingDeps     => write!(f, "missing_deps"),
            ListColumn::Changelog       => write!(f, "changelog"),
            ListColumn::Description     => write!(f, "description"),
            ListColumn::Website         => write!(f, "website"),
            ListColumn::LastUpdateLocal => write!(f, "last_update"),
            ListColumn::LastUpdateRemote    => write!(f, "last_update_remote"),
            ListColumn::HasBackup           => write!(f, "has_backup"),
            ListColumn::Filename            => write!(f, "filename"),
            ListColumn::ModURL              => write!(f, "mod_url"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ValueEnum)]
pub enum SearchColumn {
    Name,
    ModId,
    Downloads,
    Follows,
    Trending,
    Comments,
    Summary,
    ModidStrs,
    AssetId,
    Author,
    UrlAlias,
    Side,
    Type,
    Tags,
    LastReleased,
}

impl SearchColumn {
    #[allow(unused)]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Name          => "name",
            Self::ModId         => "mod_id",
            Self::AssetId       => "asset_id",
            Self::Downloads     => "downloads",
            Self::Follows       => "follows",
            Self::Trending      => "trending",
            Self::Comments      => "comments",
            Self::Summary       => "summary",
            Self::ModidStrs     => "modid_strs",
            Self::Author        => "author",
            Self::UrlAlias      => "url_alias",
            Self::Side          => "side",
            Self::Type          => "type",
            Self::Tags          => "tags",
            Self::LastReleased  => "last_released",
        }
    }
}

impl FromStr for SearchColumn {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "name"          => Ok(Self::Name),
            "mod_id"        => Ok(Self::ModId),
            "asset_id"      => Ok(Self::AssetId),
            "downloads"     => Ok(Self::Downloads),
            "follows"       => Ok(Self::Follows),
            "trending"      => Ok(Self::Trending),
            "comments"      => Ok(Self::Comments),
            "summary"       => Ok(Self::Summary),
            "modid_strs"    => Ok(Self::ModidStrs),
            "author"        => Ok(Self::Author),
            "url_alias"     => Ok(Self::UrlAlias),
            "side"          => Ok(Self::Side),
            "type"          => Ok(Self::Type),
            "tags"          => Ok(Self::Tags),
            "last_released" => Ok(Self::LastReleased),
            _ => Err(()),
        }
    }
}

impl Display for SearchColumn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchColumn::Name          => write!(f, "name"),
            SearchColumn::ModId         => write!(f, "mod_id"),
            SearchColumn::AssetId       => write!(f, "asset_id"),
            SearchColumn::Downloads     => write!(f, "downloads"),
            SearchColumn::Follows       => write!(f, "follows"),
            SearchColumn::Trending      => write!(f, "trending"),
            SearchColumn::Comments      => write!(f, "comments"),
            SearchColumn::Summary       => write!(f, "summary"),
            SearchColumn::ModidStrs     => write!(f, "modid_strs"),
            SearchColumn::Author        => write!(f, "author"),
            SearchColumn::UrlAlias      => write!(f, "url_alias"),
            SearchColumn::Side          => write!(f, "side"),
            SearchColumn::Type          => write!(f, "type"),
            SearchColumn::Tags          => write!(f, "tags"),
            SearchColumn::LastReleased  => write!(f, "last_released"),
        }
    }
}


#[derive(Deserialize, Clone, ValueEnum)]
#[derive(Debug)]
pub enum CellAttr {
    Bold,
    Italic,
    Underline,
    Reset,
    Dim,
    NoHidden,
}

impl CellAttr {
    #[allow(unused)]
    fn as_str(&self) -> &'static str {
        match self {
            Self::Bold      => "bold",
            Self::Italic    => "italic",
            Self::Underline => "underline",
            Self::Reset     => "reset",
            Self::Dim       => "dim",
            Self::NoHidden  => "nohidden",
        }
    }
}

impl FromStr for CellAttr {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bold"      => Ok(Self::Bold),
            "italic"    => Ok(Self::Italic),
            "underline" => Ok(Self::Underline),
            "reset"     => Ok(Self::Reset),
            "dim"       => Ok(Self::Dim),
            "nohidden"  => Ok(Self::NoHidden),
            _ => Err(()),
        }
    }
}

impl From<CellAttr> for Attribute {
    fn from(attr: CellAttr) -> Self {
        match attr {
            CellAttr::Bold      => Attribute::Bold,
            CellAttr::Dim       => Attribute::Dim,
            CellAttr::Italic    => Attribute::Italic,
            CellAttr::NoHidden  => Attribute::NoHidden,
            CellAttr::Reset     => Attribute::Reset,
            CellAttr::Underline => Attribute::Underlined,
        }
    }
}

impl Display for CellAttr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CellAttr::Bold      => write!(f, "bold"),
            CellAttr::Italic    => write!(f, "italic"),
            CellAttr::Underline => write!(f, "underline"),
            CellAttr::Reset     => write!(f, "reset"),
            CellAttr::Dim       => write!(f, "dim"),
            CellAttr::NoHidden  => write!(f, "nohidden"),
        }
    }
}

impl Serialize for CellAttr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Bold => serializer.serialize_str("bold"),
            Self::Italic => serializer.serialize_str("italic"),
            Self::Underline => serializer.serialize_str("underline"),
            Self::Reset => serializer.serialize_str("reset"),
            Self::Dim => serializer.serialize_str("dim"),
            Self::NoHidden => serializer.serialize_str("nohidden"),
        }
    }
}