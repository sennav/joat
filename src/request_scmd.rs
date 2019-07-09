use yaml_rust::Yaml;
use serde_json::Number;
use std::collections::HashMap;
use std::str::FromStr;
use serde_json::value::Value;

use crate::{ yaml, http, oauth, template };

fn get_parsed_yaml_key(key: &str, yaml: &Yaml, error_str: &str, context: &HashMap<String, HashMap<String, String>>) -> String {
    template::get_compiled_template_str_with_context(
        &yaml[key]
            .clone()
            .into_string()
            .expect(error_str),
        context
    ).expect(format!("Could not parse template for yaml key: {}", key).as_str())
}

fn convert_body_hash(body: HashMap<String, String>) -> HashMap<String, Value> {
    let mut result: HashMap<String, Value> = HashMap::new();
    for (key, value) in body {
        if value == "true" || value == "false" {
            let bool_value = FromStr::from_str(&value)
                .expect("Could not convert boolean value in body");
            result.insert(key, Value::Bool(bool_value));
        } else if value == "[[empty]]" {
            // Do not insert empty values in body
            continue;
        } else {
            match serde_json::from_str::<Number>(&value) {
                Ok(n) => result.insert(key, Value::Number(n)),
                Err(_e) => result.insert(key, Value::String(value)),
            };
        }
    }
    result
}

fn print_response_json(result: &Value, pretty: bool) {
    if pretty {
        print!("{}", serde_json::to_string_pretty(result)
            .expect("Could not convert response to pretty print json"));
    } else {
        print!("{}", result);
    }
}

pub fn execute_request(app_name: &String, cmd_name: &str, yaml: &Yaml, subcmd_yaml: &Yaml, context: HashMap<String, HashMap<String, String>>) {
    let subcmd_hash = subcmd_yaml.clone().into_hash().expect("Could not hash subcmd yaml");
    let mut http_method: String;
    if subcmd_hash.contains_key(&Yaml::from_str("method")) {
        http_method = subcmd_yaml["method"].clone().into_string().unwrap();
    } else {
        http_method = String::from("get")
    }

    let mut headers = yaml::get_hash_from_yaml(&yaml["headers"], &context);

    let raw_body = yaml::get_hash_from_yaml(&subcmd_yaml["body"], &context);
    let body = convert_body_hash(raw_body);

    let query_params = yaml::get_hash_from_yaml(&subcmd_yaml["query_params"], &context);

    let endpoint = http::get_endpoint(&cmd_name, &context, &yaml, &query_params);

    let oauth_yaml = &yaml["oauth"];
    if !oauth_yaml.is_badvalue() {
        let client_id = get_parsed_yaml_key("client_id", &oauth_yaml, "Missing client_id", &context);
        let client_secret = get_parsed_yaml_key("client_secret", &oauth_yaml, "Missing client_secret", &context);
        let auth_url = get_parsed_yaml_key("auth_url", &oauth_yaml, "Missing auth_url", &context);
        let token_url = get_parsed_yaml_key("token_url", &oauth_yaml, "Missing token_url", &context);
        let oauth_token = oauth::get_oauth_token(
            app_name,
            client_id,
            client_secret,
            auth_url,
            token_url,
        );

        let header_name = get_parsed_yaml_key("header_key", &oauth_yaml, "Missing header_key", &context);
        headers.insert(header_name, oauth_token);
    }

    let mut response = http::request(&http_method, &endpoint, &headers, &body);
    let result: Value = response.json().expect(&format!("Could not convert response {:?} to json", response));

    // Raw output
    if context["args"].contains_key("raw_response") {
        print_response_json(&result, false);
        return
    }

    let mut template: String;
    if context["args"].contains_key("template") {
        template = context["args"]["template"].clone();
        if template == "json" {
            print_response_json(&result, true);
            return
        }
    } else if subcmd_hash.contains_key(&Yaml::from_str("response_template")) {
        template = subcmd_yaml["response_template"].clone().into_string().unwrap();
    } else {
        print_response_json(&result, true);
        return
    }

    let mut template_parser = template::Template::new(app_name); // TODO remove mut
    let mut response_context = HashMap::new();
    response_context.insert(String::from("response"), result.clone());
    if !context["args"].contains_key("quiet") {
        print!("{}", template_parser.get_compiled_template_with_context(template, response_context));
    }
}