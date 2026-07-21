//! Minimal XML parser. Handles open/close tags, attributes, character data,
//! and self-closing elements. No DTD, no entities beyond the standard set,
//! no namespaces beyond keeping the prefixed form in the tag name.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Element {
    pub name: String,
    pub attributes: HashMap<String, String>,
    pub children: Vec<Element>,
    pub text: String,
}

#[derive(Debug)]
pub enum XmlError {
    Unterminated,
    UnexpectedToken,
}

pub fn parse(src: &str) -> Result<Element, XmlError> {
    let mut p = Parser {
        bytes: src.as_bytes(),
        pos: 0,
    };
    p.skip_prolog();
    p.skip_ws();
    p.element()
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn skip_prolog(&mut self) {
        // Skip <?xml ... ?> and <!-- ... --> comments.
        loop {
            self.skip_ws();
            if self.starts(b"<?xml") {
                while self.pos + 1 < self.bytes.len()
                    && !(self.bytes[self.pos] == b'?' && self.bytes[self.pos + 1] == b'>')
                {
                    self.pos += 1;
                }
                self.pos = (self.pos + 2).min(self.bytes.len());
            } else if self.starts(b"<!--") {
                while self.pos + 2 < self.bytes.len()
                    && !(self.bytes[self.pos] == b'-'
                        && self.bytes[self.pos + 1] == b'-'
                        && self.bytes[self.pos + 2] == b'>')
                {
                    self.pos += 1;
                }
                self.pos = (self.pos + 3).min(self.bytes.len());
            } else {
                return;
            }
        }
    }

    fn starts(&self, prefix: &[u8]) -> bool {
        self.pos + prefix.len() <= self.bytes.len()
            && &self.bytes[self.pos..self.pos + prefix.len()] == prefix
    }

    fn element(&mut self) -> Result<Element, XmlError> {
        self.skip_ws();
        if self.starts(b"<!--") {
            // skip comment, fall through to next element
            while self.pos + 2 < self.bytes.len()
                && !(self.bytes[self.pos] == b'-'
                    && self.bytes[self.pos + 1] == b'-'
                    && self.bytes[self.pos + 2] == b'>')
            {
                self.pos += 1;
            }
            self.pos = (self.pos + 3).min(self.bytes.len());
            self.skip_ws();
        }
        if self.pos >= self.bytes.len() || self.bytes[self.pos] != b'<' {
            return Err(XmlError::UnexpectedToken);
        }
        self.pos += 1;
        let name_start = self.pos;
        while self.pos < self.bytes.len()
            && !matches!(
                self.bytes[self.pos],
                b' ' | b'\t' | b'\n' | b'\r' | b'>' | b'/'
            )
        {
            self.pos += 1;
        }
        let name = String::from_utf8_lossy(&self.bytes[name_start..self.pos]).to_string();
        let mut attributes = HashMap::new();
        loop {
            self.skip_ws();
            if self.starts(b"/>") {
                self.pos += 2;
                return Ok(Element {
                    name,
                    attributes,
                    children: Vec::new(),
                    text: String::new(),
                });
            }
            if self.starts(b">") {
                self.pos += 1;
                break;
            }
            // Attribute.
            let a_start = self.pos;
            while self.pos < self.bytes.len() && self.bytes[self.pos] != b'=' {
                self.pos += 1;
            }
            let aname = String::from_utf8_lossy(&self.bytes[a_start..self.pos])
                .trim()
                .to_string();
            self.pos += 1; // skip '='
            self.skip_ws();
            if self.pos >= self.bytes.len() {
                return Err(XmlError::Unterminated);
            }
            let quote = self.bytes[self.pos];
            self.pos += 1;
            let v_start = self.pos;
            while self.pos < self.bytes.len() && self.bytes[self.pos] != quote {
                self.pos += 1;
            }
            let avalue = String::from_utf8_lossy(&self.bytes[v_start..self.pos]).to_string();
            self.pos += 1; // skip closing quote
            attributes.insert(aname, avalue);
        }
        // Parse children + text until </name>.
        let mut children: Vec<Element> = Vec::new();
        let mut text = String::new();
        loop {
            self.skip_ws();
            if self.starts(b"</") {
                // Close tag.
                self.pos += 2;
                while self.pos < self.bytes.len() && self.bytes[self.pos] != b'>' {
                    self.pos += 1;
                }
                self.pos = (self.pos + 1).min(self.bytes.len());
                return Ok(Element {
                    name,
                    attributes,
                    children,
                    text,
                });
            }
            if self.starts(b"<") {
                children.push(self.element()?);
            } else {
                let t_start = self.pos;
                while self.pos < self.bytes.len() && self.bytes[self.pos] != b'<' {
                    self.pos += 1;
                }
                text.push_str(&String::from_utf8_lossy(&self.bytes[t_start..self.pos]));
            }
        }
    }
}

impl Element {
    pub fn find_child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|c| c.name == name)
    }

    pub fn find_children(&self, name: &str) -> Vec<&Element> {
        self.children.iter().filter(|c| c.name == name).collect()
    }
}
