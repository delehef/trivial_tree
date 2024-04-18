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
pub type EAddress = [u8; 20];

pub enum Node<const HASH_LEN: usize = 32> {
    Inner { children: Vec<Node> },
    Leaf { key: EAddress, value: EValue },
    HashedSubTree { hash: [u8; HASH_LEN] },
}

impl<const HASH_LEN: usize> Node<HASH_LEN> {
    fn to_id(&self) -> u8 {
        match self {
            Node::Inner { .. } => 1,
            Node::Leaf { .. } => 2,
            Node::HashedSubTree { .. } => 3,
        }
    }

    #[cfg(feature = "std")]
    pub fn random_tree(contract_count: usize, leaf_count: usize) -> Node {
        let db = Node::Inner {
            children: (0..contract_count)
                .map(|_| make_inner(leaf_count))
                .collect(),
        };
        println!("The DB:");
        db.pretty();
        db
    }

    #[cfg(feature = "std")]
    pub fn serialize(&self) -> Vec<u8> {
        let mut out = BufWriter::new(Vec::new());
        self.serialize_into(&mut out);
        out.into_inner().unwrap()
    }

    #[cfg(feature = "std")]
    fn serialize_into<W: std::io::Write>(&self, out: &mut W) {
        match self {
            Node::Inner { children } => {
                out.write_all(&[self.to_id()]).unwrap();
                out.write_all(&children.len().to_le_bytes()).unwrap();
                for c in children {
                    c.serialize_into(out);
                }
            }
            Node::Leaf { key, value } => {
                out.write_all(&[self.to_id()]).unwrap();
                out.write_all(key).unwrap();
                out.write_all(value).unwrap();
            }
            Node::HashedSubTree { hash } => {
                out.write_all(&[self.to_id()]).unwrap();
                out.write_all(hash).unwrap();
            }
        }
    }

    pub fn parse(b: &mut BufView) -> Result<Node, String> {
        match b.read_u8() {
            1 => Self::parse_inner(b),
            2 => Self::parse_leaf(b),
            3 => Self::parse_hash(b),
            id @ _ => Err(format!("unknown node type: {id}")),
        }
    }

    fn parse_inner(b: &mut BufView) -> Result<Node, String> {
        let children_count = b.read_u64_le() as usize;
        let children = (0..children_count)
            .map(|_| Self::parse(b))
            .collect::<Result<Vec<Node>, String>>()?;

        Ok(Node::Inner { children })
    }

    fn parse_leaf(b: &mut BufView) -> Result<Node, String> {
        let mut key = vec![0; 20];
        b.read_bytes(&mut key);

        let mut value = vec![0; 32];
        b.read_bytes(&mut value);

        Ok(Node::Leaf {
            key: key.try_into().unwrap(),
            value: value.try_into().unwrap(),
        })
    }

    fn parse_hash(b: &mut BufView) -> Result<Node, String> {
        let mut hash = vec![0; HASH_LEN];
        b.read_bytes(&mut hash);
        Ok(Node::HashedSubTree {
            hash: hash.try_into().unwrap(),
        })
    }

    pub fn hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        self._hash(&mut hasher);
        hasher.finalize().to_vec()
    }

    fn _hash(&self, h: &mut Sha256) {
        match self {
            Node::Inner { children } => {
                for x in children {
                    x._hash(h);
                }
            }
            Node::Leaf { key, value } => {
                Digest::update(h, key);
                Digest::update(h, value);
            }
            Node::HashedSubTree { hash } => Digest::update(h, hash),
        }
    }

    #[cfg(feature = "std")]
    pub fn pretty(&self) {
        let mut r = String::new();
        self._pretty(0, &mut r);
        println!("{r}");
    }

    #[cfg(feature = "std")]
    fn _pretty(&self, depth: usize, r: &mut String) {
        let indent = " ".repeat(depth * 2);
        r.push_str(&indent);

        match self {
            Node::Inner { children } => {
                r.push_str(&format!("Inner-{depth}\n"));
                for c in children {
                    c._pretty(depth + 1, r);
                }
            }
            Node::Leaf { key, value } => {
                r.push_str(&format!("{} -> {}\n", s(key), s(value)));
            }
            Node::HashedSubTree { hash } => r.push_str(&format!("H := {}\n", s(hash))),
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
fn make_inner(count: usize) -> Node {
    Node::Inner {
        children: (0..count)
            .map(|_| Node::Leaf {
                key: a(&strand(20)),
                value: eword(&strand(32)),
            })
            .collect(),
    }
}
