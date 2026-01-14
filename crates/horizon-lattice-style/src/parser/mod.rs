//! CSS parsing module.

mod css_parser;
mod error;

pub use css_parser::parse_css;
pub use error::ParseError;
