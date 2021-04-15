use std::collections::HashMap;

use battlefield_rcon::{bf4::{Bf4Client, Event, Player, Weapon}, rcon::RconResult};
use futures::StreamExt;


pub struct WeaponEnforcer {

}

impl WeaponEnforcer {
    pub fn new() -> Self {
        Self {

        }
    }

    pub async fn run(&self, bf4: &Bf4Client) -> RconResult<()> {
        let mut offenses : HashMap<Player, usize> = HashMap::new();

        let mut stream = bf4.event_stream().await?;
        while let Some(event) = stream.next().await {
            match event {
                Ok(Event::Kill { killer: Some(killer), weapon, victim, headshot: _ }) if weapon == Weapon::Mortar || weapon == Weapon::Ucav => {
                    // println!("Weapon: {}", weapon);
                    let _ = bf4.say_lines(vec![
                        format!("{}: You have been killed by {} by a forbidden weapon.\n\tThey have been punished for their sins.", victim, killer),
                    ], victim).await;

                    let n = offenses.entry(killer.clone()).or_insert(0);
                    *n += 1;

                    if *n >= 2 {
                        let _ = dbg!(bf4.kick(killer.name.clone(), format!("{} is forbidden on this server!", weapon)).await);
                    } else {
                        let _ = dbg!(bf4.kill(killer.name.clone()).await); // ignore potential fails with let _ = ...
                        let _ = bf4.say(format!("{}: Sorry, but {} is not allowed on this server!", killer, weapon), killer.clone()).await;
                    }
                },
                Ok(Event::RoundOver { winning_team: _ }) => {
                    offenses.clear();
                }
                _ => {}
            }
        }

        Ok(())
    }
}