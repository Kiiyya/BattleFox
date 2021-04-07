use std::{collections::HashMap, time::Duration};

use ascii::AsciiString;
use battlefield_rcon::{
    bf4::{Bf4Client, Player},
    rcon::RconResult,
};

use tokio::{sync::Mutex, time::Instant};

use crate::guard::{
    or::Or,
    recent::{Age, MaxAge, Recent},
    Guard, Judgement, Cases
};

use either::{Left, Right};
use itertools::Itertools;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct YesVip;
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NotVip;
pub type MaybeVip = Or<YesVip, NotVip>;

impl Judgement<AsciiString> for YesVip {}
impl Judgement<Player> for YesVip {}
impl Judgement<AsciiString> for NotVip {}
impl Judgement<Player> for NotVip {}

impl MaxAge for YesVip {
    // 10 minutes
    const MAX_AGE: Duration = Duration::from_secs(60 * 10);
}
impl MaxAge for NotVip {
    // 10 minutes
    const MAX_AGE: Duration = Duration::from_secs(60 * 10);
}

#[derive(Debug)]
struct Inner {
    vips: HashMap<AsciiString, Recent<MaybeVip>>,
    /// When was the last time that we refreshed the vip list.
    last_checked: Option<Instant>,
}

impl Inner {
    /// Removes any expired judgements.
    pub fn trim_old(&mut self) {
        self.vips.retain(|_, j| j.is_recent());
    }
}

#[derive(Debug)]
pub struct Vips {
    inner: Mutex<Inner>,
}

impl Vips {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                vips: HashMap::new(),
                last_checked: None,
            }),
        }
    }

    pub async fn clear_cache(&self) {
        let mut inner = self.inner.lock().await;
        inner.vips.clear();
        inner.last_checked = None;
    }

    pub async fn get(
        &self,
        name: &AsciiString,
        bf4: &Bf4Client,
    ) -> RconResult<Guard<AsciiString, Recent<MaybeVip>>> {
        let mut inner = self.inner.lock().await;
        inner.trim_old();
        if let Some(judgement) = inner.vips.get(name) {
            unsafe {
                return Ok(Guard::new_raw(name.to_owned(), judgement.to_owned()));
            }
        }

        if let Some(last_checked) = inner.last_checked {
            if last_checked.elapsed() < MaybeVip::MAX_AGE / 10 {
                // if we checked with the last minute (10min / 10), just assume person isn't on
                // the VIP-list, and cement that for the next 10 minutes.
                let j = Recent::now(MaybeVip::right(NotVip));
                inner.vips.insert(name.to_owned(), j);
                unsafe {
                    return Ok(Guard::new_raw(name.to_owned(), j.to_owned()));
                }
            }
        }

        drop(inner); // drop lock before rcon request inside `refresh()`.
                     // Yes it is technically possible that we do two refreshes at the same time, but that's not
                     // too bad. I worry more about latency.
        Ok(self
            .refersh(bf4, |inner| {
                if let Some(j) = inner.vips.get(name) {
                    unsafe { Guard::new_raw(name.to_owned(), j.to_owned()) }
                } else {
                    let j = Recent::now(MaybeVip::right(NotVip));
                    inner.vips.insert(name.to_owned(), j);
                    unsafe { Guard::new_raw(name.to_owned(), j) }
                }
            })
            .await?)
    }

    pub async fn get_player(
        &self,
        player: &Player,
        bf4: &Bf4Client,
    ) -> RconResult<Guard<Player, Recent<MaybeVip>>> {
        let vip = self.get(&player.name, bf4).await?;
        assert_eq!(player.name, *vip);
        unsafe { Ok(Guard::new_raw(player.clone(), *vip.get_judgement())) }
    }

    pub async fn get_player_use<Ret>(
        &self,
        player: &Player,
        bf4: &Bf4Client,
        user: impl FnOnce(Guard<Player, MaybeVip>) -> Ret,
    ) -> RconResult<Ret> {
        loop {
            let vip = self.get_player(player, bf4).await?;
            match vip.cases() {
                Age::Recent(g) => break Ok(user(g)),
                Age::Old => {
                    println!("[vips.rs get_player_use] retrying... ({})", player.name);
                    tokio::time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }

    async fn refersh<T>(
        &self,
        bf4: &Bf4Client,
        getter: impl FnOnce(&mut Inner) -> T,
    ) -> RconResult<T> {
        // to prevent double refresh requests going at the same time.
        let mut inner = self.inner.lock().await;
        if let Some(last_checked) = inner.last_checked {
            if last_checked.elapsed() < Duration::from_secs(2) {
                println!("Double VIP refresh prevented, woo!");
                return Ok(getter(&mut inner));
            }
        }
        inner.last_checked = Some(Instant::now());
        drop(inner); // drop before we go into rcon call.

        let reserved_list = bf4.reserved_list().await?;

        let mut inner = self.inner.lock().await;
        inner.trim_old();
        for reserved in reserved_list.iter() {
            inner.vips.insert(reserved.to_owned(), Recent::now(MaybeVip::left(YesVip)))
                .and_then(|j| j.and_then(|or| {
                    or.cases().either(|_yes| (), |_no| println!("{} is now VIP! (was previously recorded as not)", reserved));
                }));
        }

        // :3
        inner.vips.insert(AsciiString::from_ascii("Kiiyya").unwrap(), Recent::now(MaybeVip::left(YesVip)));
        inner.vips.insert(AsciiString::from_ascii("PocketWolfy").unwrap(), Recent::now(MaybeVip::left(YesVip)));

        let mut vips = inner.vips.iter().filter_map(|(k, v)|
            v.and_then(|g| match g.cases() {
                Left(_) => format!("{} (yes)", k),
                Right(_) => format!("{} (no)", k),
            }));
        println!("VIPs: {}", vips.join(", "));

        Ok(getter(&mut inner)) // before we drop the lock, use the getter on it.
    }
}

#[cfg(test)]
mod test {
    #![allow(dead_code, unused_variables)]
    use super::*;
    use crate::guard::Guard;
    use ascii::AsciiString;

    #[test]
    fn test() {
        let ascii = unsafe { Guard::new_raw(AsciiString::new(), MaybeVip::left(YesVip)) };
        let recent =
            unsafe { Guard::new_raw(AsciiString::new(), Recent::now(MaybeVip::left(YesVip))) };

        // match recent.cases() {
        //     Age::Recent(g) => match g.cases() {},
        //     Age::Old => {}
        // }
    }
}
