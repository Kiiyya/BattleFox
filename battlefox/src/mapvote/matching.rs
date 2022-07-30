//! Helper stuff for dealing with user input mapnames in the `!maetro 3 pe pr` thing.
//!
//! Maybe some day we'll have fuzzy matching too, that'd go here I imagine.
//!
//! Given a set of alternatives (map), we compute which numbers (1..6 usually) and minimal length
//! unique prefixes can match to each map.
//! We compute a hashmap once, mapping a string input to an alternative for a specific set of
//! alternatives.
//! This computation can depend on a previous set of AltMatchers, so that numbers (1..6) are
//! preserved between the maps which remain.

use std::{cmp::max, collections::{BTreeSet, HashMap, HashSet}, hash::Hash};
use itertools::Itertools;

use crate::mapmanager::pool::MapInPool;
use super::prefixes::shortest_unique_prefixes;

/// *Alternative Matcher*.
/// When a user-provided input string should map, e.g. `pe` from `pearl` or a number like `3` to
/// be voted on by users in e.g. `!metro pe pr 3`.
///
/// Given a set of alternatives (maps), we *once* decide which numeric ID the map gets.
/// And we compute the shortest unique prefix to the map, e.g. `pe` from `pearl` (when `propaganda`
/// also exists in the set of alternatives).
///
/// The number is meant to remain constant, but the minimal prefix length can vary when new
/// alternatives are added/removed.
#[derive(Debug, Clone, Copy)]
pub struct AltMatcher {
    /// A number commonly between 1 and 6, to enable players to vote with e.g. `!1 4 2 6`.
    pub number: usize,

    /// Minimal unique prefix length length.
    /// For example, for PearlMarket you have `pearl`, which with a `minlen` of 2 would result
    /// in `pe`, `pea`, `pear`, `pearl` matching, but not `p`.
    pub minlen: usize,
}

pub type AltMatchers = HashMap<MapInPool, AltMatcher>;

/// For a given set of alternatives, find [`AltMatcher`]s, i.e. a (mostly) random number usually
/// between 1 and 6, and compute the minimal unique prefix length.
///
/// Optionally, proving the old matchers will reuse the same numbers for the same maps, only
/// filling in new numbers for new alternatives.
///
/// If there is collisions with regards to vehicles or not, then the minlen is set to 0.
/// This is because we want to print both `pearl` and `pearl[INF]` in all lowercase.
pub fn to_matchers(
    alts: &(impl IntoIterator<Item = MapInPool, IntoIter = impl Iterator<Item = MapInPool>> + Clone),
    minlen: usize,
    reserved_trie: &HashSet<impl AsRef<str> /* + std::fmt::Debug */>,
    old_matchers: Option<&AltMatchers>) -> AltMatchers
{
    let collisions: HashSet<_> = alts.clone().into_iter()
        .map(|mip: MapInPool| mip.map.short())
        .duplicates()
        .collect();

    // dbg!(reserved_trie);
    // for every alternative, get the unique prefix lenghts.
    let mut prefixes = shortest_unique_prefixes(
        alts.clone().into_iter().map(|mip| mip.map.short()),
        reserved_trie.iter().map(|asref| asref.as_ref()),
    );

    // apply minlen
    prefixes.retain(|k, v| {
        *v = max(minlen, *v);

        // if there is a collision, we want to print both "pearl" and "pearl[INF]" in all lowercase.
        if collisions.contains(k) {
            *v = 0;
        }
        true
    });

    // allocate numbers, but respect the old numbers
    let mut taken_numbers = BTreeSet::new();
    // for every mip, check if there exists an old matcher, and if yes then add the number to
    // the taken numbers.
    let old_numbers = alts.clone().into_iter().filter_map(|mip|
        old_matchers
            .and_then(|om| om.get(&mip))
            .map(|m| m.number));
    old_numbers.for_each(|n| {let _ = taken_numbers.insert(n);});

    alts.clone().into_iter().map(|mip| (
        mip.clone(),
        AltMatcher {
            // use old number, or reserve a new number.
            number: old_matchers
                .and_then(|oms| oms.get(&mip))
                .map(|m| m.number)
                .unwrap_or_else(|| reserve_next_number(&mut taken_numbers)),
            // unwrap is safe because mip.map.short() exists as key in prefixes due to postcondition of shortest_unique_prefixes.
            minlen: *prefixes.get(&mip.map.short()).unwrap(),
        }
    )).collect()
}

pub type AltMatchersInv = HashMap<String, MapInPool>;

/// Computes a hashmap of strings users may enter (such as `1` or `pe` or `propaganda`) to
/// the corresponding alternative, for faster lookup when parsing vote chat messages.
/// In a way, "compiles" the matchers.
///
/// In order to enforce minimal length of prefixes or some blocked commands (e.g. collission of
/// `!p` for punish and pearl market), you need to filter the resulting hashmap yourself.
pub fn matchers_to_matchmap(matchers: &AltMatchers) -> AltMatchersInv {
    let mut ret = HashMap::new();
    for (mip, mat) in matchers {
        // add number, e.g. `3`.
        ret.insert(mat.number.to_string(), mip.clone());

        // add prefixes of length minlen, minlen+1, minlen+2, ...
        for prefix_len in mat.minlen .. mip.map.short().len() {
            // the [..prefix_len] operates on bytes and not on unicode graphemes/chars/whatever, and
            // as such may panic. But we only deal with ASCII, so in this case it's fine.
            ret.insert(mip.map.short()[..prefix_len].to_string(), mip.clone());
        }

        // all the rest of the map shortnames, which may overwrite some of the prefixes, but that's
        // fine since they will point to the same value anyway.
        ret.extend(mip.map.short_names().map(|x| (x.to_string(), mip.clone())));
    }

    ret
}

// /// Makes sure (by removal) that all keys, prefixes, etc have the minimal length, and
// /// aren't on the banlist.
// /// Numbers are obviously not affected by minlen.
// pub fn matchmap_restrict(matchmap: &mut AltMatchersInv, minlen: usize, reserved_hidden: &HashSet<String>) {
//     matchmap.retain(|k, v| {
//         match (reserved_hidden.contains(k), k.parse::<usize>().is_ok(), k.len() >= minlen) {
//             (true, true, _) => {
//                 error!("The voting number \"!{}\" is blocked from the vote options! This is not a good idea. Context: blocked = {:?}", k, &reserved_hidden);
//                 false // reject nevertheless
//             },
//             (true, false, _) => false, // reject blocked keys
//             (false, true, _) => true, // pass any numbers, irrespective minlen
//             (false, false, longenough) => longenough, // otherwise, reject when too short
//         }
//     });
// }

/// Makes sure (by removal) that all keys, prefixes, etc have the minimal length, and
/// aren't on the banlist.
pub fn matchmap_restrict(matchmap: &mut AltMatchersInv, reserved_hidden: &HashSet<String>) {
    matchmap.retain(|k, v| {
        match (reserved_hidden.contains(k), k.parse::<usize>().is_ok()) {
            (true, true) => {
                error!("The voting number \"!{}\" is blocked from the vote options! This is not a good idea. Context: blocked = {:?}", k, &reserved_hidden);
                false // reject nevertheless
            },
            (true, false) => false, // reject blocked keys
            (false, _) => true,
        }
    });
}

/// Given a set of already used numbers, finds and allocates the minimal free one. That is, bigger
/// than 1.
fn reserve_next_number(taken : &mut BTreeSet<usize>) -> usize {
    for i in 1.. {
        if !taken.contains(&i) {
            taken.insert(i);
            return i;
        }
    }
    unreachable!()
}
