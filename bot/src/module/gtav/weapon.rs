use std::fmt;

macro_rules! weapons {
    ($($variant:ident, $id:expr, $cost:expr,)*) => {
        #[derive(Clone, Copy)]
        pub enum Weapon {
            Random,
            $($variant,)*
        }

        impl Weapon {
            /// Human-readable display of this vehicle.
            pub fn display(&self) -> String {
                use self::Weapon::*;

                match *self {
                    Random => format!("a random weapon"),
                    $($variant => format!("a {}!", $id),)*
                }
            }

            /// Map an id to a vehicle.
            pub fn from_id(id: impl AsRef<str>) -> Option<Weapon> {
                use self::Weapon::*;

                let id = id.as_ref().to_lowercase();

                match id.as_str() {
                    "random" => Some(Random),
                    "ak47" => Some(AssaultRifle),
                    "m4" => Some(CarbineRifle),
                    "c4" => Some(StickyBomb),
                    "rocketlauncher" => Some(Rpg),
                    $($id => Some($variant),)*
                    _ => None,
                }
            }

            /**
             * Get the cost of a vehicle.
             */
            pub fn cost(&self) -> u32 {
                use self::Weapon::*;

                match *self {
                    Random => 5,
                    $($variant => $cost,)*
                }
            }
        }

        impl fmt::Display for Weapon {
            fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                use self::Weapon::*;

                match *self {
                    Random => "random".fmt(fmt),
                    $($variant => $id.fmt(fmt),)*
                }
            }
        }
    }
}

weapons! {
    SniperRifle, "sniperrifle", 50,
    FireExtinguisher, "fireextinguisher", 50,
    CompactGrenadeLauncher, "compactgrenadelauncher", 50,
    Snowball, "snowball", 50,
    VintagePistol, "vintagepistol", 50,
    CombatPDW, "combatpdw", 50,
    HeavySniper, "heavysniper", 50,
    SweeperShotgun, "sweepershotgun", 50,
    MicroSMG, "microsmg", 10,
    Wrench, "wrench", 1,
    Pistol, "pistol", 5,
    PumpShotgun, "pumpshotgun", 50,
    APPistol, "appistol", 50,
    Ball, "ball", 50,
    Molotov, "molotov", 50,
    SMG, "smg", 50,
    StickyBomb, "stickybomb", 10,
    PetrolCan, "petrolcan", 50,
    StunGun, "stungun", 50,
    HeavyShotgun, "heavyshotgun", 50,
    Minigun, "minigun", 20,
    GolfClub, "golfclub", 50,
    FlareGun, "flaregun", 50,
    Flare, "flare", 50,
    GrenadeLauncherSmoke, "grenadelaunchersmoke", 10,
    Hammer, "hammer", 50,
    CombatPistol, "combatpistol", 50,
    Gusenberg, "gusenberg", 50,
    CompactRifle, "compactrifle", 50,
    HomingLauncher, "hominglauncher", 50,
    Nightstick, "nightstick", 50,
    Railgun, "railgun", 50,
    SawnOffShotgun, "sawnoffshotgun", 50,
    BullpupRifle, "bullpuprifle", 50,
    Firework, "firework", 20,
    CombatMG, "combatmg", 50,
    CarbineRifle, "carbinerifle", 10,
    Crowbar, "crowbar", 50,
    Flashlight, "flashlight", 50,
    Dagger, "dagger", 50,
    Grenade, "grenade", 5,
    PoolCue, "poolcue", 50,
    Bat, "bat", 50,
    Pistol50, "pistol50", 50,
    Knife, "knife", 1,
    MG, "mg", 50,
    BullpupShotgun, "bullpupshotgun", 50,
    BZGas, "bzgas", 50,
    Unarmed, "unarmed", 50,
    GrenadeLauncher, "grenadelauncher", 20,
    NightVision, "nightvision", 50,
    Musket, "musket", 50,
    ProximityMine, "proximitymine", 50,
    AdvancedRifle, "advancedrifle", 50,
    Rpg, "rpg", 15,
    PipeBomb, "pipebomb", 50,
    MiniSMG, "minismg", 50,
    SNSPistol, "snspistol", 50,
    AssaultRifle, "assaultrifle", 10,
    SpecialCarbine, "specialcarbine", 50,
    Revolver, "revolver", 50,
    MarksmanRifle, "marksmanrifle", 50,
    BattleAxe, "battleaxe", 50,
    HeavyPistol, "heavypistol", 50,
    KnuckleDuster, "knuckleduster", 50,
    MachinePistol, "machinepistol", 50,
    MarksmanPistol, "marksmanpistol", 50,
    Machete, "machete", 50,
    SwitchBlade, "switchblade", 50,
    AssaultShotgun, "assaultshotgun", 50,
    DoubleBarrelShotgun, "doublebarrelshotgun", 50,
    AssaultSMG, "assaultsmg", 50,
    Hatchet, "hatchet", 50,
    Bottle, "bottle", 50,
    Parachute, "parachute", 10,
    SmokeGrenade, "smokegrenade", 50,
}
