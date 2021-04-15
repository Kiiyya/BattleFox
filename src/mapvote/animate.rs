//! Animating STV in BF4 chat.
//!
//! Given a Profile, first get the log with the personal vote distribution history,
//! and then format it into ascii strings specific for BF4.

use std::{collections::HashMap, time::Instant};

use battlefield_rcon::bf4::{Map, Player};
use itertools::Itertools;
use num_rational::BigRational as Rat;
use num_traits::One;
use crate::{mapmanager::pool::MapInPool, stv::{Profile, tracing::{AnimTracer, HistEntry, LoggingTracer}}};

fn usize_of_rat(rat: &Rat) -> usize {
    if !rat.is_integer() {
        warn!("{} is not an integer. Rounding towards zero and continuing..", rat);
    }
    let (sign, digits) = rat.to_integer().to_u64_digits();
    *digits.get(0).unwrap_or(&0) as usize
}

type Bars = HashMap<MapInPool<()>, String>;

/// Make sure `alts_all` is the alternatives we *started* with, not the current iteration's
/// alternatives.
/// ```raw
/// Metro   xxxx
/// Locker  xx+
/// Pearl   x
/// ```
fn render_bars_diff(alts_start: &[MapInPool<()>], previous: Option<&Profile<MapInPool<()>>>, profile: &Profile<MapInPool<()>>) -> Bars {
    let mut ret = Bars::new();
    for alt in alts_start {
        if profile.alts.contains(alt) {
            let score = usize_of_rat(&profile.score(alt));
            let score_previous = previous.map(|p| usize_of_rat(&p.score(alt))).unwrap_or(score);
            assert!(score_previous <= score);

            ret.insert(alt.clone(), format!("{}{}{}",
                alt.map.map_constlen_tabbed(),
                "=".repeat(score_previous),
                "+".repeat(score - score_previous),
            ));
        } else {
            ret.insert(alt.clone(), format!("({})", alt.map.short()));
        }
    }
    ret
}

fn render_bars_sequence(alts_start: &[MapInPool<()>], tracer: &AnimTracer<Player, MapInPool<()>>) -> Vec<Bars> {
    let mut vec = Vec::new();
    let mut previous = None;
    for stage in tracer.log_iter() {
        match stage {
            HistEntry::Starting { profile, assignment } => {
                vec.push(render_bars_diff(alts_start, None, profile));
                previous = Some(profile);
            }
            HistEntry::Elim { profile, assignment, a } => {
                vec.push(render_bars_diff(alts_start, previous, profile));
                previous = Some(profile);
            }
            HistEntry::Elect { profile, assignment, elected } => {
                vec.push(render_bars_diff(alts_start, previous, profile));
                previous = Some(profile);
            }
        }
    }
    vec
}

/// Render the STV calculation animation.
///
/// # Returns
/// For each player:
/// - A list of frames (one frame == one `admin.say` message).
///
/// # Panics
/// - When an alternative in `alts_start` does not have an associated bar in `bars`.
/// - When a `Distr::get_single()` panics.
pub fn stv_anim_frames<'a>(alts_start: &[MapInPool<()>], players: impl Iterator<Item = &'a Player>, tracer: &AnimTracer<Player, MapInPool<()>>) -> HashMap<Player, Vec<String>> {

    let time = Instant::now();
    let mut ret = HashMap::new();
    let bars_seq = render_bars_sequence(alts_start, tracer);

    let players = players.collect_vec();
    trace!("Players: {}", players.iter().map(|p| &p.name).join(", "));

    for player in players {
        let x = tracer
            .log_iter();
            // .map(|hist| hist.get_assignment().get(player).unwrap().get_single());

        let mut player_frames = Vec::new();

        for (bars, hist_entry) in bars_seq.iter().zip_eq(x) {
            let mut lines = Vec::new();
            let frame = match hist_entry {
                HistEntry::Starting { profile, assignment } => {
                    lines.push("Starting with:".to_string());
                    let your_vote = assignment.get(player)
                        .and_then(|distr| distr.get_single());
                    render_frame(&mut lines, alts_start, bars, your_vote);
                }
                HistEntry::Elim { profile, assignment, a } => {
                    lines.push(format!("Eliminated {}:", a.map.Pretty()));
                    let your_vote = assignment.get(player)
                        .and_then(|distr| distr.get_single());
                    render_frame(&mut lines, alts_start, bars, your_vote);
                }
                HistEntry::Elect { elected, profile, assignment } => {
                    assert_eq!(1, elected.len());
                    let winner = elected.iter().next().unwrap();
                    lines.push(format!("Mapvote winner: {}", winner.map.Pretty()));
                }
            };
        }

        ret.insert(player.clone(), player_frames);
    }
    let elapsed = time.elapsed();
    trace!("Needed {}ms to generate animation", elapsed.as_millis());

    ret
}

fn render_frame(lines: &mut Vec<String>, alts_start: &[MapInPool<()>], bars: &HashMap<MapInPool<()>, String>, your_vote: Option<(&MapInPool<()>, &Rat)>) {
    for alt in alts_start {
        let mut line = String::new();
        if let Some((vote, weight)) = your_vote {
            if vote == alt {
                if weight == &(Rat::one() + Rat::one()) {
                    line += "You x2 ->\t";
                } else {
                    line += "   You ->\t";
                }
            }
        } else {
            line += "\t\t";
        }

        line += bars.get(alt).unwrap();
        lines.push(line);
    }
}