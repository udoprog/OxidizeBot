/// Helper macro to build database functions for managing groups.
macro_rules! database_group_fns {
    ($thing:ty, $key:ty) => {
        /// Set which group the thing belongs to.
        pub async fn edit_group(
            &self,
            channel: &str,
            name: &str,
            group: Option<String>,
        ) -> Result<bool, anyhow::Error> {
            let key = <$key>::new(channel, name);

            let mut inner = self.inner.write().await;

            if let Some(mut thing) = inner.get(&key).map(|v| (**v).clone()) {
                self.db.edit_group(&key, group.clone()).await?;
                thing.group = group;
                inner.insert(key, Arc::new(thing));
                return Ok(true);
            }

            Ok(false)
        }

        /// Enable the given thing.
        pub async fn enable(&self, channel: &str, name: &str) -> Result<bool, anyhow::Error> {
            let key = <$key>::new(channel, name);

            let thing = match self.db.fetch(&key).await? {
                Some(thing) => <$thing>::from_db(&thing)?,
                None => return Ok(false),
            };

            self.db.edit_disabled(&thing.key, false).await?;
            self.inner
                .write()
                .await
                .insert(thing.key.clone(), Arc::new(thing));
            Ok(true)
        }

        /// Disable the given thing.
        pub async fn disable(&self, channel: &str, name: &str) -> Result<bool, anyhow::Error> {
            let key = <$key>::new(channel, name);
            let mut inner = self.inner.write().await;

            if let Some(thing) = inner.remove(&key) {
                self.db.edit_disabled(&thing.key, true).await?;
                return Ok(true);
            }

            Ok(false)
        }

        /// Enable all things in the given group.
        pub async fn enable_group(&self, channel: &str, group: &str) -> Result<(), anyhow::Error> {
            self.db.set_group_disabled(channel, group, false).await?;

            let mut inner = self.inner.write().await;

            for thing in self.db.list_group(channel, group).await? {
                let thing = <$thing>::from_db(&thing)?;
                inner.insert(thing.key.clone(), Arc::new(thing));
            }

            Ok(())
        }

        /// Disable all things in the given group.
        pub async fn disable_group(&self, channel: &str, group: &str) -> Result<(), anyhow::Error> {
            self.db.set_group_disabled(channel, group, true).await?;

            let mut inner = self.inner.write().await;

            let mut to_delete = Vec::new();

            for (key, value) in inner.iter() {
                if value.group.as_deref() == Some(group) {
                    to_delete.push(key.clone());
                }
            }

            for key in to_delete {
                inner.remove(&key);
            }

            Ok(())
        }

        /// Get a list of all members.
        pub async fn list_all(&self, channel: &str) -> Result<Vec<$thing>, anyhow::Error> {
            let mut out = Vec::new();

            for p in self.db.list_all(channel).await? {
                out.push(<$thing>::from_db(&p)?);
            }

            Ok(out)
        }

        /// Remove thing.
        pub async fn delete(&self, channel: &str, name: &str) -> Result<bool, anyhow::Error> {
            let key = <$key>::new(channel, name);

            if !self.db.delete(&key).await? {
                return Ok(false);
            }

            self.inner.write().await.remove(&key);
            Ok(true)
        }

        /// Get the given thing by name.
        pub async fn get(&self, channel: &str, name: &str) -> Option<Arc<$thing>> {
            let key = <$key>::new(channel, name);

            let inner = self.inner.read().await;

            if let Some(thing) = inner.get(&key) {
                return Some(Arc::clone(thing));
            }

            None
        }

        /// Get the given thing by name directly from the database.
        pub async fn get_any(
            &self,
            channel: &str,
            name: &str,
        ) -> Result<Option<$thing>, anyhow::Error> {
            let key = <$key>::new(channel, name);
            let thing = match self.db.fetch(&key).await? {
                Some(thing) => thing,
                None => return Ok(None),
            };
            Ok(Some(<$thing>::from_db(&thing)?))
        }

        /// Get a list of all things.
        pub async fn list(&self, channel: &str) -> Vec<Arc<$thing>> {
            let inner = self.inner.read().await;

            let mut out = Vec::new();

            for thing in inner.values() {
                if thing.key.channel != channel {
                    continue;
                }

                out.push(Arc::clone(thing));
            }

            out
        }

        /// Try to rename the thing.
        pub async fn rename(
            &self,
            channel: &str,
            from: &str,
            to: &str,
        ) -> Result<(), super::RenameError> {
            let from_key = <$key>::new(channel, from);
            let to_key = <$key>::new(channel, to);

            let mut inner = self.inner.write().await;

            if inner.contains_key(&to_key) {
                return Err(super::RenameError::Conflict);
            }

            let thing = match inner.remove(&from_key) {
                Some(thing) => thing,
                None => return Err(super::RenameError::Missing),
            };

            let mut thing = (*thing).clone();
            thing.key = to_key.clone();

            match self.db.rename(&from_key, &to_key).await {
                Err(e) => {
                    log::error!(
                        "failed to rename {what} `{}` in database: {}",
                        from,
                        e,
                        what = <$thing>::NAME
                    );
                }
                Ok(false) => {
                    log::warn!(
                        "{what} {} not renamed in database",
                        from,
                        what = <$thing>::NAME
                    );
                }
                Ok(true) => (),
            }

            inner.insert(to_key, Arc::new(thing));
            Ok(())
        }
    };
}

/// Helper macro to build private database functions related to group management.
macro_rules! private_database_group_fns {
    ($module:ident, $thing:ident, $key:ty) => {
        /// List all members that are not disabled.
        async fn list(&self) -> Result<Vec<db::models::$thing>, anyhow::Error> {
            use db::schema::$module::dsl;

            self.0
                .asyncify(move |c| {
                    Ok(dsl::$module
                        .filter(dsl::disabled.eq(false))
                        .load::<db::models::$thing>(c)?)
                })
                .await
        }

        /// List all members, including disabled ones.
        async fn list_all(&self, channel: &str) -> Result<Vec<db::models::$thing>, anyhow::Error> {
            use db::schema::$module::dsl;
            let channel = channel.to_string();

            self.0
                .asyncify(move |c| {
                    Ok(dsl::$module
                        .filter(dsl::channel.eq(channel))
                        .load::<db::models::$thing>(c)?)
                })
                .await
        }

        /// List all members of the given group.
        async fn list_group(
            &self,
            channel: &str,
            group: &str,
        ) -> Result<Vec<db::models::$thing>, anyhow::Error> {
            use db::schema::$module::dsl;
            let channel = channel.to_string();
            let group = group.to_string();

            self.0
                .asyncify(move |c| {
                    let filter =
                        dsl::$module.filter(dsl::channel.eq(channel).and(dsl::group.eq(group)));
                    Ok(filter.load::<db::models::$thing>(c)?)
                })
                .await
        }

        /// Set if the given group is disabled or not.
        async fn set_group_disabled(
            &self,
            channel: &str,
            group: &str,
            disabled: bool,
        ) -> Result<(), anyhow::Error> {
            use db::schema::$module::dsl;
            let channel = channel.to_string();
            let group = group.to_string();

            self.0
                .asyncify(move |c| {
                    diesel::update(
                        dsl::$module.filter(dsl::channel.eq(channel).and(dsl::group.eq(group))),
                    )
                    .set(dsl::disabled.eq(disabled))
                    .execute(c)?;

                    Ok(())
                })
                .await
        }

        /// Edit the group membership of the given thing.
        async fn edit_group(&self, key: &$key, group: Option<String>) -> Result<(), anyhow::Error> {
            use db::schema::$module::dsl;
            let key = key.clone();

            self.0
                .asyncify(move |c| {
                    diesel::update(
                        dsl::$module
                            .filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name))),
                    )
                    .set(dsl::group.eq(group))
                    .execute(c)?;

                    Ok(())
                })
                .await
        }

        /// Set the disabled state of the given command.
        async fn edit_disabled(&self, key: &$key, disabled: bool) -> Result<(), anyhow::Error> {
            use db::schema::$module::dsl;
            let key = key.clone();

            self.0
                .asyncify(move |c| {
                    diesel::update(
                        dsl::$module
                            .filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name))),
                    )
                    .set(dsl::disabled.eq(disabled))
                    .execute(c)?;

                    Ok(())
                })
                .await
        }

        /// Fetch a single entity.
        async fn fetch(&self, key: &$key) -> Result<Option<db::models::$thing>, anyhow::Error> {
            use db::schema::$module::dsl;
            let key = key.clone();

            self.0
                .asyncify(move |c| {
                    let thing = dsl::$module
                        .filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name)))
                        .first::<db::models::$thing>(c)
                        .optional()?;

                    Ok(thing)
                })
                .await
        }

        /// Delete a single thing.
        async fn delete(&self, key: &$key) -> Result<bool, anyhow::Error> {
            use db::schema::$module::dsl;
            let key = key.clone();

            self.0
                .asyncify(move |c| {
                    let count = diesel::delete(
                        dsl::$module
                            .filter(dsl::channel.eq(&key.channel).and(dsl::name.eq(&key.name))),
                    )
                    .execute(c)?;
                    Ok(count == 1)
                })
                .await
        }

        /// Rename one thing to another.
        async fn rename(&self, from: &$key, to: &$key) -> Result<bool, anyhow::Error> {
            use db::schema::$module::dsl;
            let from = from.clone();
            let to = to.clone();

            self.0
                .asyncify(move |c| {
                    let count = diesel::update(
                        dsl::$module
                            .filter(dsl::channel.eq(&from.channel).and(dsl::name.eq(&from.name))),
                    )
                    .set((dsl::channel.eq(&to.channel), dsl::name.eq(&to.name)))
                    .execute(c)?;

                    Ok(count == 1)
                })
                .await
        }
    };
}
