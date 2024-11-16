use crate::authlib::environment::{Environment, PROD_ENVIRONMENT};
use crate::authlib::session_service::YggdrasilMinecraftSessionService;
use log::info;

pub struct YggdrasilAuthenticationService<'a> {
    environment: Environment<'a>,
}

impl<'a> YggdrasilAuthenticationService<'a> {
    pub fn new() -> Self {
        Self::new_with_environment(determine_environment())
    }

    pub fn new_with_environment(environment: Environment<'a>) -> Self {
        info!("Environment: {environment:?}");
        YggdrasilAuthenticationService { environment }
    }

    pub fn create_minecraft_session_service(&self) -> YggdrasilMinecraftSessionService {
        YggdrasilMinecraftSessionService::new(&self.environment)
    }
}

fn determine_environment<'a>() -> Environment<'a> {
    PROD_ENVIRONMENT
}
