//! XML parsing, generation, and manipulation.
//!
//! This module provides a convenient API for working with XML data, including
//! parsing, serialization, path-based access, and streaming.
//!
//! # Parsing XML
//!
//! ```ignore
//! use horizon_lattice::file::xml_support::{parse_xml, read_xml};
//!
//! // Parse from string
//! let doc = parse_xml(r#"
//!     <book>
//!         <title>Rust Programming</title>
//!         <author>Jane Doe</author>
//!     </book>
//! "#)?;
//!
//! // Read from file
//! let doc = read_xml("data.xml")?;
//!
//! // Access elements using path notation
//! let title = doc.get("book/title")?.text();
//! ```
//!
//! # Generating XML
//!
//! ```ignore
//! use horizon_lattice::file::xml_support::{XmlDocument, XmlElement};
//!
//! // Build document programmatically
//! let mut doc = XmlDocument::new("catalog");
//! doc.root_mut().set_attribute("version", "1.0");
//!
//! let mut book = XmlElement::new("book");
//! book.set_attribute("id", "1");
//! book.add_child_text("title", "Rust Programming");
//! book.add_child_text("author", "Jane Doe");
//! doc.root_mut().add_child(book);
//!
//! // Serialize to string
//! let xml_str = doc.to_string();
//! let xml_pretty = doc.to_string_pretty();
//!
//! // Write to file
//! doc.save("output.xml")?;
//! ```
//!
//! # Path-Based Access
//!
//! ```ignore
//! use horizon_lattice::file::xml_support::parse_xml;
//!
//! let doc = parse_xml(r#"
//!     <bookstore>
//!         <book category="fiction">
//!             <title>The Great Gatsby</title>
//!             <price>10.99</price>
//!         </book>
//!         <book category="tech">
//!             <title>Rust in Action</title>
//!             <price>39.99</price>
//!         </book>
//!     </bookstore>
//! "#)?;
//!
//! // Simple path access (returns first match)
//! let title = doc.get("bookstore/book/title")?.text();
//!
//! // Get all matching elements
//! let books = doc.get_all("bookstore/book");
//!
//! // Access attributes
//! let category = doc.get("bookstore/book")?.attribute("category");
//! ```
//!
//! # Streaming Parser
//!
//! For large XML files, use the streaming API to avoid loading the entire
//! document into memory:
//!
//! ```ignore
//! use horizon_lattice::file::xml_support::{XmlReader, XmlEvent};
//!
//! let xml = r#"<items><item>A</item><item>B</item></items>"#;
//! let mut reader = XmlReader::from_str(xml);
//!
//! while let Some(event) = reader.next()? {
//!     match event {
//!         XmlEvent::StartElement { name, attributes } => {
//!             println!("Start: {}", name);
//!         }
//!         XmlEvent::Text(text) => {
//!             println!("Text: {}", text);
//!         }
//!         XmlEvent::EndElement { name } => {
//!             println!("End: {}", name);
//!         }
//!         _ => {}
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::fmt;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use quick_xml::events::{BytesCData, BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::error::{FileError, FileErrorKind, FileResult};
use super::operations::{atomic_write, read_text};

// ============================================================================
// XmlDocument
// ============================================================================

/// Represents an XML document with optional declaration and a root element.
#[derive(Debug, Clone, PartialEq)]
pub struct XmlDocument {
    /// XML version (default: "1.0")
    pub version: String,
    /// XML encoding (default: "UTF-8")
    pub encoding: Option<String>,
    /// Standalone declaration
    pub standalone: Option<bool>,
    /// The root element
    root: XmlElement,
}

impl XmlDocument {
    /// Creates a new XML document with the specified root element name.
    pub fn new(root_name: impl Into<String>) -> Self {
        XmlDocument {
            version: "1.0".to_string(),
            encoding: Some("UTF-8".to_string()),
            standalone: None,
            root: XmlElement::new(root_name),
        }
    }

    /// Creates a document with an existing root element.
    pub fn with_root(root: XmlElement) -> Self {
        XmlDocument {
            version: "1.0".to_string(),
            encoding: Some("UTF-8".to_string()),
            standalone: None,
            root,
        }
    }

    /// Returns a reference to the root element.
    pub fn root(&self) -> &XmlElement {
        &self.root
    }

    /// Returns a mutable reference to the root element.
    pub fn root_mut(&mut self) -> &mut XmlElement {
        &mut self.root
    }

    /// Sets the root element.
    pub fn set_root(&mut self, root: XmlElement) {
        self.root = root;
    }

    /// Gets an element at the specified path.
    ///
    /// Path uses forward slashes to separate element names.
    /// Example: "bookstore/book/title"
    pub fn get(&self, path: &str) -> Option<&XmlElement> {
        self.root.get(path)
    }

    /// Gets a mutable reference to an element at the specified path.
    pub fn get_mut(&mut self, path: &str) -> Option<&mut XmlElement> {
        self.root.get_mut(path)
    }

    /// Gets all elements matching the specified path.
    pub fn get_all(&self, path: &str) -> Vec<&XmlElement> {
        self.root.get_all(path)
    }

    /// Converts the document to an XML string (compact format).
    pub fn to_string(&self) -> String {
        let mut writer = Writer::new(Vec::new());
        self.write_to(&mut writer);
        String::from_utf8(writer.into_inner()).unwrap_or_default()
    }

    /// Converts the document to an XML string (pretty-printed with indentation).
    pub fn to_string_pretty(&self) -> String {
        let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
        self.write_to(&mut writer);
        String::from_utf8(writer.into_inner()).unwrap_or_default()
    }

    /// Converts the document to XML bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = Writer::new(Vec::new());
        self.write_to(&mut writer);
        writer.into_inner()
    }

    /// Converts the document to XML bytes (pretty-printed).
    pub fn to_bytes_pretty(&self) -> Vec<u8> {
        let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
        self.write_to(&mut writer);
        writer.into_inner()
    }

    /// Writes the document to a writer.
    fn write_to<W: Write>(&self, writer: &mut Writer<W>) {
        // Write XML declaration
        let encoding = self.encoding.as_deref();
        let standalone = self.standalone.map(|s| if s { "yes" } else { "no" });
        let decl = BytesDecl::new(&self.version, encoding, standalone);
        let _ = writer.write_event(Event::Decl(decl));

        // Write root element
        self.root.write_to(writer);
    }

    /// Saves the document to a file.
    pub fn save(&self, path: impl AsRef<Path>) -> FileResult<()> {
        let bytes = self.to_bytes();
        atomic_write(&path, |writer| writer.write_all(&bytes))
    }

    /// Saves the document to a file (pretty-printed).
    pub fn save_pretty(&self, path: impl AsRef<Path>) -> FileResult<()> {
        let bytes = self.to_bytes_pretty();
        atomic_write(&path, |writer| writer.write_all(&bytes))
    }
}

impl fmt::Display for XmlDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_pretty())
    }
}

// ============================================================================
// XmlElement
// ============================================================================

/// Represents an XML element with a name, attributes, and children.
#[derive(Debug, Clone, PartialEq)]
pub struct XmlElement {
    /// Element name (tag name)
    name: String,
    /// Namespace prefix (if any)
    namespace_prefix: Option<String>,
    /// Element attributes
    attributes: HashMap<String, String>,
    /// Child nodes (elements, text, comments, CDATA)
    children: Vec<XmlNode>,
}

impl XmlElement {
    /// Creates a new element with the specified name.
    pub fn new(name: impl Into<String>) -> Self {
        let name_str = name.into();
        let (prefix, local) = split_namespace(&name_str);
        XmlElement {
            name: local.to_string(),
            namespace_prefix: prefix.map(String::from),
            attributes: HashMap::new(),
            children: Vec::new(),
        }
    }

    /// Creates a new element with namespace prefix.
    pub fn with_namespace(prefix: impl Into<String>, name: impl Into<String>) -> Self {
        XmlElement {
            name: name.into(),
            namespace_prefix: Some(prefix.into()),
            attributes: HashMap::new(),
            children: Vec::new(),
        }
    }

    /// Returns the element name (local name without prefix).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the full qualified name (prefix:local or just local).
    pub fn full_name(&self) -> String {
        match &self.namespace_prefix {
            Some(prefix) => format!("{}:{}", prefix, self.name),
            None => self.name.clone(),
        }
    }

    /// Returns the namespace prefix, if any.
    pub fn namespace_prefix(&self) -> Option<&str> {
        self.namespace_prefix.as_deref()
    }

    /// Sets the namespace prefix.
    pub fn set_namespace_prefix(&mut self, prefix: Option<String>) {
        self.namespace_prefix = prefix;
    }

    // ========================================================================
    // Attributes
    // ========================================================================

    /// Gets an attribute value.
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes.get(name).map(|s| s.as_str())
    }

    /// Sets an attribute value.
    pub fn set_attribute(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.attributes.insert(name.into(), value.into());
    }

    /// Removes an attribute.
    pub fn remove_attribute(&mut self, name: &str) -> Option<String> {
        self.attributes.remove(name)
    }

    /// Returns true if the element has the specified attribute.
    pub fn has_attribute(&self, name: &str) -> bool {
        self.attributes.contains_key(name)
    }

    /// Returns an iterator over all attributes.
    pub fn attributes(&self) -> impl Iterator<Item = (&str, &str)> {
        self.attributes.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Returns the number of attributes.
    pub fn attribute_count(&self) -> usize {
        self.attributes.len()
    }

    // ========================================================================
    // Children
    // ========================================================================

    /// Returns the child nodes.
    pub fn children(&self) -> &[XmlNode] {
        &self.children
    }

    /// Returns a mutable reference to child nodes.
    pub fn children_mut(&mut self) -> &mut Vec<XmlNode> {
        &mut self.children
    }

    /// Returns child elements only (filtering out text, comments, etc.).
    pub fn child_elements(&self) -> impl Iterator<Item = &XmlElement> {
        self.children.iter().filter_map(|node| {
            if let XmlNode::Element(el) = node {
                Some(el)
            } else {
                None
            }
        })
    }

    /// Returns mutable child elements.
    pub fn child_elements_mut(&mut self) -> impl Iterator<Item = &mut XmlElement> {
        self.children.iter_mut().filter_map(|node| {
            if let XmlNode::Element(el) = node {
                Some(el)
            } else {
                None
            }
        })
    }

    /// Returns the number of child elements.
    pub fn child_element_count(&self) -> usize {
        self.children
            .iter()
            .filter(|n| matches!(n, XmlNode::Element(_)))
            .count()
    }

    /// Adds a child node.
    pub fn add_child(&mut self, child: impl Into<XmlNode>) {
        self.children.push(child.into());
    }

    /// Adds a child element.
    pub fn add_child_element(&mut self, element: XmlElement) {
        self.children.push(XmlNode::Element(element));
    }

    /// Creates and adds a child element with text content.
    pub fn add_child_text(&mut self, name: impl Into<String>, text: impl Into<String>) {
        let mut child = XmlElement::new(name);
        child.set_text(text);
        self.children.push(XmlNode::Element(child));
    }

    /// Adds a text node.
    pub fn add_text(&mut self, text: impl Into<String>) {
        self.children.push(XmlNode::Text(text.into()));
    }

    /// Adds a comment node.
    pub fn add_comment(&mut self, comment: impl Into<String>) {
        self.children.push(XmlNode::Comment(comment.into()));
    }

    /// Adds a CDATA section.
    pub fn add_cdata(&mut self, content: impl Into<String>) {
        self.children.push(XmlNode::CData(content.into()));
    }

    /// Removes all child nodes.
    pub fn clear_children(&mut self) {
        self.children.clear();
    }

    /// Removes a child at the specified index.
    pub fn remove_child(&mut self, index: usize) -> Option<XmlNode> {
        if index < self.children.len() {
            Some(self.children.remove(index))
        } else {
            None
        }
    }

    // ========================================================================
    // Text Content
    // ========================================================================

    /// Returns the text content of this element (concatenated from all text nodes).
    pub fn text(&self) -> String {
        let mut result = String::new();
        for child in &self.children {
            match child {
                XmlNode::Text(t) => result.push_str(t),
                XmlNode::CData(c) => result.push_str(c),
                _ => {}
            }
        }
        result
    }

    /// Sets the text content, replacing all children with a single text node.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.children.clear();
        self.children.push(XmlNode::Text(text.into()));
    }

    /// Returns true if this element has any text content.
    pub fn has_text(&self) -> bool {
        self.children.iter().any(|n| matches!(n, XmlNode::Text(_) | XmlNode::CData(_)))
    }

    // ========================================================================
    // Path-Based Access
    // ========================================================================

    /// Gets a child element by name (first match).
    pub fn child(&self, name: &str) -> Option<&XmlElement> {
        self.child_elements().find(|el| el.name == name)
    }

    /// Gets a mutable child element by name (first match).
    pub fn child_mut(&mut self, name: &str) -> Option<&mut XmlElement> {
        self.child_elements_mut().find(|el| el.name == name)
    }

    /// Gets all child elements with the specified name.
    pub fn children_by_name(&self, name: &str) -> Vec<&XmlElement> {
        self.child_elements().filter(|el| el.name == name).collect()
    }

    /// Gets an element at the specified path (relative to this element).
    ///
    /// Path uses forward slashes to separate element names.
    /// Example: "book/title" would find `<book><title>...</title></book>`
    pub fn get(&self, path: &str) -> Option<&XmlElement> {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        self.get_by_parts(&parts)
    }

    /// Gets a mutable reference to an element at the specified path.
    pub fn get_mut(&mut self, path: &str) -> Option<&mut XmlElement> {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        self.get_mut_by_parts(&parts)
    }

    /// Gets all elements matching the specified path.
    pub fn get_all(&self, path: &str) -> Vec<&XmlElement> {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        self.get_all_by_parts(&parts)
    }

    fn get_by_parts(&self, parts: &[&str]) -> Option<&XmlElement> {
        if parts.is_empty() {
            return Some(self);
        }

        let first = parts[0];
        let rest = &parts[1..];

        // Check if this element matches the first part
        if self.name == first || self.full_name() == first {
            if rest.is_empty() {
                return Some(self);
            }
            // Continue searching in children
            for child in self.child_elements() {
                if let Some(found) = child.get_by_parts(rest) {
                    return Some(found);
                }
            }
        } else {
            // Search children for the first part
            for child in self.child_elements() {
                if let Some(found) = child.get_by_parts(parts) {
                    return Some(found);
                }
            }
        }

        None
    }

    fn get_mut_by_parts(&mut self, parts: &[&str]) -> Option<&mut XmlElement> {
        if parts.is_empty() {
            return Some(self);
        }

        let first = parts[0];
        let rest = &parts[1..];

        // Check if this element matches the first part
        if self.name == first || self.full_name() == first {
            if rest.is_empty() {
                return Some(self);
            }
            // Continue searching in children
            for child in self.child_elements_mut() {
                if let Some(found) = child.get_mut_by_parts(rest) {
                    return Some(found);
                }
            }
        } else {
            // Search children for the first part
            for child in self.child_elements_mut() {
                if let Some(found) = child.get_mut_by_parts(parts) {
                    return Some(found);
                }
            }
        }

        None
    }

    fn get_all_by_parts(&self, parts: &[&str]) -> Vec<&XmlElement> {
        let mut results = Vec::new();

        if parts.is_empty() {
            results.push(self);
            return results;
        }

        let first = parts[0];
        let rest = &parts[1..];

        // Check if this element matches the first part
        if self.name == first || self.full_name() == first {
            if rest.is_empty() {
                results.push(self);
            } else {
                // Continue searching in children
                for child in self.child_elements() {
                    results.extend(child.get_all_by_parts(rest));
                }
            }
        } else {
            // Search children for the first part
            for child in self.child_elements() {
                results.extend(child.get_all_by_parts(parts));
            }
        }

        results
    }

    // ========================================================================
    // Serialization
    // ========================================================================

    /// Converts the element to an XML string (compact format).
    pub fn to_string(&self) -> String {
        let mut writer = Writer::new(Vec::new());
        self.write_to(&mut writer);
        String::from_utf8(writer.into_inner()).unwrap_or_default()
    }

    /// Converts the element to an XML string (pretty-printed).
    pub fn to_string_pretty(&self) -> String {
        let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);
        self.write_to(&mut writer);
        String::from_utf8(writer.into_inner()).unwrap_or_default()
    }

    /// Writes the element to a writer.
    fn write_to<W: Write>(&self, writer: &mut Writer<W>) {
        let full_name = self.full_name();
        let mut start = BytesStart::new(&full_name);

        // Add attributes
        for (key, value) in &self.attributes {
            start.push_attribute((key.as_str(), value.as_str()));
        }

        // Check if element is empty
        if self.children.is_empty() {
            let _ = writer.write_event(Event::Empty(start));
        } else {
            let _ = writer.write_event(Event::Start(start));

            // Write children
            for child in &self.children {
                child.write_to(writer);
            }

            let end = BytesEnd::new(&full_name);
            let _ = writer.write_event(Event::End(end));
        }
    }
}

impl fmt::Display for XmlElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_pretty())
    }
}

impl From<XmlElement> for XmlNode {
    fn from(element: XmlElement) -> Self {
        XmlNode::Element(element)
    }
}

// ============================================================================
// XmlNode
// ============================================================================

/// Represents a node in an XML document.
#[derive(Debug, Clone, PartialEq)]
pub enum XmlNode {
    /// An element node
    Element(XmlElement),
    /// A text node
    Text(String),
    /// A comment node
    Comment(String),
    /// A CDATA section
    CData(String),
    /// A processing instruction
    ProcessingInstruction { target: String, data: Option<String> },
}

impl XmlNode {
    /// Returns true if this is an element node.
    pub fn is_element(&self) -> bool {
        matches!(self, XmlNode::Element(_))
    }

    /// Returns true if this is a text node.
    pub fn is_text(&self) -> bool {
        matches!(self, XmlNode::Text(_))
    }

    /// Returns true if this is a comment node.
    pub fn is_comment(&self) -> bool {
        matches!(self, XmlNode::Comment(_))
    }

    /// Returns true if this is a CDATA section.
    pub fn is_cdata(&self) -> bool {
        matches!(self, XmlNode::CData(_))
    }

    /// Returns this node as an element, if it is one.
    pub fn as_element(&self) -> Option<&XmlElement> {
        if let XmlNode::Element(el) = self {
            Some(el)
        } else {
            None
        }
    }

    /// Returns this node as a mutable element, if it is one.
    pub fn as_element_mut(&mut self) -> Option<&mut XmlElement> {
        if let XmlNode::Element(el) = self {
            Some(el)
        } else {
            None
        }
    }

    /// Returns the text content of this node (if text or CDATA).
    pub fn text(&self) -> Option<&str> {
        match self {
            XmlNode::Text(t) => Some(t),
            XmlNode::CData(c) => Some(c),
            _ => None,
        }
    }

    /// Writes the node to a writer.
    fn write_to<W: Write>(&self, writer: &mut Writer<W>) {
        match self {
            XmlNode::Element(el) => el.write_to(writer),
            XmlNode::Text(text) => {
                let _ = writer.write_event(Event::Text(BytesText::new(text)));
            }
            XmlNode::Comment(comment) => {
                let _ = writer.write_event(Event::Comment(BytesText::new(comment)));
            }
            XmlNode::CData(content) => {
                let _ = writer.write_event(Event::CData(BytesCData::new(content)));
            }
            XmlNode::ProcessingInstruction { target, data } => {
                let pi = match data {
                    Some(d) => format!("{} {}", target, d),
                    None => target.clone(),
                };
                let _ = writer.write_event(Event::PI(quick_xml::events::BytesPI::new(&pi)));
            }
        }
    }
}

impl From<String> for XmlNode {
    fn from(text: String) -> Self {
        XmlNode::Text(text)
    }
}

impl From<&str> for XmlNode {
    fn from(text: &str) -> Self {
        XmlNode::Text(text.to_string())
    }
}

// ============================================================================
// Streaming Parser (SAX-like)
// ============================================================================

/// Events emitted by the streaming XML reader.
#[derive(Debug, Clone, PartialEq)]
pub enum XmlEvent {
    /// XML declaration
    Declaration {
        version: String,
        encoding: Option<String>,
        standalone: Option<bool>,
    },
    /// Start of an element
    StartElement {
        name: String,
        attributes: HashMap<String, String>,
    },
    /// End of an element
    EndElement {
        name: String,
    },
    /// Empty element (self-closing)
    EmptyElement {
        name: String,
        attributes: HashMap<String, String>,
    },
    /// Text content
    Text(String),
    /// CDATA section
    CData(String),
    /// Comment
    Comment(String),
    /// Processing instruction
    ProcessingInstruction {
        target: String,
        data: Option<String>,
    },
}

/// A streaming XML reader for SAX-like parsing.
///
/// Use this for large XML files to avoid loading the entire document into memory.
pub struct XmlReader<R: BufRead> {
    reader: Reader<R>,
    buf: Vec<u8>,
}

impl XmlReader<BufReader<std::fs::File>> {
    /// Opens an XML file for streaming reading.
    pub fn open(path: impl AsRef<Path>) -> FileResult<Self> {
        let file = std::fs::File::open(path.as_ref()).map_err(|e| {
            FileError::new(
                FileErrorKind::NotFound,
                Some(path.as_ref().to_path_buf()),
                Some(e),
            )
        })?;
        let reader = BufReader::new(file);
        Ok(Self::new(reader))
    }
}

impl<'a> XmlReader<&'a [u8]> {
    /// Creates a reader from a string.
    pub fn from_str(s: &'a str) -> Self {
        Self::new(s.as_bytes())
    }
}

impl<R: BufRead> XmlReader<R> {
    /// Creates a new streaming reader from a BufRead source.
    pub fn new(source: R) -> Self {
        let mut reader = Reader::from_reader(source);
        reader.config_mut().trim_text(true);
        XmlReader {
            reader,
            buf: Vec::new(),
        }
    }

    /// Reads the next XML event.
    ///
    /// Returns `None` when the end of the document is reached.
    pub fn next(&mut self) -> FileResult<Option<XmlEvent>> {
        self.buf.clear();
        match self.reader.read_event_into(&mut self.buf) {
            Ok(Event::Eof) => Ok(None),
            Ok(event) => Ok(Some(convert_event(event)?)),
            Err(e) => Err(xml_error(e)),
        }
    }

    /// Returns an iterator over XML events.
    pub fn events(self) -> XmlEventIterator<R> {
        XmlEventIterator { reader: self }
    }
}

/// Iterator over XML events from a streaming reader.
pub struct XmlEventIterator<R: BufRead> {
    reader: XmlReader<R>,
}

impl<R: BufRead> Iterator for XmlEventIterator<R> {
    type Item = FileResult<XmlEvent>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.next() {
            Ok(Some(event)) => Some(Ok(event)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

/// Converts a quick_xml event to our XmlEvent type.
fn convert_event(event: Event<'_>) -> FileResult<XmlEvent> {
    match event {
        Event::Decl(decl) => {
            let version = String::from_utf8_lossy(&decl.version().map_err(xml_error)?).to_string();
            let encoding = match decl.encoding() {
                Some(Ok(e)) => Some(String::from_utf8_lossy(&e).to_string()),
                Some(Err(e)) => return Err(xml_attr_error(e)),
                None => None,
            };
            let standalone = match decl.standalone() {
                Some(Ok(s)) => Some(String::from_utf8_lossy(&s) == "yes"),
                Some(Err(e)) => return Err(xml_attr_error(e)),
                None => None,
            };
            Ok(XmlEvent::Declaration {
                version,
                encoding,
                standalone,
            })
        }
        Event::Start(start) => {
            let name = String::from_utf8_lossy(start.name().as_ref()).to_string();
            let attributes = parse_attributes(&start)?;
            Ok(XmlEvent::StartElement { name, attributes })
        }
        Event::End(end) => {
            let name = String::from_utf8_lossy(end.name().as_ref()).to_string();
            Ok(XmlEvent::EndElement { name })
        }
        Event::Empty(empty) => {
            let name = String::from_utf8_lossy(empty.name().as_ref()).to_string();
            let attributes = parse_attributes(&empty)?;
            Ok(XmlEvent::EmptyElement { name, attributes })
        }
        Event::Text(text) => {
            let content = text.unescape().map_err(xml_error)?.to_string();
            Ok(XmlEvent::Text(content))
        }
        Event::CData(cdata) => {
            let content = String::from_utf8_lossy(&cdata).to_string();
            Ok(XmlEvent::CData(content))
        }
        Event::Comment(comment) => {
            let content = String::from_utf8_lossy(&comment).to_string();
            Ok(XmlEvent::Comment(content))
        }
        Event::PI(pi) => {
            let content = String::from_utf8_lossy(&pi).to_string();
            let mut parts = content.splitn(2, ' ');
            let target = parts.next().unwrap_or("").to_string();
            let data = parts.next().map(String::from);
            Ok(XmlEvent::ProcessingInstruction { target, data })
        }
        Event::Eof => {
            // This shouldn't happen as we handle Eof separately
            Ok(XmlEvent::Text(String::new()))
        }
        Event::DocType(_) => {
            // Skip DOCTYPE declarations
            Ok(XmlEvent::Text(String::new()))
        }
    }
}

/// Parses attributes from a BytesStart element.
fn parse_attributes(start: &BytesStart<'_>) -> FileResult<HashMap<String, String>> {
    let mut attrs = HashMap::new();
    for attr in start.attributes() {
        let attr = attr.map_err(xml_attr_error)?;
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
        let value = String::from_utf8_lossy(&attr.value).to_string();
        attrs.insert(key, value);
    }
    Ok(attrs)
}

// ============================================================================
// Module-Level Functions
// ============================================================================

/// Parses an XML string into a document.
pub fn parse_xml(s: &str) -> FileResult<XmlDocument> {
    let mut reader = Reader::from_str(s);
    reader.config_mut().trim_text(true);
    parse_document(&mut reader)
}

/// Reads and parses an XML file into a document.
pub fn read_xml(path: impl AsRef<Path>) -> FileResult<XmlDocument> {
    let content = read_text(&path)?;
    parse_xml(&content).map_err(|e| {
        FileError::new(
            e.kind(),
            Some(path.as_ref().to_path_buf()),
            None,
        )
    })
}

/// Reads and deserializes an XML file into a typed value.
///
/// Requires the type to implement `serde::Deserialize`.
pub fn read_xml_as<T: DeserializeOwned>(path: impl AsRef<Path>) -> FileResult<T> {
    let content = read_text(&path)?;
    quick_xml::de::from_str(&content).map_err(|e| {
        FileError::new(
            FileErrorKind::InvalidData,
            Some(path.as_ref().to_path_buf()),
            Some(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })
}

/// Parses an XML string and deserializes it into a typed value.
pub fn parse_xml_as<T: DeserializeOwned>(s: &str) -> FileResult<T> {
    quick_xml::de::from_str(s).map_err(|e| {
        FileError::new(
            FileErrorKind::InvalidData,
            None,
            Some(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })
}

/// Writes a serializable value to an XML file.
pub fn write_xml<T: Serialize>(path: impl AsRef<Path>, value: &T) -> FileResult<()> {
    let xml = quick_xml::se::to_string(value).map_err(xml_ser_error)?;
    atomic_write(&path, |writer| writer.write_all(xml.as_bytes()))
}

/// Converts a serializable value to an XML string.
pub fn to_xml_string<T: Serialize>(value: &T) -> FileResult<String> {
    quick_xml::se::to_string(value).map_err(xml_ser_error)
}

// ============================================================================
// Internal Parsing
// ============================================================================

/// Parses a document from a reader.
fn parse_document<R: BufRead>(reader: &mut Reader<R>) -> FileResult<XmlDocument> {
    let mut buf = Vec::new();
    let mut version = "1.0".to_string();
    let mut encoding = Some("UTF-8".to_string());
    let mut standalone = None;
    let mut root: Option<XmlElement> = None;
    let mut element_stack: Vec<XmlElement> = Vec::new();

    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,
            Ok(Event::Decl(decl)) => {
                if let Ok(v) = decl.version() {
                    version = String::from_utf8_lossy(&v).to_string();
                }
                if let Some(Ok(e)) = decl.encoding() {
                    encoding = Some(String::from_utf8_lossy(&e).to_string());
                }
                if let Some(Ok(s)) = decl.standalone() {
                    standalone = Some(String::from_utf8_lossy(&s) == "yes");
                }
            }
            Ok(Event::Start(start)) => {
                let name = String::from_utf8_lossy(start.name().as_ref()).to_string();
                let mut element = XmlElement::new(&name);

                // Parse attributes
                for attr in start.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let value = String::from_utf8_lossy(&attr.value).to_string();
                    element.set_attribute(key, value);
                }

                element_stack.push(element);
            }
            Ok(Event::End(_)) => {
                if let Some(element) = element_stack.pop() {
                    if let Some(parent) = element_stack.last_mut() {
                        parent.add_child_element(element);
                    } else {
                        root = Some(element);
                    }
                }
            }
            Ok(Event::Empty(empty)) => {
                let name = String::from_utf8_lossy(empty.name().as_ref()).to_string();
                let mut element = XmlElement::new(&name);

                // Parse attributes
                for attr in empty.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let value = String::from_utf8_lossy(&attr.value).to_string();
                    element.set_attribute(key, value);
                }

                if let Some(parent) = element_stack.last_mut() {
                    parent.add_child_element(element);
                } else {
                    root = Some(element);
                }
            }
            Ok(Event::Text(text)) => {
                if let Ok(content) = text.unescape() {
                    let content = content.to_string();
                    if !content.is_empty() {
                        if let Some(parent) = element_stack.last_mut() {
                            parent.add_text(content);
                        }
                    }
                }
            }
            Ok(Event::CData(cdata)) => {
                let content = String::from_utf8_lossy(&cdata).to_string();
                if let Some(parent) = element_stack.last_mut() {
                    parent.add_cdata(content);
                }
            }
            Ok(Event::Comment(comment)) => {
                let content = String::from_utf8_lossy(&comment).to_string();
                if let Some(parent) = element_stack.last_mut() {
                    parent.add_comment(content);
                }
            }
            Ok(Event::PI(pi)) => {
                let content = String::from_utf8_lossy(&pi).to_string();
                let mut parts = content.splitn(2, ' ');
                let target = parts.next().unwrap_or("").to_string();
                let data = parts.next().map(String::from);
                if let Some(parent) = element_stack.last_mut() {
                    parent.add_child(XmlNode::ProcessingInstruction { target, data });
                }
            }
            Ok(Event::DocType(_)) => {
                // Skip DOCTYPE declarations
            }
            Err(e) => return Err(xml_error(e)),
        }
    }

    // Handle any remaining elements on the stack (malformed XML)
    while let Some(element) = element_stack.pop() {
        if let Some(parent) = element_stack.last_mut() {
            parent.add_child_element(element);
        } else {
            root = Some(element);
        }
    }

    match root {
        Some(root) => Ok(XmlDocument {
            version,
            encoding,
            standalone,
            root,
        }),
        None => Err(FileError::new(
            FileErrorKind::InvalidData,
            None,
            Some(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "XML document has no root element",
            )),
        )),
    }
}

// ============================================================================
// Internal Helpers
// ============================================================================

/// Splits a qualified name into prefix and local parts.
fn split_namespace(name: &str) -> (Option<&str>, &str) {
    if let Some(pos) = name.find(':') {
        (Some(&name[..pos]), &name[pos + 1..])
    } else {
        (None, name)
    }
}

/// Converts a quick_xml error to a FileError.
fn xml_error(e: quick_xml::Error) -> FileError {
    FileError::new(
        FileErrorKind::InvalidData,
        None,
        Some(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
    )
}

/// Converts an attribute error to a FileError.
fn xml_attr_error(e: quick_xml::events::attributes::AttrError) -> FileError {
    FileError::new(
        FileErrorKind::InvalidData,
        None,
        Some(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
    )
}

/// Converts a serialization error to a FileError.
fn xml_ser_error(e: quick_xml::SeError) -> FileError {
    FileError::new(
        FileErrorKind::InvalidData,
        None,
        Some(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
    )
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
            <book>
                <title>Rust Programming</title>
                <author>Jane Doe</author>
            </book>
        "#;

        let doc = parse_xml(xml).unwrap();
        assert_eq!(doc.root().name(), "book");
        assert_eq!(doc.get("book/title").unwrap().text(), "Rust Programming");
        assert_eq!(doc.get("book/author").unwrap().text(), "Jane Doe");
    }

    #[test]
    fn test_parse_with_attributes() {
        let xml = r#"<catalog>
            <book id="1" category="fiction">
                <title>The Great Gatsby</title>
                <price currency="USD">10.99</price>
            </book>
        </catalog>"#;

        let doc = parse_xml(xml).unwrap();
        let book = doc.get("catalog/book").unwrap();
        assert_eq!(book.attribute("id"), Some("1"));
        assert_eq!(book.attribute("category"), Some("fiction"));

        let price = doc.get("catalog/book/price").unwrap();
        assert_eq!(price.attribute("currency"), Some("USD"));
        assert_eq!(price.text(), "10.99");
    }

    #[test]
    fn test_build_document() {
        let mut doc = XmlDocument::new("catalog");
        doc.root_mut().set_attribute("version", "1.0");

        let mut book = XmlElement::new("book");
        book.set_attribute("id", "1");
        book.add_child_text("title", "Rust in Action");
        book.add_child_text("author", "Tim McNamara");
        doc.root_mut().add_child(book);

        let xml = doc.to_string();
        assert!(xml.contains("<catalog"));
        assert!(xml.contains("version=\"1.0\""));
        assert!(xml.contains("<book"));
        assert!(xml.contains("id=\"1\""));
        assert!(xml.contains("<title>Rust in Action</title>"));
    }

    #[test]
    fn test_get_all_elements() {
        let xml = r#"<bookstore>
            <book><title>Book 1</title></book>
            <book><title>Book 2</title></book>
            <book><title>Book 3</title></book>
        </bookstore>"#;

        let doc = parse_xml(xml).unwrap();
        let books = doc.get_all("bookstore/book");
        assert_eq!(books.len(), 3);

        let titles: Vec<String> = books.iter().map(|b| b.child("title").unwrap().text()).collect();
        assert_eq!(titles, vec!["Book 1", "Book 2", "Book 3"]);
    }

    #[test]
    fn test_streaming_reader() {
        let xml = r#"<items><item>A</item><item>B</item><item>C</item></items>"#;
        let mut reader = XmlReader::from_str(xml);

        let mut items = Vec::new();
        while let Ok(Some(event)) = reader.next() {
            if let XmlEvent::Text(text) = event {
                items.push(text);
            }
        }

        assert_eq!(items, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_empty_element() {
        let xml = r#"<root><empty/><with attr="value"/></root>"#;
        let doc = parse_xml(xml).unwrap();

        let empty = doc.get("root/empty").unwrap();
        assert!(empty.children().is_empty());

        let with_attr = doc.get("root/with").unwrap();
        assert_eq!(with_attr.attribute("attr"), Some("value"));
    }

    #[test]
    fn test_cdata_and_comments() {
        let xml = r#"<root>
            <!-- This is a comment -->
            <data><![CDATA[Some <special> content]]></data>
        </root>"#;

        let doc = parse_xml(xml).unwrap();
        let data = doc.get("root/data").unwrap();
        assert_eq!(data.text(), "Some <special> content");
    }

    #[test]
    fn test_namespace_prefix() {
        let xml = r#"<root xmlns:ns="http://example.com">
            <ns:element>Namespaced content</ns:element>
        </root>"#;

        let doc = parse_xml(xml).unwrap();
        // The element should be accessible by full name
        let element = doc.root().child_elements().find(|e| e.full_name() == "ns:element");
        assert!(element.is_some());
        assert_eq!(element.unwrap().text(), "Namespaced content");
    }

    #[test]
    fn test_modify_element() {
        let xml = r#"<config><value>old</value></config>"#;
        let mut doc = parse_xml(xml).unwrap();

        // Modify the value
        if let Some(value) = doc.get_mut("config/value") {
            value.set_text("new");
        }

        assert_eq!(doc.get("config/value").unwrap().text(), "new");
    }

    #[test]
    fn test_roundtrip() {
        let original = r#"<?xml version="1.0" encoding="UTF-8"?>
<catalog><book id="1"><title>Test</title></book></catalog>"#;

        let doc = parse_xml(original).unwrap();
        let output = doc.to_string();

        // Parse the output again
        let doc2 = parse_xml(&output).unwrap();
        assert_eq!(doc2.get("catalog/book").unwrap().attribute("id"), Some("1"));
        assert_eq!(doc2.get("catalog/book/title").unwrap().text(), "Test");
    }

    #[test]
    fn test_serde_deserialization() {
        use serde::Deserialize;

        #[derive(Debug, Deserialize, PartialEq)]
        struct Book {
            title: String,
            author: String,
        }

        let xml = r#"<Book><title>Rust Book</title><author>Author</author></Book>"#;
        let book: Book = parse_xml_as(xml).unwrap();

        assert_eq!(
            book,
            Book {
                title: "Rust Book".to_string(),
                author: "Author".to_string()
            }
        );
    }

    #[test]
    fn test_file_roundtrip() {
        let mut doc = XmlDocument::new("test");
        doc.root_mut().set_attribute("version", "1.0");
        doc.root_mut().add_child_text("item", "value");

        let path = std::env::temp_dir().join("horizon_xml_test.xml");

        doc.save_pretty(&path).unwrap();

        let loaded = read_xml(&path).unwrap();
        assert_eq!(loaded.root().attribute("version"), Some("1.0"));
        assert_eq!(loaded.get("test/item").unwrap().text(), "value");

        std::fs::remove_file(&path).ok();
    }
}
