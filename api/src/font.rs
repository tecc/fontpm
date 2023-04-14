use std::str::FromStr;
use crate::Error;

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
pub enum DefinedFontWeight {
    Fixed(u32),
    Variable
}
impl DefinedFontWeight {
    pub const REGULAR: Self = DefinedFontWeight::Fixed(400);
    pub fn is_covered_by(&self, other: &DefinedFontWeight) -> bool {
        match other {
            DefinedFontWeight::Fixed(other) => match self {
                DefinedFontWeight::Fixed(me) => *other == *me,
                _ => false
            },
            DefinedFontWeight::Variable => *self == DefinedFontWeight::Variable
        }
    }
    pub fn is_fixed(&self) -> bool {
        match self {
            DefinedFontWeight::Fixed(_) => true,
            DefinedFontWeight::Variable => false
        }
    }
}
#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
pub enum FontWeight {
    Defined(DefinedFontWeight),
    AllFixed, // All available fixed variants
    All, // All weights (incl. fixed *and* variable)
}
impl FontWeight {
    pub fn is_covered_by(&self, other: &FontWeight) -> bool {
        match other {
            FontWeight::All => true, // all weights are covered
            FontWeight::AllFixed => match self {
                // only fixed-weight fonts are downloaded for AllFixed, not Variable
                FontWeight::AllFixed | FontWeight::Defined(DefinedFontWeight::Fixed(_)) => true,
                _ => false
            },
            FontWeight::Defined(other_defined) => match self {
                FontWeight::Defined(self_defined) => self_defined.is_covered_by(other_defined),
                _ => false
            }
        }
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
pub enum DefinedFontStyle {
    Regular,
    Italic
}

impl DefinedFontStyle {
    pub fn is_covered_by(&self, other: &DefinedFontStyle) -> bool {
        // NOTE(tecc): this only exists to have a somewhat consistent API
        self.eq(other)
    }
}
impl FromStr for DefinedFontStyle {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(Self::Regular)
        }
        match s {
            "regular" | "normal" => Ok(Self::Regular),
            "italic" => Ok(Self::Italic),
            _ => Err(Error::Deserialisation(format!("No such font style: {}", s)))
        }
    }
}
impl AsRef<str> for DefinedFontStyle {
    fn as_ref(&self) -> &str {
        match self {
            Self::Regular => "regular",
            Self::Italic => "italic"
        }
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
pub enum FontStyle {
    Defined(DefinedFontStyle),
    All
}

impl FontStyle {
    pub fn is_covered_by(&self, other: &FontStyle) -> bool {
        match other {
            FontStyle::All => true,
            FontStyle::Defined(other_defined) => match self {
                FontStyle::Defined(self_defined) => self_defined.is_covered_by(other_defined),
                _ => false
            }
        }
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
pub struct DefinedFontVariantSpec {
    pub weight: DefinedFontWeight,
    pub style: DefinedFontStyle
}

impl DefinedFontVariantSpec {
    pub const REGULAR: Self = DefinedFontVariantSpec {
        weight: DefinedFontWeight::REGULAR,
        style: DefinedFontStyle::Regular
    };
    pub fn is_covered_by(&self, other: &DefinedFontVariantSpec) -> bool {
        let weight_is_covered = self.weight.is_covered_by(&other.weight);
        let style_is_covered = self.style.is_covered_by(&other.style);
        weight_is_covered && style_is_covered
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Hash)]
pub struct FontVariantSpec {
    pub weight: FontWeight,
    pub style: FontStyle
}

impl FontVariantSpec {
    pub fn is_covered_by(&self, other: &FontVariantSpec) -> bool {
        let weight_is_covered = self.weight.is_covered_by(&other.weight);
        let style_is_covered = self.style.is_covered_by(&other.style);
        weight_is_covered && style_is_covered
    }
}

#[derive(Eq, PartialEq, Clone, Debug, Hash)]
pub struct DefinedFontInstallSpec {
    pub id: String,
    pub styles: Vec<DefinedFontVariantSpec>
}
impl DefinedFontInstallSpec {
    pub fn new<S, I, F>(id: S, styles: I) -> Self where S: ToString, I: IntoIterator<Item = F>, F: Into<DefinedFontVariantSpec> {
        Self {
            id: id.to_string(),
            styles: styles.into_iter().map(|v| v.into()).collect()
        }
    }
}

#[derive(Eq, PartialEq, Clone, Debug, Hash)]
pub struct FontInstallSpec {
    pub id: String,
    pub styles: Vec<FontVariantSpec>
}

impl FontInstallSpec {
    pub fn new<S, I, F>(id: S, styles: I) -> Self where S: ToString, I: IntoIterator<Item = F>, F: Into<FontVariantSpec> {
        Self {
            id: id.to_string(),
            styles: styles.into_iter().map(|v| v.into()).collect()
        }
    }
    pub fn new_all_styles<S>(id: S) -> Self where S: ToString {
        Self::new(id, vec![FontVariantSpec { style: FontStyle::All, weight: FontWeight::All }])
    }
}