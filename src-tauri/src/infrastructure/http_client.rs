use std::sync::OnceLock;

use reqwest::Client;

static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

pub fn http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .pool_max_idle_per_host(2)
            .build()
            .expect("failed to create HTTP client")
    })
}