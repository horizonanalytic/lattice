//! CSS syntax parser using the `cssparser` crate.
//!
//! This module contains the core parsing logic for CSS stylesheets. The parser
//! tokenizes CSS input and constructs [`StyleRule`] objects containing selectors
//! and their associated style properties.

use crate::rules::StyleRule;
use crate::selector::{Combinator, NthExpr, PseudoClass, Selector, SelectorPart, TypeSelector};
use crate::style::StyleProperties;
use crate::types::{BorderStyle, Cursor, EdgeValues, LengthValue, StyleValue, TextAlign};
use crate::{Error, Result};
use cssparser::{ParseError as CssParseError, Parser, ParserInput, Token};
use horizon_lattice_render::{
    BoxShadow, Color, CornerRadii,
    text::{FontFamily, FontStyle, FontWeight},
};

/// Parse a CSS stylesheet string into a list of style rules.
///
/// This function tokenizes and parses the provided CSS string, extracting all
/// valid style rules. Rules that fail to parse are skipped with a warning logged.
///
/// # Arguments
///
/// * `css` - A string slice containing CSS stylesheet content.
///
/// # Returns
///
/// Returns `Ok(Vec<StyleRule>)` containing all successfully parsed rules.
/// The rules are ordered by their appearance in the source, with each rule
/// assigned an incrementing order value for specificity calculations.
///
/// Returns `Err` only for catastrophic errors (currently always returns Ok
/// due to error recovery).
///
/// # Error Recovery
///
/// Parse errors in individual rules do not cause the entire parse to fail.
/// Instead, the parser:
/// 1. Logs the error via `tracing::warn!`
/// 2. Skips to the next rule (after the closing `}`)
/// 3. Continues parsing subsequent rules
///
/// # Example
///
/// ```ignore
/// let css = "Button { color: red; } Label { color: blue; }";
/// let rules = parse_css(css)?;
/// assert_eq!(rules.len(), 2);
/// ```
pub fn parse_css(css: &str) -> Result<Vec<StyleRule>> {
    let mut input = ParserInput::new(css);
    let mut parser = Parser::new(&mut input);
    let mut rules = vec![];
    let mut order = 0u32;

    loop {
        // Skip whitespace and comments
        parser.skip_whitespace();

        if parser.is_exhausted() {
            break;
        }

        match parse_rule(&mut parser, order) {
            Ok(rule) => {
                rules.push(rule);
                order += 1;
            }
            Err(e) => {
                tracing::warn!("CSS parse error: {}", e);
                // Try to recover by skipping to next rule
                skip_to_next_rule(&mut parser);
            }
        }
    }

    Ok(rules)
}

/// Parse a single CSS rule: selector { declarations }
fn parse_rule<'i>(parser: &mut Parser<'i, '_>, order: u32) -> Result<StyleRule> {
    // Parse selector until we hit the curly brace block
    let selector = parser
        .parse_until_before(cssparser::Delimiter::CurlyBracketBlock, |p| {
            parse_selector(p).map_err(|_| p.new_custom_error(()))
        })
        .map_err(|e: CssParseError<'_, ()>| {
            Error::parse(format!("Failed to parse selector: {:?}", e), 0, 0)
        })?;

    // Consume the curly bracket block - we need to get past the opening '{'
    // by consuming it as a CurlyBracketBlock token
    let properties = match parser.next() {
        Ok(Token::CurlyBracketBlock) => parser
            .parse_nested_block(|block_parser| parse_declarations(block_parser))
            .map_err(|e: CssParseError<'_, ()>| {
                Error::parse(format!("Failed to parse declaration block: {:?}", e), 0, 0)
            })?,
        _ => {
            return Err(Error::parse(
                "Expected '{' after selector".to_string(),
                0,
                0,
            ));
        }
    };

    Ok(StyleRule::new(selector, properties, order))
}

/// Parse a CSS selector.
fn parse_selector<'i>(parser: &mut Parser<'i, '_>) -> Result<Selector> {
    let mut parts = vec![];
    let mut combinators = vec![];
    let mut current_part = SelectorPart::default();

    // Skip leading whitespace only at the start
    parser.skip_whitespace();

    loop {
        let token = match parser.next() {
            Ok(t) => t.clone(),
            Err(_) => break,
        };

        match &token {
            Token::Ident(name) => {
                // Type selector - but check if this should start a new part (descendant combinator)
                if current_part.type_selector.is_none()
                    && current_part.id.is_none()
                    && current_part.classes.is_empty()
                {
                    current_part.type_selector = Some(TypeSelector::Type(name.to_string()));
                } else if !is_empty_part(&current_part) {
                    // We have a current part and got a new type selector - descendant combinator
                    parts.push(current_part);
                    combinators.push(Combinator::Descendant);
                    current_part = SelectorPart::default();
                    current_part.type_selector = Some(TypeSelector::Type(name.to_string()));
                } else {
                    return Err(Error::invalid_selector(
                        format!("{}", name),
                        "Unexpected identifier",
                    ));
                }
            }

            Token::Delim('*') => {
                // Universal selector
                if current_part.type_selector.is_none() {
                    current_part.type_selector = Some(TypeSelector::Universal);
                }
            }

            Token::Delim('.') => {
                // Class selector
                let class = parser
                    .expect_ident()
                    .map_err(|_| Error::invalid_selector(".", "Expected class name after '.'"))?;
                current_part.classes.push(class.to_string());
            }

            Token::IDHash(id) => {
                // ID selector - might start a new part if we already have content
                if !is_empty_part(&current_part) && current_part.id.is_none() {
                    // Add ID to current part (e.g., Button#submit)
                    current_part.id = Some(id.to_string());
                } else if is_empty_part(&current_part) {
                    // Start new part with just ID
                    current_part.id = Some(id.to_string());
                } else {
                    // We have a part with an ID already - descendant combinator
                    parts.push(current_part);
                    combinators.push(Combinator::Descendant);
                    current_part = SelectorPart::default();
                    current_part.id = Some(id.to_string());
                }
            }

            Token::Colon => {
                // Pseudo-class
                let pseudo_name = parser.expect_ident().map_err(|_| {
                    Error::invalid_selector(":", "Expected pseudo-class name after ':'")
                })?;

                let pseudo = match pseudo_name.as_ref() {
                    "hover" => PseudoClass::Hover,
                    "pressed" | "active" => PseudoClass::Pressed,
                    "focused" | "focus" => PseudoClass::Focused,
                    "disabled" => PseudoClass::Disabled,
                    "enabled" => PseudoClass::Enabled,
                    "checked" => PseudoClass::Checked,
                    "unchecked" => PseudoClass::Unchecked,
                    "first-child" => PseudoClass::FirstChild,
                    "last-child" => PseudoClass::LastChild,
                    "only-child" => PseudoClass::OnlyChild,
                    "empty" => PseudoClass::Empty,
                    "nth-child" => {
                        // Parse nth-child expression
                        let expr = parser.parse_nested_block(|p| parse_nth_expr(p)).map_err(
                            |_: CssParseError<'_, ()>| {
                                Error::invalid_selector(
                                    ":nth-child",
                                    "Invalid nth-child expression",
                                )
                            },
                        )?;
                        PseudoClass::NthChild(expr)
                    }
                    "not" => {
                        // Parse :not() argument
                        let inner = parser
                            .parse_nested_block(|p| parse_simple_selector(p))
                            .map_err(|_: CssParseError<'_, ()>| {
                                Error::invalid_selector(":not", "Invalid :not() argument")
                            })?;
                        PseudoClass::Not(Box::new(inner))
                    }
                    _ => {
                        return Err(Error::invalid_selector(
                            format!(":{}", pseudo_name),
                            "Unknown pseudo-class",
                        ));
                    }
                };
                current_part.pseudo_classes.push(pseudo);
            }

            Token::Delim('>') => {
                // Child combinator
                if !is_empty_part(&current_part) {
                    parts.push(current_part);
                    combinators.push(Combinator::Child);
                    current_part = SelectorPart::default();
                }
            }

            Token::Delim('+') => {
                // Adjacent sibling combinator
                if !is_empty_part(&current_part) {
                    parts.push(current_part);
                    combinators.push(Combinator::AdjacentSibling);
                    current_part = SelectorPart::default();
                }
            }

            Token::Delim('~') => {
                // General sibling combinator
                if !is_empty_part(&current_part) {
                    parts.push(current_part);
                    combinators.push(Combinator::GeneralSibling);
                    current_part = SelectorPart::default();
                }
            }

            Token::CurlyBracketBlock => {
                // End of selector, push back
                // We can't actually push back the token, but we'll break
                break;
            }

            _ => {
                // Unknown token, end selector parsing
                break;
            }
        }
    }

    // Add final part
    if !is_empty_part(&current_part) {
        parts.push(current_part);
    }

    if parts.is_empty() {
        return Err(Error::invalid_selector("", "Empty selector"));
    }

    Ok(Selector { parts, combinators })
}

fn is_empty_part(part: &SelectorPart) -> bool {
    part.type_selector.is_none()
        && part.id.is_none()
        && part.classes.is_empty()
        && part.pseudo_classes.is_empty()
}

/// Parse a simple selector (for :not() argument).
fn parse_simple_selector<'i>(
    parser: &mut Parser<'i, '_>,
) -> std::result::Result<SelectorPart, CssParseError<'i, ()>> {
    let mut part = SelectorPart::default();

    parser.skip_whitespace();

    while let Ok(token) = parser.next() {
        match token.clone() {
            Token::Ident(name) => {
                part.type_selector = Some(TypeSelector::Type(name.to_string()));
            }
            Token::Delim('*') => {
                part.type_selector = Some(TypeSelector::Universal);
            }
            Token::Delim('.') => {
                let class = parser.expect_ident()?;
                part.classes.push(class.to_string());
            }
            Token::IDHash(id) => {
                part.id = Some(id.to_string());
            }
            _ => break,
        }
    }

    Ok(part)
}

/// Parse nth-child expression (e.g., "odd", "even", "3", "2n+1").
fn parse_nth_expr<'i>(
    parser: &mut Parser<'i, '_>,
) -> std::result::Result<NthExpr, CssParseError<'i, ()>> {
    parser.skip_whitespace();

    if let Ok(token) = parser.next() {
        match token.clone() {
            Token::Ident(name) => match name.as_ref() {
                "odd" => return Ok(NthExpr::odd()),
                "even" => return Ok(NthExpr::even()),
                _ => {}
            },
            Token::Number {
                int_value: Some(n), ..
            } => {
                return Ok(NthExpr::new(0, n));
            }
            Token::Dimension {
                int_value: Some(a),
                unit,
                ..
            } if unit.eq_ignore_ascii_case("n") => {
                // Check for +B or -B
                parser.skip_whitespace();
                let b = if let Ok(Token::Number {
                    int_value: Some(b), ..
                }) = parser.next()
                {
                    *b
                } else {
                    0
                };
                return Ok(NthExpr::new(a, b));
            }
            _ => {}
        }
    }

    // Default to n (matches all)
    Ok(NthExpr::all())
}

/// Parse CSS declarations.
fn parse_declarations<'i>(
    parser: &mut Parser<'i, '_>,
) -> std::result::Result<StyleProperties, CssParseError<'i, ()>> {
    let mut props = StyleProperties::default();

    loop {
        parser.skip_whitespace();

        if parser.is_exhausted() {
            break;
        }

        // Try to parse property name
        let property_name = match parser.expect_ident() {
            Ok(name) => name.to_string(),
            Err(_) => break,
        };

        // Expect colon
        if parser.expect_colon().is_err() {
            skip_declaration(parser);
            continue;
        }

        // Parse property value
        if let Err(e) = parse_property_value(parser, &property_name, &mut props) {
            tracing::warn!("Failed to parse property '{}': {:?}", property_name, e);
            skip_declaration(parser);
            continue;
        }

        // Skip optional semicolon
        let _ = parser.try_parse(|p| p.expect_semicolon());
    }

    Ok(props)
}

/// Parse a single property value.
fn parse_property_value<'i>(
    parser: &mut Parser<'i, '_>,
    name: &str,
    props: &mut StyleProperties,
) -> std::result::Result<(), CssParseError<'i, ()>> {
    parser.skip_whitespace();

    // Capture state before reading token so we can reset if needed
    let state = parser.state();

    // Check for special keywords first
    if let Ok(Token::Ident(ident)) = parser.next() {
        match ident.as_ref() {
            "inherit" => {
                set_inherit(name, props);
                return Ok(());
            }
            "initial" => {
                set_initial(name, props);
                return Ok(());
            }
            "unset" => {
                set_unset(name, props);
                return Ok(());
            }
            _ => {
                // Not a special keyword, reset to parse the actual value
            }
        }
    }

    // Reset and parse properly
    parser.reset(&state);

    match name {
        // === Box Model ===
        "margin" => {
            props.margin = StyleValue::Set(parse_edge_values(parser)?);
        }
        "margin-top" => update_edge(&mut props.margin, |e| {
            e.top = parse_length(parser).unwrap_or_default()
        }),
        "margin-right" => update_edge(&mut props.margin, |e| {
            e.right = parse_length(parser).unwrap_or_default()
        }),
        "margin-bottom" => update_edge(&mut props.margin, |e| {
            e.bottom = parse_length(parser).unwrap_or_default()
        }),
        "margin-left" => update_edge(&mut props.margin, |e| {
            e.left = parse_length(parser).unwrap_or_default()
        }),

        "padding" => {
            props.padding = StyleValue::Set(parse_edge_values(parser)?);
        }
        "padding-top" => update_edge(&mut props.padding, |e| {
            e.top = parse_length(parser).unwrap_or_default()
        }),
        "padding-right" => update_edge(&mut props.padding, |e| {
            e.right = parse_length(parser).unwrap_or_default()
        }),
        "padding-bottom" => update_edge(&mut props.padding, |e| {
            e.bottom = parse_length(parser).unwrap_or_default()
        }),
        "padding-left" => update_edge(&mut props.padding, |e| {
            e.left = parse_length(parser).unwrap_or_default()
        }),

        "border-width" => {
            props.border_width = StyleValue::Set(parse_edge_values(parser)?);
        }
        "border-color" => {
            props.border_color = StyleValue::Set(parse_color(parser)?);
        }
        "border-style" => {
            if let Ok(Token::Ident(s)) = parser.next()
                && let Some(style) = BorderStyle::from_css(s) {
                    props.border_style = StyleValue::Set(style);
                }
        }
        "border-radius" => {
            props.border_radius = StyleValue::Set(parse_border_radius(parser)?);
        }

        // === Background ===
        "background-color" => {
            props.background_color = StyleValue::Set(parse_color(parser)?);
        }
        "background" => {
            // For now, just parse as color
            props.background_color = StyleValue::Set(parse_color(parser)?);
        }

        // === Typography ===
        "color" => {
            props.color = StyleValue::Set(parse_color(parser)?);
        }
        "font-size" => {
            props.font_size = StyleValue::Set(parse_length(parser)?);
        }
        "font-weight" => {
            props.font_weight = StyleValue::Set(parse_font_weight(parser)?);
        }
        "font-style" => {
            props.font_style = StyleValue::Set(parse_font_style(parser)?);
        }
        "font-family" => {
            props.font_family = StyleValue::Set(parse_font_family(parser)?);
        }
        "text-align" => {
            if let Ok(Token::Ident(s)) = parser.next()
                && let Some(align) = TextAlign::from_css(s) {
                    props.text_align = StyleValue::Set(align);
                }
        }
        "line-height" => {
            if let Ok(Token::Number { value, .. }) = parser.next() {
                props.line_height = StyleValue::Set(*value);
            }
        }

        // === Effects ===
        "opacity" => {
            if let Ok(Token::Number { value, .. }) = parser.next() {
                props.opacity = StyleValue::Set(value.clamp(0.0, 1.0));
            }
        }
        "box-shadow" => {
            if let Ok(shadow) = parse_box_shadow(parser) {
                props.box_shadow = StyleValue::Set(vec![shadow]);
            }
        }

        // === Interaction ===
        "cursor" => {
            if let Ok(Token::Ident(s)) = parser.next()
                && let Some(cursor) = Cursor::from_css(s) {
                    props.cursor = StyleValue::Set(cursor);
                }
        }
        "pointer-events" => {
            if let Ok(Token::Ident(s)) = parser.next() {
                match s.as_ref() {
                    "auto" | "all" => props.pointer_events = StyleValue::Set(true),
                    "none" => props.pointer_events = StyleValue::Set(false),
                    _ => {}
                }
            }
        }

        // === Size ===
        "width" => {
            props.width = StyleValue::Set(parse_length(parser)?);
        }
        "height" => {
            props.height = StyleValue::Set(parse_length(parser)?);
        }
        "min-width" => {
            props.min_width = StyleValue::Set(parse_length(parser)?);
        }
        "min-height" => {
            props.min_height = StyleValue::Set(parse_length(parser)?);
        }
        "max-width" => {
            props.max_width = StyleValue::Set(parse_length(parser)?);
        }
        "max-height" => {
            props.max_height = StyleValue::Set(parse_length(parser)?);
        }

        _ => {
            tracing::debug!("Unknown CSS property: {}", name);
        }
    }

    Ok(())
}

fn update_edge<F: FnOnce(&mut EdgeValues)>(value: &mut StyleValue<EdgeValues>, f: F) {
    let mut edges = value.as_set().cloned().unwrap_or_default();
    f(&mut edges);
    *value = StyleValue::Set(edges);
}

fn set_inherit(name: &str, props: &mut StyleProperties) {
    match name {
        "color" => props.color = StyleValue::Inherit,
        "font-size" => props.font_size = StyleValue::Inherit,
        "font-weight" => props.font_weight = StyleValue::Inherit,
        "font-style" => props.font_style = StyleValue::Inherit,
        "font-family" => props.font_family = StyleValue::Inherit,
        "text-align" => props.text_align = StyleValue::Inherit,
        "line-height" => props.line_height = StyleValue::Inherit,
        "cursor" => props.cursor = StyleValue::Inherit,
        _ => {}
    }
}

fn set_initial(name: &str, props: &mut StyleProperties) {
    match name {
        "color" => props.color = StyleValue::Initial,
        "font-size" => props.font_size = StyleValue::Initial,
        "margin" => props.margin = StyleValue::Initial,
        "padding" => props.padding = StyleValue::Initial,
        "background-color" => props.background_color = StyleValue::Initial,
        _ => {}
    }
}

fn set_unset(name: &str, props: &mut StyleProperties) {
    match name {
        "color" => props.color = StyleValue::Unset,
        "font-size" => props.font_size = StyleValue::Unset,
        _ => {}
    }
}

/// Parse a length value.
fn parse_length<'i>(
    parser: &mut Parser<'i, '_>,
) -> std::result::Result<LengthValue, CssParseError<'i, ()>> {
    parser.skip_whitespace();

    let token = parser.next()?;

    #[allow(clippy::redundant_guards)] // CSS `0` should be zero length regardless of unit
    match token.clone() {
        Token::Number { value, .. } if value == 0.0 => Ok(LengthValue::Zero),
        Token::Dimension { value, unit, .. } => {
            match unit.as_ref() {
                "px" => Ok(LengthValue::Px(value)),
                "em" => Ok(LengthValue::Em(value)),
                "rem" => Ok(LengthValue::Rem(value)),
                "%" => Ok(LengthValue::Percent(value)),
                _ => Ok(LengthValue::Px(value)), // Default to px
            }
        }
        Token::Percentage { unit_value, .. } => Ok(LengthValue::Percent(unit_value * 100.0)),
        Token::Ident(s) if s.eq_ignore_ascii_case("auto") => Ok(LengthValue::Auto),
        _ => Err(parser.new_custom_error(())),
    }
}

/// Parse edge values (1-4 values for margin/padding shorthand).
fn parse_edge_values<'i>(
    parser: &mut Parser<'i, '_>,
) -> std::result::Result<EdgeValues, CssParseError<'i, ()>> {
    let mut values = vec![];

    while values.len() < 4 {
        parser.skip_whitespace();

        if parser.is_exhausted() {
            break;
        }

        // Check for end of values (semicolon or block end)
        let state = parser.state();
        if let Ok(Token::Semicolon) | Ok(Token::CloseCurlyBracket) = parser.next() {
            parser.reset(&state);
            break;
        }
        parser.reset(&state);

        match parse_length(parser) {
            Ok(len) => values.push(len),
            Err(_) => break,
        }
    }

    match values.len() {
        1 => Ok(EdgeValues::uniform(values[0])),
        2 => Ok(EdgeValues::symmetric(values[0], values[1])),
        3 => Ok(EdgeValues::new(values[0], values[1], values[2], values[1])),
        4 => Ok(EdgeValues::new(values[0], values[1], values[2], values[3])),
        _ => Ok(EdgeValues::default()),
    }
}

/// Parse a color value.
fn parse_color<'i>(
    parser: &mut Parser<'i, '_>,
) -> std::result::Result<Color, CssParseError<'i, ()>> {
    parser.skip_whitespace();

    let token = parser.next()?;

    match token.clone() {
        Token::Hash(hash) | Token::IDHash(hash) => {
            let hex_str = format!("#{}", hash);
            Color::from_hex(&hex_str).ok_or_else(|| parser.new_custom_error(()))
        }
        Token::Ident(name) => {
            // Named colors
            match name.as_ref().to_lowercase().as_str() {
                "transparent" => Ok(Color::TRANSPARENT),
                "black" => Ok(Color::BLACK),
                "white" => Ok(Color::WHITE),
                "red" => Ok(Color::RED),
                "green" => Ok(Color::GREEN),
                "blue" => Ok(Color::BLUE),
                "yellow" => Ok(Color::YELLOW),
                "cyan" => Ok(Color::CYAN),
                "magenta" => Ok(Color::MAGENTA),
                "gray" | "grey" => Ok(Color::GRAY),
                _ => Err(parser.new_custom_error(())),
            }
        }
        Token::Function(name)
            if name.eq_ignore_ascii_case("rgb") || name.eq_ignore_ascii_case("rgba") =>
        {
            // Parse rgb(r, g, b) or rgba(r, g, b, a)
            let (r, g, b, a) = parser.parse_nested_block(|p| {
                let r = parse_color_component(p)?;
                p.expect_comma()?;
                let g = parse_color_component(p)?;
                p.expect_comma()?;
                let b = parse_color_component(p)?;
                let a = if p.try_parse(|p| p.expect_comma()).is_ok() {
                    parse_alpha_component(p)?
                } else {
                    1.0
                };
                Ok::<_, CssParseError<'_, ()>>((r, g, b, a))
            })?;
            Ok(Color::from_rgba(r, g, b, a))
        }
        _ => Err(parser.new_custom_error(())),
    }
}

fn parse_color_component<'i>(
    parser: &mut Parser<'i, '_>,
) -> std::result::Result<f32, CssParseError<'i, ()>> {
    parser.skip_whitespace();
    match parser.next()? {
        Token::Number { value, .. } => Ok(*value / 255.0),
        Token::Percentage { unit_value, .. } => Ok(*unit_value),
        _ => Err(parser.new_custom_error(())),
    }
}

fn parse_alpha_component<'i>(
    parser: &mut Parser<'i, '_>,
) -> std::result::Result<f32, CssParseError<'i, ()>> {
    parser.skip_whitespace();
    match parser.next()? {
        Token::Number { value, .. } => Ok(value.clamp(0.0, 1.0)),
        Token::Percentage { unit_value, .. } => Ok(*unit_value),
        _ => Err(parser.new_custom_error(())),
    }
}

/// Parse border-radius (1-4 values).
fn parse_border_radius<'i>(
    parser: &mut Parser<'i, '_>,
) -> std::result::Result<CornerRadii, CssParseError<'i, ()>> {
    let mut values = vec![];

    while values.len() < 4 {
        parser.skip_whitespace();

        if parser.is_exhausted() {
            break;
        }

        let state = parser.state();
        if let Ok(Token::Semicolon) | Ok(Token::CloseCurlyBracket) = parser.next() {
            parser.reset(&state);
            break;
        }
        parser.reset(&state);

        match parse_length(parser) {
            Ok(LengthValue::Px(v)) => values.push(v),
            Ok(LengthValue::Zero) => values.push(0.0),
            _ => break,
        }
    }

    match values.len() {
        1 => Ok(CornerRadii::uniform(values[0])),
        2 => Ok(CornerRadii {
            top_left: values[0],
            top_right: values[1],
            bottom_right: values[0],
            bottom_left: values[1],
        }),
        3 => Ok(CornerRadii {
            top_left: values[0],
            top_right: values[1],
            bottom_right: values[2],
            bottom_left: values[1],
        }),
        4 => Ok(CornerRadii {
            top_left: values[0],
            top_right: values[1],
            bottom_right: values[2],
            bottom_left: values[3],
        }),
        _ => Ok(CornerRadii::uniform(0.0)),
    }
}

/// Parse font-weight.
fn parse_font_weight<'i>(
    parser: &mut Parser<'i, '_>,
) -> std::result::Result<FontWeight, CssParseError<'i, ()>> {
    parser.skip_whitespace();

    match parser.next()? {
        Token::Number {
            int_value: Some(n), ..
        } => Ok(FontWeight::new(*n as u16)),
        Token::Ident(name) => match name.as_ref().to_lowercase().as_str() {
            "normal" => Ok(FontWeight::NORMAL),
            "bold" => Ok(FontWeight::BOLD),
            "lighter" => Ok(FontWeight::LIGHT),
            "bolder" => Ok(FontWeight::BOLD),
            "thin" => Ok(FontWeight::THIN),
            "light" => Ok(FontWeight::LIGHT),
            "medium" => Ok(FontWeight::MEDIUM),
            "semibold" | "semi-bold" => Ok(FontWeight::SEMI_BOLD),
            "extrabold" | "extra-bold" => Ok(FontWeight::EXTRA_BOLD),
            "black" => Ok(FontWeight::BLACK),
            _ => Ok(FontWeight::NORMAL),
        },
        _ => Err(parser.new_custom_error(())),
    }
}

/// Parse font-style.
fn parse_font_style<'i>(
    parser: &mut Parser<'i, '_>,
) -> std::result::Result<FontStyle, CssParseError<'i, ()>> {
    parser.skip_whitespace();

    if let Ok(Token::Ident(name)) = parser.next() {
        match name.as_ref().to_lowercase().as_str() {
            "normal" => return Ok(FontStyle::Normal),
            "italic" => return Ok(FontStyle::Italic),
            "oblique" => return Ok(FontStyle::Oblique),
            _ => {}
        }
    }

    Ok(FontStyle::Normal)
}

/// Parse font-family.
fn parse_font_family<'i>(
    parser: &mut Parser<'i, '_>,
) -> std::result::Result<Vec<FontFamily>, CssParseError<'i, ()>> {
    let mut families = vec![];

    loop {
        parser.skip_whitespace();

        if parser.is_exhausted() {
            break;
        }

        match parser.next()? {
            Token::Ident(name) => {
                let family = match name.as_ref().to_lowercase().as_str() {
                    "serif" => FontFamily::Serif,
                    "sans-serif" => FontFamily::SansSerif,
                    "monospace" => FontFamily::Monospace,
                    "cursive" => FontFamily::Cursive,
                    "fantasy" => FontFamily::Fantasy,
                    _ => FontFamily::Name(name.to_string()),
                };
                families.push(family);
            }
            Token::QuotedString(name) => {
                families.push(FontFamily::Name(name.to_string()));
            }
            Token::Comma => continue,
            _ => break,
        }
    }

    if families.is_empty() {
        families.push(FontFamily::SansSerif);
    }

    Ok(families)
}

/// Parse a box-shadow value.
fn parse_box_shadow<'i>(
    parser: &mut Parser<'i, '_>,
) -> std::result::Result<BoxShadow, CssParseError<'i, ()>> {
    parser.skip_whitespace();

    // Check for "none"
    let state = parser.state();
    if let Ok(Token::Ident(name)) = parser.next()
        && name.eq_ignore_ascii_case("none") {
            return Err(parser.new_custom_error(())); // No shadow
        }
    parser.reset(&state);

    let mut offset_x = 0.0;
    let mut offset_y = 0.0;
    let mut blur_radius = 0.0;
    let mut spread_radius = 0.0;
    let mut color = Color::from_rgba(0.0, 0.0, 0.0, 0.5);
    let mut inset = false;

    // Parse values
    let mut lengths_parsed = 0;

    loop {
        parser.skip_whitespace();

        if parser.is_exhausted() {
            break;
        }

        let state = parser.state();
        if let Ok(Token::Semicolon) | Ok(Token::CloseCurlyBracket) = parser.next() {
            parser.reset(&state);
            break;
        }
        parser.reset(&state);

        // Try to parse as length
        if let Ok(LengthValue::Px(v)) = parse_length(parser) {
            match lengths_parsed {
                0 => offset_x = v,
                1 => offset_y = v,
                2 => blur_radius = v,
                3 => spread_radius = v,
                _ => {}
            }
            lengths_parsed += 1;
            continue;
        }

        // Try to parse as color
        let state = parser.state();
        if let Ok(c) = parse_color(parser) {
            color = c;
            continue;
        }
        parser.reset(&state);

        // Try to parse "inset"
        if let Ok(Token::Ident(name)) = parser.next()
            && name.eq_ignore_ascii_case("inset") {
                inset = true;
                continue;
            }

        break;
    }

    Ok(BoxShadow {
        color,
        offset_x,
        offset_y,
        blur_radius,
        spread_radius,
        inset,
    })
}

/// Skip to the next rule (error recovery).
fn skip_to_next_rule(parser: &mut Parser<'_, '_>) {
    let mut depth = 0;
    loop {
        match parser.next() {
            Ok(Token::CurlyBracketBlock) => {
                depth += 1;
                if depth == 1 {
                    // Skip block contents
                    let _ = parser.parse_nested_block(|p| {
                        while !p.is_exhausted() {
                            let _ = p.next();
                        }
                        Ok::<_, CssParseError<'_, ()>>(())
                    });
                    return;
                }
            }
            Ok(Token::CloseCurlyBracket) => {
                if depth > 0 {
                    depth -= 1;
                }
                if depth == 0 {
                    return;
                }
            }
            Err(_) => return,
            _ => {}
        }
    }
}

/// Skip to the end of the current declaration (error recovery).
fn skip_declaration(parser: &mut Parser<'_, '_>) {
    loop {
        match parser.next() {
            Ok(Token::Semicolon) | Err(_) => return,
            Ok(Token::CloseCurlyBracket) => return,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_rule() {
        let css = "Button { color: red; }";
        let rules = parse_css(css).unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].selector.to_string(), "Button");
    }

    #[test]
    fn parse_class_selector() {
        let css = ".primary { background-color: blue; }";
        let rules = parse_css(css).unwrap();

        assert_eq!(rules.len(), 1);
        assert!(
            rules[0].selector.parts[0]
                .classes
                .contains(&"primary".to_string())
        );
    }

    #[test]
    fn parse_multiple_rules() {
        let css = r#"
            Button { color: red; }
            Label { color: blue; }
        "#;
        let rules = parse_css(css).unwrap();

        assert_eq!(rules.len(), 2);
    }

    #[test]
    fn parse_pseudo_class() {
        let css = "Button:hover { background-color: #ccc; }";
        let rules = parse_css(css).unwrap();

        assert_eq!(rules.len(), 1);
        assert!(
            rules[0].selector.parts[0]
                .pseudo_classes
                .contains(&PseudoClass::Hover)
        );
    }

    #[test]
    fn parse_descendant_selector() {
        let css = "Container Button { color: white; }";
        let rules = parse_css(css).unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].selector.parts.len(), 2);
        assert_eq!(rules[0].selector.combinators.len(), 1);
        assert_eq!(rules[0].selector.combinators[0], Combinator::Descendant);
    }

    #[test]
    fn parse_child_selector() {
        let css = "Container > Button { color: white; }";
        let rules = parse_css(css).unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].selector.combinators[0], Combinator::Child);
    }

    #[test]
    fn parse_edge_values_shorthand() {
        let css = "Button { margin: 10px 20px; }";
        let rules = parse_css(css).unwrap();

        if let StyleValue::Set(edges) = &rules[0].properties.margin {
            assert!(matches!(edges.top, LengthValue::Px(v) if v == 10.0));
            assert!(matches!(edges.right, LengthValue::Px(v) if v == 20.0));
            assert!(matches!(edges.bottom, LengthValue::Px(v) if v == 10.0));
            assert!(matches!(edges.left, LengthValue::Px(v) if v == 20.0));
        } else {
            panic!("margin should be set");
        }
    }

    #[test]
    fn parse_color_formats() {
        // Hex color
        let css = "Button { color: #ff0000; }";
        let rules = parse_css(css).unwrap();
        assert!(rules[0].properties.color.is_set());

        // Named color
        let css = "Button { color: red; }";
        let rules = parse_css(css).unwrap();
        assert!(rules[0].properties.color.is_set());
    }
}
