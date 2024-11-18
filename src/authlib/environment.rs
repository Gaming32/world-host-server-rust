pub const PROD_ENVIRONMENT: Environment = Environment {
    session_host: "https://sessionserver.mojang.com",
    services_host: "https://api.minecraftservices.com",
    name: "PROD",
};

#[derive(Copy, Clone, Debug)]
#[allow(dead_code)] // Useful for Debug output
pub struct Environment<'a> {
    pub session_host: &'a str,
    pub services_host: &'a str,
    pub name: &'a str,
}
