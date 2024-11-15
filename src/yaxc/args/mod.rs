use crate::error::Error;

use std::collections::HashMap;
use std::env;
use std::iter::{Peekable, Skip};

#[derive(Clone, Copy)]
pub enum Rule<T: Clone + Copy> {
    Flag(T),
    Integer(T),
    Hex(T),
}

#[derive(Debug)]
pub enum Argument<T: std::fmt::Debug> {
    Flag { kind: T },
    Integer { kind: T, value: u32 },
    Hex { kind: T, value: u32 },
}

pub struct Args<T: Clone + Copy + std::fmt::Debug> {
    rules: HashMap<String, Rule<T>>,
    args: Peekable<Skip<env::Args>>,
}

impl<T> Args<T>
where
    T: Clone + Copy + std::fmt::Debug,
{
    pub fn new() -> Args<T> {
        Args {
            rules: HashMap::new(),
            args: env::args().skip(1).peekable(),
        }
    }

    pub fn append(&mut self, key: &str, rule: Rule<T>) {
        self.rules.insert(key.to_string(), rule);
    }

    pub fn is_empty(&mut self) -> bool {
        self.args.peek().is_none()
    }

    fn parse_rule(&mut self, rule: Rule<T>) -> Result<Argument<T>, Box<dyn std::error::Error>> {
        match rule {
            Rule::Flag(kind) => Ok(Argument::Flag { kind }),
            Rule::Integer(kind) => Ok(Argument::Integer {
                kind,
                value: self.parse_next()?.parse::<u32>()?,
            }),
            Rule::Hex(kind) => Ok(Argument::Hex {
                kind,
                value: u32::from_str_radix(&self.parse_next()?, 16)?,
            }),
        }
    }

    fn parse_next(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        self.args
            .next()
            .ok_or(Box::new(Error::ArgsEmpty))
            .map_err(|err| err.into())
    }

    fn parse(&mut self, arg: String) -> Result<Argument<T>, Box<dyn std::error::Error>> {
        let rules = self.rules.clone();

        rules
            .get(&arg)
            .ok_or(Error::Unknown { arg })
            .map_err(|err| err.into())
            .and_then(|rule| self.parse_rule(*rule))
    }

    pub fn next(&mut self) -> Result<Argument<T>, Box<dyn std::error::Error>> {
        self.args
            .next()
            .ok_or(Error::ArgsEmpty)
            .map_err(|err| err.into())
            .and_then(|arg| self.parse(arg))
    }
}
