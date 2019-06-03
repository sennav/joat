extern crate base64;
extern crate oauth2;
extern crate rand;
extern crate url;

use oauth2::basic::BasicClient;
use oauth2::prelude::*;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, TokenUrl,
};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use url::Url;
use std::path::Path;
use std::fs;
use std::process::Command;

fn start_callback_server(
        client_id_str: String,
        client_secret_str: String,
        client: BasicClient,
        csrf_state: CsrfToken) -> Option<String>
{
    // A very naive implementation of the redirect server.
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    for stream in listener.incoming() {
        if let Ok(mut stream) = stream {
            let code;
            let state;
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

                let state_pair = url
                    .query_pairs()
                    .find(|pair| {
                        let &(ref key, _) = pair;
                        key == "state"
                    })
                    .unwrap();

                let (_, value) = state_pair;
                state = CsrfToken::new(value.into_owned());
            }

            if state != csrf_state {
                let message = "Authentication failed, exiting";
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
                    message.len(),
                    message
                );
                stream.write_all(response.as_bytes()).unwrap();
                panic!("Authentication failed");
            }

            let message = include_str!("../templates/oauth_success.html");
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
                message.len(),
                message
            );
            stream.write_all(response.as_bytes()).unwrap();

            // Exchange the code with a token.
            let token = client.exchange_code_extension(
                code,
                &[
                    ("client_id", client_id_str),
                    ("client_secret", client_secret_str),
                ],
            );

            return Some(
                token
                .expect("Could not get token")
                .get_access_token()
                .secret()
                .to_string());
        }
    };
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
        let access_token = fs::read_to_string(token_path)
            .expect("Could not read token file");
        return Some(access_token);
    }
    return None;
}

fn write_access_token(app_name: &str, access_token: &String) {
    let token_path = get_token_file_path(app_name);
    println!("Trying to write in {:?}\nContent:{:?}", token_path, access_token);
    fs::write(token_path, access_token).expect("Unable to write token file");
}

fn oauth_flow(
    client_id_str: String,
    client_secret_str: String,
    auth_url_str: String,
    token_url_str: String)-> String 
{
    let client_id = ClientId::new(client_id_str.clone());
    let client_secret = ClientSecret::new(client_secret_str.clone());
    let auth_url = AuthUrl::new(
        Url::parse(&auth_url_str).expect("Invalid authorization endpoint URL"));
    let token_url = TokenUrl::new(
        Url::parse(&token_url_str).expect("Invalid token endpoint URL"));

    let client = BasicClient::new(
        client_id,
        Some(client_secret),
        auth_url,
        Some(token_url))
        .set_redirect_url(RedirectUrl::new(
            Url::parse("http://localhost:8080").expect("Invalid redirect URL"),
        ));

    let (authorize_url, csrf_state) = client.authorize_url(CsrfToken::new_random);
    Command::new("open")
        .arg(authorize_url.to_string())
        .output()
        .expect("failed to execute script");

    return start_callback_server(
        client_id_str,
        client_secret_str,
        client,
        csrf_state)
        .expect("Could not get access_token with oauth_flow");
}

pub fn get_oauth_token(
        app_name: &str,
        client_id_str: String,
        client_secret_str: String,
        auth_url_str: String,
        token_url_str: String) -> String
{
    let access_token = get_token_from_file(app_name);
    match access_token {
        Some(t) => t,
        None => {
            let access_token = oauth_flow(
                client_id_str,
                client_secret_str,
                auth_url_str,
                token_url_str);
            write_access_token(app_name, &access_token);
            access_token
        }
    }
}
