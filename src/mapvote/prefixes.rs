//! Helper for finding sh children: (), value: (), count: ()ortest unique prefixes of a set of strings.

use std::{collections::{HashMap, HashSet}, fmt::Debug, hash::Hash};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Node<V> {
    children: HashMap<char, Node<V>>,
    value: Option<V>,
    count: usize,
}

impl <V: Debug> Node<V> {
    fn new() -> Self {
        Self {
            children: HashMap::new(),
            value: None,
            count: 0,
        }
    }

    fn insert(&mut self, key: &str, value: V) -> Option<V> {
        if key.is_empty() {
            if self.value.is_none() {
                self.count += 1;
            }
            self.value.replace(value)
        } else {
            // split key into (head :: tail).
            let mut iter = key.chars();
            let head = iter.next().unwrap();
            let tail = iter.as_str();

            let child = self.children.entry(head).or_insert_with(Node::new);
            let result = child.insert(tail, value);
            // only increment the count when we have inserted a fully new element.
            // replacing elements returns Some(_), count doesn't change then.
            // we need to increase the length along the whole path from root to leaf.
            if result.is_none() {
                self.count += 1;
            }
            result
        }
    }

    /// Get the leaf at the end of the (single-path) tree.
    /// # Returns
    /// - Some(leaf) when only a single value exists in this tree, and it is at the very end.
    /// - None otherwise.
    fn get_leaf(&self) -> Option<&Node<V>> {
        if self.count() == 1 && !self.children.is_empty(){
            self.children.values().next().unwrap().get_leaf()
        } else if self.count() == 1 && self.children.is_empty(){
            Some(self)
        } else {
            None
        }
    }

    /// Gets the value of *this node*, if any.
    fn get(&self) -> Option<&V> {
        self.value.as_ref()
    }

    /// Counts how many elements are in this node.
    fn count(&self) -> usize {
        let x = self.value.as_ref().map_or(0usize, |_| 1usize);
        let y : usize = self.children.iter().map(|(_, child)| child.count()).sum();
        let count_real = x + y;
        assert_eq!(count_real, self.count); // TODO: move into test.
        self.count
    }

    /// Collect all branches at the earliest point where only one value is in the subtree,
    /// and additionally record at which depth that is (root = 0).
    fn leaf_branches<'a>(&'a self, depth: usize, collect: &mut Vec<(usize, &'a Node<V>)>) {
        if self.children.is_empty() {
            collect.push((depth, self));
        } else {
            // first, submit any leaf-like branches into `collect` of this node.
            // do not descend further.
            self.children.values()
                .filter(|n| n.count() == 1)
                .for_each(|n| collect.push((depth + 1, n)));

            // then, branch out into any not-leaf-like branches.
            self.children.values()
                .filter(|n| n.count() != 1)
                .for_each(|n| n.leaf_branches(depth + 1, collect));
        }
    }

    /// Go through **all** (even non-key) nodes.
    fn traverse<'a>(&'a self, f: &mut impl FnMut(&'a Node<V>)) {
        f(self);
        for child in self.children.values() {
            child.traverse(f);
        }
    }
}

#[derive(Debug)]
enum Type<'a> { Set(&'a str), Blocked }

/// Shortens each element in `set` so that it is still unique.
/// # Panics
/// When `blocked` contains an element of the set.
pub fn shortest_unique_prefixes<'a, 'b>(set: impl Iterator<Item = &'a str>, minlen: usize, blocked: impl Iterator<Item = &'b str>, extend: bool) -> HashSet<&'a str> {
    let mut trie = Node::new();
    set.for_each(|s| {
        trie.insert(s, Type::Set(s));
    });
    blocked.for_each(|s| {
        if trie.insert(s, Type::Blocked).is_some() {
            panic!("cannot block prefixes which are in the search set. collision: {}", s);
        }
    });

    let mut branches = Vec::new();
    trie.leaf_branches(0, &mut branches);
    let mut ret = HashSet::new();
    for (depth, branch) in branches {
        // depth is the shortest unique length of our prefix.
        let leaf = branch.get_leaf().unwrap(); // unwrap safe: postcondition of leaf_branches().
        match leaf.get().unwrap() { // unwrap safe: leafs always contain a value.
            Type::Set(str) => {
                assert_eq!(str.len(), str.chars().count()); // TODO: This code assumes ASCII, bad.
                let x = std::cmp::max(depth, minlen);
                if extend {
                    // add stuff like "p", "pe", "pea", "pearl".
                    for i in x..=str.len() {
                        let y = &str[0..i];
                        // println!("i={}, y={}", i, y);
                        ret.insert(y);
                    }
                } else {
                    ret.insert(&str[0..x]);
                }
            }
            Type::Blocked => {
                // ignore
            }
        }
    }

    ret
}

#[cfg(test)]
mod test {
    use super::{Node, shortest_unique_prefixes};

    #[test]
    fn re_add() {
        let mut root = Node::new();
        assert_eq!(0, root.count());
        assert!(root.insert("a", 123).is_none());
        assert_eq!(1, root.count());
        assert!(root.insert("a", 123).is_some());
        assert_eq!(1, root.count());
    }

    #[test]
    fn get_leaf() {
        let mut root = Node::new();
        assert!(root.get_leaf().is_none());
        root.insert("a", 123);
        assert_eq!(Some(&123), root.get_leaf().unwrap().get());
        root.insert("ab", 123);
        dbg!(&root);
        assert_eq!(2, root.count());
        assert!(root.get_leaf().is_none());
    }

    #[test]
    fn get_leaf2() {
        let mut root = Node::new();
        assert!(root.get_leaf().is_none());
        root.insert("a", 123);
        assert_eq!(Some(&123), root.get_leaf().unwrap().get());
        root.insert("b", 123);
        dbg!(&root);
        assert_eq!(2, root.count());
        assert!(root.get_leaf().is_none());
    }

    #[test]
    fn leaf_branches() {
        let mut root = Node::new();
        let mut collect = Vec::new();
        root.leaf_branches(0, &mut collect);
        assert_eq!(collect.len(), 1);

        root.insert("aaab", 111);
        root.insert("aaacd", 222);
        root.insert("e", 333);
        let mut collect = Vec::new();
        root.leaf_branches(0, &mut collect);
        let aaab = Node {
            children: hashmap!{},
            value: Some(111),
            count: 1,
        };
        let aaac = Node {
            children: hashmap!{
                'd' => Node {
                    children: hashmap! {},
                    value: Some(222),
                    count: 1,
                }
            },
            value: None,
            count: 1,
        };
        let e = Node {
            children: hashmap!{},
            value: Some(333),
            count: 1,
        };

        dbg!(&collect);

        assert!(collect.contains(&(4, &aaab)));
        assert!(collect.contains(&(4, &aaac)));
        assert!(collect.contains(&(1, &e)));
    }

    #[test]
    fn shortest_prefixes() {
        let src = vec!["pearl", "prop", "met"];
        let none : Vec<&str> = Vec::new();
        let x = shortest_unique_prefixes(src.iter().copied(), 0, none.iter().copied(), true);

        let should = hashset! {
            "pe", "pea", "pear", "pearl",
            "pr", "pro", "prop",
            "m", "me", "met",
        };

        assert_eq!(should, x);
    }

    #[test]
    fn shortest_prefixes2() {
        let src = vec!["pearl", "prop", "met"];
        let none : Vec<&str> = Vec::new();
        let x = shortest_unique_prefixes(src.iter().copied(), 2, none.iter().copied(), true);

        let should = hashset! {
            "pe", "pea", "pear", "pearl",
            "pr", "pro", "prop",
            "me", "met",
        };

        assert_eq!(should, x);
    }

    #[test]
    fn shortest_prefixes3() {
        let src = vec!["pearl", "met"];
        let none : Vec<&str> = Vec::new();
        let x = shortest_unique_prefixes(src.iter().copied(), 0, vec!["pe"].iter().copied(), true);

        let should = hashset! {
            "pea", "pear", "pearl",
            "m", "me", "met",
        };

        assert_eq!(should, x);
    }

    #[test]
    fn shortest_prefixes_nonextend() {
        let src = vec!["pearl", "met"];
        let none : Vec<&str> = Vec::new();
        let x = shortest_unique_prefixes(src.iter().copied(), 0, vec!["pe"].iter().copied(), false);

        let should = hashset! {
            "pea",
            "m",
        };

        assert_eq!(should, x);
    }
}
