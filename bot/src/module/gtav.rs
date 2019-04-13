use crate::{command, currency, db, irc, module, player, settings, utils};
use failure::format_err;
use futures::{sync::mpsc, Future as _, Stream as _};
use parking_lot::RwLock;
use std::{fmt, net::SocketAddr, sync::Arc};
use tokio::net::UdpSocket;

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
    ExplodingBullets,
    /// Make moderate drunk.
    Drunk,
    /// Make very drunk.
    VeryDrunk,
    /// Set player on fire.
    SetOnFire,
    /// Set pedestrians on fire.
    SetPedsOnFire,
    /// Send a raw command to ChaosMod.
    Raw(String),
}

impl Command {
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
            ExplodingBullets => "reward",
            Drunk => "punished",
            VeryDrunk => "punished",
            SetOnFire => "punished",
            SetPedsOnFire => "punished",
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
            ExplodingBullets => format!("exploding-bullets"),
            Drunk => format!("drunk"),
            VeryDrunk => format!("very-drunk"),
            SetOnFire => format!("set-on-fire"),
            SetPedsOnFire => format!("set-peds-on-fire"),
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
            ExplodingBullets => 50,
            Drunk => 20,
            VeryDrunk => 40,
            SetOnFire => 40,
            SetPedsOnFire => 20,
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
            ExplodingBullets => write!(fmt, "enabling exploding bullets CurseLit"),
            Drunk => write!(fmt, "making them drunk"),
            VeryDrunk => write!(fmt, "making them VERY drunk"),
            SetOnFire => write!(fmt, "setting them on fire"),
            SetPedsOnFire => write!(fmt, "setting ALL the pedestrians on fire"),
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
    db: db::Database,
    player: Option<player::PlayerClient>,
    currency: currency::Currency,
    cooldown: Arc<RwLock<utils::Cooldown>>,
    prefix: Arc<RwLock<String>>,
    other_scaling: Arc<RwLock<u32>>,
    punish_scaling: Arc<RwLock<u32>>,
    reward_scaling: Arc<RwLock<u32>>,
    id_counter: usize,
    tx: mpsc::UnboundedSender<(irc::OwnedUser, usize, Command)>,
}

impl Handler {
    /// Play the specified theme song.
    fn play_theme_song(&mut self, ctx: &mut command::Context<'_, '_>, id: &str) {
        if let Some(player) = self.player.as_ref() {
            ctx.spawn(player.play_theme(id).then(|result| {
                match result {
                    Ok(()) => {}
                    Err(player::PlayThemeError::NoSuchTheme) => {
                        log::error!("you need to configure the theme `running90s`");
                    }
                    Err(player::PlayThemeError::Error(e)) => {
                        log::error!("error when playing theme: {}", e);
                    }
                }

                Ok(())
            }));
        }
    }

    /// Handle the other commands.
    fn handle_other(
        &mut self,
        ctx: &mut command::Context<'_, '_>,
    ) -> Result<Option<(Command, u32)>, failure::Error> {
        let command = match ctx.next() {
            Some("randomize-color") => Command::RandomizeColor,
            Some("randomize-weather") => Command::RandomizeWeather,
            Some("randomize-character") => Command::RandomizeCharacter,
            Some("license") => match ctx.next() {
                Some(s) => match license(s, ctx) {
                    Some(license) => Command::License(license.to_string()),
                    None => return Ok(None),
                },
                None => {
                    ctx.respond(format!(
                        "{c} requires an argument, like \"{c} g2gfast\"",
                        c = ctx.alias.unwrap_or("!gtav other license plate"),
                    ));

                    return Ok(None);
                }
            },
            Some("raw") => {
                ctx.check_moderator()?;
                Command::Raw(ctx.rest().to_string())
            }
            Some(..) | None => {
                ctx.respond(format!(
                    "Available mods are: \
                     {c} randomize-color, \
                     {c} randomize-weather, \
                     {c} randomize-character, \
                     {c} license <license>. \
                     See !chaos% for more details.",
                    c = ctx.alias.unwrap_or("!gtav other"),
                ));

                return Ok(None);
            }
        };

        Ok(Some((command, *self.other_scaling.read())))
    }

    /// Handle the punish command.
    fn handle_punish(
        &mut self,
        ctx: &mut command::Context<'_, '_>,
    ) -> Result<Option<(Command, u32)>, failure::Error> {
        let command = match ctx.next() {
            Some("stumble") => Command::Stumble,
            Some("fall") => Command::Fall,
            Some("tires") => Command::BlowTires,
            Some("engine") => Command::KillEngine,
            Some("weapon") => Command::TakeWeapon,
            Some("all-weapons") => Command::TakeAllWeapons,
            Some("health") => Command::TakeHealth,
            Some("wanted") => match ctx.next().map(str::parse) {
                Some(Ok(n)) if n >= 1 && n <= 5 => Command::Wanted(n),
                _ => {
                    ctx.respond(format!(
                        "Expected number between 1 and 5, like \"{} wanted 3\"",
                        ctx.alias.unwrap_or("!gtav punish")
                    ));
                    return Ok(None);
                }
            },
            Some("brake") => Command::Brake,
            Some("ammo") => Command::TakeAmmo,
            Some("enemy") => match ctx.next().map(str::parse) {
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
                    ctx.respond(format!(
                        "Expected {c} <number>",
                        c = ctx.alias.unwrap_or("!gtav punish enemy")
                    ));
                    return Ok(None);
                }
            },
            Some("drunk") => Command::Drunk,
            Some("very-drunk") => Command::VeryDrunk,
            Some("set-on-fire") => Command::SetOnFire,
            Some("set-peds-on-fire") => Command::SetPedsOnFire,
            _ => {
                ctx.respond(format!(
                    "Available punishments are: \
                     {c} stumble, \
                     {c} fall, \
                     {c} tires, \
                     {c} engine, \
                     {c} weapon, \
                     {c} all-weapons, \
                     {c} health, \
                     {c} wanted <level> \
                     {c} weather, \
                     {c} brake, \
                     {c} ammo, \
                     {c} enemy, \
                     {c} drunk, \
                     {c} very-drunk, \
                     {c} set-on-fire, \
                     {c} set-peds-on-fire. \
                     See !chaos% for more details.",
                    c = ctx.alias.unwrap_or("!gtav punish"),
                ));

                return Ok(None);
            }
        };

        Ok(Some((command, *self.punish_scaling.read())))
    }

    /// Handle the reward command.
    fn handle_reward(
        &mut self,
        ctx: &mut command::Context<'_, '_>,
    ) -> Result<Option<(Command, u32)>, failure::Error> {
        let command = match ctx.next() {
            Some("car") => Command::SpawnRandomVehicle(Vehicle::random_car()),
            Some("vehicle") => {
                let vehicle = match ctx.next().and_then(Vehicle::from_id) {
                    Some(vehicle) => vehicle,
                    None => {
                        let vehicles = Vehicle::all()
                            .into_iter()
                            .map(|v| format!("{} ({})", v, v.cost()))
                            .collect::<Vec<String>>()
                            .join(", ");

                        ctx.respond(format!(
                            "You give the streamer a vehicle using for example {c} random. \
                             Available vehicles are: {vehicles}. ",
                            c = ctx.alias.unwrap_or("!gtav reward vehicle"),
                            vehicles = vehicles,
                        ));

                        return Ok(None);
                    }
                };

                Command::SpawnVehicle(vehicle)
            }
            Some("repair") => Command::Repair,
            Some("wanted") => Command::Wanted(0),
            Some("parachute") => Command::GiveWeapon(Weapon::Parachute),
            Some("weapon") => {
                let weapon = match ctx.next().and_then(Weapon::from_id) {
                    Some(weapon) => weapon,
                    None => {
                        let weapons = Weapon::all()
                            .into_iter()
                            .map(|w| format!("{} ({})", w, w.cost()))
                            .collect::<Vec<String>>()
                            .join(", ");

                        ctx.respond(format!(
                            "You give the streamer a weapon using for example {c} random. \
                             Available weapons are: {weapons}. ",
                            c = ctx.alias.unwrap_or("!gtav reward weapon"),
                            weapons = weapons,
                        ));

                        return Ok(None);
                    }
                };

                Command::GiveWeapon(weapon)
            }
            Some("health") => Command::GiveHealth,
            Some("armor") => Command::GiveArmor,
            Some("boost") => Command::Boost,
            Some("superboost") => {
                self.play_theme_song(ctx, "gtav/superboost");
                Command::SuperBoost
            }
            Some("superspeed") => {
                self.play_theme_song(ctx, "gtav/superspeed");
                Command::SuperSpeed(10f32)
            }
            Some("superswim") => Command::SuperSwim(10f32),
            Some("superjump") => Command::SuperJump(10f32),
            Some("invincibility") => Command::Invincibility(10f32),
            Some("ammo") => Command::GiveAmmo,
            Some("exploding-bullets") => Command::ExplodingBullets,
            _ => {
                ctx.respond(format!(
                    "Available rewards are: \
                     {c} vehicle, \
                     {c} repair, \
                     {c} weapon, \
                     {c} wanted, \
                     {c} armor, \
                     {c} health, \
                     {c} boost, \
                     {c} superboost, \
                     {c} superspeed, \
                     {c} superswim, \
                     {c} superjump, \
                     {c} invincibility, \
                     {c} ammo, \
                     {c} exploding-bullets. \
                     See !chaos% for more details.",
                    c = ctx.alias.unwrap_or("!gtav reward"),
                ));

                return Ok(None);
            }
        };

        Ok(Some((command, *self.reward_scaling.read())))
    }
}

impl command::Handler for Handler {
    fn handle<'m>(&mut self, mut ctx: command::Context<'_, '_>) -> Result<(), failure::Error> {
        let result = match ctx.next() {
            Some("other") => self.handle_other(&mut ctx)?,
            Some("punish") => self.handle_punish(&mut ctx)?,
            Some("reward") => self.handle_reward(&mut ctx)?,
            _ => {
                ctx.respond(format!(
                    "You have the following actions available: \
                        {c} reward - To reward the streamer, \
                        {c} punish - To punish the streamer,
                        {c} other - To do other kinds of modifications.",
                    c = ctx.alias.unwrap_or("!gtav")
                ));
                return Ok(());
            }
        };

        let (command, scaling) = match result {
            Some((command, scaling)) => (command, scaling),
            None => return Ok(()),
        };

        if !ctx.is_moderator() && !self.cooldown.write().is_open() {
            ctx.respond("A command was recently issued, please wait a bit longer!");
            return Ok(());
        }

        let cost = command.cost() * scaling / 100;

        let balance = self
            .db
            .balance_of(ctx.user.target, ctx.user.name)?
            .unwrap_or_default();
        let balance = if balance < 0 { 0u32 } else { balance as u32 };

        if balance < cost {
            ctx.respond(format!(
                "{prefix}\
                 You need at least {limit} {currency} to reward the streamer, \
                 you currently have {balance} {currency}. \
                 Keep watching to earn more!",
                prefix = *self.prefix.read(),
                limit = cost,
                currency = self.currency.name,
                balance = balance,
            ));

            return Ok(());
        }

        ctx.spawn(
            self.db
                .balance_add(ctx.user.target, ctx.user.name, -(cost as i64))
                .or_else(|e| {
                    utils::log_err("failed to modify balance of user", e);
                    Ok(())
                }),
        );

        ctx.privmsg(format!(
            "{prefix}{user} {what} the streamer for {cost} {currency} by {command}",
            prefix = *self.prefix.read(),
            user = ctx.user.name,
            what = command.what(),
            command = command,
            cost = cost,
            currency = self.currency.name,
        ));

        let id = self.id_counter;
        self.id_counter += 1;

        if let Err(_) = self
            .tx
            .unbounded_send((ctx.user.as_owned_user(), id, command))
        {
            failure::bail!("failed to send event");
        }

        Ok(())
    }
}

/// Parse a license plate.Arc
fn license(input: &str, ctx: &mut command::Context<'_, '_>) -> Option<String> {
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

pub struct Module {
    cooldown: utils::Cooldown,
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    #[serde(default = "default_cooldown")]
    cooldown: utils::Cooldown,
}

fn default_cooldown() -> utils::Cooldown {
    utils::Cooldown::from_duration(utils::Duration::seconds(10))
}

impl Module {
    pub fn load(module: &Config) -> Result<Self, failure::Error> {
        Ok(Module {
            cooldown: module.cooldown.clone(),
        })
    }
}

impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "gtav"
    }

    /// Set up command handlers for this module.
    fn hook(
        &self,
        module::HookContext {
            core,
            db,
            handlers,
            currency,
            settings,
            futures,
            player,
            ..
        }: module::HookContext<'_>,
    ) -> Result<(), failure::Error> {
        let currency = currency
            .ok_or_else(|| format_err!("currency required for !gtav module"))?
            .clone();

        let cooldown = settings.sync_var(
            core,
            "gtav/cooldown",
            self.cooldown.clone(),
            settings::Type::Duration,
        )?;

        let prefix = settings.sync_var(
            core,
            "gtav/chat-prefix",
            String::from("ChaosMod: "),
            settings::Type::String,
        )?;
        let other_scaling =
            settings.sync_var(core, "gtav/other-scaling", 100, settings::Type::U32)?;
        let punish_scaling =
            settings.sync_var(core, "gtav/punish-scaling", 100, settings::Type::U32)?;
        let reward_scaling =
            settings.sync_var(core, "gtav/reward-scaling", 100, settings::Type::U32)?;

        let (tx, rx) = mpsc::unbounded();

        handlers.insert(
            "gtav",
            Handler {
                db: db.clone(),
                player: player.map(|p| p.client()),
                currency,
                cooldown,
                prefix,
                other_scaling,
                punish_scaling,
                reward_scaling,
                id_counter: 0,
                tx,
            },
        );

        let mut socket = UdpSocket::bind(&str::parse::<SocketAddr>("127.0.0.1:0")?)?;
        socket.connect(&str::parse::<SocketAddr>("127.0.0.1:7291")?)?;

        let future = rx.for_each(move |(user, id, command)| {
            let message = format!("{} {} {}", user.name, id, command.command());

            log::info!("sent: {}", message);

            socket
                .poll_send(message.as_bytes())
                .map(|_| ())
                .or_else(|e| {
                    log::error!("failed to send message: {}", e);
                    Ok(())
                })
        });

        futures.push(Box::new(
            future.map_err(|_| failure::format_err!("udp socket sender failed")),
        ));

        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Weapon {
    Random,

    Ak47,
    AssaultRifle,
    AssaultRifleMk2,

    M4,
    CarbineRifle,
    CarbineRifleMk2,

    Grenade,
    C4,
    StickyBomb,

    GrenadeLauncher,
    GrenadeLauncherSmoke,

    RocketLauncher,
    Rpg,

    Knife,

    Minigun,

    Parachute,

    Firework,
}

impl Weapon {
    /// Human-readable display of this weapon.
    pub fn display(&self) -> String {
        use self::Weapon::*;

        match *self {
            Random => format!("a random weapon BlessRNG"),
            Ak47 | AssaultRifle | AssaultRifleMk2 => format!("an assault rifle!"),
            M4 | CarbineRifle | CarbineRifleMk2 => format!("an assault rifle!"),
            Grenade => format!("grenades!"),
            C4 | StickyBomb => format!("sticky bombs!"),
            GrenadeLauncher => format!("a grenade launcher"),
            GrenadeLauncherSmoke => format!("a smoke grenade launcher"),
            RocketLauncher | Rpg => format!("a rocket launcher!"),
            Knife => format!("a knife!"),
            Minigun => format!("a minigun!"),
            Parachute => format!("a parachute!"),
            Firework => format!("fireworks!"),
        }
    }

    /// Map an id to a weapon.
    pub fn from_id(id: &str) -> Option<Weapon> {
        use self::Weapon::*;

        match id {
            "random" => Some(Random),
            "ak47" => Some(Ak47),
            "assault-rifle" => Some(AssaultRifle),
            "assault-rifle-mk2" => Some(AssaultRifleMk2),
            "m4" => Some(M4),
            "carbine-rifle" => Some(CarbineRifle),
            "carbine-rifle-mk2" => Some(CarbineRifleMk2),
            "grenade" => Some(Grenade),
            "c4" => Some(C4),
            "sticky-bomb" => Some(StickyBomb),
            "grenade-launcher" => Some(GrenadeLauncher),
            "grenade-launcher-smoke" => Some(GrenadeLauncherSmoke),
            "rocket-launcher" => Some(RocketLauncher),
            "rpg" => Some(Rpg),
            "knife" => Some(Knife),
            "minigun" => Some(Minigun),
            "parachute" => Some(Parachute),
            "firework" => Some(Firework),
            _ => None,
        }
    }

    /**
     * Get the cost of a vehicle.
     */
    fn cost(&self) -> u32 {
        use self::Weapon::*;

        match *self {
            Random => 5,

            M4 => 10,
            AssaultRifle => 10,
            AssaultRifleMk2 => 15,

            Ak47 => 10,
            CarbineRifle => 10,
            CarbineRifleMk2 => 15,

            Grenade => 5,
            C4 => 10,
            StickyBomb => 10,

            GrenadeLauncher => 20,
            GrenadeLauncherSmoke => 10,

            RocketLauncher => 15,
            Rpg => 15,

            Knife => 1,

            Minigun => 20,
            Parachute => 10,

            Firework => 20,
        }
    }

    /// Get a list of all vehicles.
    fn all() -> Vec<Weapon> {
        use self::Weapon::*;

        vec![
            Random,
            Ak47,
            AssaultRifle,
            AssaultRifleMk2,
            M4,
            CarbineRifle,
            CarbineRifleMk2,
            Grenade,
            C4,
            StickyBomb,
            GrenadeLauncher,
            GrenadeLauncherSmoke,
            RocketLauncher,
            Rpg,
            Knife,
            Minigun,
            Parachute,
            Firework,
        ]
    }
}

impl fmt::Display for Weapon {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Weapon::*;

        let s = match *self {
            Random => "random",

            Ak47 => "ak47",
            AssaultRifle => "assault-rifle",
            AssaultRifleMk2 => "assault-rifle-mk2",

            M4 => "m4",
            CarbineRifle => "carbine-rifle",
            CarbineRifleMk2 => "carbine-rifle-mk2",

            Grenade => "grenade",
            C4 => "c4",
            StickyBomb => "sticky-bomb",

            GrenadeLauncher => "grenade-launcher",
            GrenadeLauncherSmoke => "grenade-launcher-smoke",

            RocketLauncher => "rocket-launcher",
            Rpg => "rpg",

            Knife => "knife",

            Minigun => "minigun",

            Parachute => "parachute",

            Firework => "firework",
        };

        s.fmt(fmt)
    }
}

#[derive(Clone, Copy)]
enum Vehicle {
    Random,
    Slow,
    Normal,
    Fast,
    Bike,
    PedalBike,
    Gauntlet,
    FighterJet,
    JetSki,
    Blimp,
    Tank,
    Hydra,
    Sub,
}

impl Vehicle {
    /// Human-readable display of this vehicle.
    pub fn display(&self) -> String {
        use self::Vehicle::*;

        match *self {
            Random => format!("a random vehicle BlessRNG"),
            Slow => format!("something slow PepeHands"),
            Normal => format!("a normal car"),
            Fast => format!("something fast monkaSpeed"),
            Bike => format!("a bike!"),
            PedalBike => format!("a... bike?"),
            Gauntlet => format!("a Gauntlet SwiftRage"),
            FighterJet => format!("a fighter jet DansRage"),
            JetSki => format!("a jet ski DansRage"),
            Blimp => format!("a blimp SeemsGood"),
            Tank => format!("a tank!"),
            Hydra => format!("a hydra!"),
            Sub => format!("a submarine!"),
        }
    }

    /// Map an id to a vehicle.
    pub fn from_id(id: &str) -> Option<Vehicle> {
        match id {
            "random" => Some(Vehicle::Random),
            "slow" => Some(Vehicle::Slow),
            "normal" => Some(Vehicle::Normal),
            "fast" => Some(Vehicle::Fast),
            "bike" => Some(Vehicle::Bike),
            "pedalbike" => Some(Vehicle::PedalBike),
            "gauntlet" => Some(Vehicle::Gauntlet),
            "fighter-jet" => Some(Vehicle::FighterJet),
            "jet-ski" => Some(Vehicle::JetSki),
            "blimp" => Some(Vehicle::Blimp),
            "tank" => Some(Vehicle::Tank),
            "sub" => Some(Vehicle::Sub),
            "hydra" => Some(Vehicle::Hydra),
            _ => None,
        }
    }

    /**
     * Get the cost of a vehicle.
     */
    fn cost(&self) -> u32 {
        use self::Vehicle::*;

        match *self {
            Random => 10,
            Slow => 20,
            Normal => 20,
            Fast => 30,
            Bike => 50,
            PedalBike => 10,
            Gauntlet => 30,
            FighterJet => 50,
            JetSki => 10,
            Blimp => 50,
            Tank => 50,
            Sub => 20,
            Hydra => 60,
        }
    }

    /// Get a list of all cars.
    fn cars() -> Vec<Vehicle> {
        use self::Vehicle::*;

        vec![Slow, Normal, Fast]
    }

    /// Get a list of all vehicles.
    fn all() -> Vec<Vehicle> {
        use self::Vehicle::*;

        vec![
            Random, Slow, Normal, Fast, Bike, PedalBike, Gauntlet, FighterJet, Blimp, JetSki, Tank,
            Sub, Hydra,
        ]
    }

    /// Get a random car.
    fn random_car() -> Vehicle {
        use rand::Rng as _;
        let mut rng = rand::thread_rng();
        let cars = Self::cars();
        cars[rng.gen_range(0, cars.len())]
    }
}

impl fmt::Display for Vehicle {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        use self::Vehicle::*;

        match *self {
            Random => "random".fmt(fmt),
            Slow => "slow".fmt(fmt),
            Normal => "normal".fmt(fmt),
            Fast => "fast".fmt(fmt),
            Bike => "bike".fmt(fmt),
            PedalBike => "pedalbike".fmt(fmt),
            Gauntlet => "gauntlet".fmt(fmt),
            FighterJet => "fighter-jet".fmt(fmt),
            JetSki => "jet-ski".fmt(fmt),
            Blimp => "blimp".fmt(fmt),
            Tank => "tank".fmt(fmt),
            Hydra => "hydra".fmt(fmt),
            Sub => "sub".fmt(fmt),
        }
    }
}
