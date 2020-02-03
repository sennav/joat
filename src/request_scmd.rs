use log::debug;
use reqwest::header::HeaderMap;
use serde_json::value::Value;
use serde_json::Map;
use yaml_rust::Yaml;

use crate::{http, oauth, template, yaml, Context};

fn get_parsed_yaml_key(key: &str, yaml: &Yaml, error_str: &str, context: &Context) -> String {
    template::get_compiled_template_str_with_context(
        &yaml[key].clone().into_string().expect(error_str),
        context,
    )
    .expect(format!("Could not parse template for yaml key: {}", key).as_str())
}

fn print_response_json(result: &Value, pretty: bool) {
    if pretty {
        print!(
            "{}",
            serde_json::to_string_pretty(result)
                .expect("Could not convert response to pretty print json")
        );
    } else {
        print!("{}", result);
    }
}

fn print_response_template(
    template: String,
    app_name: &String,
    mut context: Context,
    response_body: Value,
    headers_context: Value,
) {
    let template_parser = template::Template::new(app_name);

    context.insert(String::from("response"), response_body);
    context.insert(String::from("response_headers"), headers_context);
    print!(
        "{}",
        template_parser.get_compiled_template_with_context(template, context)
    );
}

fn get_headers_map(headers: &HeaderMap) -> Value {
    let mut map = Map::new();
    for (key, value) in headers {
        let key_str = key.as_str();
        let value_str = value.to_str().unwrap();
        map.insert(key_str.to_string(), Value::String(value_str.to_string()));
    }
    Value::Object(map)
}

pub fn execute_request(
    app_name: &String,
    cmd_name: &str,
    yaml: &Yaml,
    subcmd_yaml: &Yaml,
    context: Context,
) {
    let subcmd_hash = subcmd_yaml
        .clone()
        .into_hash()
        .expect("Could not hash subcmd yaml");
    let http_method: String;
    if subcmd_hash.contains_key(&Yaml::from_str("method")) {
        let method_template = subcmd_yaml["method"].clone().into_string().unwrap();
        http_method = template::get_compiled_template_str_with_context(&method_template, &context)
            .expect("Could not parse request method");
    } else {
        http_method = String::from("get")
    }

    let mut headers = yaml::get_hash_from_yaml(&yaml["headers"], &context, false);

    let body = yaml::get_hash_from_yaml(&subcmd_yaml["body"], &context, true);
    let form = yaml::get_hash_from_yaml(&subcmd_yaml["form"], &context, true);

    let query_params_yaml =
        yaml::combine_hash_yaml(&subcmd_yaml["query_params"], &yaml["query_params"]);
    let query_params = yaml::get_hash_from_yaml(&query_params_yaml, &context, false);

    let endpoint = http::get_endpoint(&cmd_name, &context, &yaml, &query_params);

    let oauth_yaml = &yaml["oauth"];
    if !oauth_yaml.is_badvalue() {
        let client_id =
            get_parsed_yaml_key("client_id", &oauth_yaml, "Missing client_id", &context);
        let client_secret = get_parsed_yaml_key(
            "client_secret",
            &oauth_yaml,
            "Missing client_secret",
            &context,
        );
        let auth_url = get_parsed_yaml_key("auth_url", &oauth_yaml, "Missing auth_url", &context);
        let token_url =
            get_parsed_yaml_key("token_url", &oauth_yaml, "Missing token_url", &context);
        let oauth_token =
            oauth::get_oauth_token(app_name, client_id, client_secret, auth_url, token_url);

        let header_name =
            get_parsed_yaml_key("header_key", &oauth_yaml, "Missing header_key", &context);
        headers.insert(header_name, Value::String(oauth_token));
    }
    debug!("Request Body {:?}", body);
    let mut response = http::request(&http_method, &endpoint, &headers, &body, &form);
    let response_body: Value = match response.json() {
        Ok(r) => r,
        Err(_e) => {
            let response_str = response
                .text()
                .expect("Could not convert response to json or text");
            Value::String(response_str)
        }
    };
    debug!("{:?}", response_body);

    let context_args = context["args"].as_object().unwrap();

    // Quiet
    if context_args.contains_key("quiet") {
        return;
    }

    // Raw output
    if context_args.contains_key("raw_response") {
        print_response_json(&response_body, false);
        return;
    }

    let headers_map = get_headers_map(response.headers());

    if context_args.contains_key("template") {
        let template = context["args"]["template"].clone();
        if template == "json" {
            print_response_json(&response_body, true);
        } else {
            let template_str = template
                .as_str() // avoids quotes on the string
                .expect("Could not convert template str")
                .to_string();
            print_response_template(template_str, app_name, context, response_body, headers_map);
        }
    } else if subcmd_hash.contains_key(&Yaml::from_str("response_template")) {
        let response_template = subcmd_yaml["response_template"]
            .clone()
            .into_string()
            .unwrap();
        print_response_template(
            response_template,
            app_name,
            context,
            response_body,
            headers_map,
        );
    } else {
        print_response_json(&response_body, true);
        return;
    }
}
