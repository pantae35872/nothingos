use core::{error::Error, fmt::Display, ops::Index};

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::hash_map::HashMap;

use super::token::TomlToken;

type Array = Vec<TomlValue>;
type Table = HashMap<String, TomlValue>;

#[derive(Clone, Debug, PartialEq)]
pub enum TomlValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Array),
    Table(Table),
}

#[derive(Debug)]
pub enum TomlParserError {
    UnexpectedToken(Option<TomlToken>),
    KeyNotFound(String),
    NotATable,
}

impl Display for TomlParserError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnexpectedToken(token) => write!(f, "Unexpected Token: {:?}", token),
            Self::KeyNotFound(key) => write!(f, "Key not found: {}", key),
            Self::NotATable => write!(f, "Toml value is not a table"),
        }
    }
}

impl Error for TomlParserError {}

pub struct TomlParser {
    tokens: Vec<TomlToken>,
    index: usize,
}

impl TomlValue {
    fn insert_deep(&mut self, keys: Vec<String>, value: TomlValue) -> Result<(), TomlParserError> {
        let mut table = self.as_table_mut().ok_or(TomlParserError::NotATable)?;
        for key in &keys[..keys.len() - 1] {
            table = table
                .get_or_insert_mut(key, || TomlValue::Table(HashMap::new()))
                .as_table_mut()
                .ok_or(TomlParserError::NotATable)?;
        }
        if let Some(key) = keys.last() {
            table.insert(key.clone(), value);
        }
        return Ok(());
    }

    fn as_table(&self) -> Option<&Table> {
        match self {
            TomlValue::Table(table) => return Some(table),
            _ => return None,
        }
    }

    fn as_table_mut(&mut self) -> Option<&mut Table> {
        match self {
            TomlValue::Table(table) => return Some(table),
            _ => return None,
        }
    }

    pub fn get<K: AsRef<str>>(&self, key: K) -> Option<&TomlValue> {
        return self.as_table()?.get(key.as_ref());
    }

    pub fn as_array(&self) -> Option<&Array> {
        match self {
            TomlValue::Array(array) => return Some(array),
            _ => return None,
        }
    }

    pub fn as_string(&self) -> Option<&String> {
        match self {
            TomlValue::String(value) => return Some(value),
            _ => return None,
        }
    }

    pub fn as_interger(&self) -> Option<i64> {
        match self {
            TomlValue::Integer(value) => return Some(*value),
            _ => return None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            TomlValue::Float(value) => return Some(*value),
            _ => return None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            TomlValue::Boolean(value) => return Some(*value),
            _ => return None,
        }
    }
}

impl<K: AsRef<str>> Index<K> for TomlValue {
    type Output = TomlValue;

    fn index(&self, key: K) -> &Self::Output {
        return self
            .as_table()
            .expect("Cannot index into a table because TomlValue is not a table")
            .get(key.as_ref())
            .expect("Key not found");
    }
}

impl Display for TomlValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::String(value) => write!(f, "String with value: {}", value),
            Self::Array(array) => write!(f, "Array with value: {:?}", array),
            Self::Table(table_value) => write!(f, "Table with value: {:?}", table_value),
            Self::Boolean(boolean) => write!(f, "Boolean with value: {}", boolean),
            Self::Integer(interger) => write!(f, "Interger with value: {}", interger),
            Self::Float(float) => write!(f, "Float with value: {}", float),
        }
    }
}

impl TomlParser {
    pub fn new(tokens: Vec<TomlToken>) -> Self {
        Self { tokens, index: 0 }
    }

    fn peek(&self, offset: usize) -> Option<&TomlToken> {
        return self.tokens.get(self.index + offset);
    }

    fn consume(&mut self) -> Option<&TomlToken> {
        if let Some(token) = self.tokens.get(self.index) {
            self.index += 1;
            return Some(token);
        } else {
            return None;
        }
    }

    pub fn parse(&mut self) -> Result<TomlValue, TomlParserError> {
        let mut main_map = TomlValue::Table(HashMap::new());

        while let Some(token) = self.peek(0).cloned() {
            match token {
                TomlToken::Identifier(identifier) | TomlToken::String(identifier) => {
                    self.consume();
                    let keys = self.parse_keys(identifier)?;
                    self.expect_token(TomlToken::Equal)?;
                    let value = self.parse_value()?;
                    main_map.insert_deep(keys, value)?;
                    self.expect_token(TomlToken::NewLine)?;
                }
                TomlToken::Interger(interger) => {
                    let identifier = interger.to_string();
                    self.consume();
                    let keys = self.parse_keys(identifier)?;
                    self.expect_token(TomlToken::Equal)?;
                    let value = self.parse_value()?;
                    main_map.insert_deep(keys, value)?;
                    self.expect_token(TomlToken::NewLine)?;
                }
                TomlToken::LBracket => {
                    self.consume();
                    let _table_name = self.parse_table_name()?;
                    todo!();
                }
                TomlToken::NewLine => {
                    self.consume();
                }
                unexpected => {
                    return Err(TomlParserError::UnexpectedToken(Some(unexpected.clone())))
                }
            }
        }

        return Ok(main_map);
    }

    fn parse_keys(&mut self, first_iden: String) -> Result<Vec<String>, TomlParserError> {
        let mut keys = Vec::new();
        keys.push(first_iden);
        loop {
            match self.peek(0).ok_or(TomlParserError::UnexpectedToken(None))? {
                TomlToken::Dot => {
                    self.consume();
                    match self
                        .consume()
                        .ok_or(TomlParserError::UnexpectedToken(None))?
                    {
                        TomlToken::Identifier(identifier) | TomlToken::String(identifier) => {
                            keys.push(identifier.clone());
                        }
                        unexpected => {
                            return Err(TomlParserError::UnexpectedToken(Some(unexpected.clone())))
                        }
                    };
                }
                TomlToken::Equal | TomlToken::RBracket => {
                    break;
                }
                unexpected => {
                    return Err(TomlParserError::UnexpectedToken(Some(unexpected.clone())))
                }
            }
        }
        return Ok(keys);
    }

    fn parse_table_name(&mut self) -> Result<String, TomlParserError> {
        match self
            .consume()
            .ok_or(TomlParserError::UnexpectedToken(None))?
        {
            TomlToken::String(identifier) | TomlToken::Identifier(identifier) => {
                return Ok(identifier.clone());
            }
            unexpected => return Err(TomlParserError::UnexpectedToken(Some(unexpected.clone()))),
        };
    }

    fn parse_value(&mut self) -> Result<TomlValue, TomlParserError> {
        match self
            .consume()
            .ok_or(TomlParserError::UnexpectedToken(None))?
        {
            TomlToken::String(string) => return Ok(TomlValue::String(string.clone())),
            TomlToken::Interger(interger) => return Ok(TomlValue::Integer(*interger)),
            TomlToken::Boolean(boolean) => return Ok(TomlValue::Boolean(*boolean)),
            TomlToken::LBracket => return Ok(TomlValue::Array(self.parse_array()?)),
            TomlToken::LCurly => return Ok(TomlValue::Table(self.parse_inline_table()?)),
            _ => return Err(TomlParserError::UnexpectedToken(None)),
        };
    }

    fn parse_inline_table(&mut self) -> Result<Table, TomlParserError> {
        todo!();
    }

    fn parse_array(&mut self) -> Result<Array, TomlParserError> {
        todo!();
    }

    fn expect_token(&mut self, expected: TomlToken) -> Result<(), TomlParserError> {
        let token = self.peek(0).ok_or(TomlParserError::UnexpectedToken(None))?;
        if *token == expected {
            self.consume();
            return Ok(());
        } else {
            return Err(TomlParserError::UnexpectedToken(Some(token.clone())));
        }
    }
}
