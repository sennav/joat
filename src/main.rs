#[macro_use]
extern crate clap;
extern crate reqwest;
extern crate serde_json;
extern crate tera;
extern crate yaml_rust;
extern crate serde;

use clap::App;
use clap::ArgMatches;
use serde_json::value::Value;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::string::String;
use tera::{Context, Tera, Result as TeraResult};
use yaml_rust::Yaml;
use serde::ser::Serialize;
// use std::vec::Vec;

fn get_string_from_yaml(yaml: &Yaml) -> String {
    match yaml.clone().into_string() {
        Some(s) => s,
        None => {
            println!("Failed to convert {:?} into string, exiting.", yaml);
            ::std::process::exit(1);
        },
    }
}

fn get_hash_from_yaml(yaml: &Yaml) -> HashMap<String, String> {
    let yaml_btree = match yaml.clone().into_hash() {
        Some(t) => t,
        None => {
            println!("Failed to convert to hash map, exiting.");
            ::std::process::exit(1);
        }
    };
    let mut yaml_hash = HashMap::new();
    for (key, value) in yaml_btree.iter() {
        let str_key = get_string_from_yaml(key);
        let parsed_value = get_compiled_template_str_from_yaml(value);
        yaml_hash.insert(str_key, parsed_value);
    };
    return yaml_hash;
}

fn get_subcommand_from_yaml(cmd_name: &str, yaml: &Yaml) -> Yaml {
    let subcommands = &yaml["subcommands"];
    let subcommands_vec = match subcommands.clone().into_vec() {
        Some(t) => t,
        None => {
            println!("Failed to retrieve subcommands, exiting.");
            ::std::process::exit(1);
        }
    };
    let cmd_name_yaml = Yaml::from_str(cmd_name);
    let scmd_option = subcommands_vec.iter().find(|&s| {
        match s.clone().into_hash() {
            Some(sl) => sl.contains_key(&cmd_name_yaml),
            None => false
        }
    });
    let scmd_hash = match scmd_option {
        Some(s) => s,
        None => {
            println!("Failed to retrieve subcommands hash, exiting.");
            ::std::process::exit(1);   
        }
    };
    return scmd_hash[cmd_name].clone();
}

fn get_compiled_template_str_from_yaml(template_yaml_str: &Yaml) -> String {
    let template_str = match template_yaml_str.clone().into_string() {
        Some(t) => t,
        None => {
            println!("Failed to convert {:?} to string", template_yaml_str);
            ::std::process::exit(1); 
        },
    };
    return get_compiled_template_str(&template_str);
}

// Custom function based on tera
fn object(value: Option<Value>, params: Vec<Value>) -> TeraResult<bool> {
    Ok(value.unwrap().is_object())
}

fn get_compiled_template_with_context<V: Serialize>(template: &String, context_hashes: &HashMap<String, HashMap<String, V>>) -> String {
    let mut context = Context::new();
    for (key, value) in context_hashes.iter() {
        context.insert(&key, &value);
    }
    let mut tera = match Tera::new("./templates/*") {
        Ok(t) => t,
        Err(e) => {
            println!("Could not start tera: {}", e);
            ::std::process::exit(1);
        }
    };
    tera.register_tester("object", object);
    let result = match tera.render(&template, &context) {
        Ok(s) => s,
        Err(e) => {
            println!("Could not render template {:?}", template);
            println!("Error: {}", e);
            ::std::process::exit(1);
        }
    };
    return result;
}

fn get_compiled_template_str(template: &String) -> String {
    let mut context = Context::new();
    let mut env_vars = HashMap::new();
    for (key, value) in env::vars() {
        env_vars.insert(key, value);
    }
    context.insert("env", &env_vars);

    let result = match Tera::one_off(&template, &context, false) {
        Ok(s) => s,
        Err(e) => {
            println!("Could not compile template {:?}", template);
            println!("Error: {}", e);
            ::std::process::exit(1);
        }
    };
    return result;
}

fn get_env_hash() -> HashMap<String, Value> {
    let mut env_vars = HashMap::new();
    for (key, value) in env::vars() {
        let v: Value = value.into();
        env_vars.insert(key, v);
    }
    return env_vars;
}

fn get_resource(endpoint: &String, headers: &HashMap<String, String>) -> Result<HashMap<String, Value>, reqwest::Error> {
    let client = reqwest::Client::new();
    println!("Endpoint {:?}", endpoint);
    let mut client_get = client.get(endpoint);
    for (name, value) in headers.iter() {
        client_get = client_get.header(&name[..], &value[..]);
    }
    let mut result = client_get.send()?;
    println!("RESULT {:?}", result);
    return result.json();
}

fn get_complete_endpoint(base_endpoint: &Yaml, path_yaml: &Yaml) -> String {
    let mut endpoint = get_string_from_yaml(base_endpoint);
    let path_str = get_string_from_yaml(path_yaml);
    endpoint.push_str(&path_str);
    return endpoint;
}

fn execute(cmd_name: &str, args: &ArgMatches, yaml: &Yaml) {
    let subcmd_yaml = get_subcommand_from_yaml(cmd_name, yaml);
    let raw_endpoint = get_complete_endpoint(&yaml["base_endpoint"], &subcmd_yaml["path"]);
    let parsed_endpoint = get_compiled_template_str(&raw_endpoint);
    let headers = get_hash_from_yaml(&yaml["headers"]);
    let result = match get_resource(&parsed_endpoint, &headers) {
        Ok(t) => t,
        Err(e) => {
            println!("Fail to get_resource {:?}", e);
            ::std::process::exit(1);
        },
    };
    let mut api_results_context = HashMap::new();
    api_results_context.insert(String::from("api_results"), result);
    api_results_context.insert(String::from("env"), get_env_hash());
    // let template = fs::read_to_string("debug")
    //     .expect("Something went wrong reading the file");
    print!("{}", get_compiled_template_with_context(&String::from("debug"), &api_results_context));
}

fn main() {
    
    let yaml = load_yaml!("../cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    // let config = matches.value_of("config");
    // println!("Value for config: {:?}", config);

    match matches.subcommand() {
        (name, sub_cmd_option) => {
            match sub_cmd_option {
                Some(sub_cmd) => execute(name, sub_cmd, yaml),
                _ => ::std::process::exit(1)
            }
        }
    }

    // // Calling .unwrap() is safe here because "INPUT" is required (if "INPUT" wasn't
    // // required we could have used an 'if let' to conditionally get the value)

    // // Vary the output based on how many times the user used the "verbose" flag
    // // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    // match matches.occurrences_of("v") {
    //     0 => println!("No verbose info"),
    //     1 => println!("Some verbose info"),
    //     2 => println!("Tons of verbose info"),
    //     3 | _ => println!("Don't be crazy"),
    // }

    // // You can handle information about subcommands by requesting their matches by name
    // // (as below), requesting just the name used, or both at the same time
    // if let Some(matches) = matches.subcommand_matches("test") {
    //     if matches.is_present("debug") {
    //         println!("Printing debug info...");
    //     } else {
    //         println!("Printing normally...");
    //     }
    // }
    // println!("Yaml");
    // println!("{:?}", yaml);
    // println!("{:?}", yaml["subcommands"][0]["test"]["execute"]);

    // // more program logic goes here...
}