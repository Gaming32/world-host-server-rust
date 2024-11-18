use crate::authlib::client::MinecraftClient;
use crate::authlib::environment::Environment;
use crate::authlib::response::HasJoinedMinecraftServerResponse;
use reqwest::Url;
use uuid::Uuid;

pub struct YggdrasilMinecraftSessionService {
    client: MinecraftClient,
    check_url: Url,
}

impl YggdrasilMinecraftSessionService {
    pub fn new(env: &Environment) -> Self {
        let base_url = format!("{}/session/minecraft/", env.session_host);
        Self {
            client: MinecraftClient::unauthenticated(),
            check_url: format!("{base_url}hasJoined").parse().unwrap(),
        }
    }

    pub async fn has_joined_server(
        &self,
        profile_name: &str,
        server_id: &str,
    ) -> anyhow::Result<Option<Uuid>> {
        let arguments = vec![("username", profile_name), ("serverId", server_id)];
        let url = format!("{}?{}", self.check_url, querystring::stringify(arguments));
        self.client
            .get::<HasJoinedMinecraftServerResponse, _>(url)
            .await
            .map(|o| o.map(|r| r.id))
    }
}
