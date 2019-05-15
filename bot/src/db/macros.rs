/// Helper macro to build database functions for managing groups.
macro_rules! database_group_fns {
    ($thing:ty, $key:ty) => {
        /// Set which group the thing belongs to.
        pub fn edit_group(
            &self,
            channel: &str,
            name: &str,
            group: Option<String>,
        ) -> Result<bool, failure::Error> {
            let key = Key::new(channel, name);

            let mut inner = self.inner.write();

            if let Some(mut thing) = inner.get(&key).map(|v| (**v).clone()) {
                self.db.edit_group(&key, group.clone())?;
                thing.group = group;
                inner.insert(key, Arc::new(thing));
                return Ok(true);
            }

            Ok(false)
        }

        /// Enable the given thing.
        pub fn enable(&self, channel: &str, name: &str) -> Result<bool, failure::Error> {
            let key = <$key>::new(channel, name);

            let thing = match self.db.fetch(&key)? {
                Some(thing) => <$thing>::from_db(thing)?,
                None => return Ok(false),
            };

            self.db.edit_disabled(&thing.key, false)?;
            self.inner.write().insert(thing.key.clone(), Arc::new(thing));
            Ok(true)
        }

        /// Disable the given thing.
        pub fn disable(&self, channel: &str, name: &str) -> Result<bool, failure::Error> {
            let key = <$key>::new(channel, name);
            let mut inner = self.inner.write();

            if let Some(thing) = inner.remove(&key) {
                self.db.edit_disabled(&thing.key, true)?;
                return Ok(true);
            }

            Ok(false)
        }

        /// Enable all things in the given group.
        pub fn enable_group(&self, channel: &str, group: &str) -> Result<(), failure::Error> {
            self.db.set_group_disabled(channel, group, false)?;

            let mut inner = self.inner.write();

            for thing in self.db.list_group(channel, group)? {
                let thing = <$thing>::from_db(thing)?;
                inner.insert(thing.key.clone(), Arc::new(thing));
            }

            Ok(())
        }

        /// Disable all things in the given group.
        pub fn disable_group(&self, channel: &str, group: &str) -> Result<(), failure::Error> {
            self.db.set_group_disabled(channel, group, true)?;

            let mut inner = self.inner.write();

            let mut to_delete = Vec::new();

            for (key, value) in inner.iter() {
                if value.group.as_ref().map(|s| s.as_str()) == Some(group) {
                    to_delete.push(key.clone());
                }
            }

            for key in to_delete {
                inner.remove(&key);
            }

            Ok(())
        }

        /// Get a list of all members.
        pub fn list_all(&self, channel: &str) -> Result<Vec<$thing>, failure::Error> {
            let mut out = Vec::new();

            for p in self.db.list_all(channel)? {
                out.push(<$thing>::from_db(p)?);
            }

            Ok(out)
        }
    };
}

/// Helper macro to build private database functions related to group management.
macro_rules! private_database_group_fns {
    ($module:ident, $thing:ident, $key:ty) => {
        /// List all members that are not disabled.
        fn list(&self) -> Result<Vec<db::models::$thing>, failure::Error> {
            use db::schema::$module::dsl;
            let c = self.0.pool.lock();
            Ok(dsl::$module
                .filter(dsl::disabled.eq(false))
                .load::<db::models::$thing>(&*c)?)
        }

        /// List all members, including disabled ones.
        fn list_all(&self, channel: &str) -> Result<Vec<db::models::$thing>, failure::Error> {
            use db::schema::$module::dsl;
            let c = self.0.pool.lock();
            Ok(dsl::$module
                .filter(dsl::channel.eq(channel))
                .load::<db::models::$thing>(&*c)?)
        }

        /// List all members of the given group.
        fn list_group(
            &self,
            channel: &str,
            group: &str,
        ) -> Result<Vec<db::models::$thing>, failure::Error> {
            use db::schema::$module::dsl;
            let c = self.0.pool.lock();

            let filter = dsl::$module.filter(dsl::channel.eq(channel).and(dsl::group.eq(group)));
            Ok(filter.load::<db::models::$thing>(&*c)?)
        }

        /// Set if the given group is disabled or not.
        fn set_group_disabled(
            &self,
            channel: &str,
            group: &str,
            disabled: bool,
        ) -> Result<(), failure::Error> {
            use db::schema::$module::dsl;
            let c = self.0.pool.lock();

            diesel::update(dsl::$module.filter(dsl::channel.eq(channel).and(dsl::group.eq(group))))
                .set(dsl::disabled.eq(disabled))
                .execute(&*c)?;

            Ok(())
        }

        /// Edit the group membership of the given thing.
        fn edit_group(&self, key: &$key, group: Option<String>) -> Result<(), failure::Error> {
            use db::schema::$module::dsl;
            let c = self.0.pool.lock();

            diesel::update(
                dsl::$module.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name))),
            )
            .set(dsl::group.eq(group))
            .execute(&*c)?;

            Ok(())
        }

        /// Set the disabled state of the given command.
        fn edit_disabled(&self, key: &$key, disabled: bool) -> Result<(), failure::Error> {
            use db::schema::$module::dsl;
            let c = self.0.pool.lock();

            diesel::update(
                dsl::$module.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name))),
            )
            .set(dsl::disabled.eq(disabled))
            .execute(&*c)?;

            Ok(())
        }

        /// Fetch a single entity.
        fn fetch(&self, key: &$key) -> Result<Option<db::models::$thing>, failure::Error> {
            use db::schema::$module::dsl;
            let c = self.0.pool.lock();

            let thing = dsl::$module.filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name)))
                .first::<db::models::$thing>(&*c)
                .optional()?;

            Ok(thing)
        }
    }
}
