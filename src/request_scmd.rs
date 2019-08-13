use serde_json::value::Value;
use serde_json::Map;
use std::collections::HashMap;
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

fn print_response_template(template: String, app_name: &String, context: Context, result: Value) {
    let template_parser = template::Template::new(app_name);
    let mut response_context = HashMap::new();
    response_context.insert(String::from("response"), result.clone());
    let complete_context = get_complete_context(response_context, context.clone());
    print!(
        "{}",
        template_parser.get_compiled_template_with_context(template, complete_context)
    );
}

fn get_complete_context(
    mut response_context: HashMap<String, Value>,
    general_context: Context,
) -> HashMap<String, Value> {
    for (key, inner_hashmap) in general_context {
        let mut map = Map::new();
        for (ikey, ivalue) in inner_hashmap {
            map.insert(ikey, ivalue);
        }
        response_context.insert(key, Value::Object(map));
    }
    response_context
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
    let mut http_method: String;
    if subcmd_hash.contains_key(&Yaml::from_str("method")) {
        let method_template = subcmd_yaml["method"].clone().into_string().unwrap();
        http_method = template::get_compiled_template_str_with_context(&method_template, &context)
            .expect("Could not parse request method");
    } else {
        http_method = String::from("get")
    }

    let mut headers = yaml::get_hash_from_yaml(&yaml["headers"], &context, false);

    let body = yaml::get_hash_from_yaml(&subcmd_yaml["body"], &context, true);

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

    let mut response = http::request(&http_method, &endpoint, &headers, &body);
    let result: Value = response.json().expect(&format!(
        "Could not convert response {:?} to json",
        response
    ));

    // Quiet
    if context["args"].contains_key("quiet") {
        return;
    }

    // Raw output
    if context["args"].contains_key("raw_response") {
        print_response_json(&result, false);
        return;
    }

    if context["args"].contains_key("template") {
        let template = context["args"]["template"].clone();
        if template == "json" {
            print_response_json(&result, true);
        } else {
            let template_str = template
                .as_str() // avoids quotes on the string
                .expect("Could not convert template str")
                .to_string();
            print_response_template(template_str, app_name, context, result);
        }
    } else if subcmd_hash.contains_key(&Yaml::from_str("response_template")) {
        let response_template = subcmd_yaml["response_template"]
            .clone()
            .into_string()
            .unwrap();
        print_response_template(response_template, app_name, context, result);
    } else {
        print_response_json(&result, true);
        return;
    }
}
