use reqwest::Response;
use reqwest::Method;
use std::collections::HashMap;
use yaml_rust::Yaml;
use std::vec::Vec;
use serde_json::value::Value;

use crate::yaml;
use crate::template;

fn get_complete_endpoint(base_endpoint: &Yaml, path_yaml: &Yaml) -> String {
    let mut endpoint = yaml::get_string_from_yaml(base_endpoint);
    let path_str = yaml::get_string_from_yaml(path_yaml);
    endpoint.push_str(&path_str);
    return endpoint;
}

fn stringify(query: Vec<(String, String)>) -> String {
    query.iter().fold(String::new(), |acc, tuple| {
        acc + &tuple.0 + "=" + &tuple.1 + "&"
    })
}

fn get_endpoint_with_qp(
    endpoint: String,
    query_params: &HashMap<String, String>,
    context: &HashMap<String, HashMap<String, String>>) -> String
{
    if endpoint.contains("?") {
        return endpoint;
    }
    let mut param_vec = Vec::new();
    for (key, value) in query_params {
        if value.is_empty() {
            continue;
        }
        let parsed_qp_value = template::get_compiled_template_str_with_context(&value, &context)
            .expect("Error parsing query param option, check your template");
        param_vec.push((key.clone(), parsed_qp_value));
    }
    if param_vec.is_empty() {
        return endpoint;
    }
    format!("{}?{}", endpoint, stringify(param_vec))
}

pub fn get_endpoint(cmd_name: &str,
    context: &HashMap<String, HashMap<String, String>>,
    yaml: &Yaml,
    query_params: &HashMap<String, String>) -> String
{
    let subcmd_yaml = yaml::get_subcommand_from_yaml(cmd_name, yaml);
    let raw_endpoint = get_complete_endpoint(&yaml["base_endpoint"], &subcmd_yaml["path"]);
    let endpoint_with_qp = get_endpoint_with_qp(raw_endpoint, query_params, context);
    let parsed_endpoint = template::get_compiled_template_str_with_context(&endpoint_with_qp, &context)
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
        headers: &HashMap<String, String>,
        body: &HashMap<String, Value>) -> Response
{
    let client = reqwest::Client::new();
    let reqwest_method = get_method(&method);
    let mut client_get = client.request(reqwest_method, endpoint);
    for (name, value) in headers.iter() {
        client_get = client_get.header(&name[..], &value[..]);
    }
    client_get = client_get.json(&body);

    let response = match client_get.send() {
        Ok(t) => t,
        Err(e) => {
            println!("Could not get response for endpoint {}", endpoint);
            println!("Error: {}", e);
            ::std::process::exit(1);
        }
    };
    return response;
}