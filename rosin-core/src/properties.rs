#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlignContent {
    Center,
    FlexEnd,
    FlexStart,
    SpaceAround,
    SpaceBetween,
    Stretch,
}

impl AlignContent {
    pub(crate) fn from_css_token(token: &str) -> Result<Self, ()> {
        match token {
            "stretch" => Ok(AlignContent::Stretch),
            "center" => Ok(AlignContent::Center),
            "flex-start" => Ok(AlignContent::FlexStart),
            "flex-end" => Ok(AlignContent::FlexEnd),
            "space-between" => Ok(AlignContent::SpaceBetween),
            "space-around" => Ok(AlignContent::SpaceAround),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlignItems {
    Stretch,
    Center,
    FlexStart,
    FlexEnd,
}

impl AlignItems {
    pub(crate) fn from_css_token(token: &str) -> Result<Self, ()> {
        match token {
            "stretch" => Ok(AlignItems::Stretch),
            "center" => Ok(AlignItems::Center),
            "flex-start" => Ok(AlignItems::FlexStart),
            "flex-end" => Ok(AlignItems::FlexEnd),
            _ => Err(()),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
pub enum Cursor {
    Default,
    None,
    ContextMenu,
    Help,
    Pointer,
    Progress,
    Wait,
    Cell,
    Crosshair,
    Text,
    VerticalText,
    Alias,
    Copy,
    Move,
    NoDrop,
    NotAllowed,
    Grab,
    Grabbing,
    E_Resize,
    N_Resize,
    NE_Resize,
    NW_Resize,
    S_Resize,
    SE_Resize,
    SW_Resize,
    W_Resize,
    WE_Resize,
    NS_Resize,
    NESW_Resize,
    NWSE_Resize,
    ColResize,
    RowResize,
    AllScroll,
    ZoomIn,
    ZoomOut,
}

impl Cursor {
    pub(crate) fn from_css_token(token: &str) -> Result<Self, ()> {
        match token {
            "default" => Ok(Cursor::Default),
            "none" => Ok(Cursor::None),
            "context-menu" => Ok(Cursor::ContextMenu),
            "help" => Ok(Cursor::Help),
            "pointer" => Ok(Cursor::Pointer),
            "progress" => Ok(Cursor::Progress),
            "wait" => Ok(Cursor::Wait),
            "cell" => Ok(Cursor::Cell),
            "crosshair" => Ok(Cursor::Crosshair),
            "text" => Ok(Cursor::Text),
            "vertical-text" => Ok(Cursor::VerticalText),
            "alias" => Ok(Cursor::Alias),
            "copy" => Ok(Cursor::Copy),
            "move" => Ok(Cursor::Move),
            "no-drop" => Ok(Cursor::NoDrop),
            "not-allowed" => Ok(Cursor::NotAllowed),
            "grab" => Ok(Cursor::Grab),
            "grabbing" => Ok(Cursor::Grabbing),
            "e-resize" => Ok(Cursor::E_Resize),
            "n-resize" => Ok(Cursor::N_Resize),
            "ne-resize" => Ok(Cursor::NE_Resize),
            "nw-resize" => Ok(Cursor::NW_Resize),
            "s-resize" => Ok(Cursor::S_Resize),
            "se-resize" => Ok(Cursor::SE_Resize),
            "sw-resize" => Ok(Cursor::SW_Resize),
            "w-resize" => Ok(Cursor::W_Resize),
            "we-resize" => Ok(Cursor::WE_Resize),
            "ns-resize" => Ok(Cursor::NS_Resize),
            "nesw-resize" => Ok(Cursor::NESW_Resize),
            "nwse-resize" => Ok(Cursor::NWSE_Resize),
            "col-resize" => Ok(Cursor::ColResize),
            "row-resize" => Ok(Cursor::RowResize),
            "all-scroll" => Ok(Cursor::AllScroll),
            "zoom-in" => Ok(Cursor::ZoomIn),
            "zoom-out" => Ok(Cursor::ZoomOut),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FlexDirection {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

impl FlexDirection {
    pub fn is_row(&self) -> bool {
        match self {
            FlexDirection::Row | FlexDirection::RowReverse => true,
            FlexDirection::Column | FlexDirection::ColumnReverse => false,
        }
    }

    pub fn is_reverse(&self) -> bool {
        match self {
            FlexDirection::RowReverse | FlexDirection::ColumnReverse => true,
            FlexDirection::Row | FlexDirection::Column => false,
        }
    }

    pub(crate) fn from_css_token(token: &str) -> Result<Self, ()> {
        match token {
            "row" => Ok(FlexDirection::Row),
            "row-reverse" => Ok(FlexDirection::RowReverse),
            "column" => Ok(FlexDirection::Column),
            "column-reverse" => Ok(FlexDirection::ColumnReverse),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexWrap {
    NoWrap,
    Wrap,
    WrapReverse,
}

impl FlexWrap {
    pub(crate) fn from_css_token(token: &str) -> Result<Self, ()> {
        match token {
            "no-wrap" => Ok(FlexWrap::NoWrap),
            "wrap" => Ok(FlexWrap::Wrap),
            "wrap-reverse" => Ok(FlexWrap::WrapReverse),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum JustifyContent {
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

impl JustifyContent {
    pub(crate) fn from_css_token(token: &str) -> Result<Self, ()> {
        match token {
            "flex-start" => Ok(JustifyContent::FlexStart),
            "flex-end" => Ok(JustifyContent::FlexEnd),
            "center" => Ok(JustifyContent::Center),
            "space-between" => Ok(JustifyContent::SpaceBetween),
            "space-around" => Ok(JustifyContent::SpaceAround),
            "space-evenly" => Ok(JustifyContent::SpaceEvenly),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Position {
    Static,
    Relative,
    Fixed,
}

impl Position {
    pub(crate) fn from_css_token(token: &str) -> Result<Self, ()> {
        match token {
            "static" => Ok(Position::Static),
            "relative" => Ok(Position::Relative),
            "fixed" => Ok(Position::Fixed),
            _ => Err(()),
        }
    }
}
