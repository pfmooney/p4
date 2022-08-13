use std::collections::HashSet;
use std::net::IpAddr;

use num::bigint::BigUint;
use num::ToPrimitive;
use serde::{Deserialize, Serialize};

// TODO transition from BigUint to BitVec<u8, Msb0>
#[derive(Debug, Clone, PartialEq, Hash, Eq, Serialize, Deserialize)]
pub enum Key {
    Exact(BigUint),
    Range(BigUint, BigUint),
    Ternary(Ternary),
    Lpm(Prefix),
}

impl Default for Key {
    fn default() -> Self {
        Self::Ternary(Ternary::default())
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Eq, Serialize, Deserialize)]
pub enum Ternary {
    DontCare,
    Value(BigUint),
    Masked(BigUint, BigUint),
}

impl Default for Ternary {
    fn default() -> Self {
        Self::DontCare
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Eq, Serialize, Deserialize)]
pub struct Prefix {
    pub addr: IpAddr,
    pub len: u8,
}

pub struct Table<const D: usize, A: Clone> {
    pub entries: HashSet<TableEntry<D, A>>,
}

impl<const D: usize, A: Clone> Table<D, A> {
    pub fn new() -> Self {
        Self {
            entries: HashSet::new(),
        }
    }

    pub fn match_selector(
        &self,
        keyset: &[BigUint; D],
    ) -> Vec<TableEntry<D, A>> {
        let mut result = Vec::new();
        for entry in &self.entries {
            if keyset_matches(keyset, &entry.key) {
                result.push(entry.clone());
            }
        }
        let sorted = sort_entries(result);
        sorted
    }

    pub fn dump(&self) -> String {
        let mut s = String::new();
        for e in &self.entries {
            s += &format!("{:?}\n", e.key)
        }
        s
    }
}

pub fn sort_entries<const D: usize, A: Clone>(
    mut entries: Vec<TableEntry<D, A>>,
) -> Vec<TableEntry<D, A>> {
    if entries.is_empty() {
        return entries;
    }

    // First determine how to sort. All the entries have the same keyset
    // structure, so it's sufficient to iterate over the keys of the first
    // entry. The basic logic we follow here is
    //
    // - If we find an lpm key sort on the prefix lenght of that dimension.
    // - Otherwise simply sort on priority.
    //
    // Notes:
    //
    // It's assumed that multiple lpm keys do not exist in a single keyset.
    // This is an implicit assumption in BVM2
    //
    //      https://github.com/p4lang/behavioral-model/issues/698
    //
    for (i, k) in entries[0].key.iter().enumerate() {
        match k {
            Key::Lpm(_) => {
                let mut entries = prune_entries_by_lpm(i, &entries);
                sort_entries_by_priority(&mut entries);
                return entries;
            }
            _ => continue,
        }
    }

    sort_entries_by_priority(&mut entries);
    entries
}

// TODO - the data structures here are quite dumb. The point of this table
// implemntation is not efficiency, it's a simple bruteish apprach that is easy
// to move around until we nail down the, tbh, rather undefined (in terms of p4
// spec)semantics of what the relative priorities between match types in a
// common keyset are.
pub fn prune_entries_by_lpm<const D: usize, A: Clone>(
    d: usize,
    entries: &Vec<TableEntry<D, A>>,
) -> Vec<TableEntry<D, A>> {
    let mut longest_prefix = 0u8;

    for e in entries {
        match &e.key[d] {
            Key::Lpm(x) => {
                if x.len > longest_prefix {
                    longest_prefix = x.len
                }
            }
            _ => {}
        }
    }

    let mut result = Vec::new();
    for e in entries {
        match &e.key[d] {
            Key::Lpm(x) => {
                if x.len == longest_prefix {
                    result.push(e.clone())
                }
            }
            _ => {}
        }
    }

    result
}

pub fn sort_entries_by_priority<const D: usize, A: Clone>(
    entries: &mut Vec<TableEntry<D, A>>,
) {
    entries
        .sort_by(|a, b| -> std::cmp::Ordering { b.priority.cmp(&a.priority) });
}

pub fn key_matches(selector: &BigUint, key: &Key) -> bool {
    match key {
        Key::Exact(x) => {
            //println!("{:x} =?= {:x}", selector, x);
            selector == x
        }
        Key::Range(begin, end) => selector >= begin && selector <= end,
        Key::Ternary(t) => match t {
            Ternary::DontCare => true,
            Ternary::Value(x) => selector == x,
            Ternary::Masked(x, m) => selector & m == x & m,
        },
        Key::Lpm(p) => {
            match p.addr {
                IpAddr::V6(addr) => {
                    assert!(p.len <= 128);
                    let key: u128 = addr.into();
                    //let key = key.to_be();
                    let mask = if p.len == 128 {
                        u128::MAX
                    } else {
                        ((1u128 << p.len) - 1) << (128 - p.len)
                    };
                    let selector_v6 = selector.to_u128().unwrap();
                    //let selector_v6 = selector_v6.to_be();
                    let result = selector_v6 & mask == key & mask;
                    //println!("{:x} & {:x} =?= {:x} & {:x}", selector_v6, mask, key, mask);
                    //println!("{:x} =?= {:x}", selector_v6 & mask, key & mask);
                    //println!("{}", result);
                    result
                }
                IpAddr::V4(addr) => {
                    assert!(p.len <= 32);
                    let key: u32 = addr.into();
                    let mask = if p.len == 32 {
                        u32::MAX
                    } else {
                        ((1u32 << p.len) - 1) << (32 - p.len)
                    };
                    let selector_v4: u32 = selector.to_u32().unwrap();
                    selector_v4 & mask == key & mask
                }
            }
        }
    }
}

pub fn keyset_matches<const D: usize>(
    selector: &[BigUint; D],
    key: &[Key; D],
) -> bool {
    for i in 0..D {
        if !key_matches(&selector[i], &key[i]) {
            return false;
        }
    }
    true
}

#[derive(Clone)]
pub struct TableEntry<const D: usize, A: Clone> {
    pub key: [Key; D],
    pub action: A,
    pub priority: u32,
    pub name: String,
}

// TODO: Cannot hash on just the key, this does not work for multipath.
impl<const D: usize, A: Clone> std::hash::Hash for TableEntry<D, A> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
    }
}

impl<const D: usize, A: Clone> std::cmp::PartialEq for TableEntry<D, A> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl<const D: usize, A: Clone> std::fmt::Debug for TableEntry<D, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!("TableEntry<{}>", D))
            .field("key", &self.key)
            .field("priority", &self.priority)
            .field("name", &self.name)
            .finish()
    }
}

impl<const D: usize, A: Clone> std::cmp::Eq for TableEntry<D, A> {}

#[cfg(test)]
mod tests {

    use super::*;
    use std::net::Ipv6Addr;

    fn contains_entry<const D: usize, A: Clone>(
        entries: &Vec<TableEntry<D, A>>,
        name: &str,
    ) -> bool {
        for e in entries {
            if e.name.as_str() == name {
                return true;
            }
        }
        false
    }

    fn tk(
        name: &str,
        addr: Ternary,
        ingress: Ternary,
        icmp: Ternary,
        priority: u32,
    ) -> TableEntry<3, ()> {
        TableEntry::<3, ()> {
            key: [
                Key::Ternary(addr),
                Key::Ternary(ingress),
                Key::Ternary(icmp),
            ],
            priority,
            name: name.into(),
            action: (),
        }
    }

    #[test]
    /// A few tests on the following table.
    ///
    /// +--------+-------------+--------------+---------+
    /// | Action | switch addr | ingress port | is icmp |
    /// +--------+-------------+--------------+---------+
    /// | a0     | true        | _            | true    |
    /// | a1     | true        | _            | false   |
    /// | a2     | _           | 2            | _       |
    /// | a3     | _           | 4            | _       |
    /// | a4     | _           | 7            | _       |
    /// | a5     | _           | 19           | _       |
    /// | a6     | _           | 33           | _       |
    /// | a7     | _           | 47           | _       |
    /// +--------+-------------+--------------+---------+
    fn match_ternary_1() {
        let table = Table::<3, ()> {
            entries: HashSet::from([
                tk(
                    "a0",
                    Ternary::Value(1u8.into()),
                    Ternary::DontCare,
                    Ternary::Value(1u8.into()),
                    10,
                ),
                tk(
                    "a1",
                    Ternary::Value(1u8.into()),
                    Ternary::DontCare,
                    Ternary::Value(0u8.into()),
                    1,
                ),
                tk(
                    "a2",
                    Ternary::DontCare,
                    Ternary::Value(2u8.into()),
                    Ternary::DontCare,
                    1,
                ),
                tk(
                    "a3",
                    Ternary::DontCare,
                    Ternary::Value(4u8.into()),
                    Ternary::DontCare,
                    1,
                ),
                tk(
                    "a4",
                    Ternary::DontCare,
                    Ternary::Value(7u8.into()),
                    Ternary::DontCare,
                    1,
                ),
                tk(
                    "a5",
                    Ternary::DontCare,
                    Ternary::Value(19u8.into()),
                    Ternary::DontCare,
                    1,
                ),
                tk(
                    "a6",
                    Ternary::DontCare,
                    Ternary::Value(33u8.into()),
                    Ternary::DontCare,
                    1,
                ),
                tk(
                    "a7",
                    Ternary::DontCare,
                    Ternary::Value(47u8.into()),
                    Ternary::DontCare,
                    1,
                ),
            ]),
        };

        //println!("M1 ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
        let selector =
            [BigUint::from(1u8), BigUint::from(99u16), BigUint::from(1u8)];
        let matches = table.match_selector(&selector);
        //println!("{:#?}", matches);
        assert_eq!(matches.len(), 1);
        assert!(contains_entry(&matches, "a0"));

        //println!("M2 ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
        let selector =
            [BigUint::from(1u8), BigUint::from(47u16), BigUint::from(1u8)];
        let matches = table.match_selector(&selector);
        //println!("{:#?}", matches);
        assert_eq!(matches.len(), 2);
        assert!(contains_entry(&matches, "a0"));
        assert!(contains_entry(&matches, "a7"));
        // check priority
        assert_eq!(matches[0].name.as_str(), "a0");
    }

    fn lpm(name: &str, addr: &str, len: u8) -> TableEntry<1, ()> {
        TableEntry::<1, ()> {
            key: [Key::Lpm(Prefix {
                addr: addr.parse().unwrap(),
                len: len,
            })],
            priority: 1,
            name: name.into(),
            action: (),
        }
    }

    #[test]
    /// A few tests on the following table.
    ///
    /// +--------+--------------------------+
    /// | Action | Prefix                   |
    /// +--------+--------------------------+
    /// | a0     | fd00:4700::/24           |
    /// | a1     | fd00:4701::/32           |
    /// | a2     | fd00:4702::/32           |
    /// | a3     | fd00:4701:0001::/48      |
    /// | a4     | fd00:4701:0002::/48      |
    /// | a5     | fd00:4702:0001::/48      |
    /// | a6     | fd00:4702:0002::/48      |
    /// | a7     | fd00:4701:0001:0001::/64 |
    /// | a8     | fd00:4701:0001:0002::/64 |
    /// | a9     | fd00:4702:0001:0001::/64 |
    /// | a10    | fd00:4702:0001:0002::/64 |
    /// | a11    | fd00:4701:0002:0001::/64 |
    /// | a12    | fd00:4701:0002:0002::/64 |
    /// | a13    | fd00:4702:0002:0001::/64 |
    /// | a14    | fd00:4702:0002:0002::/64 |
    /// | a15    | fd00:1701::/32           |
    /// +--------+----------------+---------+
    fn match_lpm_1() {
        let mut table = Table::<1, ()>::new();
        table.entries.insert(lpm("a0", "fd00:4700::", 24));
        table.entries.insert(lpm("a1", "fd00:4701::", 32));
        table.entries.insert(lpm("a2", "fd00:4702::", 32));
        table.entries.insert(lpm("a3", "fd00:4701:0001::", 48));
        table.entries.insert(lpm("a4", "fd00:4701:0002::", 48));
        table.entries.insert(lpm("a5", "fd00:4702:0001::", 48));
        table.entries.insert(lpm("a6", "fd00:4702:0002::", 48));
        table.entries.insert(lpm("a7", "fd00:4701:0001:0001::", 64));
        table.entries.insert(lpm("a8", "fd00:4701:0001:0002::", 64));
        table.entries.insert(lpm("a9", "fd00:4702:0001:0001::", 64));
        table
            .entries
            .insert(lpm("a10", "fd00:4702:0001:0002::", 64));
        table
            .entries
            .insert(lpm("a11", "fd00:4701:0002:0001::", 64));
        table
            .entries
            .insert(lpm("a12", "fd00:4701:0002:0002::", 64));
        table
            .entries
            .insert(lpm("a13", "fd00:4702:0002:0001::", 64));
        table
            .entries
            .insert(lpm("a14", "fd00:4702:0002:0002::", 64));
        table.entries.insert(lpm("a15", "fd00:1701::", 32));

        let addr: Ipv6Addr = "fd00:4700::1".parse().unwrap();
        let selector = [BigUint::from(u128::from_be_bytes(addr.octets()))];
        let matches = table.match_selector(&selector);
        //println!("{:#?}", matches);
        assert_eq!(matches.len(), 1);
        assert!(contains_entry(&matches, "a0"));

        let addr: Ipv6Addr = "fd00:4800::1".parse().unwrap();
        let selector = [BigUint::from(u128::from_be_bytes(addr.octets()))];
        let matches = table.match_selector(&selector);
        assert_eq!(matches.len(), 0);
        //println!("{:#?}", matches);

        let addr: Ipv6Addr = "fd00:4702:0002:0002::1".parse().unwrap();
        let selector = [BigUint::from(u128::from_be_bytes(addr.octets()))];
        let matches = table.match_selector(&selector);
        //println!("{:#?}", matches);
        assert_eq!(matches.len(), 1); // only one longest prefix match
        assert!(contains_entry(&matches, "a14"));
        // longest prefix first
        assert_eq!(matches[0].name.as_str(), "a14");
    }

    fn tlpm(
        name: &str,
        addr: &str,
        len: u8,
        zone: Ternary,
        priority: u32,
    ) -> TableEntry<2, ()> {
        TableEntry::<2, ()> {
            key: [
                Key::Lpm(Prefix {
                    addr: addr.parse().unwrap(),
                    len: len,
                }),
                Key::Ternary(zone),
            ],
            priority,
            name: name.into(),
            action: (),
        }
    }

    #[test]
    fn match_lpm_ternary_1() {
        let table = Table::<2, ()> {
            entries: HashSet::from([
                tlpm("a0", "fd00:1::", 64, Ternary::DontCare, 1),
                tlpm("a1", "fd00:1::", 64, Ternary::Value(1u16.into()), 10),
                tlpm("a2", "fd00:1::", 64, Ternary::Value(2u16.into()), 10),
                tlpm("a3", "fd00:1::", 64, Ternary::Value(3u16.into()), 10),
            ]),
        };

        let dst: Ipv6Addr = "fd00:1::1".parse().unwrap();
        let selector = [
            BigUint::from(u128::from_be_bytes(dst.octets())),
            BigUint::from(0u16),
        ];
        let matches = table.match_selector(&selector);
        println!("zone-0: {:#?}", matches);
        let selector = [
            BigUint::from(u128::from_be_bytes(dst.octets())),
            BigUint::from(2u16),
        ];
        let matches = table.match_selector(&selector);
        println!("zone-2: {:#?}", matches);
    }

    fn lpre(
        name: &str,
        addr: &str,
        len: u8,
        zone: Ternary,
        range: (u32, u32),
        tag: u64,
        priority: u32,
    ) -> TableEntry<4, ()> {
        TableEntry::<4, ()> {
            key: [
                Key::Lpm(Prefix {
                    addr: addr.parse().unwrap(),
                    len: len,
                }),
                Key::Ternary(zone),
                Key::Range(range.0.into(), range.1.into()),
                Key::Exact(tag.into()),
            ],
            priority,
            name: name.into(),
            action: (),
        }
    }

    struct ActionData {
        value: u64,
    }

    #[test]
    fn match_lpm_ternary_range() {
        let table = Table::<4, ()> {
            entries: HashSet::from([
                lpre("a0", "fd00:1::", 64, Ternary::DontCare, (80, 80), 100, 1),
                lpre(
                    "a1",
                    "fd00:1::",
                    64,
                    Ternary::DontCare,
                    (443, 443),
                    100,
                    1,
                ),
                lpre("a2", "fd00:1::", 64, Ternary::DontCare, (80, 80), 200, 1),
                lpre(
                    "a3",
                    "fd00:1::",
                    64,
                    Ternary::DontCare,
                    (443, 443),
                    200,
                    1,
                ),
                lpre(
                    "a4",
                    "fd00:1::",
                    64,
                    Ternary::Value(99u16.into()),
                    (443, 443),
                    200,
                    10,
                ),
            ]),
        };
        let dst: Ipv6Addr = "fd00:1::1".parse().unwrap();
        let selector = [
            BigUint::from(u128::from_be_bytes(dst.octets())),
            BigUint::from(0u16),
            BigUint::from(80u32),
            BigUint::from(100u64),
        ];
        let matches = table.match_selector(&selector);
        println!("m1: {:#?}", matches);

        let selector = [
            BigUint::from(u128::from_be_bytes(dst.octets())),
            BigUint::from(0u16),
            BigUint::from(443u32),
            BigUint::from(200u64),
        ];
        let matches = table.match_selector(&selector);
        println!("m2: {:#?}", matches);

        let selector = [
            BigUint::from(u128::from_be_bytes(dst.octets())),
            BigUint::from(99u16),
            BigUint::from(443u32),
            BigUint::from(200u64),
        ];
        let matches = table.match_selector(&selector);
        println!("m3: {:#?}", matches);

        let selector = [
            BigUint::from(u128::from_be_bytes(dst.octets())),
            BigUint::from(99u16),
            BigUint::from(80u32),
            BigUint::from(200u64),
        ];
        let matches = table.match_selector(&selector);
        println!("m4: {:#?}", matches);
    }

    #[test]
    fn match_with_action() {
        let mut data = ActionData { value: 47 };

        let table = Table::<1, Arc<dyn Fn(&mut ActionData)>> {
            entries: HashSet::from([
                TableEntry::<1, Arc<dyn Fn(&mut ActionData)>> {
                    key: [Key::Exact(1u8.into())],
                    priority: 0,
                    name: "a0".into(),
                    action: Arc::new(|a: &mut ActionData| {
                        a.value += 10;
                    }),
                },
                TableEntry::<1, Arc<dyn Fn(&mut ActionData)>> {
                    key: [Key::Exact(2u8.into())],
                    priority: 0,
                    name: "a1".into(),
                    action: Arc::new(|a: &mut ActionData| {
                        a.value -= 10;
                    }),
                },
            ]),
        };

        let selector = [BigUint::from(1u8)];
        let matches = table.match_selector(&selector);
        println!("m4: {:#?}", matches);
        assert_eq!(matches.len(), 1);
        (matches[0].action)(&mut data);
        assert_eq!(data.value, 57);
    }
}
