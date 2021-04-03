#![allow(unused_variables, unused_imports)]

use crate::{guard::{
        recent::Age::{Old, Recent},
        Cases, Guard,
    }, mapmanager::{
        pool::{MapInPool, MapPool, Vehicles, VehiclesSpecified},
        CallbackResult, MapManager, PopState,
    }, players::Players, stv::{CheckBallotResult, Profile, tracing::ElectElimTiebreakTracer}, vips::{MaybeVip, Vips, YesVip}};

use self::config::MapVoteConfig;

use super::stv::tracing::{NoTracer, StvAction};
use super::stv::Ballot;
use ascii::{AsciiString, IntoAsciiString};
use battlefield_rcon::{
    bf4::{
        error::{Bf4Error, Bf4Result},
        Bf4Client, Event, GameMode, Map, Player, Visibility,
    },
    rcon::{RconError, RconResult},
};
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



#[derive(Debug)]
struct Inner {
    alternatives: MapPool<()>,
    /// Invariant: All ballots have at least one option on them.
    votes: HashMap<Player, Ballot<MapInPool<()>>>,

    pop_state: PopState<Vehicles>,
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
}

impl Inner {
    /// part of what gets printed when a person types in `!v`, but also on spammer, etc.
    fn fmt_options(&self, lines: &mut Vec<String>) {
        let x = self
            .alternatives
            .pool
            .iter()
            .map(|alt| alt.map.Pretty())
            .join(", ");
        lines.push(format!("Options: {}", x));
    }

    /// part of what gets printed when a person types in `!v`, but also on spammer, etc.
    fn fmt_personal_status(&self, lines: &mut Vec<String>, perspective: &Player) {
        if let Some(ballot) = self.votes.get(perspective) {
            if ballot.preferences.len() >= 2 {
                // nice
                lines.push(format!("Your ballot: {}", ballot));
                lines.push("You can still change your ballot.".to_string());
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
            let suggestion = self.pop_state.pool.choose_random(3);
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
        println!(
            "I've set up a new vote with pool {:?}, so options are {:?}.",
            self.pop_state, self.alternatives
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
    },

    /// User submitted duplicates but they could not be untangled. Need to retry.
    UnresolvableDuplicate { problem: MapInPool<E> },

    /// A map which the user voted on is not in the current map pool.
    NotInPopstate { missing: HashSet<Map> },

    /// A map is in the current pool, but is not up for vote.
    ///
    /// It can be nominated though.
    NotInOptions { missing: HashSet<Map> },

    /// For some reason, managed to pass a list with zero options...
    Empty,

    /// There is no vote currently ongoing, this may be because:
    /// - Mapvote is currently in initialization phase
    /// - In the future: maybe have it disable at round start.
    Inactive,
}

impl Mapvote {
    /// Creates a new instance of `MapVote`, but doesn't start it yet, just sets stuff up.
    pub async fn new(mapman: Arc<MapManager>, vips: Arc<Vips>, players: Arc<Players>, config: MapVoteConfig) -> Arc<Self> {
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

        // holy shit this is ugly.
        let myself_weak = Arc::downgrade(&myself);
        mapman
            .register_pool_change_callback(move |bf4, popstate| {
                let weak = myself_weak.clone();
                Box::pin(async move {
                    // try to upgrade to strong Arc<MapVote>. If that fails, it means the mapvote
                    // instance was dropped. In that case, RemoveMe.
                    if let Some(strong) = weak.upgrade() {
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

        myself
    }

    async fn on_popstate_changed(&self, bf4: Arc<Bf4Client>, popstate: PopState<Vehicles>) {
        let mut lock = self.inner.lock().await;
        if let Some(inner) = &mut *lock {
            println!("Popstate changed! New: {}", popstate.name);

            let mut futures = Vec::new();
            let direction = PopState::change_direction(&inner.pop_state, &popstate);
            match direction {
                Ordering::Less => {
                    println!(
                        "Mapman: PopState downgrade from {} to {}",
                        inner.pop_state.name, popstate.name
                    );
                }
                Ordering::Equal => {
                    println!("Uhhh, popstate didn't change direction? Wot.");
                    return; // or maybe panic instead...?
                }
                Ordering::Greater => {
                    // TODO: Notify every single VIP individually that they can nominate more now.
                    println!(
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
            inner.alternatives
                .pool
                .retain(|mip| popstate.pool.contains_mapmode(mip.map, &mip.mode));
            inner.alternatives.pool.append(&mut alternatives_additions.clone().extra_remove().pool);

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
                    futures.push(bf4.say_lines(
                        vec![
                            "Server population shrunk, and with it the map pool.".to_string(),
                            format!("{} has been replaced with {}.",
                            alternatives_removals.iter().map(|map| map.Pretty()).join(", "),
                            alternatives_additions.pool.iter().map(|mip| mip.map.Pretty()).join(", ")),
                        ],
                        Visibility::All,
                    ))
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
            };
            init.set_up_new_vote(self.config.n_options);
            println!("Popstate initialized! New: {}", init.pop_state.name);
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
                mapvote.spam_status(bf4).await;
                println!("mapvote spammer sutatus done");
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
                        // let's wait like 10 seconds because people might still vote in the end screen.
                        tokio::time::sleep(Duration::from_secs(12)).await;

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

    async fn spam_status(&self, bf4: Arc<Bf4Client>) {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;

            let players = self.players.players(&bf4).await;
            let mut futures = Vec::new();
            let lock = self.inner.lock().await;
            if let Some(inner) = &*lock { // only do something when we are initialized
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

            tokio::time::sleep(Duration::from_secs(60)).await;
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
        alts: &[(Map, GameMode)],
    ) -> VoteResult<()> {
        let prefs_maps = alts
            .iter()
            .map(|(map, mode)| map)
            .cloned()
            .collect::<HashSet<Map>>();
        let mut lock = self.inner.lock().await;
        if let Some(inner) = &mut *lock {
            let mapvote_maps = inner
                .pop_state
                .pool
                .pool
                .iter()
                .map(|mip| mip.map)
                .collect::<HashSet<Map>>();
            let too_many1 = prefs_maps
                .difference(&mapvote_maps)
                .cloned()
                .collect::<HashSet<_>>();
            if !too_many1.is_empty() {
                // Maps are forbidden by pop state.
                return VoteResult::NotInPopstate { missing: too_many1 };
            }

            let mapvote_opts = inner
                .alternatives
                .pool
                .iter()
                .map(|mip| mip.map)
                .collect::<HashSet<Map>>();
            let too_many2 = prefs_maps
                .difference(&mapvote_opts)
                .cloned()
                .collect::<HashSet<_>>();
            if !too_many2.is_empty() {
                // the maps are in the pool, but aren't up to be chosen right now.
                // Nomination possible.
                return VoteResult::NotInOptions { missing: too_many2 };
            }

            let weight = match player.clone().cases() {
                Left(yesvip) => Rat::one() + Rat::one(), // 2
                Right(novip) => Rat::one(),
            };

            // now, attempt to deduplicate (Ballot:from_iter(..) does that for us)
            let alts = alts.iter().map(|(map, mode)| MapInPool {
                map: *map,
                mode: mode.clone(),
                extra: (),
            });
            let (ballot, soft_dups) = match Ballot::from_iter(Rat::one(), alts) {
                CheckBallotResult::Ok { ballot, soft_dups } => (ballot, soft_dups),
                CheckBallotResult::UnresolvableDuplicate { problem } => {
                    return VoteResult::UnresolvableDuplicate { problem }
                }
                CheckBallotResult::Empty => return VoteResult::Empty,
            };

            // so now we have a ballot which can be cast. Let's check for existing ballot, and cast it!
            let old = inner.votes.insert((**player).to_owned(), ballot.to_owned());
            VoteResult::Ok {
                new: ballot,
                old,
                soft_dups,
            }
        } else {
            VoteResult::Inactive
        }
    }

    async fn handle_maps(
        &self,
        bf4: &Arc<Bf4Client>,
        player: Player,
        maps: &[(Map, GameMode)],
    ) -> RconResult<()> {
        let vip = self.vips.get(&player.name, bf4).await?;

        match vip.cases() {
            Recent(g) => {
                let ugh = unsafe { Guard::new_raw(player.clone(), *g.get_judgement()) };
                match self.vote(&ugh, maps).await {
                    VoteResult::Ok {
                        new,
                        old,
                        soft_dups,
                    } => {
                        if let Some(old) = old {
                            let _ = bf4
                                .say_lines(
                                    vec![format!("{} changed their ballot to {}", player, new)],
                                    Visibility::All,
                                )
                                .await;
                        } else {
                            let _ = bf4
                                .say_lines(
                                    vec![format!("{} voted: {}", player, new)],
                                    Visibility::All,
                                )
                                .await;
                        }
                    }
                    VoteResult::UnresolvableDuplicate { problem } => {
                        let _ = bf4
                            .say_lines(
                                vec![
                                    format!(
                                        "{}: Could not figure out your preference order",
                                        player
                                    ),
                                    format!("The issue is with {}", problem.map.Pretty()),
                                ],
                                player,
                            )
                            .await;
                    }
                    VoteResult::NotInPopstate { missing } => {
                        let _ = bf4.say_lines(vec![
                            format!("{}: Maps {} are not available with the current population level. Try again.", player, missing.iter().map(|mip| mip.Pretty()).join(", ")),
                        ], player).await;
                    }
                    VoteResult::NotInOptions { missing } => match ugh.cases() {
                        Left(yesvip) => {
                            let _ = bf4
                                .say_lines(
                                    vec![
                                        format!(
                                            "{}: Maps {} are not up for vote right now.",
                                            player,
                                            missing.iter().map(|mip| mip.Pretty()).join(", ")
                                        ),
                                        format!(
                                            "...but you are VIP <3!! Try this: !nominate {}",
                                            missing.iter().next().unwrap().short()
                                        ),
                                    ],
                                    player,
                                )
                                .await;
                        }
                        Right(notvip) => {
                            let _ = bf4.say_lines(vec![
                                    format!("{}: Maps {} are not up for vote right now.", player, missing.iter().map(|mip| mip.Pretty()).join(", ")),
                                    "VIPs can !nominate maps, get your VIP slot for $5/month at bfcube.com!".to_string(),
                                ], player).await;
                        }
                    },
                    VoteResult::Empty => {}
                    VoteResult::Inactive => {
                        let _ = bf4.say("Mapvote is currently inactive, try again later :)".to_string(), player).await;
                    }
                }
            }
            Old => {
                println!(
                    "[mapvote.rs handle_maps()] Couldn't resolve vip for {}?",
                    player
                );
                // tokio::time::sleep(Duration::from_secs(1)).await;
                // return self.handle_maps(bf4, player, maps).await; // just retry.
            }
        }

        Ok(())
    }

    async fn handle_chat_msg(
        self: Arc<Self>,
        bf4: Arc<Bf4Client>,
        vis: Visibility,
        player: Player,
        msg: AsciiString,
    ) -> RconResult<()> {
        let split = msg.as_str().split(' ').collect::<Vec<_>>();
        match split[0] {
            "/v" | "!v" => {
                let mut lines = Vec::new();
                let lock = self.inner.lock().await;
                if let Some(inner) = &*lock {
                    inner.fmt_options(&mut lines);
                    inner.fmt_personal_status(&mut lines, &player);
                } else {
                    let _ = bf4.say("Mapvote is currently inactive, try again later :)".to_string(), player.clone()).await;
                }

                // if let Some(vote) = inner.votes.get(&player) {
                //     // if player has already voted, just print that and nothing else.
                //     lines.push(format!("You voted for {}", vote));
                // } else {
                //     // otherwise, print instructions on how to vote.
                //     lines.push(
                //         "You can vote first, second, etc... preferences like this:".to_string(),
                //     );
                //     lines.push("!metro gulf-of-oman pearlmarket".to_string());
                //     lines.push("You haven't voted yet! Vote for ANY map you like".to_string());
                // }

                // drop(inner);
                let _ = bf4.say_lines(lines, player).await;
                Ok(())
            }
            "/haha next map" => {
                tokio::spawn({
                    let myself = Arc::clone(&self);
                    async move {
                        myself.handle_round_over(&bf4).await;
                    }
                });
                Ok(())
            }
            _ => {
                // if no command matched, try parsing !metro pearl etc
                match parse_maps(&msg.as_str()[1..]) {
                    ParseMapsResult::Ok(maps) => self.handle_maps(&bf4, player, &maps).await,
                    ParseMapsResult::Nothing => Ok(()), // silently ignore
                    ParseMapsResult::NotAMapName { orig } => {
                        let _ = bf4
                            .say(
                                format!("{}: \"{}\" is not a valid map name.", player, orig),
                                player,
                            )
                            .await;
                        Ok(())
                    }
                }
            }
        }
    }

    async fn handle_round_over(&self, bf4: &Arc<Bf4Client>) {
        let profile = {
            let mut lock = self.inner.lock().await;
            if let Some(inner) = &mut *lock {
                let ret = inner.to_profile();
                inner.set_up_new_vote(self.config.n_options);
                Some(ret)
            } else {
                None
            }
        };

        // only do something if we have an Inner.
        if let Some(profile) = profile {
            // let mut tracer = ElectElimTiebreakTracer::new();
            if let Some((winner, runner_up)) = profile.vanilla_stv_1_with_runnerup(&mut NoTracer) {
                // let x = tracer.trace.iter().map(|t| match t {
                //     StvAction::Elected { elected, profile_afterwards } => {

                //     }
                //     StvAction::Eliminated { alt, profile_afterwards } => {}
                //     StvAction::RejectTiebreak { tied, chosen, score } => {}
                //     StvAction::Stv1WinnerTiebreak { tied, chosen } => {}
                //     _ => {}
                // });
                let runner_up_text = if let Some(runner_up) = runner_up {
                    format!("(runner-up: {})", runner_up.map.Pretty())
                } else {
                    "".to_string()
                };
                bf4.say_lines(vec![
                    format!("Mapvote: {} people voted", profile.ballots.len()),
                    format!("Winner: {:?} {}", winner.map, runner_up_text),
                ], Visibility::All)
                .await
                .unwrap();
                self.mapman.switch_to(bf4, &winner).await.unwrap();
                // maplist.switch_to(bf4, mipmap, mode, false).await.unwrap();
            } else {
                bf4.say("Round over, no winner", Visibility::All)
                    .await
                    .unwrap(); // TODO!!
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
    let words = str.split(' ').collect::<Vec<_>>();

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

    ParseMapsResult::Ok(res)
}
