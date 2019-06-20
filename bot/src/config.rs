use crate::{settings, web};

#[derive(Debug)]
pub struct Spotify;

impl OAuth2Params for Spotify {
    fn new_flow_builder(
        web: web::Server,
        settings: settings::Settings,
        shared_settings: settings::Settings,
    ) -> Result<crate::oauth2::FlowBuilder, failure::Error> {
        crate::oauth2::spotify(web, settings, shared_settings)
    }
}

/// Define defaults for fields.
pub trait OAuth2Params {
    fn new_flow_builder(
        web: web::Server,
        settings: settings::Settings,
        shared_settings: settings::Settings,
    ) -> Result<crate::oauth2::FlowBuilder, failure::Error>;
}

/// Create a new flow based on a statis configuration.
pub fn new_oauth2_flow<T>(
    web: web::Server,
    local: &str,
    shared: &str,
    settings: &settings::Settings,
) -> Result<crate::oauth2::FlowBuilder, failure::Error>
where
    T: OAuth2Params,
{
    let local_settings = settings.scoped(local);
    let shared_settings = settings.scoped(shared);
    Ok(T::new_flow_builder(web, local_settings, shared_settings)?)
}
