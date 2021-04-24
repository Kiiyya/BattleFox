#![allow(unused_variables, unused_imports)]

use crate::{guard::{
        recent::Age::{Old, Recent},
        Cases, Guard,
    }, mapmanager::{
        pool::{MapInPool, MapPool, Vehicles, VehiclesSpecified},
        CallbackResult, MapManager, PopState,
    }, players::Players, stv::{CheckBallotResult, Profile, tracing::{Assignment, DetailedTracer, Distr}}, vips::{MaybeVip, Vips, YesVip}};

use self::config::MapVoteConfig;

use super::stv::tracing::{NoTracer, StvAction, LoggingTracer, AnimTracer};
use super::stv::Ballot;
use ascii::{AsciiString, IntoAsciiString};
use battlefield_rcon::{bf4::{Bf4Client, Event, GameMode, Map, Player, Visibility, error::{Bf4Error, Bf4Result}, wrap_msg_chars}, rcon::{RconError, RconResult}};
use either::Either::{Left, Right};
use futures::{future::join_all, StreamExt};
use itertools::Itertools;
use std::hash::Hash;
use std::{
    any::Any,
    collections::{HashMap, HashSet},
    fmt::Display,
    future::Future,
    pin::Pin,
    sync::{Arc, Weak},
    time::Duration,
};
use std::{cmp::Ordering, fmt::Debug};
use tokio::{
    sync::Mutex,
    time::{sleep, Interval},
};

use num_rational::BigRational as Rat;
use num_traits::One;

pub mod config;
mod animate;
mod prefixes;

#[derive(Debug)]
struct Inner {
    alternatives: MapPool<()>,
    /// Invariant: All ballots have at least one option on them.
    votes: HashMap<Player, Ballot<MapInPool<()>>>,

    pop_state: PopState<Vehicles>,

    nominations: HashMap<Guard<Player, YesVip>, HashSet<Map>>,

    anim_override_override: HashMap<Player, bool>,
}

#[derive(Debug)]
pub struct Mapvote {
    inner: Mutex<Option<Inner>>,
    mapman: Arc<MapManager>,
    vips: Arc<Vips>,
    players: Arc<Players>,
    config: MapVoteConfig,
}

impl Inner {
    pub fn to_profile(&self) -> Profile<MapInPool<()>> {
        Profile {
            alts: self.alternatives.to_set(),
            ballots: self.votes.values().cloned().collect(),
        }
    }

    /// # Panics
    /// When any ballot does not have a first preference, panics.
    /// This should never happen since it's an invariant that all ballots must have at least
    /// one preference
    pub fn to_assignment(&self) -> Assignment<Player, MapInPool<()>> {
        self.votes.iter().map(|(player, ballot)| {
            (player.clone(), Distr::single(ballot.preferences[0].clone(), ballot.weight.clone()))
        }).collect()
    }

    /// Gets the amount of nominations that the VIP has done this round.
    fn vip_n_noms(&self, vip: &Guard<Player, YesVip>) -> usize {
        if let Some(v) = self.nominations.get(vip) {
            v.len()
        } else {
            0
        }
    }

    fn vip_nom(&mut self, vip: &Guard<Player, YesVip>, map: Map) {
        if let Some(v) = self.nominations.get_mut(vip) {
            v.insert(map);
        } else {
            let mut hs = HashSet::new();
            hs.insert(map);
            self.nominations.insert(vip.to_owned(), hs);
        }
        self.alternatives.pool.push(MapInPool {
            map,
            mode: GameMode::Rush,
            extra: (),
        });
    }

    /// part of what gets printed when a person types in `!v`, but also on spammer, etc.
    fn fmt_options(&self, messages: &mut Vec<String>) {
        let mut msg = format!("Vote with numbers or names:\n{}", 5);

        // let x = self
        //     .alternatives
        //     .pool
        //     .iter()
        //     .map(|alt| alt.map.Pretty())
        //     .join(", ");
        // lines.push(format!("Options: {}", x));
    }

    /// part of what gets printed when a person types in `!v`, but also on spammer, etc.
    fn fmt_personal_status(&self, lines: &mut Vec<String>, perspective: &Player) {
        if let Some(ballot) = self.votes.get(perspective) {
            if ballot.preferences.len() >= 2 {
                // nice
                lines.push(format!("Your ballot: {}", ballot));
                // lines.push("You can still change your ballot.".to_string());
            } else {
                let single = ballot.preferences.first().unwrap();
                // person only voted for a single alternative, tell them how to do it better.
                // first unwrap: safe, assumes ballots.length() >= 1. That is an invariant.
                lines.push(format!("You only voted for a single map ({}), but you can specify multiple preferences here!",
                    ballot.preferences.first().unwrap().map.Pretty()));

                // construct a random example vote, but where the first vote is the same as the
                // person had already voted.
                let suggestion_tail_pool = self.pop_state.pool.without(single.map).choose_random(2);
                let mut suggestion_pref = vec![single.to_owned()];
                suggestion_pref.append(&mut suggestion_tail_pool.extra_remove().pool);
                let suggestion_string = suggestion_pref.iter().map(|mip| mip.map.short()).join(" ");

                lines.push(format!("Try it: !{}", suggestion_string));
            }
        } else {
            // person hasn't voted yet at all.
            let suggestion = self.alternatives.choose_random(3);
            let suggestion_str = suggestion.pool.iter().map(|mip| mip.map.short()).join(" ");
            lines.push(format!("Vote like this: !{}", suggestion_str));
        }
    }

    // fn fmt_personal_vip_status(
    //     &self,
    //     lines: &mut Vec<String>,
    //     perspective: &Guard<Player, YesVip>,
    // ) {
    //     // TODO: put the Leader and Runner-up in here.
    // }

    fn set_up_new_vote(&mut self, n: usize) {
        self.alternatives = self.pop_state.pool.choose_random(n).extra_remove();
        self.votes.clear();
        self.nominations.clear();
        let mut pool = self.pop_state.pool.pool.iter().map(|mip| mip.map.short());
        let mut options = self.alternatives.pool.iter().map(|mip| mip.map.short());
        debug!(
            "I've set up a new vote with pool {}: [{}], so options are [{}].",
            self.pop_state.name,
            pool.join(", "),
            options.join(", ")
        );
    }
}

/// When a user votes, they can still fuck up :)
#[derive(Debug, Clone)]
enum VoteResult<E: Eq + Clone> {
    Ok {
        new: Ballot<MapInPool<E>>,
        old: Option<Ballot<MapInPool<E>>>,

        /// User submitted duplicate votes, but they were continuously together, and thus could be
        /// contracted into one. Emit warning, but accept vote.
        soft_dups: HashSet<MapInPool<E>>,

        not_in_options: MapPool<E>,

        not_in_popstate: MapPool<E>,
    },

    /// User submitted duplicates but they could not be untangled. Need to retry.
    UnresolvableDuplicate { problem: MapInPool<E> },

    // /// A map which the user voted on is not in the current map pool.
    // NotInPopstate { missing: HashSet<Map> },

    // /// A map is in the current pool, but is not up for vote.
    // ///
    // /// It can be nominated though.
    // NotInOptions { missing: HashSet<Map> },

    /// For some reason, managed to pass a list with zero options...
    Empty {
        not_in_options: MapPool<E>,
        not_in_popstate: MapPool<E>,
    },

    /// There is no vote currently ongoing, this may be because:
    /// - Mapvote is currently in initialization phase
    /// - In the future: maybe have it disable at round start.
    Inactive,
}

enum NomMapParseResult {
    Ok(Map),
    Empty,
    Other,
}

impl Mapvote {
    /// Creates a new instance of `MapVote`, but doesn't start it yet, just sets stuff up.
    pub async fn new(
        mapman: Arc<MapManager>,
        vips: Arc<Vips>,
        players: Arc<Players>,
        config: MapVoteConfig,
    ) -> Arc<Self> {
        let myself = Arc::new(Self {
            inner: Mutex::new(None),
            //     Inner {
            //     alternatives: MapPool::new(),
            //     votes: HashMap::new(),
            //     pop_state: mapman.pop_state().await,
            // }),
            mapman: mapman.clone(),
            vips,
            players,
            config,
        });


        // trace!("before setting up ugly callback");

        // holy shit this is ugly.
        let myself_weak = Arc::downgrade(&myself);
        mapman
            .register_pool_change_callback(move |bf4, popstate| {
                let weak = myself_weak.clone();
                Box::pin(async move {
                    // try to upgrade to strong Arc<MapVote>. If that fails, it means the mapvote
                    // instance was dropped. In that case, RemoveMe.
                    if let Some(strong) = weak.upgrade() {
                        trace!("popstate changed!");
                        tokio::spawn(async move {
                            strong.on_popstate_changed(bf4, popstate).await;
                        });
                        CallbackResult::KeepGoing
                    } else {
                        CallbackResult::RemoveMe
                    }
                })
            })
            .await;

        // trace!("after setting up ugly callback");

        myself
    }

    async fn on_popstate_changed(&self, bf4: Arc<Bf4Client>, popstate: PopState<Vehicles>) {
        let mut lock = self.inner.lock().await;
        if let Some(inner) = &mut *lock {
            info!("Popstate changed! New: {}", popstate.name);

            let mut futures = Vec::new();
            let direction = PopState::change_direction(&inner.pop_state, &popstate);
            match direction {
                Ordering::Less => {
                    debug!(
                        "Mapman: PopState downgrade from {} to {}",
                        inner.pop_state.name, popstate.name
                    );
                }
                Ordering::Equal => {
                    debug!("Uhhh, popstate didn't change direction? Wot.");
                    return; // or maybe panic instead...?
                }
                Ordering::Greater => {
                    // TODO: Notify every single VIP individually that they can nominate more now.
                    debug!(
                        "Mapman: PopState upgrade from {} to {}",
                        inner.pop_state.name, popstate.name
                    );
                }
            }

            let removals = dbg!(MapPool::removals(&inner.pop_state.pool, &popstate.pool));
            let additions = dbg!(MapPool::additions(&inner.pop_state.pool, &popstate.pool));

            // first, remove the current voting options fittingly and choose replacements.
            let alternatives_removals = inner
                .alternatives // removals is old pop -> current pop, but what about our current options?
                .intersect(&removals.extra_remove())
                .to_mapset();
            let alternatives_additions = if inner.alternatives.pool.len() < self.config.n_options {
                popstate
                    .pool
                    // .without_many(&removed_alternatives)
                    .without_many(&inner.alternatives.to_mapset())
                    .choose_random(self.config.n_options - alternatives_removals.len())
            } else {
                MapPool::new()
            };

            dbg!(&alternatives_removals);
            dbg!(&alternatives_additions);

            // actually remove and replace the alternatives.
            inner
                .alternatives
                .pool
                .retain(|mip| popstate.pool.contains_mapmode(mip.map, &mip.mode));
            inner
                .alternatives
                .pool
                .append(&mut alternatives_additions.clone().extra_remove().pool);

            // and then inform players
            if alternatives_removals.len() == 1 {
                // special case so that the messages are nicer.
                if alternatives_additions.pool.len() == 1 {
                    futures.push(bf4.say_lines(
                        vec![
                            "Server population shrunk, and with it the map pool.".to_string(),
                            format!(
                                "{} has been removed from voting options, and replaced with {}",
                                alternatives_removals.iter().next().unwrap().Pretty(), // unwrap: safe because we tested len to be 1.
                                alternatives_additions.pool[0].map.Pretty() // index: safe because we tested len to be 1.
                            ),
                        ],
                        Visibility::All,
                    ));
                } else if alternatives_additions.pool.is_empty() {
                    futures.push(bf4.say_lines(vec![
                        "Server population shrunk, and with it the map pool.".to_string(),
                        format!("{} has been removed from voting options, and no map is available to replace it",
                            alternatives_removals.iter().next().unwrap().Pretty()), // unwrap: safe because we tested len to be 1.
                    ], Visibility::All));
                } else {
                    assert!(alternatives_additions.pool.len() >= 2);
                    futures.push(
                        bf4.say_lines(
                            vec![
                                "Server population shrunk, and with it the map pool.".to_string(),
                                format!(
                                    "{} has been replaced with {}.",
                                    alternatives_removals
                                        .iter()
                                        .map(|map| map.Pretty())
                                        .join(", "),
                                    alternatives_additions
                                        .pool
                                        .iter()
                                        .map(|mip| mip.map.Pretty())
                                        .join(", ")
                                ),
                            ],
                            Visibility::All,
                        ),
                    )
                }
            } else if alternatives_removals.len() >= 2 {
                futures.push(bf4.say_lines(
                    vec![
                        "Server population shrunk, and with it the map pool.".to_string(),
                        format!("{} have been replaced with {}.",
                        alternatives_removals.iter().map(|map| map.Pretty()).join(", "),
                        alternatives_additions.pool.iter().map(|mip| mip.map.Pretty()).join(", ")),
                    ],
                    Visibility::All,
                ))
            }

            // and now notify each individual person of their concrete changes to their ballot.
            let mut futures_removals = Vec::new();
            inner.votes.retain(|player, ballot| {
                let yoinked = ballot.preferences.iter()
                    .filter(|&mip| removals.contains_map(mip.map)).cloned().collect::<HashSet<_>>();
                if yoinked.len() == ballot.preferences.len() {
                    // ALL choices on the person's ballot were yoinked! Whoops!
                    futures_removals.push(bf4.say_lines(vec![
                        format!("{}: Sorry, all maps you had voted for were removed from the map pool :(", player.to_owned()),
                        "Please vote again :)".to_string(),
                    ], player.to_owned()));
                    false // remove this ballot entirely.
                } else {
                    if !yoinked.is_empty() {
                        // remove all yoinked maps.
                        ballot.preferences.retain(|mip| {
                            !yoinked.contains(mip)
                        });
                        futures_removals.push(bf4.say_lines(vec![
                            format!("{}: Sorry, {} is/are no longer in the map pool and was removed from your ballot",
                                player.to_owned(),
                                yoinked.iter().join(", ")),
                            format!("Your ballot was changed to: {}", ballot),
                        ], player.to_owned()));
                    }
                    assert!(!ballot.preferences.is_empty());
                    true
                }
            });

            // notify any VIP who nominated a map that is has been removed,
            // and that they can nominate something again.
            for (vip, noms) in &mut inner.nominations {
                let yoinked = noms
                    .intersection(&alternatives_removals)
                    .cloned()
                    .collect::<HashSet<Map>>();
                if !yoinked.is_empty() {
                    futures_removals.push(bf4.say_lines(vec![
                        "Your nomination(s) have been retracted, you can now nominate something else :)".to_string()
                    ], vip.clone().get()));
                    noms.retain(|m| !alternatives_removals.contains(m));
                }
            }

            inner.pop_state = popstate;
            drop(lock);

            // actually run the futures here, after we dropped the lock.
            // This has a nice side effect of running them all in parallel.
            join_all(futures).await;
            tokio::time::sleep(Duration::from_secs(10)).await;
            join_all(futures_removals).await;
        } else {
            let mut init = Inner {
                alternatives: MapPool::new(),
                votes: HashMap::new(),
                pop_state: popstate,
                nominations: HashMap::new(),
                anim_override_override: HashMap::new(),
            };
            init.set_up_new_vote(self.config.n_options);
            info!("Popstate initialized! New: {}", init.pop_state.name);
            *lock = Some(init);
        }
    }

    /// Starts the main loop, listening for events, etc.
    pub async fn run(self: Arc<Self>, bf4: Arc<Bf4Client>) -> RconResult<()> {
        // nothing set up yet, we're waiting for the Inner to be initialized.

        let jh_spammer = {
            let mapvote = self.clone();
            let bf4 = bf4.clone();
            tokio::spawn(async move {
                mapvote.status_spammer(bf4).await;
            })
        };

        let mut events = bf4.event_stream().await?;
        while let Some(event) = events.next().await {
            match event {
                Ok(Event::Chat { vis, player, msg }) => {
                    let bf4 = bf4.clone();
                    let mapvote = self.clone();

                    if msg.as_str().starts_with("/haha next map") {
                        let mapman = self.mapman.clone();
                        tokio::spawn(async move {
                            mapvote.handle_round_over(&bf4).await;
                        });
                    } else {
                        tokio::spawn(async move {
                            let _ = mapvote.handle_chat_msg(bf4, vis, player, msg).await;
                        });
                    }
                }
                Ok(Event::RoundOver { winning_team: _ }) => {
                    let bf4 = bf4.clone();
                    let mapvote = self.clone();
                    let mapman = self.mapman.clone();
                    // fire and forget about it, so we don't block other events. Yay concurrency!
                    tokio::spawn(async move {
                        mapvote.handle_round_over(&bf4).await;
                    });
                }
                Err(Bf4Error::Rcon(RconError::ConnectionClosed)) => break,
                _ => {} // ignore everything else.
            }
        }

        jh_spammer.await.unwrap();
        Ok(())
    }

    async fn broadcast_status(&self, bf4: &Bf4Client) {
        let players = self.players.players(&bf4).await;
        let mut futures = Vec::new();
        let lock = self.inner.lock().await;
        if let Some(inner) = &*lock {
            // only do something when we are initialized
            let mut lines = Vec::new();
            for player in players.keys() {
                inner.fmt_options(&mut lines);
                inner.fmt_personal_status(&mut lines, player);
                futures.push(bf4.say_lines(lines.clone(), player.clone()));
                lines.clear();
            }
        }

        // drop lock before we spend potentially 64 * 5 * 17ms = 5.4s in rcon calls...
        drop(lock);

        join_all(futures).await; // up to 5.4s, ouchies.
    }

    async fn status_spammer(&self, bf4: Arc<Bf4Client>) {
        loop {
            tokio::time::sleep(Duration::from_secs(15)).await;

            self.broadcast_status(&bf4).await;

            tokio::time::sleep(self.config.spammer_interval).await;
        }
    }

    /// Returns:
    /// - Ok:
    ///   - The ballot that ended up as the current vote.
    ///   - Optionally, old ballot of the previous time the player voted.
    /// - Err:
    ///   - Player did a derp.
    async fn vote(
        &self,
        player: &Guard<Player, MaybeVip>,
        mut prefs: Vec<(Map, GameMode)>,
    ) -> VoteResult<()> {
        let mut lock = self.inner.lock().await;
        if let Some(inner) = &mut *lock {
            let not_in_popstate = prefs
                .iter()
                .filter(|(map, mode)| !inner.pop_state.pool.contains_map(*map))
                .cloned()
                .collect::<Vec<_>>();
            // Remove maps which are forbidden by pop state.
            prefs.retain(|(map, mode)| inner.pop_state.pool.contains_map(*map));

            let not_in_options = prefs
                .iter()
                .filter(|(map, mode)| !inner.alternatives.contains_map(*map))
                .cloned()
                .collect::<Vec<_>>();
            // the maps are in the popstate, but aren't up to be chosen right now.
            // Nomination possible.
            prefs.retain(|(map, mode)| inner.alternatives.contains_map(*map));

            let weight = match player.clone().cases() {
                Left(yesvip) => Rat::one() + Rat::one(), // 2
                Right(novip) => Rat::one(),
            };

            // now, attempt to deduplicate (Ballot:from_iter(..) does that for us)
            let alts = prefs.iter().map(|(map, mode)| MapInPool {
                map: *map,
                mode: mode.clone(),
                extra: (),
            });
            let (ballot, soft_dups) = match Ballot::from_iter(weight, alts) {
                CheckBallotResult::Ok { ballot, soft_dups } => (ballot, soft_dups),
                CheckBallotResult::UnresolvableDuplicate { problem } => {
                    return VoteResult::UnresolvableDuplicate { problem }
                }
                CheckBallotResult::Empty => return VoteResult::Empty {
                    not_in_options: not_in_options.into(),
                    not_in_popstate: not_in_popstate.into(),
                },
            };

            // so now we have a ballot which can be cast. Let's check for existing ballot, and cast it!
            let old = inner.votes.insert((**player).to_owned(), ballot.to_owned());
            VoteResult::Ok {
                new: ballot,
                old,
                soft_dups,
                not_in_options: not_in_options.into(),
                not_in_popstate: not_in_popstate.into(),
            }
        } else {
            VoteResult::Inactive
        }
    }

    async fn notify_skipped(&self, not_in_options: MapPool<()>, not_in_popstate: MapPool<()>, g: Guard<Player, MaybeVip>, bf4: &Bf4Client, player: &Player) {
        if !not_in_options.pool.is_empty() {
            let list = not_in_options.to_mapset().union(&not_in_popstate.to_mapset())
                .map(|map| map.short())
                .join(", ");
            match g.cases() {
                Left(yesvip) => {
                    let _ = bf4.say_lines(vec![
                        format!("Skipped {}: Currently not in options", list),
                        format!("..but you are VIP <3!! Try this: !nominate {}", not_in_options.pool[0].map.short()),
                    ], player).await;
                }
                Right(notvip) => {
                    let _ = bf4.say_lines(vec![
                        format!("Skipped {}: Currently not in options", list),
                        self.config.vip_nom.clone(),
                    ], player).await;
                }
            }
        } else if !not_in_popstate.pool.is_empty() {
            let list = not_in_options.to_mapset().union(&not_in_popstate.to_mapset())
                .map(|map| map.short())
                .join(", ");
            let _ = bf4.say_lines(vec![format!("Skipped {}: Not available due to population", list)], player).await;
        }
    }

    async fn handle_maps(
        &self,
        bf4: &Arc<Bf4Client>,
        player: Player,
        maps: Vec<(Map, GameMode)>,
        vis: Visibility,
    ) -> RconResult<()> {
        let vip = self.vips.get_player(&player, bf4).await?;

        match vip.cases() {
            Recent(g) => {
                match self.vote(&g, maps).await {
                    VoteResult::Ok {
                        new,
                        old,
                        soft_dups,
                        not_in_options,
                        not_in_popstate
                    } => {
                        self.notify_skipped(not_in_options, not_in_popstate, g, bf4, &player).await;

                        if let Some(old) = old {
                            let _ = bf4.say_lines(vec![format!("{} changed their ballot to {}", player, new)], vis.clone()).await;
                        } else {
                            let _ = bf4.say_lines(vec![format!("{} voted: {}", player, new)], vis.clone()).await;
                        }
                    }
                    VoteResult::UnresolvableDuplicate { problem } => {
                        let _ = bf4.say_lines(vec![
                            format!("{}: Could not figure out your preference order", player),
                            format!("The issue is with {} (cycle?)", problem.map.Pretty()),
                        ],player).await;
                    }
                    VoteResult::Empty { not_in_options, not_in_popstate } => {
                        self.notify_skipped(not_in_options, not_in_popstate, g, bf4, &player).await;
                        let _ = bf4.say("Try again.", player).await;
                    }
                    VoteResult::Inactive => {
                        let _ = bf4.say("Mapvote is currently inactive, try again later :)".to_string(), player).await;
                    }
                }
            }
            Old => {
                warn!(
                    "[mapvote.rs handle_maps()] Couldn't resolve vip for {}? (this is a bug, report it to Kiiya#0456)",
                    player
                );
            }
        }

        Ok(())
    }

    async fn handle_nomination(
        &self,
        bf4: Arc<Bf4Client>,
        player: Guard<Player, MaybeVip>,
        map: NomMapParseResult,
    ) {
        match player.cases() {
            Left(player) => {
                // make sure the map was parsed correctly
                match map {
                    NomMapParseResult::Ok(map) => {
                        let mut futures = Vec::new();
                        let mut lock = self.inner.lock().await;
                        // make sure we have a mapvote actually going at all.
                        if let Some(inner) = &mut *lock {
                            // make sure people don't nominate excessively much.
                            if inner.alternatives.pool.len() < self.config.max_options {
                                // make sure map isn't already in the options
                                if !inner.alternatives.pool.iter().any(|mip| mip.map == map) {
                                    // make sure the map is in the pool
                                    if inner.pop_state.pool.contains_map(map) {
                                        // make sure this VIP hasn't exceeded their nomination limit this round.
                                        if inner.vip_n_noms(&player) < self.config.max_noms_per_vip
                                        {
                                            // phew, that's a lot of ifs...
                                            inner.vip_nom(&player, map);
                                            futures.push(bf4.say_lines(vec![
                                                format!("Our beloved VIP {} has nominated {}!", &*player, map.Pretty()),
                                                format!("{} has been added to the options, everyone can vote on it now <3", map.Pretty()),
                                            ], Visibility::All));
                                        } else {
                                            futures.push(bf4.say_lines(vec![
                                                format!("Apologies, {}, you can't nominate more maps.", &*player),
                                                format!("The maximum nominations per round per VIP are {}.", self.config.max_noms_per_vip),
                                            ], player.get().into()));
                                        }
                                    } else {
                                        futures.push(bf4.say_lines(vec![
                                            format!("Sorry, {} is not avilable in this population level :(", map.Pretty()),
                                            "Maybe once more players join, it'll become available :)".to_string(),
                                        ], player.get().into()));
                                    }
                                } else {
                                    futures.push(bf4.say_lines(vec![
                                        format!("{} is already in the options..", map.Pretty()),
                                        "Try nominating some other map".to_string(),
                                    ], player.get().into()));
                                }
                            } else {
                                futures.push(bf4.say_lines(vec![
                                    format!("Apologies, but {} options at once is the maximum!", inner.alternatives.pool.len()),
                                    "Try again next round!".to_string(),
                                ], player.get().into()));
                            }
                        } else {
                            futures.push(bf4.say_lines(vec![
                                "There is no mapvote going currently. Try again in a couple minutes :).".to_string()
                            ], player.get().into()));
                        }
                        drop(lock); // very important to free this lock before we do rcon calls.
                        join_all(futures).await;
                    }
                    NomMapParseResult::Empty => {
                        // print which maps can be nominated.
                        let mut futures = Vec::new();
                        let lock = self.inner.lock().await;
                        if let Some(inner) = &*lock {
                            let nominatable = MapPool::additions(
                                &inner.alternatives,
                                &inner.pop_state.pool.extra_remove(),
                            );
                            let nominatable = nominatable
                                .pool
                                .iter()
                                .map(|mip| mip.map.short().to_string())
                                .collect::<Vec<_>>();
                            let lines = wrap_msg_chars("You can nominate the following: ", &nominatable, ", ", "");
                            futures.push(bf4.say_lines(lines,&*player));
                        } else {
                            futures.push(bf4.say_lines(vec![
                                "There is no mapvote going currently. Try again in a couple minutes :).".to_string()
                            ], &*player));
                        }
                        drop(lock);
                        join_all(futures).await;
                    }
                    NomMapParseResult::Other => {
                        let _ = bf4.say_lines(vec![
                            "You are VIP! But I couldn't understand which map you want to nominate :(",
                            "Example usage: !nominate metro",
                        ], player.get());
                    }
                }
            }
            Right(player) => {
                let _ = bf4.say_lines(vec![
                    format!("Sorry {}, but you are not a VIP (yet), and thus can't nominate maps :(", &*player),
                    self.config.vip_ad.clone(),
                ], &*player).await;
            }
        }
    }

    async fn handle_chat_msg(
        self: Arc<Self>,
        bf4: Arc<Bf4Client>,
        vis: Visibility,
        player: Player,
        mut msg: AsciiString,
    ) -> RconResult<()> {
        msg.make_ascii_lowercase();
        let split = msg.as_str()
            .split(' ')
            .filter(|&s| !s.is_empty())
            .collect::<Vec<_>>();

        if split.is_empty() {
            return Ok(())
        }

        match split[0] {
            "/v" | "!v" => {
                let mut lines = Vec::new();
                let lock = self.inner.lock().await;
                if let Some(inner) = &*lock {
                    inner.fmt_options(&mut lines);
                    inner.fmt_personal_status(&mut lines, &player);
                } else {
                    lines.push("Mapvote is currently inactive, try again later :)".to_owned());
                }

                drop(lock);
                let _ = bf4.say_lines(lines, player).await;
            }
            "!nominate" | "/nominate" | "!nom" | "/nom" => {
                let map = match split.get(1) {
                    Some(&word) => match Map::try_from_short(word) {
                        Some(map) => NomMapParseResult::Ok(map),
                        None => NomMapParseResult::Other,
                    },
                    None => NomMapParseResult::Empty,
                };
                tokio::spawn({
                    let player = player.clone();
                    let bf4 = bf4.clone();
                    let myself = self.clone();
                    async move {
                        let vip = myself
                            .vips
                            .get_player_use(&player, &bf4, |g| async {
                                myself.handle_nomination(bf4.clone(), g, map).await;
                            })
                            .await;
                        if let Ok(vip) = vip {
                            vip.await
                        }
                    }
                });
            }
            "!anim" | "/anim" => {
                let yesno = if split.len() >= 2 {
                    match split[1] {
                        "yes" | "true" | "on" | "1" | "+" => true,
                        "no" | "false" | "off" | "0" | "-" => false,
                        _ => true,
                    }
                } else {
                    true
                };
                let mut opt_inner = self.inner.lock().await;
                if let Some(inner) = &mut *opt_inner {
                    inner.anim_override_override.insert(player.clone(), yesno);
                    drop(opt_inner);
                    let _ = bf4.say(format!("Animation of vote result calculation at round end: {}", yesno), player).await;
                }
            }
            _ => {
                // if no command matched, try parsing !metro pearl etc
                if !msg.is_empty() && (msg[0] == '/' || msg[0] == '!') {
                    let vis = if msg[0] == '/' {
                        Visibility::Player(player.name.clone())
                    } else {
                        vis
                    };
                    match parse_maps(&msg.as_str()[1..]) {
                        ParseMapsResult::Ok(maps) => self.handle_maps(&bf4, player, maps, vis).await?,
                        ParseMapsResult::Nothing => {}, // silently ignore
                        ParseMapsResult::NotAMapName { orig } => {
                            let _ = bf4
                                .say(
                                    format!("{}: \"{}\" is not a valid map name.", player, orig),
                                    player,
                                )
                                .await;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_round_over(&self, bf4: &Arc<Bf4Client>) {
        self.broadcast_status(bf4).await; // send everyone the voting options.
        // let's wait like 10 seconds because people might still vote in the end screen.
        let _ = bf4.say(format!("Mapvote is still going for {}s! Hurry!", self.config.endscreen_votetime.as_secs()), Visibility::All).await;
        tokio::time::sleep(self.config.endscreen_votetime - Duration::from_secs(7)).await; // FIXME: Replace with checked substraction.
        self.broadcast_status(bf4).await; // send everyone the voting options.
        tokio::time::sleep(Duration::from_secs(7)).await; // FIXME: ^

        let players = self.players.players(bf4).await;

        debug!("Voting ended");

        let maybe = {
            let mut lock = self.inner.lock().await;
            if let Some(inner) = &mut *lock {
                let profile = inner.to_profile();
                let assignment = inner.to_assignment();

                inner.set_up_new_vote(self.config.n_options);
                Some((profile, assignment, inner.anim_override_override.clone()))
            } else {
                None
            }
            // important: lock is dropped here at end of scope!
        };

        dbg!(&maybe);

        // only do something if we have an Inner.
        if let Some((profile, assignment, anim_override_override)) = maybe {
            let mut tracer = AnimTracer::start(profile.clone(), assignment);
            if let Some(winner) = profile.vanilla_stv_1(&mut tracer) {
                trace!("AnimTracer: {:#?}", &tracer);

                for ass in tracer.log_iter() {
                    debug!("Ass: {:?}", ass);
                }

                let alts_start = profile.alts.iter()
                    .sorted_by(|a, b| Ord::cmp(&profile.score(b), &profile.score(a)))
                    .cloned()
                    .collect_vec();
                let animation = animate::stv_anim_frames(&alts_start, players.keys(), &tracer);
                trace!("Animations for all players: {:?}", animation);

                let mut jhs = Vec::new();
                for (player, frames) in animation {
                    let bf4clone = bf4.clone();
                    let animate = *anim_override_override
                        .get(&player)
                        .unwrap_or_else(|| self.config.animate_override
                            .get(&player.name)
                            .unwrap_or(&self.config.animate));

                    let winner = winner.clone();
                    jhs.push(tokio::spawn(async move {
                        trace!("Animate for {}: {}", &player.name, animate);
                        if animate {
                            for frame in frames {
                                // let f1 = bf4clone.say(frame, &player);
                                // let f2 = tokio::time::sleep(Duration::from_secs(2));
                                // join_all(vec![f1, f2]).await;
                                let _ = bf4clone.say(frame, &player).await;
                                tokio::time::sleep(Duration::from_secs(2)).await;
                            }
                        } else {
                            let _ = bf4clone.say(format!("Winner: {}", winner.map.Pretty()), player).await;
                        }
                    }));
                }
                join_all(jhs).await;
                trace!("Done sending animation");

                tokio::time::sleep(self.config.endscreen_post_votetime).await;

                self.mapman.switch_to(bf4, &winner).await.unwrap();
            } else {
                warn!("No mapvote winner somehow? This is likely a bug, report to Kiiya#0456 on Discord. Profile: {}", &profile);
                let _ = bf4.say("Round over, no winner", Visibility::All).await;
            }
        }
    }
}

pub enum ParseMapsResult {
    Ok(Vec<(Map, GameMode)>),

    /// Nothing, silently fail. E.g. when someone entered a normal command and not a map name.
    /// Returned when the first map name wasn't exact.
    Nothing,
    NotAMapName {
        orig: String, /*suggestions: Vec<(AsciiString, f64)> */
    },
}

/// expects a space-delimited list of maps with optional gamemode specifiers
///
/// The first map name must be exact, after that it'll trigger and give proper error messages.
/// If the first map is not an exact map name, it will just return `Nothing`.
pub fn parse_maps(str: &str) -> ParseMapsResult {
    let mut res = Vec::new();
    let words = str
        .split(' ')
        .filter(|&s| !s.is_empty())
        .collect::<Vec<_>>();

    #[allow(clippy::needless_range_loop)]
    for i in 0..words.len() {
        // TODO: Add map@mode or map/mode or map:mode syntax
        if let Some(map) = Map::try_from_short(words[i]) {
            res.push((map, GameMode::Rush));
        } else if i == 0 {
            return ParseMapsResult::Nothing;
        } else {
            return ParseMapsResult::NotAMapName {
                orig: words[i].to_owned(),
            };
        }
    }

    if res.is_empty() {
        return ParseMapsResult::Nothing;
    }

    ParseMapsResult::Ok(res)
}

#[cfg(test)]
mod test {
    use std::time::Duration;
    use tokio::time::Instant;
    use futures::future::join_all;

    pub async fn sleeper(str: String) {
        // tokio::time::sleep(Duration::from_millis(10)).await;
    }

    #[tokio::test]
    #[ignore]
    async fn bench_tokio_spawn() {
        const N: u32 = 100_000;
        let mut jhs = Vec::with_capacity(N as usize);

        let t = Instant::now();

        for i in 0..N {
            jhs.push(tokio::spawn(sleeper(format!("I'm {}!", i))));
        }

        let duration = t.elapsed();
        println!("Spawned {} tasks in {}ms (--> {}ns/task)", N, duration.as_millis(), (duration / N).as_nanos());
        let t = Instant::now();

        join_all(jhs).await;
        // for jh in jhs {
        //     jh.await;
        // }

        let duration = t.elapsed();
        println!("Joined {} tasks in {}ms (--> {}ns/task)", N, duration.as_millis(), (duration / N).as_nanos());

        // assert!(false);
    }
}
