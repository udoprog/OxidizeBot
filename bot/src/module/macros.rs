/// Helper macro for constructing an enable command.
macro_rules! command_enable {
    ($ctx:expr, $db:expr, $what:expr, $edit_scope:ident) => {{
        $ctx.check_scope(::auth::Scope::$edit_scope).await?;

        let name = match $ctx.next() {
            Some(name) => name,
            None => {
                chat::respond!($ctx, "Expected <name> to enable");
                return Ok(());
            }
        };

        if !$db.enable($ctx.channel(), &name).await? {
            chat::respond!($ctx, "No {} named `{}`.", $what, name);
            return Ok(());
        }

        chat::respond!($ctx, "Enabled {} `{}`", $what, name);
    }};
}

/// Helper macro for constructing an disable command.
macro_rules! command_disable {
    ($ctx:expr, $db:expr, $what:expr, $edit_scope:ident) => {{
        $ctx.check_scope(::auth::Scope::$edit_scope).await?;

        let name = match $ctx.next() {
            Some(name) => name,
            None => {
                chat::respond!($ctx, "Expected <name> to disable");
                return Ok(());
            }
        };

        if !$db.disable($ctx.channel(), &name).await? {
            chat::respond!($ctx, "No {} named `{}`.", $what, name);
            return Ok(());
        }

        chat::respond!($ctx, "Disabled {} `{}`", $what, name);
    }};
}

/// Helper macro for constructing a clear-group command.
macro_rules! command_clear_group {
    ($ctx:expr, $db:expr, $what:expr, $edit_scope:ident) => {{
        $ctx.check_scope(::auth::Scope::$edit_scope).await?;

        let name = match $ctx.next() {
            Some(name) => name,
            None => {
                chat::respond!($ctx, "Expected <name> to remove from a group");
                return Ok(());
            }
        };

        if !$db.edit_group($ctx.channel(), &name, None).await? {
            chat::respond!($ctx, "No {} named `{}`.", $what, name);
            return Ok(());
        }

        chat::respond!($ctx, "Removed {} `{}` from its group", $what, name);
    }};
}

/// Helper macro for constructing a build command.
macro_rules! command_group {
    ($ctx:expr, $db:expr, $what:expr, $edit_scope:ident) => {{
        $ctx.check_scope(::auth::Scope::$edit_scope).await?;

        let name = match $ctx.next() {
            Some(name) => name,
            None => {
                chat::respond!($ctx, "Expected <name> to add to a group");
                return Ok(());
            }
        };

        let group = match $ctx.next() {
            Some(name) => name.to_string(),
            None => {
                let thing = match $db.get($ctx.channel(), &name).await {
                    Some(thing) => thing,
                    None => {
                        chat::respond!($ctx, "No {} named `{}`", $what, name);
                        return Ok(());
                    }
                };

                match thing.group.as_ref() {
                    Some(group) => {
                        chat::respond!(
                            $ctx,
                            "{} `{}` belongs to group: {}",
                            $what,
                            thing.key.name,
                            group
                        );
                    }
                    None => {
                        chat::respond!(
                            $ctx,
                            "{} `{}` does not belong to a group",
                            $what,
                            thing.key.name
                        );
                    }
                }

                return Ok(());
            }
        };

        if !$db
            .edit_group($ctx.channel(), &name, Some(group.clone()))
            .await?
        {
            chat::respond!($ctx, "no such {}", $what);
            return Ok(());
        }

        chat::respond!($ctx, "set group for {} `{}` to {}", $what, name, group);
    }};
}

macro_rules! command_list {
    ($ctx:expr, $db:expr, $what:expr) => {{
        let mut names = $db
            .list($ctx.channel())
            .await
            .into_iter()
            .map(|c| c.key.name.to_string())
            .collect::<Vec<_>>();

        if names.is_empty() {
            chat::respond!($ctx, "No custom {}.", $what);
        } else {
            names.sort();
            chat::respond!($ctx, "{}", names.join(", "));
        }
    }};
}

macro_rules! command_delete {
    ($ctx:expr, $db:expr, $what:expr, $edit_scope:ident) => {{
        $ctx.check_scope(::auth::Scope::$edit_scope).await?;

        let name = match $ctx.next() {
            Some(name) => name,
            None => {
                chat::respond!($ctx, "Expected <name>");
                return Ok(());
            }
        };

        if $db.delete($ctx.channel(), &name).await? {
            chat::respond!($ctx, "Deleted {} `{}`", $what, name);
        } else {
            chat::respond!($ctx, "No such {}", $what);
        }
    }};
}

macro_rules! command_rename {
    ($ctx:expr, $db:expr, $what:expr, $edit_scope:ident) => {{
        $ctx.check_scope(::auth::Scope::$edit_scope).await?;

        let (from, to) = match ($ctx.next(), $ctx.next()) {
            (Some(from), Some(to)) => (from, to),
            _ => {
                chat::respond!($ctx, "Expected <from> <to>");
                return Ok(());
            }
        };

        match $db.rename($ctx.channel(), &from, &to).await {
            Ok(()) => {
                chat::respond!($ctx, "Renamed {} {} -> {}.", $what, from, to);
            }
            Err(::db::RenameError::Conflict) => {
                chat::respond!($ctx, "Already an {} named `{}`.", $what, to);
            }
            Err(::db::RenameError::Missing) => {
                chat::respond!($ctx, "No {} named `{}`.", $what, from);
            }
        }
    }};
}

macro_rules! command_show {
    ($ctx:expr, $db:expr, $what:expr) => {{
        let name = match $ctx.next() {
            Some(name) => name,
            None => {
                chat::respond!($ctx, "Expected <name> to show");
                return Ok(());
            }
        };

        let thing = $db.get_any($ctx.channel(), &name).await?;

        match thing {
            Some(thing) => {
                chat::respond!($ctx, format!("{} -> {}", thing.key.name, thing));
            }
            None => {
                chat::respond!($ctx, format!("No {} named `{}`.", $what, name));
            }
        }
    }};
}

macro_rules! command_base {
    ($ctx:expr, $db:expr, $what:expr, $edit_scope:ident) => {{
        let arg = $ctx.next();

        match arg.as_deref() {
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
            _ => arg,
        }
    }};
}
