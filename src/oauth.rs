extern crate base64;
extern crate oauth2;
extern crate rand;
extern crate url;

use oauth2::basic::{BasicErrorResponse, BasicTokenType};
use oauth2::helpers;
use oauth2::reqwest::http_client;
use oauth2::{
    AccessToken, Client, EmptyExtraTokenFields, ExtraTokenFields, RefreshToken, Scope,
    TokenResponse, TokenType,
};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, TokenUrl,
};

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use url::Url;

//
// Custom oauth client implementation for non standard oauth providers
//

type SpecialTokenResponse = NonStandardTokenResponse<EmptyExtraTokenFields>;
type SpecialClient = Client<BasicErrorResponse, SpecialTokenResponse, BasicTokenType>;

fn default_token_type() -> Option<BasicTokenType> {
    Some(BasicTokenType::Bearer)
}

///
/// Non Standard OAuth2 token response.
///
/// Some providers don't follow the RFC correctly this Token response deals with it.
/// This struct includes the fields defined in
/// [Section 5.1 of RFC 6749](https://tools.ietf.org/html/rfc6749#section-5.1), as well as
/// extensions defined by the `EF` type parameter.
///
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NonStandardTokenResponse<EF: ExtraTokenFields> {
    access_token: AccessToken,
    #[serde(default = "default_token_type")]
    token_type: Option<BasicTokenType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_in: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh_token: Option<RefreshToken>,
    #[serde(rename = "scope")]
    #[serde(deserialize_with = "helpers::deserialize_space_delimited_vec")]
    #[serde(serialize_with = "helpers::serialize_space_delimited_vec")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    scopes: Option<Vec<Scope>>,

    #[serde(bound = "EF: ExtraTokenFields")]
    #[serde(flatten)]
    extra_fields: EF,
}

impl<EF> TokenResponse<BasicTokenType> for NonStandardTokenResponse<EF>
where
    EF: ExtraTokenFields,
    BasicTokenType: TokenType,
{
    ///
    /// REQUIRED. The access token issued by the authorization server.
    ///
    fn access_token(&self) -> &AccessToken {
        &self.access_token
    }
    ///
    /// REQUIRED. The type of the token issued as described in
    /// [Section 7.1](https://tools.ietf.org/html/rfc6749#section-7.1).
    /// Value is case insensitive and deserialized to the generic `TokenType` parameter.
    ///
    fn token_type(&self) -> &BasicTokenType {
        match &self.token_type {
            Some(t) => t,
            None => &BasicTokenType::Bearer,
        }
    }
    ///
    /// RECOMMENDED. The lifetime in seconds of the access token. For example, the value 3600
    /// denotes that the access token will expire in one hour from the time the response was
    /// generated. If omitted, the authorization server SHOULD provide the expiration time via
    /// other means or document the default value.
    ///
    fn expires_in(&self) -> Option<Duration> {
        self.expires_in.map(Duration::from_secs)
    }
    ///
    /// OPTIONAL. The refresh token, which can be used to obtain new access tokens using the same
    /// authorization grant as described in
    /// [Section 6](https://tools.ietf.org/html/rfc6749#section-6).
    ///
    fn refresh_token(&self) -> Option<&RefreshToken> {
        self.refresh_token.as_ref()
    }
    ///
    /// OPTIONAL, if identical to the scope requested by the client; otherwise, REQUIRED. The
    /// scipe of the access token as described by
    /// [Section 3.3](https://tools.ietf.org/html/rfc6749#section-3.3). If included in the response,
    /// this space-delimited field is parsed into a `Vec` of individual scopes. If omitted from
    /// the response, this field is `None`.
    ///
    fn scopes(&self) -> Option<&Vec<Scope>> {
        self.scopes.as_ref()
    }
}

fn start_callback_server(
    client_id_str: String,
    client_secret_str: String,
    client: SpecialClient,
    _csrf_state: CsrfToken,
) -> Option<String> {
    // TODO: is there a non naive way of doing this?
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    for stream in listener.incoming() {
        if let Ok(mut stream) = stream {
            let code;
            {
                let mut reader = BufReader::new(&stream);

                let mut request_line = String::new();
                reader.read_line(&mut request_line).unwrap();

                let redirect_url = request_line.split_whitespace().nth(1).unwrap();
                let url = Url::parse(&("http://localhost".to_string() + redirect_url)).unwrap();

                let code_pair = url
                    .query_pairs()
                    .find(|pair| {
                        let &(ref key, _) = pair;
                        key == "code"
                    })
                    .unwrap();

                let (_, value) = code_pair;
                code = AuthorizationCode::new(value.into_owned());
            }

            let message = include_str!("../templates/oauth_success.html");
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
                message.len(),
                message
            );
            stream.write_all(response.as_bytes()).unwrap();

            // Exchange the code with a token.
            let code_token_request = client
                .exchange_code(code)
                .add_extra_param("client_id", client_id_str)
                .add_extra_param("client_secret", client_secret_str);
            let token = code_token_request.request(http_client);

            return Some(
                token
                    .expect("Could not get token")
                    .access_token()
                    .clone()
                    .secret()
                    .to_string(),
            );
        }
    }
    return None;
}

fn get_token_file_path(app_name: &str) -> String {
    let home_dir_path = dirs::home_dir().expect("Could not find home dir");
    let home_dir_str = home_dir_path.into_os_string().into_string().unwrap();
    let home_path_str = String::from(format!("{}/.{}.joat/", home_dir_str, app_name));
    format!("{}.{}.token", home_path_str, app_name)
}

fn get_token_from_file(app_name: &str) -> Option<String> {
    let token_path = get_token_file_path(app_name);
    if Path::new(&token_path).exists() {
        let access_token = fs::read_to_string(token_path).expect("Could not read token file");
        return Some(access_token);
    }
    return None;
}

fn write_access_token(app_name: &str, access_token: &String) {
    let token_path = get_token_file_path(app_name);
    fs::write(token_path, access_token).expect("Unable to write token file");
}

fn oauth_flow(
    client_id_str: String,
    client_secret_str: String,
    auth_url_str: String,
    token_url_str: String,
) -> String {
    let client_id = ClientId::new(client_id_str.clone());
    let client_secret = ClientSecret::new(client_secret_str.clone());
    let auth_url =
        AuthUrl::new(Url::parse(&auth_url_str).expect("Invalid authorization endpoint URL"));
    let token_url = TokenUrl::new(Url::parse(&token_url_str).expect("Invalid token endpoint URL"));

    let client = SpecialClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
        .set_redirect_url(RedirectUrl::new(
            Url::parse("http://localhost:8080").expect("Invalid redirect URL"),
        ));

    let (authorize_url, csrf_state) = client.authorize_url(CsrfToken::new_random).url();
    Command::new("open")
        .arg(authorize_url.to_string())
        .output()
        .expect("failed to execute script");

    return start_callback_server(client_id_str, client_secret_str, client, csrf_state)
        .expect("Could not get access_token with oauth_flow");
}

pub fn get_oauth_token(
    app_name: &str,
    client_id_str: String,
    client_secret_str: String,
    auth_url_str: String,
    token_url_str: String,
) -> String {
    let access_token = get_token_from_file(app_name);
    match access_token {
        Some(t) => t,
        None => {
            let access_token = oauth_flow(
                client_id_str,
                client_secret_str,
                auth_url_str,
                token_url_str,
            );
            write_access_token(app_name, &access_token);
            access_token
        }
    }
}
