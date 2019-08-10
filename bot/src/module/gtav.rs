use crate::{
    auth::Scope,
    command, currency, irc, module, player,
    prelude::*,
    utils::{compact_duration, Cooldown, Duration},
};
use failure::{bail, Error};
use hashbrown::{hash_map, HashMap};
use parking_lot::RwLock;
use std::{fmt, net::SocketAddr, sync::Arc, time};
use tokio::net::UdpSocket;

const VEHICLE_URL: &'static str = "http://bit.ly/gtavvehicles";

mod vehicle;
mod weapon;

use self::vehicle::Vehicle;
use self::weapon::Weapon;

macro_rules! vehicle {
    ($ctx:expr, $pfx:expr) => {
        match $ctx
            .next()
            .map(|s| s.to_lowercase())
            .and_then(Vehicle::from_id)
        {
            Some(vehicle) => vehicle,
            None => {
                let vehicles = Vehicle::categories()
                    .into_iter()
                    .map(|v| format!("{} ({})", v, v.cost()))
                    .collect::<Vec<String>>()
                    .join(", ");

                $ctx.respond(format!(
                    "You give the streamer a vehicle using for example `random`. \
                     You can pick a vehicle by its name or a category. \
                     Available names are listed here: {url} - \
                     Available categories are: {vehicles}. ",
                    url = VEHICLE_URL,
                    vehicles = vehicles,
                ));

                return Ok(None);
            }
        }
    };
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CommandConfig {
    name: String,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    cooldown: Option<Duration>,
    #[serde(default)]
    cost: Option<u32>,
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
struct CommandsConfig(Vec<CommandConfig>);

#[derive(Debug)]
struct CommandSetting {
    enabled: bool,
    cooldown: Option<Arc<RwLock<Cooldown>>>,
    cost: Option<u32>,
}

impl Default for CommandSetting {
    fn default() -> Self {
        CommandSetting {
            enabled: true,
            cooldown: None,
            cost: None,
        }
    }
}

impl CommandsConfig {
    fn into_map(self) -> HashMap<String, CommandSetting> {
        let mut m = HashMap::new();

        for c in self.0 {
            let s = CommandSetting {
                enabled: c.enabled.unwrap_or(true),
                cooldown: c
                    .cooldown
                    .map(|c| Arc::new(RwLock::new(Cooldown::from_duration(c)))),
                cost: c.cost,
            };

            m.insert(c.name, s);
        }

        m
    }
}

enum Command {
    /// Spawn a vehicle.
    SpawnVehicle(Vehicle),
    /// Spawn a random vehicle.
    SpawnRandomVehicle(Vehicle),
    /// Destroy the engine of the current vehicle.
    KillEngine,
    /// Fix the engine of the current vehicle.
    BlowTires,
    /// Repair the current vehicle.
    Repair,
    /// Give a random weapon.
    GiveWeapon(Weapon),
    /// Take weapon.
    TakeWeapon,
    /// Take all weapons.
    TakeAllWeapons,
    /// Make the player stumble.
    Stumble,
    /// Make the player fall.
    Fall,
    /// Increase wanted level.
    Wanted(u32),
    /// Give maximum health.
    GiveHealth,
    /// Give maximum armor.
    GiveArmor,
    /// Take all health.
    TakeHealth,
    /// Set the license plate of the current vehicle.
    License(String),
    /// Randomize current vehicle.
    RandomizeColor,
    /// Random weather.
    RandomizeWeather,
    /// Random charcter.
    RandomizeCharacter,
    /// Brake the current vehicle.
    Brake,
    /// Take ammo from current weapon.
    TakeAmmo,
    /// Give full ammo for current weapon.
    GiveAmmo,
    /// Boost the current vehicle.
    Boost,
    /// Boost the current vehicle in a ridiculous way.
    SuperBoost,
    /// Enable super speed for the given amount of time.
    SuperSpeed(f32),
    /// Enable super swim for the given amount of time.
    SuperSwim(f32),
    /// Enable super jump for the given amount of time.
    SuperJump(f32),
    /// Enable invincibility for the given amount of time.
    Invincibility(f32),
    /// Spawn a number of enemies around the player.
    SpawnEnemy(u32),
    /// Enable exploding bullets.
    ExplodingBullets(f32),
    /// Enable fire ammunition.
    FireAmmo(f32),
    /// Enable exploding punches.
    ExplodingPunches(f32),
    /// Make moderate drunk.
    Drunk,
    /// Make very drunk.
    VeryDrunk,
    /// Set player on fire.
    SetOnFire,
    /// Set pedestrians on fire.
    SetPedsOnFire,
    /// Make a number of close by peds aggressive.
    MakePedsAggressive,
    /// Perform a matrix slam.
    MatrixSlam,
    /// Close the player's parachute.
    CloseParachute,
    /// Disable a control for a short period of time.
    DisableControl(Control),
    /// Mod the current vehicle.
    ModVehicle(VehicleMod),
    /// Cause the current player or vehicle to levitate.
    Levitate,
    /// Cause other game entities to levitate
    LevitateEntities,
    /// Eject from the current vehicle.
    Eject,
    /// Slow down time.
    SlowDownTime,
    /// Make fire proof for n seconds.
    MakeFireProof(f32),
    /// Make the current car leak all its fuel in 30 seconds.
    FuelLeakage,
    /// Change the current vehicle of the player.
    ChangeCurrentVehicle(Vehicle),
    /// Randomize doors of the current vehicle.
    RandomizeDoors,
    /// Shoot the player up in the air with a parachute.
    Skyfall,
    /// Taze the player.
    Taze,
    /// Taze people around the player.
    TazeOthers,
    /// Reduce gravity.
    ReduceGravity,
    /// Send a raw command to ChaosMod.
    Raw(String),
}

impl Command {
    /// The name of the command.
    fn command_name(&self) -> &'static str {
        use self::Command::*;

        match *self {
            SpawnVehicle(..) => "SpawnVehicle",
            SpawnRandomVehicle(..) => "SpawnRandomVehicle",
            KillEngine => "KillEngine",
            BlowTires => "BlowTires",
            Repair => "Repair",
            GiveWeapon(..) => "GiveWeapon",
            TakeWeapon => "TakeWeapon",
            TakeAllWeapons => "TakeAllWeapons",
            Stumble => "Stumble",
            Fall => "Fall",
            Wanted(0) => "ClearWanted",
            Wanted(..) => "Wanted",
            GiveHealth => "GiveHealth",
            GiveArmor => "GiveArmor",
            TakeHealth => "TakeHealth",
            License(..) => "License",
            RandomizeColor => "RandomizeColor",
            RandomizeWeather => "RandomizeWeather",
            RandomizeCharacter => "RandomizeCharacter",
            Brake => "Brake",
            TakeAmmo => "TakeAmmo",
            GiveAmmo => "GiveAmmo",
            Boost => "Boost",
            SuperBoost => "SuperBoost",
            SuperSpeed(..) => "SuperSpeed",
            SuperSwim(..) => "SuperSwim",
            SuperJump(..) => "SuperJump",
            Invincibility(..) => "Invincibility",
            SpawnEnemy(..) => "SpawnEnemy",
            ExplodingBullets(..) => "ExplodingBullets",
            FireAmmo(..) => "FireAmmo",
            ExplodingPunches(..) => "ExplodingPunches",
            Drunk => "Drunk",
            VeryDrunk => "VeryDrunk",
            SetOnFire => "SetOnFire",
            SetPedsOnFire => "SetPedsOnFire",
            MakePedsAggressive => "MakePedsAggressive",
            MatrixSlam => "MatrixSlam",
            CloseParachute => "CloseParachute",
            DisableControl(..) => "DisableControl",
            ModVehicle(..) => "ModVehicle",
            Levitate => "Levitate",
            LevitateEntities => "LevitateEntities",
            Eject => "Eject",
            SlowDownTime => "SlowDownTime",
            MakeFireProof(..) => "MakeFireProof",
            FuelLeakage => "FuelLeakage",
            ChangeCurrentVehicle(..) => "ChangeCurrentVehicle",
            RandomizeDoors => "RandomizeDoors",
            Skyfall => "Skyfall",
            Taze => "Taze",
            TazeOthers => "TazeOthers",
            ReduceGravity => "ReduceGravity",
            Raw(..) => "Raw",
        }
    }

    /// If the command is a reward or a punishment.
    fn what(&self) -> &'static str {
        use self::Command::*;

        match *self {
            SpawnVehicle(..) => "rewarded",
            SpawnRandomVehicle(..) => "rewarded",
            KillEngine => "punished",
            BlowTires => "punished",
            Repair => "rewarded",
            GiveWeapon(..) => "rewarded",
            TakeWeapon => "punished",
            TakeAllWeapons => "punished severely",
            Stumble => "punished",
            Fall => "punished",
            Wanted(0) => "rewarded",
            Wanted(..) => "punished",
            GiveHealth => "rewarded",
            GiveArmor => "rewarded",
            TakeHealth => "punished",
            License(..) => "spiced up",
            RandomizeColor => "spiced up",
            RandomizeWeather => "spiced up",
            RandomizeCharacter => "spiced up",
            Brake => "punished",
            TakeAmmo => "punished",
            GiveAmmo => "rewarded",
            Boost => "rewarded",
            SuperBoost => "rewarded (?)",
            SuperSpeed(..) => "rewarded",
            SuperSwim(..) => "rewarded",
            SuperJump(..) => "rewarded",
            Invincibility(..) => "rewarded",
            SpawnEnemy(..) => "punished",
            ExplodingBullets(..) => "reward",
            FireAmmo(..) => "reward",
            ExplodingPunches(..) => "reward",
            Drunk => "punished",
            VeryDrunk => "punished",
            SetOnFire => "punished",
            SetPedsOnFire => "punished",
            MakePedsAggressive => "punished",
            MatrixSlam => "rewarded",
            CloseParachute => "close-parachute",
            DisableControl(..) => "punished",
            ModVehicle(..) => "rewarded",
            Levitate => "rewarded",
            LevitateEntities => "rewarded",
            Eject => "punished",
            SlowDownTime => "rewarded",
            MakeFireProof(..) => "rewarded",
            FuelLeakage => "punished",
            ChangeCurrentVehicle(..) => "rewarded",
            RandomizeDoors => "rewarded",
            Skyfall => "rewarded",
            Taze => "punished",
            TazeOthers => "punished",
            ReduceGravity => "rewarded",
            Raw(..) => "?",
        }
    }

    /// The string-based command.
    fn command(&self) -> String {
        use self::Command::*;

        match *self {
            SpawnRandomVehicle(vehicle) | SpawnVehicle(vehicle) => {
                format!("spawn-vehicle {}", vehicle)
            }
            Repair => String::from("repair"),
            KillEngine => String::from("kill-engine"),
            BlowTires => String::from("blow-tires"),
            GiveWeapon(ref weapon) => format!("give-weapon {}", weapon),
            TakeWeapon => String::from("take-weapon"),
            TakeAllWeapons => String::from("take-all-weapons"),
            Stumble => String::from("stumble"),
            Fall => String::from("fall"),
            Wanted(n) => format!("wanted {}", n),
            GiveHealth => format!("give-health"),
            GiveArmor => format!("give-armor"),
            TakeHealth => format!("take-health"),
            License(ref license) => format!("license {}", license),
            RandomizeColor => format!("randomize-color"),
            RandomizeWeather => format!("randomize-weather"),
            RandomizeCharacter => format!("randomize-character"),
            Brake => format!("brake"),
            TakeAmmo => format!("take-ammo"),
            GiveAmmo => format!("give-ammo"),
            Boost => format!("boost"),
            SuperBoost => format!("super-boost"),
            SuperSpeed(n) => format!("super-speed {}", n),
            SuperSwim(n) => format!("super-swim {}", n),
            SuperJump(n) => format!("super-jump {}", n),
            Invincibility(n) => format!("invincibility {}", n),
            SpawnEnemy(n) => format!("spawn-enemy {}", n),
            ExplodingBullets(n) => format!("exploding-bullets {}", n),
            FireAmmo(n) => format!("fire-ammo {}", n),
            ExplodingPunches(n) => format!("exploding-punches {}", n),
            Drunk => format!("drunk"),
            VeryDrunk => format!("very-drunk"),
            SetOnFire => format!("set-on-fire"),
            SetPedsOnFire => format!("set-peds-on-fire"),
            MakePedsAggressive => format!("make-peds-aggressive"),
            MatrixSlam => format!("matrix-slam"),
            CloseParachute => format!("close-parachute"),
            DisableControl(ref control) => format!("disable-control {}", control),
            ModVehicle(ref m) => format!("mod-vehicle {}", m),
            Levitate => format!("levitate"),
            LevitateEntities => format!("levitate-entities"),
            Eject => format!("eject"),
            SlowDownTime => format!("slow-down-time"),
            MakeFireProof(n) => format!("make-fire-proof {}", n),
            FuelLeakage => format!("fuel-leakage"),
            ChangeCurrentVehicle(ref vehicle) => format!("change-current-vehicle {}", vehicle),
            RandomizeDoors => format!("randomize-doors"),
            Skyfall => format!("skyfall"),
            Taze => format!("taze"),
            TazeOthers => format!("taze-others"),
            ReduceGravity => format!("reduce-gravity"),
            Raw(ref cmd) => cmd.to_string(),
        }
    }

    /// The string-based command.
    fn cost(&self) -> u32 {
        use self::Command::*;

        match *self {
            // punishments
            KillEngine => 10,
            BlowTires => 10,
            TakeWeapon => 15,
            TakeAllWeapons => 30,
            Stumble => 15,
            Fall => 30,
            // rewards
            SpawnRandomVehicle(..) => 10,
            SpawnVehicle(ref vehicle, ..) => vehicle.cost(),
            Repair => 10,
            GiveWeapon(weapon) => weapon.cost(),
            Wanted(0) => 15,
            Wanted(n) => 10 + 5 * n,
            GiveHealth => 10,
            GiveArmor => 30,
            TakeHealth => 20,
            License(..) => 0,
            RandomizeColor => 0,
            RandomizeWeather => 0,
            RandomizeCharacter => 0,
            Brake => 10,
            TakeAmmo => 10,
            GiveAmmo => 10,
            Boost => 10,
            SuperBoost => 100,
            SuperSpeed(n) => n as u32,
            SuperSwim(n) => n as u32,
            SuperJump(n) => n as u32,
            Invincibility(n) => 2 * (n as u32),
            SpawnEnemy(n) => 10 * n,
            ExplodingBullets(..) => 50,
            FireAmmo(..) => 50,
            ExplodingPunches(..) => 50,
            Drunk => 20,
            VeryDrunk => 40,
            SetOnFire => 40,
            SetPedsOnFire => 20,
            MakePedsAggressive => 40,
            MatrixSlam => 50,
            CloseParachute => 50,
            DisableControl(ref control) => control.cost(),
            ModVehicle(ref m) => m.cost(),
            Levitate => 25,
            LevitateEntities => 50,
            Eject => 50,
            SlowDownTime => 25,
            MakeFireProof(..) => 50,
            FuelLeakage => 10,
            ChangeCurrentVehicle(ref vehicle) => vehicle.cost(),
            RandomizeDoors => 0,
            Skyfall => 50,
            Taze => 25,
            TazeOthers => 50,
            ReduceGravity => 25,
            Raw(..) => 0,
        }
    }
}

impl fmt::Display for Command {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Command::*;

        match *self {
            SpawnRandomVehicle(vehicle) | SpawnVehicle(vehicle) => {
                write!(fmt, "giving them {}", vehicle.display())
            }
            Repair => "repairing their car TakeNRG".fmt(fmt),
            KillEngine => "killing their engine PepeHands".fmt(fmt),
            BlowTires => "blowing their tires monkaMegaS".fmt(fmt),
            GiveWeapon(weapon) => write!(fmt, "giving them {} TakeNRG", weapon.display()),
            TakeWeapon => "taking their weapon SwiftRage".fmt(fmt),
            TakeAllWeapons => "taking ALL their weapons SwiftRage".fmt(fmt),
            Stumble => "making them stumble SwiftRage".fmt(fmt),
            Fall => "making them fall TriHard".fmt(fmt),
            Wanted(n) => write!(fmt, "changing their wanted level to {}", n),
            GiveHealth => "giving them FULL health TakeNRG".fmt(fmt),
            GiveArmor => "giving them FULL armor TakeNRG".fmt(fmt),
            TakeHealth => "taking away their health TriHard".fmt(fmt),
            License(ref license) => write!(fmt, "setting the license plate to \"{}\"!", license),
            RandomizeColor => write!(fmt, "randomizing vehicle color BlessRNG"),
            RandomizeWeather => write!(fmt, "randomizing current weather DatSheffy"),
            RandomizeCharacter => write!(fmt, "randomizing current character!"),
            Brake => write!(fmt, "braking the current vehicle SwiftRage"),
            TakeAmmo => write!(fmt, "taking all their ammo FeelsBadMan"),
            GiveAmmo => write!(fmt, "given them ammo!"),
            Boost => write!(fmt, "BOOSTING the current vehicle monkaSpeed"),
            SuperBoost => write!(fmt, "SUPER BOOSTING the current vehicle FireSpeed"),
            SuperSpeed(n) => write!(fmt, "SUPER SPEED for {} seconds monkaSpeed", n),
            SuperSwim(n) => write!(fmt, "SUPER SWIM for {} seconds monkaSpeed", n),
            SuperJump(n) => write!(fmt, "SUPER JUMP for {} seconds monkaS", n),
            Invincibility(n) => write!(
                fmt,
                "giving them invincibility for {} seconds FeelsGoodMan",
                n
            ),
            SpawnEnemy(1) => write!(fmt, "spawning an enemy monkaS"),
            SpawnEnemy(n) => write!(fmt, "spawning {} enemies monkaS", n),
            ExplodingBullets(..) => write!(fmt, "enabling exploding bullets CurseLit"),
            FireAmmo(..) => write!(fmt, "enabling fire ammo CurseLit"),
            ExplodingPunches(..) => write!(fmt, "enabling exploding punches CurseLit"),
            Drunk => write!(fmt, "making them drunk"),
            VeryDrunk => write!(fmt, "making them VERY drunk"),
            SetOnFire => write!(fmt, "setting them on fire"),
            SetPedsOnFire => write!(fmt, "setting ALL the pedestrians on fire"),
            MakePedsAggressive => write!(fmt, "setting the pedestrians on them"),
            MatrixSlam => write!(fmt, "performing a Matrix slam"),
            CloseParachute => write!(fmt, "opening their parachute"),
            DisableControl(ref control) => {
                write!(fmt, "disabling their {} control", control.display())
            }
            ModVehicle(ref m) => write!(fmt, "adding {} mod to their current vehicle", m.display()),
            Levitate => write!(fmt, "causing them to levitate"),
            LevitateEntities => write!(fmt, "causing other things to levitate"),
            Eject => write!(fmt, "causing them to eject"),
            SlowDownTime => write!(fmt, "causing time to slow down"),
            MakeFireProof(..) => write!(fmt, "making them fire proof"),
            FuelLeakage => write!(fmt, "slowly leaking all their fuel"),
            ChangeCurrentVehicle(..) => write!(fmt, "changing their current vehicle"),
            RandomizeDoors => write!(fmt, "randomizing their doors and windows"),
            Skyfall => write!(fmt, "causing them to skyfall"),
            Taze => write!(fmt, "tazing them"),
            TazeOthers => write!(fmt, "tazing everyone around them"),
            ReduceGravity => write!(fmt, "reducing their gravity"),
            Raw(..) => write!(fmt, "sending a raw command"),
        }
    }
}

#[derive(Clone)]
pub struct Reward {
    user: String,
    amount: i32,
}

pub struct Handler {
    enabled: Arc<RwLock<bool>>,
    player: Arc<RwLock<Option<player::Player>>>,
    currency: Arc<RwLock<Option<currency::Currency>>>,
    cooldown: Arc<RwLock<Cooldown>>,
    reward_cooldown: Arc<RwLock<Cooldown>>,
    punish_cooldown: Arc<RwLock<Cooldown>>,
    per_user_cooldown: Arc<RwLock<Cooldown>>,
    per_command_cooldown: Arc<RwLock<Cooldown>>,
    prefix: Arc<RwLock<String>>,
    other_percentage: Arc<RwLock<u32>>,
    punish_percentage: Arc<RwLock<u32>>,
    reward_percentage: Arc<RwLock<u32>>,
    success_feedback: Arc<RwLock<bool>>,
    id_counter: usize,
    tx: mpsc::UnboundedSender<(irc::User, usize, Command)>,
    per_user_cooldowns: HashMap<String, Cooldown>,
    per_command_cooldowns: HashMap<&'static str, Cooldown>,
    per_command_configs: Arc<RwLock<HashMap<String, CommandSetting>>>,
}

impl Handler {
    /// Play the specified theme song.
    fn play_theme_song(&mut self, ctx: &command::Context<'_>, id: &str) {
        if let Some(player) = self.player.read().clone() {
            let target = ctx.user.target().to_string();
            let id = id.to_string();

            ctx.spawn(async move {
                match player.play_theme(target.as_str(), id.as_str()).await {
                    Ok(()) => (),
                    Err(player::PlayThemeError::NoSuchTheme) => {
                        log::error!("you need to configure the theme `{}`", id);
                    }
                    Err(player::PlayThemeError::NotConfigured) => {
                        log::error!("themes system is not configured");
                    }
                    Err(player::PlayThemeError::Error(e)) => {
                        log::error!("error when playing theme: {}", e);
                    }
                }
            });
        }
    }

    /// Check if the given user is subject to cooldown right now.
    fn check_cooldown(
        &mut self,
        ctx: &command::Context<'_>,
        command: &Command,
        category_cooldown: Option<Arc<RwLock<Cooldown>>>,
    ) -> Option<(&'static str, time::Duration)> {
        let per_user_cooldown = self.per_user_cooldown.read();

        let user_cooldown = match self.per_user_cooldowns.entry(ctx.user.name().to_string()) {
            hash_map::Entry::Vacant(e) => e.insert(per_user_cooldown.clone()),
            hash_map::Entry::Occupied(e) => {
                let cooldown = e.into_mut();

                if cooldown.cooldown != per_user_cooldown.cooldown {
                    cooldown.cooldown = per_user_cooldown.cooldown.clone();
                }

                cooldown
            }
        };

        let per_command_cooldown = self.per_command_cooldown.read();

        let command_cooldown = match self.per_command_cooldowns.entry(command.command_name()) {
            hash_map::Entry::Vacant(e) => e.insert(per_command_cooldown.clone()),
            hash_map::Entry::Occupied(e) => {
                let cooldown = e.into_mut();

                if cooldown.cooldown != per_command_cooldown.cooldown {
                    cooldown.cooldown = per_command_cooldown.cooldown.clone();
                }

                cooldown
            }
        };

        let per_command_configs = self.per_command_configs.read();
        let command_specific = match per_command_configs.get(command.command_name()) {
            Some(setting) => setting.cooldown.clone(),
            None => None,
        };

        let now = time::Instant::now();

        let mut cooldown = self.cooldown.write();

        let mut remaining = smallvec::SmallVec::<[_; 4]>::new();

        remaining.extend(user_cooldown.check(now.clone()).map(|d| ("User", d)));

        if let Some(command_specific) = command_specific.as_ref() {
            let mut cooldown = command_specific.write();

            remaining.extend(cooldown.check(now.clone()).map(|d| ("Command specific", d)));
        } else {
            remaining.extend(cooldown.check(now.clone()).map(|d| ("Global", d)));
            remaining.extend(command_cooldown.check(now.clone()).map(|d| ("Command", d)));

            if let Some(category_cooldown) = category_cooldown.as_ref() {
                let mut cooldown = category_cooldown.write();

                remaining.extend(cooldown.check(now.clone()).map(|d| ("Category", d)));
            }
        }

        remaining.sort_by(|a, b| b.1.cmp(&a.1));

        match remaining.into_iter().next() {
            Some((name, remaining)) => Some((name, remaining)),
            None => {
                cooldown.poke(now);
                user_cooldown.poke(now);
                command_cooldown.poke(now);

                if let Some(category_cooldown) = category_cooldown.as_ref() {
                    category_cooldown.write().poke(now);
                }

                if let Some(command_specific) = command_specific.as_ref() {
                    command_specific.write().poke(now);
                }

                None
            }
        }
    }

    /// Handle the other commands.
    fn handle_other(
        &mut self,
        ctx: &mut command::Context<'_>,
    ) -> Result<Option<(Command, u32)>, Error> {
        let command = match ctx.next().as_ref().map(String::as_str) {
            Some("randomize-color") => Command::RandomizeColor,
            Some("randomize-weather") => Command::RandomizeWeather,
            Some("randomize-character") => Command::RandomizeCharacter,
            Some("randomize-doors") => Command::RandomizeDoors,
            Some("license") => match license(ctx.rest(), &ctx) {
                Some(license) => Command::License(license),
                None => return Ok(None),
            },
            Some("raw") => {
                ctx.check_scope(Scope::GtavRaw)?;
                Command::Raw(ctx.rest().to_string())
            }
            Some(..) | None => {
                ctx.respond(
                    "Available mods are: \
                     randomize-color, \
                     randomize-weather, \
                     randomize-character, \
                     license <license>. \
                     See !chaos% for more details.",
                );

                return Ok(None);
            }
        };

        Ok(Some((command, *self.other_percentage.read())))
    }

    /// Handle the punish command.
    fn handle_punish(
        &mut self,
        ctx: &mut command::Context<'_>,
    ) -> Result<Option<(Command, u32)>, Error> {
        let command = match ctx.next().as_ref().map(String::as_str) {
            Some("stumble") => Command::Stumble,
            Some("fall") => Command::Fall,
            Some("tires") => Command::BlowTires,
            Some("engine") => Command::KillEngine,
            Some("weapon") => Command::TakeWeapon,
            Some("all-weapons") => Command::TakeAllWeapons,
            Some("health") => Command::TakeHealth,
            Some("wanted") => match ctx.next().map(|s| str::parse(&s)) {
                Some(Ok(n)) if n >= 1 && n <= 5 => Command::Wanted(n),
                _ => {
                    ctx.respond("Expected number between 1 and 5");
                    return Ok(None);
                }
            },
            Some("brake") => Command::Brake,
            Some("ammo") => Command::TakeAmmo,
            Some("enemy") => match ctx.next().map(|s| str::parse(&s)) {
                None => Command::SpawnEnemy(1),
                Some(Ok(n)) if n > 0 && n <= 5 => Command::SpawnEnemy(n),
                Some(Ok(0)) => {
                    ctx.respond("Please specify more than 0 enemies to spawn.");
                    return Ok(None);
                }
                Some(Ok(_)) => {
                    ctx.respond("Cannot spawn more than 5 enemies.");
                    return Ok(None);
                }
                Some(Err(_)) => {
                    ctx.respond("Expected <number>");
                    return Ok(None);
                }
            },
            Some("drunk") => Command::Drunk,
            Some("very-drunk") => Command::VeryDrunk,
            Some("set-on-fire") => Command::SetOnFire,
            Some("set-peds-on-fire") => Command::SetPedsOnFire,
            Some("make-peds-aggressive") => Command::MakePedsAggressive,
            Some("close-parachute") => Command::CloseParachute,
            Some("disable-control") => {
                let control = match ctx.next().and_then(|s| Control::from_id(&s)) {
                    Some(weapon) => weapon,
                    None => {
                        let controls = Control::all()
                            .into_iter()
                            .map(|w| format!("{} ({})", w, w.cost()))
                            .collect::<Vec<String>>()
                            .join(", ");

                        ctx.respond(format!(
                            "You disable controls like `steering`. \
                             Available controls to disable are: {controls}. ",
                            controls = controls,
                        ));

                        return Ok(None);
                    }
                };

                Command::DisableControl(control)
            }
            Some("eject") => Command::Eject,
            Some("leak-fuel") => Command::FuelLeakage,
            Some("taze") => Command::Taze,
            Some("taze-others") => Command::TazeOthers,
            _ => {
                ctx.respond(format!("See !chaos% for available punishments.",));

                return Ok(None);
            }
        };

        Ok(Some((command, *self.punish_percentage.read())))
    }

    /// Handle the reward command.
    fn handle_reward(
        &mut self,
        ctx: &mut command::Context<'_>,
    ) -> Result<Option<(Command, u32)>, Error> {
        let command = match ctx.next().as_ref().map(String::as_str) {
            Some("car") => Command::SpawnRandomVehicle(Vehicle::random_car()),
            Some("vehicle") => {
                let vehicle = vehicle!(ctx, "!gtav reward vehicle");
                Command::SpawnVehicle(vehicle)
            }
            Some("repair") => Command::Repair,
            Some("wanted") => Command::Wanted(0),
            Some("parachute") => Command::GiveWeapon(Weapon::Parachute),
            Some("weapon") => {
                let weapon = match ctx.next().and_then(Weapon::from_id) {
                    Some(weapon) => weapon,
                    None => {
                        ctx.respond(format!("No such weapon, sorry :(.",));

                        return Ok(None);
                    }
                };

                Command::GiveWeapon(weapon)
            }
            Some("health") => Command::GiveHealth,
            Some("armor") => Command::GiveArmor,
            Some("boost") => Command::Boost,
            Some("superboost") => {
                self.play_theme_song(&ctx, "gtav/superboost");
                Command::SuperBoost
            }
            Some("superspeed") => {
                self.play_theme_song(&ctx, "gtav/superspeed");
                Command::SuperSpeed(30f32)
            }
            Some("superswim") => Command::SuperSwim(30f32),
            Some("superjump") => Command::SuperJump(30f32),
            Some("invincibility") => Command::Invincibility(30f32),
            Some("ammo") => Command::GiveAmmo,
            Some("exploding-bullets") => Command::ExplodingBullets(30f32),
            Some("fire-ammo") => Command::FireAmmo(30f32),
            Some("exploding-punches") => Command::ExplodingPunches(30f32),
            Some("matrix-slam") => Command::MatrixSlam,
            Some("mod-vehicle") => {
                let m = match ctx.next().and_then(|s| VehicleMod::from_id(&s)) {
                    Some(weapon) => weapon,
                    None => {
                        let mods = VehicleMod::all()
                            .into_iter()
                            .map(|w| format!("{} ({})", w, w.cost()))
                            .collect::<Vec<String>>()
                            .join(", ");

                        ctx.respond(format!(
                            "You give the streamer vehicle mods using for example `random`. \
                             Available mods are: {mods}. ",
                            mods = mods,
                        ));

                        return Ok(None);
                    }
                };

                Command::ModVehicle(m)
            }
            Some("levitate") => Command::Levitate,
            Some("levitate-entities") => Command::LevitateEntities,
            Some("slow-down-time") => Command::SlowDownTime,
            Some("fire-proof") => Command::MakeFireProof(30f32),
            Some("change-current-vehicle") => {
                let vehicle = vehicle!(ctx, "!gtav reward change-current-vehicle");
                Command::ChangeCurrentVehicle(vehicle)
            }
            Some("skyfall") => Command::Skyfall,
            Some("reduce-gravity") => Command::ReduceGravity,
            _ => {
                ctx.respond(format!("See !chaos% for available rewards."));
                return Ok(None);
            }
        };

        Ok(Some((command, *self.reward_percentage.read())))
    }
}

#[async_trait]
impl command::Handler for Handler {
    async fn handle<'ctx>(
        &mut self,
        mut ctx: command::Context<'ctx>,
    ) -> Result<(), failure::Error> {
        if !*self.enabled.read() {
            return Ok(());
        }

        let currency = self.currency.read().as_ref().cloned();
        let currency = match currency {
            Some(currency) => currency.clone(),
            None => {
                ctx.respond("No currency configured for stream, sorry :(");
                return Ok(());
            }
        };

        let (result, category_cooldown) = match ctx.next().as_ref().map(String::as_str) {
            Some("other") => {
                let command = self.handle_other(&mut ctx)?;
                (command, None)
            }
            Some("punish") => {
                let command = self.handle_punish(&mut ctx)?;
                let cooldown = self.punish_cooldown.clone();
                (command, Some(cooldown))
            }
            Some("reward") => {
                let command = self.handle_reward(&mut ctx)?;
                let cooldown = self.reward_cooldown.clone();
                (command, Some(cooldown))
            }
            _ => {
                ctx.respond(format!(
                    "You have the following actions available: \
                    reward - To reward the streamer, \
                    punish - To punish the streamer,
                    other - To do other kinds of modifications.",
                ));

                return Ok(());
            }
        };

        let (command, percentage) = match result {
            Some((command, percentage)) => (command, percentage),
            None => return Ok(()),
        };

        let mut cost = command.cost();

        {
            let per_command_configs = self.per_command_configs.read();

            if let Some(setting) = per_command_configs.get(command.command_name()) {
                if !setting.enabled {
                    return Ok(());
                }

                if let Some(c) = setting.cost {
                    cost = c;
                }
            }
        }

        let bypass_cooldown = ctx.user.has_scope(Scope::GtavBypassCooldown);

        if !bypass_cooldown {
            if let Some((what, remaining)) = self.check_cooldown(&ctx, &command, category_cooldown)
            {
                ctx.respond(format!(
                    "{} cooldown in effect, please wait at least {}!",
                    what,
                    compact_duration(&remaining),
                ));

                return Ok(());
            }
        }

        let id = self.id_counter;
        self.id_counter += 1;

        let cost = cost * percentage / 100;
        let user = ctx.user.clone();
        let sender = ctx.sender.clone();
        let prefix = self.prefix.read().clone();
        let success_feedback = self.success_feedback.clone();
        let tx = self.tx.clone();

        let future = async move {
            let balance = currency
                .balance_of(user.target(), user.name())
                .await?
                .unwrap_or_default();

            let balance = if balance < 0 { 0u32 } else { balance as u32 };

            if balance < cost {
                user.respond(format!(
                    "{prefix}\
                     You need at least {limit} {currency} to reward the streamer, \
                     you currently have {balance} {currency}. \
                     Keep watching to earn more!",
                    prefix = prefix,
                    limit = cost,
                    currency = currency.name,
                    balance = balance,
                ));

                return Ok(());
            }

            currency
                .balance_add(user.target(), user.name(), -(cost as i64))
                .await?;

            if *success_feedback.read() {
                sender.privmsg(format!(
                    "{prefix}{user} {what} the streamer for {cost} {currency} by {command}",
                    prefix = prefix,
                    user = user.display_name(),
                    what = command.what(),
                    command = command,
                    cost = cost,
                    currency = currency.name,
                ));
            }

            if let Err(_) = tx.unbounded_send((user, id, command)) {
                bail!("failed to send event");
            }

            Ok(())
        };

        ctx.spawn(future.map(|result| match result {
            Ok(()) => (),
            Err(e) => {
                log_err!(e, "failed to modify balance of user");
            }
        }));

        Ok(())
    }
}

/// Parse a license plate.Arc
fn license(input: &str, ctx: &command::Context<'_>) -> Option<String> {
    match input {
        "" => None,
        license if license.len() > 8 => {
            ctx.respond("License plates only support up to 8 characters.");
            None
        }
        license if !license.is_ascii() => {
            ctx.respond("License plate can only contain ASCII characters.");
            None
        }
        license => Some(license.to_string()),
    }
}

pub struct Module;

impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "gtav"
    }

    /// Set up command handlers for this module.
    fn hook(
        &self,
        module::HookContext {
            handlers,
            settings,
            futures,
            injector,
            ..
        }: module::HookContext<'_, '_>,
    ) -> Result<(), Error> {
        let currency = injector.var()?;
        let settings = settings.scoped("gtav");

        let default_reward_cooldown = Cooldown::from_duration(Duration::seconds(60));
        let default_punish_cooldown = Cooldown::from_duration(Duration::seconds(60));
        let default_per_user_cooldown = Cooldown::from_duration(Duration::seconds(60));
        let default_per_command_cooldown = Cooldown::from_duration(Duration::seconds(5));

        let (mut enabled_stream, enabled) = settings.stream("enabled").or_default()?;
        let enabled = Arc::new(RwLock::new(enabled));

        let cooldown = settings.var("cooldown", Cooldown::from_duration(Duration::seconds(1)))?;
        let reward_cooldown = settings.var("reward-cooldown", default_reward_cooldown)?;
        let punish_cooldown = settings.var("punish-cooldown", default_punish_cooldown)?;
        let per_user_cooldown = settings.var("per-user-cooldown", default_per_user_cooldown)?;
        let per_command_cooldown =
            settings.var("per-command-cooldown", default_per_command_cooldown)?;
        let prefix = settings.var("chat-prefix", String::from("ChaosMod: "))?;
        let other_percentage = settings.var("other%", 100)?;
        let punish_percentage = settings.var("punish%", 100)?;
        let reward_percentage = settings.var("reward%", 100)?;
        let success_feedback = settings.var("success-feedback", false)?;

        let (mut commands_config_stream, commands_config) = settings
            .stream::<CommandsConfig>("command-configs")
            .or_default()?;

        let per_command_configs = Arc::new(RwLock::new(HashMap::new()));
        *per_command_configs.write() = commands_config.into_map();

        let player = injector.var()?;

        let (tx, mut rx) = mpsc::unbounded();

        handlers.insert(
            "gtav",
            Handler {
                enabled: enabled.clone(),
                player,
                currency,
                cooldown,
                reward_cooldown,
                punish_cooldown,
                per_user_cooldown,
                per_user_cooldowns: Default::default(),
                per_command_cooldown,
                per_command_cooldowns: Default::default(),
                per_command_configs: per_command_configs.clone(),
                prefix,
                other_percentage,
                punish_percentage,
                reward_percentage,
                success_feedback,
                id_counter: 0,
                tx,
            },
        );

        let mut socket = UdpSocket::bind(&str::parse::<SocketAddr>("127.0.0.1:0")?)?;
        socket.connect(&str::parse::<SocketAddr>("127.0.0.1:7291")?)?;

        let future = async move {
            let mut receiver = match *enabled.read() {
                true => Some(&mut rx),
                false => None,
            };

            loop {
                futures::select! {
                    update = commands_config_stream.select_next_some() => {
                        *per_command_configs.write() = update.into_map();
                    }
                    update = enabled_stream.select_next_some() => {
                        receiver = match update {
                            true => Some(&mut rx),
                            false => None,
                        };

                        *enabled.write() = update;
                    }
                    command = receiver.next() => {
                        if let Some((user, id, command)) = command {
                            let message = format!("{} {} {}", user.name(), id, command.command());
                            log::info!("sent: {}", message);

                            match socket.poll_send(message.as_bytes()) {
                                Ok(_) => (),
                                Err(e) => {
                                    log::error!("failed to send message: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        };

        futures.push(future.boxed());
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum VehicleMod {
    Random,
    LowTier,
    MidTier,
    HighTier,
}

impl VehicleMod {
    /// Human-readable display of this mod.
    pub fn display(&self) -> String {
        use self::VehicleMod::*;

        match *self {
            Random => format!("random mods BlessRNG"),
            LowTier => format!("low tier mods"),
            MidTier => format!("mid tier mods"),
            HighTier => format!("high tier mods"),
        }
    }

    /// Map an id to a mod.
    pub fn from_id(id: &str) -> Option<VehicleMod> {
        use self::VehicleMod::*;

        match id {
            "random" => Some(Random),
            "low-tier" => Some(LowTier),
            "mid-tier" => Some(MidTier),
            "high-tier" => Some(HighTier),
            _ => None,
        }
    }

    /// Get the cost of a mod tier.
    fn cost(&self) -> u32 {
        use self::VehicleMod::*;

        match *self {
            Random => 5,
            LowTier => 5,
            MidTier => 5,
            HighTier => 10,
        }
    }

    /// Get a list of all mod tiers.
    fn all() -> Vec<VehicleMod> {
        use self::VehicleMod::*;

        vec![Random, LowTier, MidTier, HighTier]
    }
}

impl fmt::Display for VehicleMod {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::VehicleMod::*;

        let s = match *self {
            Random => "random",
            LowTier => "low-tier",
            MidTier => "mid-tier",
            HighTier => "high-tier",
        };

        s.fmt(fmt)
    }
}

#[derive(Clone, Copy)]
enum Control {
    Steering,
}

impl Control {
    /// Human-readable display of this control.
    pub fn display(&self) -> String {
        use self::Control::*;

        match *self {
            Steering => format!("steering"),
        }
    }

    /// Map an id to a mod.
    pub fn from_id(id: &str) -> Option<Control> {
        use self::Control::*;

        match id {
            "steering" => Some(Steering),
            _ => None,
        }
    }

    /// Get the cost of a control.
    fn cost(&self) -> u32 {
        use self::Control::*;

        match *self {
            Steering => 50,
        }
    }

    /// Get a list of all mod tiers.
    fn all() -> Vec<Control> {
        use self::Control::*;
        vec![Steering]
    }
}

impl fmt::Display for Control {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Control::*;

        let s = match *self {
            Steering => "steering",
        };

        s.fmt(fmt)
    }
}
