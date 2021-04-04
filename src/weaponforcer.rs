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
                Ok(Event::Kill { killer, weapon, victim, headshot: _ }) => {
                    println!("Weapon: {}", weapon);
                    if let Some(killer) = killer {
                        if weapon == Weapon::Mortar || weapon == Weapon::Ucav {
                            if let Some(n) = offenses.get(&killer) {
                                if *n >= 3 {
                                    let _ = bf4.kick(killer.name.clone(), format!("{} is forbidden on this server!", weapon)).await;
                                } else {
                                    let _ = bf4.kill(killer.name.clone()).await; // ignore potential fails with let _ = ...
                                    let _ = bf4.say(format!("{}: Sorry, but {} is not allowed on this server!", killer, weapon), killer.clone()).await;
                                }
                            }
                            let _ = bf4.say_lines(vec![
                                format!("{}: You have been killed by {} by a forbidden weapon.\n\tThey have been punished for their sins.", victim, killer),
                            ], victim).await;
                            offenses.entry(killer).and_modify(|n| *n += 1);
                        }
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