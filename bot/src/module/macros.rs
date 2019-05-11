/// Helper macro for constructing an enable command.
macro_rules! command_enable {
    ($ctx:expr, $db:expr, $pfx:expr, $what:expr) => {{
        $ctx.check_moderator()?;

        let name = match $ctx.next() {
            Some(name) => name,
            None => {
                $ctx.respond(format!(
                    "Expected: {p} <name>",
                    p = $ctx.alias.unwrap_or($pfx)
                ));
                return Ok(());
            }
        };

        if !$db.enable($ctx.user.target, name)? {
            $ctx.respond(format!("No {} named `{}`.", $what, name));
            return Ok(());
        }

        $ctx.respond(format!("Enabled {} `{}`", $what, name));
    }};
}

/// Helper macro for constructing an disable command.
macro_rules! command_disable {
    ($ctx:expr, $db:expr, $pfx:expr, $what:expr) => {{
        $ctx.check_moderator()?;

        let name = match $ctx.next() {
            Some(name) => name,
            None => {
                $ctx.respond(format!(
                    "Expected: {p} <name>",
                    p = $ctx.alias.unwrap_or($pfx)
                ));
                return Ok(());
            }
        };

        if !$db.disable($ctx.user.target, name)? {
            $ctx.respond(format!("No {} named `{}`.", $what, name));
            return Ok(());
        }

        $ctx.respond(format!("Disabled {} `{}`", $what, name));
    }};
}

/// Helper macro for constructing a clear-group command.
macro_rules! command_clear_group {
    ($ctx:expr, $db:expr, $pfx:expr, $what:expr) => {{
        $ctx.check_moderator()?;

        let name = match $ctx.next() {
            Some(name) => name,
            None => {
                $ctx.respond(format!(
                    "Expected: {p} <name>",
                    p = $ctx.alias.unwrap_or($pfx)
                ));
                return Ok(());
            }
        };

        if !$db.edit_group($ctx.user.target, name, None)? {
            $ctx.respond(format!("No {} named `{}`.", $what, name));
            return Ok(());
        }

        $ctx.respond(format!("Removed {} `{}` from its group", $what, name));
    }};
}

/// Helper macro for constructing a build command.
macro_rules! command_group {
    ($ctx:expr, $db:expr, $pfx:expr, $what:expr) => {{
        let name = match $ctx.next() {
            Some(name) => name,
            None => {
                $ctx.respond(format!(
                    "Expected: {p} <name>",
                    p = $ctx.alias.unwrap_or($pfx)
                ));
                return Ok(());
            }
        };

        let group = match $ctx.next() {
            Some(name) => name.to_string(),
            None => {
                let thing = match $db.get($ctx.user.target, &name) {
                    Some(thing) => thing,
                    None => {
                        $ctx.respond(format!("No {} named `{}`", $what, name));
                        return Ok(());
                    }
                };

                match thing.group.as_ref() {
                    Some(group) => {
                        $ctx.respond(format!(
                            "{} `{}` belongs to group: {}",
                            $what, thing.key.name, group
                        ));
                    }
                    None => {
                        $ctx.respond(format!(
                            "{} `{}` does not belong to a group",
                            $what, thing.key.name
                        ));
                    }
                }

                return Ok(());
            }
        };

        if !$db.edit_group($ctx.user.target, name, Some(group.clone()))? {
            $ctx.respond(format!("no such {}", $what));
            return Ok(());
        }

        $ctx.respond(format!("set group for {} `{}` to {}", $what, name, group));
    }};
}
