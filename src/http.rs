use reqwest::Method;
use reqwest::Response;
use serde_json::value::Value;
use std::collections::HashMap;
use std::vec::Vec;
use yaml_rust::Yaml;

use crate::{template, yaml, Context};

fn get_complete_endpoint(base_endpoint: &Yaml, path_yaml: &Yaml) -> String {
    let mut endpoint = yaml::get_string_from_yaml(base_endpoint);
    let path_str = yaml::get_string_from_yaml(path_yaml);
    endpoint.push_str(&path_str);
    return endpoint;
}

fn stringify(query: Vec<(String, String)>) -> String {
    let mut query_params = query.iter().fold(String::new(), |acc, tuple| {
        acc + &tuple.0 + "=" + &tuple.1 + "&"
    });
    query_params.pop();
    query_params
}

fn get_string_from_value(value: &Value) -> &str {
    match value.as_str() {
        Some(s) => s,
        None => "",
    }
}

fn get_endpoint_with_qp(
    endpoint: String,
    query_params: &HashMap<String, Value>,
    context: &Context,
) -> String {
    if endpoint.contains("?") || query_params.is_empty() {
        return endpoint;
    }
    let mut param_vec = Vec::new();
    for (key, value) in query_params {
        if value.is_null() {
            continue;
        }
        let parsed_qp_value = template::get_compiled_template_str_with_context(
            &get_string_from_value(value).to_string(),
            &context,
        )
        .expect("Error parsing query param option, check your template");
        param_vec.push((key.clone(), parsed_qp_value));
    }
    format!("{}?{}", endpoint, stringify(param_vec))
}

pub fn get_endpoint(
    cmd_name: &str,
    context: &Context,
    yaml: &Yaml,
    query_params: &HashMap<String, Value>,
) -> String {
    let subcmd_yaml = yaml::get_subcommand_from_yaml(cmd_name, yaml);
    let raw_endpoint = get_complete_endpoint(&yaml["base_endpoint"], &subcmd_yaml["path"]);
    let endpoint_with_qp = get_endpoint_with_qp(raw_endpoint, query_params, context);
    let parsed_endpoint =
        template::get_compiled_template_str_with_context(&endpoint_with_qp, &context)
            .expect(format!("Could not parse endpoint {}", endpoint_with_qp).as_str());

    return parsed_endpoint;
}

fn get_method(method: &String) -> Method {
    if method == "GET" {
        return Method::GET;
    }
    if method == "PUT" {
        return Method::PUT;
    }
    if method == "POST" {
        return Method::POST;
    }
    if method == "PATCH" {
        return Method::PATCH;
    }
    if method == "DELETE" {
        return Method::DELETE;
    }
    return Method::GET;
}

pub fn request(
    method: &String,
    endpoint: &String,
    headers: &HashMap<String, Value>,
    body: &HashMap<String, Value>,
) -> Response {
    let client = reqwest::Client::new();
    let reqwest_method = get_method(&method);
    let mut request = client.request(reqwest_method, endpoint);
    for (name, value) in headers.iter() {
        let header_value = get_string_from_value(value);
        request = request.header(&name[..], header_value);
    }
    request = request.json(&body);

    let response = match request.send() {
        Ok(t) => t,
        Err(e) => {
            println!("Could not get response for endpoint {}", endpoint);
            println!("Error: {}", e);
            panic!("Failed request");
        }
    };
    return response;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_get_string_from_yaml() {
        // Arrange
        let base_endpoint = Yaml::String("http://example.com".to_string());
        let path = Yaml::String("/path".to_string());

        // Act
        let endpoint = get_complete_endpoint(&base_endpoint, &path);

        // Assert
        assert_eq!("http://example.com/path", endpoint);
    }

    #[test]
    fn test_stringify() {
        // Arrange
        let mut query_params = Vec::new();
        query_params.push((String::from("foo"), String::from("bar")));
        query_params.push((String::from("foo"), String::from("bar")));

        // Act
        let query_params_str = stringify(query_params);

        // Assert
        assert_eq!("foo=bar&foo=bar", query_params_str);
    }

    #[test]
    fn test_get_endpoint_with_qp() {
        // Arrange
        let endpoint = String::from("http://example.com/path");
        let mut query_params = HashMap::new();
        query_params.insert(String::from("foo"), Value::String("bar".to_string()));
        let context = HashMap::new();

        // Act
        let endpoint = get_endpoint_with_qp(endpoint, &query_params, &context);

        // Assert
        assert_eq!("http://example.com/path?foo=bar", endpoint);
    }

    #[test]
    fn test_get_endpoint_with_qp_empty_qp() {
        // Arrange
        let endpoint = String::from("http://example.com/path");
        let query_params = HashMap::new();
        let context = HashMap::new();

        // Act
        let endpoint = get_endpoint_with_qp(endpoint, &query_params, &context);

        // Assert
        assert_eq!("http://example.com/path", endpoint);
    }

    fn get_yaml_string(rust_str: &str) -> Yaml {
        Yaml::String(String::from(rust_str))
    }

    fn create_sample_subcommand(name: &str) -> Yaml {
        let mut scmd_btree = BTreeMap::new();
        let mut scmd_options_btree = BTreeMap::new();

        let name = get_yaml_string(name);
        let about = get_yaml_string("about");
        let about_value = get_yaml_string("This is a sample scmd");

        let path = get_yaml_string("path");
        let path_value = get_yaml_string("path/to/resource");

        scmd_options_btree.insert(about, about_value);
        scmd_options_btree.insert(path, path_value);
        scmd_btree.insert(name, Yaml::Hash(scmd_options_btree));

        Yaml::Hash(scmd_btree)
    }

    fn create_sample_yaml() -> Yaml {
        let mut yaml_btree = BTreeMap::new();

        let name = get_yaml_string("name");
        let name_value = get_yaml_string("test");
        let base_endpoint = get_yaml_string("base_endpoint");
        let base_endpoint_value = get_yaml_string("http://example.com/");
        let subcommands_label = get_yaml_string("subcommands");
        let mut subcommands = Vec::new();
        subcommands.push(create_sample_subcommand("scmd1"));
        subcommands.push(create_sample_subcommand("scmd2"));

        yaml_btree.insert(name, name_value);
        yaml_btree.insert(base_endpoint, base_endpoint_value);
        yaml_btree.insert(subcommands_label, Yaml::Array(subcommands));
        Yaml::Hash(yaml_btree)
    }

    #[test]
    fn test_get_endpoint() {
        // Arrange
        let cmd_name = String::from("scmd2");
        let context = HashMap::new();
        let yaml = create_sample_yaml();
        let query_params = HashMap::new();

        // Act
        let endpoint = get_endpoint(&cmd_name, &context, &yaml, &query_params);

        // Assert
        assert_eq!("http://example.com/path/to/resource", endpoint);
    }
}
