use crate::authlib::client::MinecraftClient;
use crate::authlib::environment::Environment;
use crate::authlib::response::HasJoinedMinecraftServerResponse;
use reqwest::Url;
use uuid::Uuid;

pub struct YggdrasilMinecraftSessionService {
    client: MinecraftClient,
    base_url: String,
    join_url: Url,
    check_url: Url,
}

impl YggdrasilMinecraftSessionService {
    pub fn new(env: &Environment) -> Self {
        let base_url = format!("{}/session/minecraft/", env.session_host);
        Self {
            client: MinecraftClient::unauthenticated(),
            join_url: format!("{base_url}join").parse().unwrap(),
            check_url: format!("{base_url}hasJoined").parse().unwrap(),
            base_url,
        }
    }

    pub async fn has_joined_server(
        &self,
        profile_name: &str,
        server_id: &str,
    ) -> anyhow::Result<Option<Uuid>> {
        let arguments = vec![("username", profile_name), ("serverId", server_id)];
        let url: Url =
            format!("{}{}", self.check_url, querystring::stringify(arguments)).parse()?;
        self.client
            .get::<HasJoinedMinecraftServerResponse>(url)
            .await
            .map(|o| o.map(|r| r.id))
    }
}