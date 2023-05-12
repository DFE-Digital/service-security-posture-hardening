use crate::crazy_splunk_x509v1::crazy_x509_tls_clientconfig;
use anyhow::{anyhow, Context, Result};
use reqwest::header;
mod crazy_splunk_x509v1;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Client {
    /// Server URI to connct to
    server_uri: String,
    reqwest_client: reqwest::Client,
}

impl Client {
    pub fn new<S: Into<String>>(server_uri: S, session_key: S, verify: bool) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        let mut auth_value =
            header::HeaderValue::from_str(&format!("Splunk {}", &session_key.into()))?;
        auth_value.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_value);

        let reqwest_client = reqwest::ClientBuilder::new()
            .user_agent("splunk_rest_client")
            .default_headers(headers)
            .danger_accept_invalid_certs(verify)
            // Needed because Splunk ships with broken certs :(
            .use_preconfigured_tls(crazy_x509_tls_clientconfig())
            .build()?;

        Ok(Self {
            server_uri: server_uri.into(),
            //            verify,
            reqwest_client,
        })
    }

    pub async fn new_from_username_password<S: Into<String>>(
        server_uri: S,
        username: S,
        password: S,
        verify: bool,
    ) -> Result<Self> {
        let reqwest_client = reqwest::ClientBuilder::new()
            .user_agent("splunk_rest_client")
            .danger_accept_invalid_certs(verify)
            // Needed because Splunk ships with broken certs :(
            .use_preconfigured_tls(crazy_x509_tls_clientconfig())
            .build()?;
        let server_uri = server_uri.into();
        let auth = reqwest_client
            .post(format!(
                "{}/services/auth/login?output_mode=json",
                &server_uri
            ))
            .body(format!(
                "username={}&password={}",
                username.into(),
                password.into()
            ))
            .send()
            .await?
            .json::<AuthResponse>()
            .await?;
        Self::new(server_uri, auth.session_key, verify)
    }

    fn path(&self, path: &str) -> String {
        format!("{}/services{}?output_mode=json", self.server_uri, path)
    }

    async fn get(&self, path: &str) -> Result<reqwest::Response> {
        self.reqwest_client
            .get(self.path(path))
            .send()
            .await
            .context(format!("Reqwest to {} failed!", &path))
    }

    async fn post<S: Into<String>>(&self, path: S, body: S) -> Result<reqwest::Response> {
        let path = path.into();
        self.reqwest_client
            .post(self.path(&path))
            .body(body.into())
            .send()
            .await
            .context(format!("Reqwest to {} failed!", &path))
    }

    pub async fn list_passwords(&self, app: &str) -> Result<reqwest::Response> {
        self.get(&format!("NS/nobody/{}/storage/passwords", app))
            .await
    }

    pub async fn get_password(&self, app: &str, user: &str, realm: Option<&str>) -> Result<String> {
        let path = if let Some(realm) = realm {
            format!("NS/nobody/{}/storage/passwords/{}:{}", app, realm, user)
        } else {
            format!("NS/nobody/{}/storage/passwords/{}", app, user)
        };

        let response = self.get(&path).await?;

        if response.status().is_success() {
            Ok(response
                .json::<PasswordResponse>()
                .await?
                .entry
                .first()
                .context("No Splunk passwords found!")?
                .content
                .clear_password
                .to_owned())
        } else {
            Err(anyhow!(format!(
                "Failed to get Splunk password: '{}' {}",
                path,
                response.text().await?
            )))
        }
    }

    /// https://dev.splunk.com/enterprise/docs/developapps/manageknowledge/secretstorage/secretstoragerest/
    pub async fn set_password(
        &self,
        app: &str,
        name: &str,
        password: &str,
        realm: Option<&str>,
    ) -> Result<reqwest::Response> {
        let path = format!("NS/nobody/{}/storage/passwords", app);
        let body = if let Some(realm) = realm {
            format!("name={}&password={}&realm={}", name, password, realm)
        } else {
            format!("name={}&password={}", name, password)
        };
        self.post(path, body).await
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct PasswordResponse {
    entry: Vec<PasswordEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PasswordEntry {
    content: PasswordContent,
    name: String,
}
#[derive(Serialize, Deserialize, Debug)]
struct PasswordContent {
    clear_password: String,
    realm: String,
    username: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct AuthResponse {
    #[serde(alias = "sessionKey")]
    session_key: String,
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    //#[tokio::test]
    fn create_client() -> Client {
        let session_key = std::env::var("splunk_session_key")
            .context("`splunk_session_key` env variable not set")
            .unwrap();
        Client::new("https://localhost:8089", &session_key, false).unwrap()
    }

    #[tokio::test]
    async fn client_from_username_password() {
        let client = Client::new_from_username_password(
            "https://localhost:8089",
            &std::env::var("splunk_username").unwrap(),
            &std::env::var("splunk_password").unwrap(),
            false,
        )
        .await
        .unwrap();
        dbg!(client);
    }

    #[tokio::test]
    async fn test_list_passwords() {
        let result = create_client().list_passwords("search").await.unwrap();
        dbg!(&result.json::<Value>().await.unwrap());
    }

    #[tokio::test]
    async fn test_get_password_user() {
        let password = create_client()
            .get_password("search", "user1", None)
            .await
            .unwrap();
        assert_eq!(&password, &"password1");
        assert!(false);
    }

    #[tokio::test]
    async fn test_get_password_user_realm() {
        let password = create_client()
            .get_password("search", "user2aaa", Some("realm1"))
            .await
            .unwrap();
        assert_eq!(&password, &"realmpass");
    }

    #[tokio::test]
    async fn test_set_password() {
        let client = create_client();
        client
            .set_password("search", "user1", "password1", None)
            .await
            .unwrap();
        client
            .set_password("search", "user2", "realmpass", Some("realm1"))
            .await
            .unwrap();
    }
}
