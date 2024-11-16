use crate::authlib::auth_service::YggdrasilAuthenticationService;
use crate::server_state::ServerState;

pub async fn run_main_server(server: &ServerState) {
    let session_service = YggdrasilAuthenticationService::new().create_minecraft_session_service();
}
