//! What a run added up to: counted as it happens, shown when it ends.

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stats {
    pub seconds: f64,
    pub walked: f64,
    pub sprinted: f64,
    pub jumps: u32,
    pub shots: u32,
    pub hits: u32,
    pub headshots: u32,
    pub reloads: u32,
    pub rounds_found: u32,
    pub gun_kills: u32,
    /// Kills by a shot to the head.
    pub headshot_kills: u32,
    pub melee_kills: u32,
    /// The dead killed by a Spitter's burst (one the player killed).
    pub burst_kills: u32,
    pub longest_kill: f64,
    pub damage_dealt: f64,
    pub biggest_horde: u32,
    pub times_hit: u32,
    pub damage_taken: f64,
    pub healed: f64,
    pub bandages_used: u32,
    pub medkits_used: u32,
    pub dummies_downed: u32,
    pub plates_rung: u32,
    pub containers_searched: u32,
    /// Of those, supply cages.
    pub cages_opened: u32,
    pub items_looted: u32,
    /// What was carried at the end.
    pub loot_value: u32,
}

/// One group of lines on the stats screen.
pub struct Group {
    pub title: &'static str,
    pub lines: Vec<(&'static str, String)>,
}

fn clock(seconds: f64) -> String {
    let s = seconds.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

fn metres(m: f64) -> String {
    if m >= 1000.0 { format!("{:.2} km", m / 1000.0) } else { format!("{:.0} m", m) }
}

impl Stats {
    /// Hits as a share of shots fired, whole percent.
    pub fn accuracy(&self) -> u32 {
        if self.shots == 0 { 0 } else { (f64::from(self.hits) / f64::from(self.shots) * 100.0).round() as u32 }
    }

    pub fn kills(&self) -> u32 {
        self.gun_kills + self.melee_kills + self.burst_kills
    }

    /// The screen's groups, in reading order (PRACTICE only if there was
    /// any).
    pub fn groups(&self) -> Vec<Group> {
        let mut groups = vec![
            Group {
                title: "SURVIVAL",
                lines: vec![("Time survived", clock(self.seconds)), ("Distance walked", metres(self.walked)), ("Distance sprinted", metres(self.sprinted)), ("Jumps", self.jumps.to_string())],
            },
            Group {
                title: "SHOOTING",
                lines: vec![
                    ("Shots fired", self.shots.to_string()),
                    ("Hits", self.hits.to_string()),
                    ("Accuracy", format!("{}%", self.accuracy())),
                    ("Headshots", self.headshots.to_string()),
                    ("Reloads", self.reloads.to_string()),
                    ("Rounds found", self.rounds_found.to_string()),
                ],
            },
            Group {
                title: "LOOT",
                lines: vec![
                    ("Containers searched", self.containers_searched.to_string()),
                    ("Things picked up", self.items_looted.to_string()),
                    ("Value carried", format!("${}", self.loot_value)),
                ],
            },
            Group {
                title: "KILLS",
                lines: vec![
                    ("The dead killed", self.kills().to_string()),
                    ("By gun", self.gun_kills.to_string()),
                    ("By hand", self.melee_kills.to_string()),
                    ("By a Spitter's burst", self.burst_kills.to_string()),
                    ("Longest kill", metres(self.longest_kill)),
                    ("Damage dealt", format!("{:.0}", self.damage_dealt)),
                    ("Most up at once", self.biggest_horde.to_string()),
                ],
            },
            Group {
                title: "HURT",
                lines: vec![
                    ("Times hit", self.times_hit.to_string()),
                    ("Damage taken", format!("{:.0}", self.damage_taken)),
                    ("Health healed", format!("{:.0}", self.healed)),
                    ("Bandages used", self.bandages_used.to_string()),
                    ("Medkits used", self.medkits_used.to_string()),
                ],
            },
            Group { title: "PRACTICE", lines: vec![("Dummies knocked down", self.dummies_downed.to_string()), ("Plates rung", self.plates_rung.to_string())] },
        ];
        if self.dummies_downed + self.plates_rung == 0 {
            groups.pop();
        }
        groups
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_reads_back_sensibly() {
        let s = Stats { seconds: 125.4, walked: 1234.0, shots: 8, hits: 3, gun_kills: 2, melee_kills: 1, longest_kill: 17.6, ..Stats::default() };
        assert_eq!(s.accuracy(), 38);
        assert_eq!(Stats::default().accuracy(), 0, "no shots, no division by zero");
        let groups = s.groups();
        let find = |label: &str| groups.iter().flat_map(|g| &g.lines).find(|(l, _)| *l == label).map(|(_, v)| v.clone()).unwrap();
        assert_eq!(find("Time survived"), "2:05");
        assert_eq!(find("Distance walked"), "1.23 km");
        assert_eq!(find("The dead killed"), "3");
        assert_eq!(find("Longest kill"), "18 m");
    }
}
