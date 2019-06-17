/// Helper macro for constructing an enable command.
macro_rules! command_enable {
    ($ctx:expr, $db:expr, $what:expr, $edit_scope:ident) => {{
        $ctx.check_scope(crate::auth::Scope::$edit_scope)?;

        let name = match $ctx.next() {
            Some(name) => name,
            None => {
                $ctx.respond("Expected <name> to enable");
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
    ($ctx:expr, $db:expr, $what:expr, $edit_scope:ident) => {{
        $ctx.check_scope(crate::auth::Scope::$edit_scope)?;

        let name = match $ctx.next() {
            Some(name) => name,
            None => {
                $ctx.respond("Expected <name> to disable");
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
    ($ctx:expr, $db:expr, $what:expr, $edit_scope:ident) => {{
        $ctx.check_scope(crate::auth::Scope::$edit_scope)?;

        let name = match $ctx.next() {
            Some(name) => name,
            None => {
                $ctx.respond("Expected <name> to remove from a group");
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
    ($ctx:expr, $db:expr, $what:expr, $edit_scope:ident) => {{
        $ctx.check_scope(crate::auth::Scope::$edit_scope)?;

        let name = match $ctx.next() {
            Some(name) => name,
            None => {
                $ctx.respond("Expected <name> to add to a group");
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

macro_rules! command_list {
    ($ctx:expr, $db:expr, $what:expr) => {{
        let mut names = $db
            .list($ctx.user.target)
            .into_iter()
            .map(|c| c.key.name.to_string())
            .collect::<Vec<_>>();

        if names.is_empty() {
            $ctx.respond(format!("No custom {}.", $what));
        } else {
            names.sort();
            $ctx.respond(format!("{}", names.join(", ")));
        }
    }};
}

macro_rules! command_delete {
    ($ctx:expr, $db:expr, $what:expr, $edit_scope:ident) => {{
        $ctx.check_scope(crate::auth::Scope::$edit_scope)?;

        let name = match $ctx.next() {
            Some(name) => name,
            None => {
                $ctx.respond("Expected <name>");
                return Ok(());
            }
        };

        if $db.delete($ctx.user.target, name)? {
            $ctx.respond(format!("Deleted {} `{}`", $what, name));
        } else {
            $ctx.respond(format!("No such {}", $what));
        }
    }};
}

macro_rules! command_rename {
    ($ctx:expr, $db:expr, $what:expr, $edit_scope:ident) => {{
        $ctx.check_scope(crate::auth::Scope::$edit_scope)?;

        let (from, to) = match ($ctx.next(), $ctx.next()) {
            (Some(from), Some(to)) => (from, to),
            _ => {
                $ctx.respond("Expected <from> <to>");
                return Ok(());
            }
        };

        match $db.rename($ctx.user.target, from, to) {
            Ok(()) => $ctx.respond(format!("Renamed {} {} -> {}.", $what, from, to)),
            Err(crate::db::RenameError::Conflict) => {
                $ctx.respond(format!("Already an {} named `{}`.", $what, to))
            }
            Err(crate::db::RenameError::Missing) => {
                $ctx.respond(format!("No {} named `{}`.", $what, from))
            }
        }
    }};
}

macro_rules! command_show {
    ($ctx:expr, $db:expr, $what:expr) => {{
        let name = match $ctx.next() {
            Some(name) => name,
            None => {
                $ctx.respond("Expected <name> to show");
                return Ok(());
            }
        };

        let thing = $db.get_any($ctx.user.target, &name)?;

        match thing {
            Some(thing) => {
                $ctx.respond(format!("{} -> {}", thing.key.name, thing));
            }
            None => {
                $ctx.respond(format!("No {} named `{}`.", $what, name));
            }
        }
    }};
}

macro_rules! command_base {
    ($ctx:expr, $db:expr, $what:expr, $edit_scope:ident) => {
        match $ctx.next() {
            Some("clear-group") => {
                command_clear_group!($ctx, $db, $what, $edit_scope);
                return Ok(());
            }
            Some("group") => {
                command_group!($ctx, $db, $what, $edit_scope);
                return Ok(());
            }
            Some("enable") => {
                command_enable!($ctx, $db, $what, $edit_scope);
                return Ok(());
            }
            Some("disable") => {
                command_disable!($ctx, $db, $what, $edit_scope);
                return Ok(());
            }
            Some("list") => {
                command_list!($ctx, $db, $what);
                return Ok(());
            }
            Some("delete") => {
                command_delete!($ctx, $db, $what, $edit_scope);
                return Ok(());
            }
            Some("rename") => {
                command_rename!($ctx, $db, $what, $edit_scope);
                return Ok(());
            }
            Some("show") => {
                command_show!($ctx, $db, $what);
                return Ok(());
            }
            other => other,
        }
    };
}
