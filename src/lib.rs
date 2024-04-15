#![cfg_attr(not(feature = "std"), no_std)]

pub mod buf_view;

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, format, string::String, vec, vec::Vec};
use buf_view::BufView;
#[cfg(not(feature = "std"))]
use core::{convert::TryInto, iter::Iterator, panic, result::Result, result::Result::*, todo};
#[cfg(feature = "std")]
use rand::distributions::{Alphanumeric, DistString};
use sha2::{Digest, Sha256};
#[cfg(feature = "std")]
use std::io::BufWriter;

#[cfg(feature = "std")]
fn s(x: &[u8]) -> String {
    std::str::from_utf8(x).unwrap().to_string()
}

type ShittyString = Vec<u8>;
pub type EValue = [u8; 32];

pub enum Node {
    Block {
        number: usize,
        contracts: Vec<Node>,
    },
    Contract {
        address: [u8; 20],
        storage: Vec<Node>,
    },
    Variable {
        name: ShittyString,
        value: EValue,
    },
    Struct {
        name: ShittyString,
        fields: Vec<Node>,
    },
    Mapping {
        name: ShittyString,
        entries: Vec<Node>,
    },
    Entry {
        key: EValue,
        entry: Box<Node>,
    },
}

impl Node {
    fn to_id(&self) -> u8 {
        match self {
            Node::Variable { .. } => 1,
            Node::Struct { .. } => 2,
            Node::Mapping { .. } => 3,
            _ => panic!("nope"),
        }
    }

    #[cfg(feature = "std")]
    fn serialize<W: std::io::Write>(&self, out: &mut W) {
        match self {
            Node::Block { number, contracts } => {
                out.write_all(&number.to_le_bytes()).unwrap();
                out.write_all(&contracts.len().to_le_bytes()).unwrap();
                for c in contracts {
                    c.serialize(out);
                }
            }
            Node::Contract { address, storage } => {
                out.write_all(address).unwrap();
                out.write_all(&storage.len().to_le_bytes()).unwrap();
                for s in storage {
                    s.serialize(out);
                }
            }
            Node::Variable { name, value } => {
                out.write_all(&[self.to_id()]).unwrap();
                out.write_all(&name.len().to_le_bytes()).unwrap();
                out.write_all(name).unwrap();
                out.write_all(value).unwrap();
            }
            Node::Struct { name, fields } => {
                out.write_all(&[self.to_id()]).unwrap();
                todo!()
            }
            Node::Mapping { name, entries } => {
                out.write_all(&[self.to_id()]).unwrap();
                out.write_all(&name.len().to_le_bytes()).unwrap();
                out.write_all(name).unwrap();
                out.write_all(&entries.len().to_le_bytes()).unwrap();
                for e in entries {
                    e.serialize(out);
                }
            }
            Node::Entry { key, entry } => {
                out.write_all(key).unwrap();
                entry.serialize(out);
            }
        }
    }

    pub fn parse(b: &mut BufView) -> Result<Node, String> {
        Node::parse_block(b)
    }

    fn parse_block(b: &mut BufView) -> Result<Node, String> {
        let n = b.read_u64_le() as usize;
        let contract_count = b.read_u64_le() as usize;
        let contracts = (0..contract_count)
            .map(|_| Node::parse_contract(b))
            .collect::<Result<Vec<Node>, String>>()?;

        Ok(Node::Block {
            number: n,
            contracts,
        })
    }

    fn parse_contract(b: &mut BufView) -> Result<Node, String> {
        let mut address = [0u8; 20];
        b.read_bytes(&mut address);

        let slot_count = b.read_u64_le() as usize;
        let storage = (0..slot_count)
            .map(|_| Node::parse_slot(b))
            .collect::<Result<Vec<Node>, String>>()?;
        Ok(Node::Contract { address, storage })
    }

    fn parse_slot(b: &mut BufView) -> Result<Node, String> {
        let t = b.read_u8();
        match t {
            1 => Node::parse_variable(b),
            2 => Node::parse_struct(b),
            3 => Node::parse_mapping(b),
            _ => Err(format!("unknown node type: {}", t)),
        }
    }

    fn parse_variable(b: &mut BufView) -> Result<Node, String> {
        let name_length = b.read_u64_le() as usize;
        let mut name = vec![0; name_length];
        b.read_bytes(&mut name);

        let mut value = vec![0; 32];
        b.read_bytes(&mut value);

        Ok(Node::Variable {
            name,
            value: value.try_into().unwrap(),
        })
    }

    fn parse_struct(b: &mut BufView) -> Result<Node, String> {
        todo!()
    }

    fn parse_mapping(b: &mut BufView) -> Result<Node, String> {
        let name_length = b.read_u64_le() as usize;
        let mut name = vec![0; name_length];
        b.read_bytes(&mut name);

        let slot_count = b.read_u64_le();
        let entries = (0..slot_count)
            .map(|_| Node::parse_entry(b))
            .collect::<Result<Vec<Node>, String>>()?;
        Ok(Node::Mapping { name, entries })
    }

    fn parse_entry(b: &mut BufView) -> Result<Node, String> {
        let mut k = vec![0; 32];
        b.read_bytes(&mut k);

        let v = Node::parse_slot(b)?;
        Ok(Node::Entry {
            key: k.try_into().unwrap(),
            entry: Box::new(v),
        })
    }

    pub fn hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        self._hash(&mut hasher);
        hasher.finalize().to_vec()
    }

    fn _hash(&self, h: &mut Sha256) {
        match self {
            Node::Block { number, contracts } => {
                Digest::update(h, &number.to_be_bytes());
                for x in contracts {
                    x._hash(h);
                }
            }
            Node::Contract { address, storage } => {
                Digest::update(h, address);
                for s in storage {
                    s._hash(h)
                }
            }
            Node::Variable { name, value } => {
                Digest::update(h, name);
                Digest::update(h, value);
            }
            Node::Struct { name, fields } => {
                Digest::update(h, name);
                for f in fields {
                    f._hash(h)
                }
            }
            Node::Mapping { name, entries } => {
                Digest::update(h, name);
                for e in entries {
                    e._hash(h)
                }
            }
            Node::Entry { key, entry } => {
                Digest::update(h, key);
                entry._hash(h);
            }
        }
    }

    #[cfg(feature = "std")]
    pub fn pretty(&self) {
        let mut r = String::new();
        self._pretty(0, &mut r);
    }

    #[cfg(feature = "std")]
    fn _pretty(&self, depth: usize, r: &mut String) {
        let indent = " ".repeat(depth);
        r.push_str(&indent);

        match self {
            Node::Block { number, contracts } => {
                r.push_str(&format!("Block #{number}\n"));
                for c in contracts {
                    c._pretty(depth + 2, r);
                }
            }
            Node::Contract { address, storage } => {
                r.push_str(&format!("Contract@{}\n", s(address)));
                for s in storage {
                    s._pretty(depth + 2, r);
                }
            }
            Node::Variable { name, value } => {
                r.push_str(&format!("{} -> {}\n", s(name), s(value)));
            }
            Node::Struct { name, fields } => todo!(),
            Node::Mapping { name, entries } => {
                r.push_str(&format!("{} :=\n", s(name)));
                for e in entries {
                    e._pretty(depth + 2, r);
                }
            }
            Node::Entry { key, entry } => {
                r.push_str(&format!("{} := \n", s(key)));
                entry._pretty(depth + 1, r);
            }
        }
    }
}

#[cfg(feature = "std")]
fn eword(x: &[u8]) -> EValue {
    let mut bs = x.to_vec();
    assert!(bs.len() <= 32);
    bs.resize(32, 0u8);
    bs.try_into().unwrap()
}

#[cfg(feature = "std")]
fn a(x: &[u8]) -> [u8; 20] {
    let mut bs = x.to_vec();
    assert!(bs.len() <= 20);
    bs.resize(20, 0u8);
    bs.try_into().unwrap()
}

#[cfg(feature = "std")]
fn strand(l: usize) -> ShittyString {
    Alphanumeric
        .sample_string(&mut rand::thread_rng(), l)
        .as_bytes()
        .to_vec()
}

#[cfg(feature = "std")]
fn make_var() -> Node {
    Node::Variable {
        name: strand(8),
        value: eword(&strand(32)),
    }
}

#[cfg(feature = "std")]
fn make_contract(count: usize) -> Node {
    Node::Contract {
        address: a(&strand(20)),
        storage: (0..count).map(|_| make_var()).collect(),
    }
}

#[cfg(feature = "std")]
pub fn random_tree(contract_count: usize, var_count: usize) -> Vec<u8> {
    let db = Node::Block {
        number: 125,
        contracts: (0..contract_count)
            .map(|_| make_contract(var_count))
            .collect(),
    };
    println!("The DB:");
    db.pretty();
    let mut out = BufWriter::new(Vec::new());
    db.serialize(&mut out);
    out.into_inner().unwrap()
}
