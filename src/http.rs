use reqwest::Response;
use std::collections::HashMap;
use yaml_rust::Yaml;
use std::vec::Vec;
use clap::ArgMatches;

use crate::yaml;
use crate::template;

pub fn get_resource(endpoint: &String, headers: &HashMap<String, String>) -> Response {
    let client = reqwest::Client::new();
    let mut client_get = client.get(endpoint);
    for (name, value) in headers.iter() {
        client_get = client_get.header(&name[..], &value[..]);
    }
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

pub fn get_complete_endpoint(base_endpoint: &Yaml, path_yaml: &Yaml) -> String {
    let mut endpoint = yaml::get_string_from_yaml(base_endpoint);
    let path_str = yaml::get_string_from_yaml(path_yaml);
    endpoint.push_str(&path_str);
    return endpoint;
}

fn get_param_split(param: String) -> (String, String) {
    let mut parts = param.split("=");
    if parts.clone().count() != 2 {
        println!("Params should be formatted as foo=bar");
        ::std::process::exit(1);
    }
    let raw_key = &String::from(parts.next().unwrap());
    let raw_value = &String::from(parts.next().unwrap());
    let key = template::get_compiled_template_str(raw_key);
    let value = template::get_compiled_template_str(raw_value);
    return (key, value);
}

fn stringify(query: Vec<(String, String)>) -> String {
    query.iter().fold(String::new(), |acc, tuple| {
        acc + &tuple.0 + "=" + &tuple.1 + "&"
    })
}

pub fn get_params(args: &ArgMatches) -> String {
    let mut param_vec = Vec::new();
    let params = match args.values_of("param") {
        Some(t) => t,
        None => {
            return String::from("");
        },
    };
    for p in params {
        let param = String::from(p);
        let splitted_param = get_param_split(param);
        param_vec.push(splitted_param);
    }
    return stringify(param_vec);
}